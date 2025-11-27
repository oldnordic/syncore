//! Cognitive Orchestrator Module
//!
//! Integrates intent classification and router logic into query processing.
//! Automatically enriches queries with RAGGraph results before LLM execution.

use super::context_bundle::ContextBundle;
use super::context_composer::ContextComposer;
use super::continuity_engine::{
    build_reasoning_continuity, decide_continuity_route, ReasoningContinuity,
};
use super::intent_classifier::{classify_intent, QueryIntent};
use super::pattern_engine::{recommend_patterns_for_query, ReasoningPattern};
use super::plan_engine::{generate_plan, Plan};
use super::plan_executor::{ExecutionResult};
use super::reasoning_ledger::fetch_recent_episodes_sql;
use super::router_logic::{route_query, RoutingDecision};
use super::self_consistency::{evaluate_self_consistency, SelfConsistencyResult};
use crate::code_graph::CodeGraph;
use crate::graph::Neo4jClient;
use crate::memory::Memory;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Enriched context for LLM prompts
///
/// Contains RAGGraph results and metadata to provide
/// better context for worker model execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedContext {
    /// Original query
    pub query: String,
    /// Classified intent
    pub intent: QueryIntent,
    /// Routing decision
    pub decision: RoutingDecision,
    /// Selected fusion mode
    pub selected_mode: Option<String>,
    /// RAGGraph results (JSON serialized)
    pub raggraph_results: Option<String>,
    /// Whether RAGGraph was actually invoked
    pub raggraph_invoked: bool,
    /// Phase R3.2: Unified context bundle with all memory systems
    pub context_bundle: Option<ContextBundle>,
    /// Phase R3.3: Reasoning continuity from historical episodes
    pub reasoning_continuity: Option<ReasoningContinuity>,
    /// Phase R4.1: Recommended reasoning patterns (passive hints)
    pub recommended_patterns: Option<Vec<ReasoningPattern>>,
    /// Phase R4.2: Self-consistency check result
    pub self_consistency: Option<SelfConsistencyResult>,
    /// Phase R5.0: Generated execution plan
    pub plan: Option<Plan>,
    /// Phase R5.0: Plan execution result
    pub execution_result: Option<ExecutionResult>,
    /// Debug information
    pub debug_info: String,
}

/// Enrich a query with RAGGraph results
///
/// This is the main orchestration function that:
/// 1. Classifies query intent
/// 2. Routes to appropriate fusion mode
/// 3. Calls RAGGraph if needed
/// 4. Produces enriched context for LLM
///
/// # Arguments
/// * `query` - User's input query
/// * `code_graph` - CodeGraph instance
/// * `neo4j` - Neo4j client
///
/// # Returns
/// EnrichedContext with RAGGraph results (if applicable)
pub async fn enrich_query_with_raggraph(
    query: &str,
    code_graph: &CodeGraph,
    neo4j: &Neo4jClient,
) -> Result<EnrichedContext> {
    // Step 1: Classify intent
    let intent = classify_intent(query);

    // Step 2: Route query
    let decision = route_query(&intent, query);

    // Step 3: Initialize enriched context
    let mut enriched = EnrichedContext {
        query: query.to_string(),
        intent,
        decision: decision.clone(),
        selected_mode: decision.mode_hint.clone(),
        raggraph_results: None,
        raggraph_invoked: false,
        context_bundle: None,
        reasoning_continuity: None,
        recommended_patterns: None,
        self_consistency: None,
        plan: None,
        execution_result: None,
        debug_info: format!(
            "Intent: {:?}, ShouldCallRAGGraph: {}",
            intent, decision.should_call_raggraph
        ),
    };

    // Step 4: Call RAGGraph if decision says so
    if decision.should_call_raggraph {
        match call_raggraph(query, code_graph, neo4j, &decision).await {
            Ok(results_json) => {
                enriched.raggraph_results = Some(results_json);
                enriched.raggraph_invoked = true;
                enriched.debug_info.push_str(" | RAGGraph: SUCCESS");
            }
            Err(e) => {
                enriched
                    .debug_info
                    .push_str(&format!(" | RAGGraph: FAILED ({})", e));
            }
        }
    }

    Ok(enriched)
}

