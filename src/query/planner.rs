//! Graph-Accelerated Query Planner
//!
//! Provides deterministic planning for HopGraph → RAGGraph → VectorStore → FusionQuery pipeline.
//! The planner decides which steps to execute based on query characteristics and constraints.
//!
//! ## Planner Rules (Explicit, No Magic)
//!
//! 1. **Scope-based rules**:
//!    - If scope == "file" and query < 4 tokens → [VectorRefine, Fusion]
//!    - If scope == "file" and query contains structural hints → [HopGraph, VectorRefine, Fusion]
//!    - If scope == "project" → [HopGraph, VectorRefine, Fusion]
//!    - If scope == "global" → [VectorRefine, Fusion] (no graph restriction needed)
//!
//! 2. **Query characteristic rules**:
//!    - If query contains paths/symbols (::, ->, /) → [HopGraph, VectorRefine, Fusion]
//!    - If query contains causal keywords (trace, dependency, from, to, path, chain) → [HopGraph, RAGGraph, VectorRefine, Fusion]
//!    - If query is semantic-only (why, explain, how, what) → [VectorRefine, Fusion]
//!    - If query is multi-sentence → [VectorRefine, Fusion]
//!
//! 3. **Guardrail rules**:
//!    - If HopGraph returns 0 results AND graph_required → short-circuit to empty
//!    - If HopGraph returns 0 results AND !graph_required → skip to VectorRefine
//!    - If HopGraph returns > 500 results → trim to 500 before next step
//!    - If VectorRefine returns empty → Fusion must not fabricate results
//!
//! 4. **Performance rules**:
//!    - Never call the same step twice unless explicitly encoded
//!    - Respect max_results constraints at each step
//!    - Use deterministic ordering (no map iteration dependence)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Query planner step enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlannerStep {
    /// Restrict domain using HopGraph (graph-based filtering)
    HopGraph,
    /// Expand results using RAGGraph (multi-hop reasoning)
    RAGGraph,
    /// Refine results using vector search
    VectorRefine,
    /// Combine scores from multiple sources
    Fusion,
}

/// Query constraints for planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConstraints {
    /// Query scope: "file" | "project" | "workspace" | "global" | "auto"
    pub scope: String,
    /// Maximum number of results to return
    pub max_results: Option<usize>,
    /// Whether graph steps are required (vs optional)
    pub graph_required: bool,
    /// Whether to allow HopGraph execution
    pub allow_hopgraph: bool,
    /// Whether to allow RAGGraph execution
    pub allow_raggraph: bool,
    /// Whether to allow vector search
    pub allow_vector: bool,
    /// Project label for filtering
    pub project_label: Option<String>,
    /// Local root path for file-level scoping
    pub local_root: Option<String>,
}

impl Default for QueryConstraints {
    fn default() -> Self {
        Self {
            scope: "project".to_string(),
            max_results: Some(10),
            graph_required: false,
            allow_hopgraph: true,
            allow_raggraph: true,
            allow_vector: true,
            project_label: None,
            local_root: None,
        }
    }
}

/// Query plan with explicit step sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    /// Ordered sequence of steps to execute
    pub steps: Vec<PlannerStep>,
    /// Query constraints used for planning
    pub constraints: QueryConstraints,
    /// Planning metadata for debugging
    pub metadata: HashMap<String, String>,
}

/// Graph-Accelerated Query Planner
pub struct QueryPlanner {
    /// Default constraints for planning
    default_constraints: QueryConstraints,
}

impl QueryPlanner {
    /// Create new query planner with default constraints
    pub fn new() -> Self {
        Self {
            default_constraints: QueryConstraints::default(),
        }
    }

    /// Create query planner with custom default constraints
    pub fn with_constraints(default_constraints: QueryConstraints) -> Self {
        Self {
            default_constraints,
        }
    }

    /// Plan a query using default constraints
    ///
    /// # Arguments
    /// * `query` - Query text to analyze
    ///
    /// # Returns
    /// QueryPlan with explicit step sequence
    pub fn plan(&self, query: &str) -> Result<QueryPlan> {
        self.plan_with_constraints(query, self.default_constraints.clone())
    }

