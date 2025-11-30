//! Planning Engine Module
//!
//! Generates high-level execution plans based on:
//! - Query intent (Symbolic, Semantic, Causal)
//! - Recommended reasoning patterns from historical successes
//! - Self-consistency evaluation results
//! - Context bundle (RAGGraph entities, LTMC memory)
//!
//! Plans are minimal (3-8 steps) and use real SynCore tools.
//! No chain-of-thought expansion or agent overreach.

use super::context_bundle::ContextBundle;
use super::intent_classifier::QueryIntent;
use super::pattern_engine::{PatternGraphUsage, ReasoningPattern};
use super::self_consistency::{SelfConsistencyIssueKind, SelfConsistencyResult};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A single step in an execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Tool name (e.g., "code_index", "code_search", "memory_query")
    pub tool: String,
    /// Tool arguments as JSON
    pub args: serde_json::Value,
}

/// An execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Sequence of steps to execute (3-8 steps max)
    pub steps: Vec<PlanStep>,
    /// Optional notes about the plan
    pub notes: Option<String>,
}

impl Plan {
    /// Create empty plan
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            notes: None,
        }
    }

    /// Add a step to the plan
    pub fn add_step(&mut self, tool: String, args: serde_json::Value) {
        self.steps.push(PlanStep { tool, args });
    }
}