/// Call RAGGraph with appropriate parameters
///
/// Note: This function needs to construct a RagGraphAPI which requires ownership.
/// For the orchestrator use case, we'll need to refactor to work with borrowed data
/// or find an alternative approach.
async fn call_raggraph(
    query: &str,
    _code_graph: &CodeGraph,
    _neo4j: &Neo4jClient,
    _decision: &RoutingDecision,
) -> Result<String> {
    // TODO: Implement actual RAGGraph call
    // For now, return a placeholder to allow tests to compile
    // This will be integrated into the MCP server layer where ownership is available

    let placeholder = serde_json::json!({
        "entities": [],
        "selected_mode": "simple",
        "debug_info": format!("Placeholder for query: {}", query)
    });

    Ok(serde_json::to_string_pretty(&placeholder)?)
}

/// Format enriched context into LLM prompt augmentation
///
/// Produces a formatted string that can be prepended to LLM input.
pub fn format_enriched_context_for_llm(enriched: &EnrichedContext) -> String {
    let mut prompt = String::new();

    prompt.push_str("=== CODE CONTEXT ===\n");
    prompt.push_str(&format!("Query Intent: {:?}\n", enriched.intent));
    prompt.push_str(&format!("Fusion Mode: {:?}\n", enriched.selected_mode));
    prompt.push_str(&format!("Reasoning: {}\n", enriched.decision.reasoning));

    if let Some(ref results) = enriched.raggraph_results {
        prompt.push_str("\n=== RAGGraph Results ===\n");
        prompt.push_str(results);
        prompt.push('\n');
    }

    prompt.push_str("=== END CONTEXT ===\n\n");
    prompt.push_str("Original Query: ");
    prompt.push_str(&enriched.query);
    prompt.push_str("\n\n");

    prompt
}