    /// Plan a query with explicit constraints
    ///
    /// # Arguments
    /// * `query` - Query text to analyze
    /// * `constraints` - Query constraints for planning
    ///
    /// # Returns
    /// QueryPlan with explicit step sequence and metadata
    pub fn plan_with_constraints(
        &self,
        query: &str,
        constraints: QueryConstraints,
    ) -> Result<QueryPlan> {
        let mut steps = Vec::new();
        let mut metadata = HashMap::new();

        let query_lower = query.to_lowercase();
        let tokens: Vec<&str> = query.split_whitespace().collect();
        let token_count = tokens.len();

        // Record analysis in metadata
        metadata.insert("query_length".to_string(), query.len().to_string());
        metadata.insert("token_count".to_string(), token_count.to_string());
        metadata.insert("scope".to_string(), constraints.scope.clone());

        // Rule 1: Scope-based planning
        match constraints.scope.as_str() {
            "file" => {
                metadata.insert("planning_rule".to_string(), "file_scope".to_string());
                if token_count < 4 && !self.has_structural_hints(query) {
                    // Simple file-scoped query: vector only
                    if constraints.allow_vector {
                        steps.push(PlannerStep::VectorRefine);
                        steps.push(PlannerStep::Fusion);
                        metadata.insert(
                            "rationale".to_string(),
                            "Short file query, no structural hints".to_string(),
                        );
                    }
                } else {
                    // File query with structural hints: graph + vector
                    if constraints.allow_hopgraph {
                        steps.push(PlannerStep::HopGraph);
                    }
                    if constraints.allow_vector {
                        steps.push(PlannerStep::VectorRefine);
                    }
                    steps.push(PlannerStep::Fusion);
                    metadata.insert(
                        "rationale".to_string(),
                        "File query with structural hints".to_string(),
                    );
                }
            }
            "project" => {
                metadata.insert("planning_rule".to_string(), "project_scope".to_string());
                // Project scope: always use graph to restrict domain
                if constraints.allow_hopgraph {
                    steps.push(PlannerStep::HopGraph);
                }
                if constraints.allow_vector {
                    steps.push(PlannerStep::VectorRefine);
                }
                steps.push(PlannerStep::Fusion);
                metadata.insert(
                    "rationale".to_string(),
                    "Project scope requires graph restriction".to_string(),
                );
            }
            "global" => {
                metadata.insert("planning_rule".to_string(), "global_scope".to_string());
                // Global scope: vector search only (no graph restriction needed)
                if constraints.allow_vector {
                    steps.push(PlannerStep::VectorRefine);
                    steps.push(PlannerStep::Fusion);
                    metadata.insert(
                        "rationale".to_string(),
                        "Global scope, no graph restriction needed".to_string(),
                    );
                }
            }
            "workspace" => {
                metadata.insert("planning_rule".to_string(), "workspace_scope".to_string());
                // Workspace scope: similar to project but lighter filtering
                if constraints.allow_hopgraph {
                    steps.push(PlannerStep::HopGraph);
                }
                if constraints.allow_vector {
                    steps.push(PlannerStep::VectorRefine);
                }
                steps.push(PlannerStep::Fusion);
                metadata.insert(
                    "rationale".to_string(),
                    "Workspace scope with light graph filtering".to_string(),
                );
            }
            "auto" => {
                metadata.insert("planning_rule".to_string(), "auto_scope".to_string());
                // Auto scope: use query characteristics to decide
                let auto_steps = self.plan_by_characteristics(query, &constraints)?;
                steps.extend(auto_steps);
                metadata.insert(
                    "rationale".to_string(),
                    "Auto scope based on query characteristics".to_string(),
                );
            }
            _ => {
                // Unknown scope: default to project behavior
                metadata.insert("planning_rule".to_string(), "unknown_scope_fallback".to_string());
                if constraints.allow_hopgraph {
                    steps.push(PlannerStep::HopGraph);
                }
                if constraints.allow_vector {
                    steps.push(PlannerStep::VectorRefine);
                }
                steps.push(PlannerStep::Fusion);
                metadata.insert(
                    "rationale".to_string(),
                    "Unknown scope, defaulting to project behavior".to_string(),
                );
            }
        }

        // Rule 2: Query characteristic overrides
        if self.has_causal_keywords(&query_lower) && constraints.allow_raggraph {
            // Add RAGGraph for causal reasoning if not already present
            if !steps.contains(&PlannerStep::RAGGraph) {
                let hopgraph_pos = steps.iter().position(|s| s == &PlannerStep::HopGraph);
                if let Some(pos) = hopgraph_pos {
                    steps.insert(pos + 1, PlannerStep::RAGGraph);
                } else {
                    steps.insert(0, PlannerStep::HopGraph);
                    steps.insert(1, PlannerStep::RAGGraph);
                }
                metadata.insert(
                    "causal_override".to_string(),
                    "Added RAGGraph for causal reasoning".to_string(),
                );
            }
        }

        // Rule 3: Apply permission constraints
        steps.retain(|step| match step {
            PlannerStep::HopGraph => constraints.allow_hopgraph,
            PlannerStep::RAGGraph => constraints.allow_raggraph,
            PlannerStep::VectorRefine => constraints.allow_vector,
            PlannerStep::Fusion => true, // Fusion is always allowed
        });

        // Rule 4: Ensure plan ends with Fusion (unless empty)
        if !steps.is_empty() && steps.last() != Some(&PlannerStep::Fusion) {
            steps.push(PlannerStep::Fusion);
            metadata.insert("fusion_added".to_string(), "Added Fusion as final step".to_string());
        }

        // Record final plan
        metadata.insert("step_count".to_string(), steps.len().to_string());
        metadata.insert("steps".to_string(), format!("{:?}", steps));

        Ok(QueryPlan {
            steps,
            constraints,
            metadata,
        })
    }