impl Default for Plan {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate an execution plan based on cognitive context
///
/// # Arguments
/// * `query` - User's input query
/// * `intent` - Classified intent (Symbolic, Semantic, Causal, Unknown)
/// * `selected_mode` - Routing mode ("simple", "attention", "reasoning")
/// * `recommended_patterns` - Historical patterns with success rates
/// * `consistency` - Self-consistency evaluation result
/// * `bundle` - Context bundle with RAGGraph entities and LTMC memory
///
/// # Returns
/// A minimal execution plan (3-8 steps) using real SynCore tools
pub fn generate_plan(
    query: &str,
    intent: &QueryIntent,
    selected_mode: &str,
    recommended_patterns: &[ReasoningPattern],
    consistency: &SelfConsistencyResult,
    bundle: &ContextBundle,
) -> Result<Plan> {
    let mut plan = Plan::new();

    // Step 1: Find best pattern for this intent + mode
    let best_pattern = find_best_pattern(intent, selected_mode, recommended_patterns);

    // Step 2: Check for consistency issues that should be avoided
    let has_repeated_failures = consistency
        .issues
        .iter()
        .any(|i| matches!(i.kind, SelfConsistencyIssueKind::RepeatedFailedSequence));

    // Step 3: Decide on base strategy
    let strategy = determine_strategy(intent, selected_mode, bundle);

    // Step 4: Build plan based on strategy
    match strategy {
        PlanStrategy::IndexAndSearch => {
            build_index_search_plan(&mut plan, query, bundle, best_pattern);
        }
        PlanStrategy::GraphTraversal => {
            build_graph_traversal_plan(&mut plan, query, bundle, best_pattern);
        }
        PlanStrategy::MemoryRetrieval => {
            build_memory_retrieval_plan(&mut plan, query, best_pattern);
        }
        PlanStrategy::Symbolic => {
            build_symbolic_plan(&mut plan, query);
        }
    }

    // Step 5: If consistency flags issues, add fallback or modify plan
    if has_repeated_failures && consistency.suggested_plan.is_some() {
        if let Some(suggested) = &consistency.suggested_plan {
            plan.notes = Some(format!(
                "Note: Avoiding repeated failures. Suggested: {:?}",
                suggested.recommended_tool_sequence
            ));
        }
    }

    // Step 6: Ensure plan is within bounds (3-8 steps)
    limit_plan_size(&mut plan);

    Ok(plan)
}

/// Strategy for plan generation
#[derive(Debug, Clone, PartialEq)]
enum PlanStrategy {
    IndexAndSearch,  // For semantic queries with code entities
    GraphTraversal,  // For complex causal/reasoning queries
    MemoryRetrieval, // For queries about stored knowledge
    Symbolic,        // For simple symbolic/string operations
}

/// Determine best strategy based on intent, mode, and context
fn determine_strategy(
    intent: &QueryIntent,
    selected_mode: &str,
    bundle: &ContextBundle,
) -> PlanStrategy {
    // If we have RAGGraph entities, use graph-based approaches
    let has_entities = !bundle.raggraph_entities.is_empty();

    match intent {
        QueryIntent::Symbolic => PlanStrategy::Symbolic,
        QueryIntent::Semantic if has_entities => {
            if selected_mode == "reasoning" {
                PlanStrategy::GraphTraversal
            } else {
                PlanStrategy::IndexAndSearch
            }
        }
        QueryIntent::Semantic => PlanStrategy::IndexAndSearch,
        QueryIntent::Causal => PlanStrategy::GraphTraversal,
        QueryIntent::Unknown => PlanStrategy::MemoryRetrieval,
    }
}

/// Find best pattern matching intent and mode
fn find_best_pattern<'a>(
    intent: &QueryIntent,
    selected_mode: &str,
    patterns: &'a [ReasoningPattern],
) -> Option<&'a ReasoningPattern> {
    patterns
        .iter()
        .filter(|p| {
            std::mem::discriminant(&p.intent_type) == std::mem::discriminant(intent)
                && p.selected_mode == selected_mode
        })
        .max_by(|a, b| {
            a.success_rate
                .partial_cmp(&b.success_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Build index + search plan
fn build_index_search_plan(
    plan: &mut Plan,
    query: &str,
    bundle: &ContextBundle,
    pattern: Option<&ReasoningPattern>,
) {
    // If pattern exists and has high success rate, use it
    if let Some(p) = pattern {
        if p.success_rate > 0.8 {
            for tool in &p.tool_sequence {
                plan.add_step(tool.clone(), serde_json::json!({"query": query}));
            }
            return;
        }
    }

    // Default: code_index + code_search
    if !bundle.raggraph_entities.is_empty() {
        // Use entities from bundle
        for entity in bundle.raggraph_entities.iter().take(2) {
            plan.add_step(
                "code_index".to_string(),
                serde_json::json!({"file_path": entity.file_path}),
            );
        }
    }

    plan.add_step(
        "code_search".to_string(),
        serde_json::json!({"query": query, "limit": 10}),
    );
}

/// Build graph traversal plan
fn build_graph_traversal_plan(
    plan: &mut Plan,
    query: &str,
    bundle: &ContextBundle,
    pattern: Option<&ReasoningPattern>,
) {
    // If pattern exists with graph usage, follow it
    if let Some(p) = pattern {
        if !matches!(p.graph_usage, PatternGraphUsage::None) && p.success_rate > 0.7 {
            for tool in &p.tool_sequence {
                plan.add_step(tool.clone(), serde_json::json!({"query": query}));
            }
            return;
        }
    }

    // Default: code_graph_fusion_query
    let mode_hint = if matches!(
        pattern.map(|p| &p.graph_usage),
        Some(PatternGraphUsage::Heavy)
    ) {
        "reasoning"
    } else {
        "attention"
    };

    plan.add_step(
        "code_graph_fusion_query".to_string(),
        serde_json::json!({
            "query": query,
            "mode_hint": mode_hint,
            "top_k": 10
        }),
    );

    // Add multi-hop if entities present
    if !bundle.raggraph_entities.is_empty() {
        let entity_ids: Vec<i64> = bundle
            .raggraph_entities
            .iter()
            .filter_map(|e| e.entity_id)
            .take(3)
            .collect();

        plan.add_step(
            "raggraph_multihop".to_string(),
            serde_json::json!({"seed_nodes": entity_ids}),
        );
    }
}

/// Build memory retrieval plan
fn build_memory_retrieval_plan(plan: &mut Plan, query: &str, pattern: Option<&ReasoningPattern>) {
    // If pattern exists, use it
    if let Some(p) = pattern {
        if p.success_rate > 0.8 {
            for tool in &p.tool_sequence {
                plan.add_step(tool.clone(), serde_json::json!({"query": query}));
            }
            return;
        }
    }

    // Default: vector_search + memory_query
    plan.add_step(
        "vector_search".to_string(),
        serde_json::json!({"query": query, "limit": 5}),
    );

    // Extract key terms for memory query
    let key = query.split_whitespace().next().unwrap_or("query");
    plan.add_step("memory_query".to_string(), serde_json::json!({"key": key}));
}

/// Build symbolic plan (simple string operations)
fn build_symbolic_plan(plan: &mut Plan, query: &str) {
    // Symbolic queries typically need parser analysis
    plan.add_step(
        "parser_search".to_string(),
        serde_json::json!({"pattern": query}),
    );
}

/// Limit plan size to 3-8 steps
fn limit_plan_size(plan: &mut Plan) {
    const MAX_STEPS: usize = 8;

    if plan.steps.is_empty() {
        // Add default fallback step
        plan.add_step(
            "memory_query".to_string(),
            serde_json::json!({"key": "default"}),
        );
    }

    if plan.steps.len() > MAX_STEPS {
        plan.steps.truncate(MAX_STEPS);
        if plan.notes.is_none() {
            plan.notes = Some("Plan truncated to 8 steps".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::pattern_engine::ReasoningPatternId;

    #[test]
    fn test_plan_creation() {
        let mut plan = Plan::new();
        plan.add_step(
            "code_index".to_string(),
            serde_json::json!({"file_path": "test.rs"}),
        );

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].tool, "code_index");
    }

    #[test]
    fn test_determine_strategy_symbolic() {
        let intent = QueryIntent::Symbolic;
        let bundle = ContextBundle::new();
        let strategy = determine_strategy(&intent, "simple", &bundle);

        assert_eq!(strategy, PlanStrategy::Symbolic);
    }

    #[test]
    fn test_determine_strategy_semantic_with_entities() {
        let intent = QueryIntent::Semantic;
        let mut bundle = ContextBundle::new();
        bundle.add_raggraph_entity(crate::cognition::context_bundle::CodeEntityWithScore {
            entity_id: Some(1),
            file_path: "test.rs".to_string(),
            entity_type: "function".to_string(),
            name: "test_fn".to_string(),
            signature: None,
            score: 0.9,
            rank: 1,
        });

        let strategy = determine_strategy(&intent, "simple", &bundle);
        assert_eq!(strategy, PlanStrategy::IndexAndSearch);
    }
}