/// Phase R3.2: Enrich query with full ContextBundle
///
/// This function extends R3.1's RAGGraph enrichment with full LTMC integration.
/// It composes a unified ContextBundle merging:
/// - RAGGraph results
/// - LTMC vector memory
/// - LTMC SQL memory
/// - LTMC graph memory (Neo4j)
/// - LTMC cache memory
///
/// # Arguments
/// * `query` - User's input query
/// * `code_graph` - CodeGraph instance
/// * `memory` - LTMC Memory instance
/// * `neo4j` - Optional Neo4j client
///
/// # Returns
/// EnrichedContext with full ContextBundle
pub async fn enrich_query_with_context_bundle(
    query: &str,
    code_graph: &CodeGraph,
    memory: &Memory,
    neo4j: Option<&Neo4jClient>,
) -> Result<EnrichedContext> {
    // Step 1: Classify intent
    let intent = classify_intent(query);

    // Step 2: Route query
    let decision = route_query(&intent, query);

    // Step 3: Create ContextComposer
    let composer = ContextComposer::new();

    // Step 4: Compose unified ContextBundle
    let context_bundle = composer
        .compose(query, &decision, code_graph, memory, neo4j)
        .await?;

    // Step 5: Build reasoning continuity (Phase R3.3)
    // Decide continuity route based on intent and context
    let continuity_route = decide_continuity_route(&intent, &context_bundle);

    // Extract entity IDs from context bundle
    let mut entity_ids = Vec::new();
    for entity in &context_bundle.raggraph_entities {
        if let Some(id) = entity.entity_id {
            entity_ids.push(format!("entity_{}", id));
        }
    }

    // Build continuity (best-effort, don't fail query if this errors)
    let reasoning_continuity = build_reasoning_continuity(
        query,
        &entity_ids,
        &continuity_route,
        memory,
        neo4j,
        10, // max 10 historical episodes
    )
    .await
    .ok(); // Convert Result to Option, ignore errors

    // Step 5b: Recommend patterns (Phase R4.1 - passive hints only)
    let recommended_patterns = if let Some(ref mode) = decision.mode_hint {
        recommend_patterns_for_query(&intent, mode, memory, "default", 5).ok() // Best-effort, ignore errors
    } else {
        None
    };

    // Step 5c: Self-consistency check (Phase R4.2 - advanced cognitive constraint)
    let self_consistency = if let (Some(ref bundle), Some(ref mode), Some(ref cont)) = (
        &Some(context_bundle.clone()),
        &decision.mode_hint,
        &reasoning_continuity,
    ) {
        // Fetch recent episodes for consistency check
        let episodes = fetch_recent_episodes_sql(memory, query, 10).unwrap_or_default();

        // Assume planned tools from context (use empty vec if not available)
        let planned_tools = Vec::new(); // Will be populated by actual planner in future

        // Evaluate self-consistency
        let result = evaluate_self_consistency(
            query,
            &intent,
            mode,
            &planned_tools,
            bundle,
            cont,
            recommended_patterns.as_deref().unwrap_or(&[]),
            &episodes,
        );

        Some(result)
    } else {
        None
    };

    // Step 6: Generate execution plan (Phase R5.0)
    let plan = if let (Some(ref bundle), Some(ref mode), Some(ref consistency)) = (
        &Some(context_bundle.clone()),
        &decision.mode_hint,
        &self_consistency,
    ) {
        generate_plan(
            query,
            &intent,
            mode,
            recommended_patterns.as_deref().unwrap_or(&[]),
            consistency,
            bundle,
        )
        .ok()
    } else {
        None
    };

    // Step 7: Build EnrichedContext
    let enriched = EnrichedContext {
        query: query.to_string(),
        intent,
        decision: decision.clone(),
        selected_mode: Some(context_bundle.fusion_mode.clone()),
        raggraph_results: None, // Deprecated in favor of context_bundle
        raggraph_invoked: decision.should_call_raggraph,
        context_bundle: Some(context_bundle),
        reasoning_continuity,
        recommended_patterns,
        self_consistency,
        plan,
        execution_result: None, // Execution happens externally if needed
        debug_info: format!(
            "Intent: {:?}, ContextBundle created with mode: {}",
            intent,
            decision.mode_hint.as_deref().unwrap_or("auto")
        ),
    };

    Ok(enriched)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enriched_context_creation() {
        let intent = QueryIntent::Symbolic;
        let decision = route_query(&intent, "test");

        let enriched = EnrichedContext {
            query: "test".to_string(),
            intent,
            decision,
            selected_mode: Some("simple".to_string()),
            raggraph_results: None,
            raggraph_invoked: false,
            context_bundle: None,
            reasoning_continuity: None,
            recommended_patterns: None,
            self_consistency: None,
            plan: None,
            execution_result: None,
            debug_info: "test".to_string(),
        };

        assert_eq!(enriched.query, "test");
        assert_eq!(enriched.intent, QueryIntent::Symbolic);
    }

    #[test]
    fn test_format_enriched_context() {
        let intent = QueryIntent::Semantic;
        let decision = route_query(&intent, "explain parse");

        let enriched = EnrichedContext {
            query: "explain parse".to_string(),
            intent,
            decision,
            selected_mode: Some("attention".to_string()),
            raggraph_results: Some("{\"entities\": []}".to_string()),
            raggraph_invoked: true,
            context_bundle: None,
            reasoning_continuity: None,
            recommended_patterns: None,
            self_consistency: None,
            plan: None,
            execution_result: None,
            debug_info: "test".to_string(),
        };

        let formatted = format_enriched_context_for_llm(&enriched);
        assert!(formatted.contains("CODE CONTEXT"));
        assert!(formatted.contains("Semantic"));
        assert!(formatted.contains("RAGGraph Results"));
    }
}