    /// Plan based on query characteristics (used for "auto" scope)
    fn plan_by_characteristics(
        &self,
        query: &str,
        constraints: &QueryConstraints,
    ) -> Result<Vec<PlannerStep>> {
        let query_lower = query.to_lowercase();
        let _tokens: Vec<&str> = query.split_whitespace().collect();
        let mut steps = Vec::new();

        // Check for structural patterns
        if self.has_structural_hints(query) && constraints.allow_hopgraph {
            steps.push(PlannerStep::HopGraph);
        }

        // Check for causal reasoning patterns
        if self.has_causal_keywords(&query_lower) && constraints.allow_raggraph {
            if !steps.contains(&PlannerStep::HopGraph) && constraints.allow_hopgraph {
                steps.push(PlannerStep::HopGraph);
            }
            steps.push(PlannerStep::RAGGraph);
        }

        // Check for semantic patterns
        if self.has_semantic_keywords(&query_lower) && constraints.allow_vector {
            steps.push(PlannerStep::VectorRefine);
        }

        // For structural patterns, always add vector refinement if allowed
        if self.has_structural_hints(query)
            && constraints.allow_vector
            && !steps.contains(&PlannerStep::VectorRefine)
        {
            steps.push(PlannerStep::VectorRefine);
        }

        // Default to vector search if no specific patterns detected
        if steps.is_empty() && constraints.allow_vector {
            steps.push(PlannerStep::VectorRefine);
        }

        Ok(steps)
    }

    /// Check if query contains structural hints (paths, symbols, etc.)
    fn has_structural_hints(&self, query: &str) -> bool {
        query.contains("::") || query.contains("->") || query.contains('/') || query.contains('.')
    }

    /// Check if query contains causal reasoning keywords
    fn has_causal_keywords(&self, query_lower: &str) -> bool {
        let causal_keywords =
            ["trace", "dependency", "from", "to", "path", "chain", "cause", "effect"];
        causal_keywords.iter().any(|kw| query_lower.contains(kw))
    }

    /// Check if query contains semantic/explanation keywords
    fn has_semantic_keywords(&self, query_lower: &str) -> bool {
        let semantic_keywords =
            ["why", "explain", "how", "what", "understand", "describe", "meaning"];
        semantic_keywords.iter().any(|kw| query_lower.contains(kw))
    }
}

impl Default for QueryPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_file_query_plan() {
        let planner = QueryPlanner::new();
        let constraints = QueryConstraints {
            scope: "file".to_string(),
            ..Default::default()
        };

        let plan = planner.plan_with_constraints("fmt", constraints).unwrap();

        // Should be: [VectorRefine, Fusion]
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0], PlannerStep::VectorRefine);
        assert_eq!(plan.steps[1], PlannerStep::Fusion);
        assert_eq!(
            plan.metadata.get("rationale").unwrap(),
            "Short file query, no structural hints"
        );
    }

    #[test]
    fn test_structural_file_query_plan() {
        let planner = QueryPlanner::new();
        let constraints = QueryConstraints {
            scope: "file".to_string(),
            ..Default::default()
        };

        let plan = planner.plan_with_constraints("std::fmt::Display", constraints).unwrap();

        // Should be: [HopGraph, VectorRefine, Fusion]
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[0], PlannerStep::HopGraph);
        assert_eq!(plan.steps[1], PlannerStep::VectorRefine);
        assert_eq!(plan.steps[2], PlannerStep::Fusion);
        assert_eq!(plan.metadata.get("rationale").unwrap(), "File query with structural hints");
    }

    #[test]
    fn test_project_scope_plan() {
        let planner = QueryPlanner::new();
        let constraints = QueryConstraints {
            scope: "project".to_string(),
            ..Default::default()
        };

        let plan = planner.plan_with_constraints("test function", constraints).unwrap();

        // Should be: [HopGraph, VectorRefine, Fusion]
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[0], PlannerStep::HopGraph);
        assert_eq!(plan.steps[1], PlannerStep::VectorRefine);
        assert_eq!(plan.steps[2], PlannerStep::Fusion);
    }

    #[test]
    fn test_global_scope_plan() {
        let planner = QueryPlanner::new();
        let constraints = QueryConstraints {
            scope: "global".to_string(),
            ..Default::default()
        };

        let plan = planner.plan_with_constraints("search query", constraints).unwrap();

        // Should be: [VectorRefine, Fusion]
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0], PlannerStep::VectorRefine);
        assert_eq!(plan.steps[1], PlannerStep::Fusion);
    }

    #[test]
    fn test_causal_query_adds_raggraph() {
        let planner = QueryPlanner::new();
        let constraints = QueryConstraints {
            scope: "project".to_string(),
            allow_raggraph: true,
            ..Default::default()
        };

        let plan =
            planner.plan_with_constraints("trace dependency from A to B", constraints).unwrap();

        // Should include RAGGraph: [HopGraph, RAGGraph, VectorRefine, Fusion]
        assert!(plan.steps.contains(&PlannerStep::RAGGraph));
        assert_eq!(
            plan.metadata.get("causal_override").unwrap(),
            "Added RAGGraph for causal reasoning"
        );
    }

    #[test]
    fn test_permission_constraints_respected() {
        let planner = QueryPlanner::new();
        let constraints = QueryConstraints {
            scope: "project".to_string(),
            allow_hopgraph: false,
            allow_vector: true,
            ..Default::default()
        };

        let plan = planner.plan_with_constraints("test query", constraints).unwrap();

        // Should not include HopGraph: [VectorRefine, Fusion]
        assert!(!plan.steps.contains(&PlannerStep::HopGraph));
        assert!(plan.steps.contains(&PlannerStep::VectorRefine));
        assert!(plan.steps.contains(&PlannerStep::Fusion));
    }

    #[test]
    fn test_auto_scope_planning() {
        let planner = QueryPlanner::new();
        let constraints = QueryConstraints {
            scope: "auto".to_string(),
            ..Default::default()
        };

        let plan = planner.plan_with_constraints("explain why this fails", constraints).unwrap();

        // Should detect semantic pattern: [VectorRefine, Fusion]
        assert!(plan.steps.contains(&PlannerStep::VectorRefine));
        assert!(plan.steps.contains(&PlannerStep::Fusion));
        assert_eq!(plan.metadata.get("planning_rule").unwrap(), "auto_scope");
    }

    #[test]
    fn test_deterministic_planning() {
        let planner = QueryPlanner::new();
        let constraints = QueryConstraints::default();

        let plan1 = planner.plan_with_constraints("test query", constraints.clone()).unwrap();
        let plan2 = planner.plan_with_constraints("test query", constraints).unwrap();

        // Same input should produce same plan
        assert_eq!(plan1.steps, plan2.steps);
        assert_eq!(plan1.metadata, plan2.metadata);
    }
}
