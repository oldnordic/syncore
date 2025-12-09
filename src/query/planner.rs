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
use crate::config::{get_project_label, get_project_root, ProjectContext};
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

impl QueryConstraints {
    /// Create QueryConstraints with precedence-based project context resolution
    ///
    /// Precedence order:
    /// 1. User-provided explicit parameters (highest priority)
    /// 2. Environment variables (for expert override)
    /// 3. Automatic project detection from current directory
    /// 4. None values (no project filtering)
    pub fn with_project_context(
        scope: Option<String>,
        max_results: Option<usize>,
        project_label: Option<String>,
        local_root: Option<String>,
        graph_required: Option<bool>,
        allow_hopgraph: Option<bool>,
        allow_raggraph: Option<bool>,
        allow_vector: Option<bool>,
    ) -> Self {
        // Resolve project_label using precedence logic
        let resolved_project_label = if project_label.is_some() {
            // 1. User-provided explicit parameter
            project_label
        } else {
            // 2. Automatic detection (env is handled inside get_project_label)
            get_project_label(None)
        };

        // Resolve local_root using precedence logic
        let resolved_local_root = if local_root.is_some() {
            // 1. User-provided explicit parameter
            local_root
        } else {
            // 2. Automatic detection from current directory
            get_project_root().map(|root| root.to_string_lossy().to_string())
        };

        // Handle 'auto' scope resolution
        let resolved_scope = if let Some(scope) = scope {
            if scope == "auto" {
                // Auto-scope: prefer project-scoped when we have project context
                if resolved_project_label.is_some() {
                    "project".to_string()
                } else {
                    "global".to_string()
                }
            } else {
                scope
            }
        } else {
            // Default scope for backward compatibility
            "project".to_string()
        };

        Self {
            scope: resolved_scope,
            max_results: max_results.or(Some(10)), // Default limit
            graph_required: graph_required.unwrap_or(false),
            allow_hopgraph: allow_hopgraph.unwrap_or(true),
            allow_raggraph: allow_raggraph.unwrap_or(true),
            allow_vector: allow_vector.unwrap_or(true),
            project_label: resolved_project_label,
            local_root: resolved_local_root,
        }
    }

    /// Create QueryConstraints from user-facing MCP tool parameters
    /// This is the main entry point for MCP tools that accept project context parameters
    pub fn from_mcp_params(
        query: &str,
        scope: Option<String>,
        top_k: Option<usize>,
        project_label: Option<String>,
        local_root: Option<String>,
    ) -> Self {
        // Use the precedence-based constructor for MCP parameters
        Self::with_project_context(
            scope,
            top_k,
            project_label,
            local_root,
            None, // graph_required - use default
            None, // allow_hopgraph - use default
            None, // allow_raggraph - use default
            None, // allow_vector - use default
        )
    }

    /// Get the effective namespace for this query
    /// Derives namespace from project_label for consistency
    pub fn get_effective_namespace(&self) -> Option<String> {
        self.project_label.clone()
    }

    /// Check if this query has project-scoped constraints
    pub fn is_project_scoped(&self) -> bool {
        self.project_label.is_some() || matches!(self.scope.as_str(), "project" | "file")
    }

    /// Get the effective local root for file-scoped queries
    pub fn get_effective_local_root(&self) -> Option<String> {
        if self.scope == "file" && self.local_root.is_none() {
            // For file scope without explicit local_root, try current directory
            get_project_root().map(|root| root.to_string_lossy().to_string())
        } else {
            self.local_root.clone()
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

    // ========== PROJECT ISOLATION TESTS ==========

    #[test]
    fn test_query_constraints_precedence_user_provided() {
        // Test that user-provided parameters take highest precedence
        let constraints = QueryConstraints::with_project_context(
            Some("project".to_string()),
            Some(20),
            Some("my-custom-project".to_string()),
            Some("/custom/path".to_string()),
            Some(true),
            Some(false),
            Some(true),
            Some(false),
        );

        assert_eq!(constraints.scope, "project");
        assert_eq!(constraints.max_results, Some(20));
        assert_eq!(constraints.project_label, Some("my-custom-project".to_string()));
        assert_eq!(constraints.local_root, Some("/custom/path".to_string()));
        assert_eq!(constraints.graph_required, true);
        assert_eq!(constraints.allow_hopgraph, false);
        assert_eq!(constraints.allow_raggraph, true);
        assert_eq!(constraints.allow_vector, false);
    }

    #[test]
    fn test_query_constraints_precedence_auto_detection() {
        // Test that when no user parameters are provided, auto-detection kicks in
        let constraints = QueryConstraints::with_project_context(
            None, // scope - should default to "project"
            None, // max_results - should default to Some(10)
            None, // project_label - should auto-detect
            None, // local_root - should auto-detect
            None, // graph_required - should default to false
            None, // allow_hopgraph - should default to true
            None, // allow_raggraph - should default to true
            None, // allow_vector - should default to true
        );

        assert_eq!(constraints.scope, "project");
        assert_eq!(constraints.max_results, Some(10));
        assert_eq!(constraints.graph_required, false);
        assert_eq!(constraints.allow_hopgraph, true);
        assert_eq!(constraints.allow_raggraph, true);
        assert_eq!(constraints.allow_vector, true);

        // Project detection should find "syncore" since we're running from within the syncore project
        assert_eq!(constraints.project_label, Some("syncore".to_string()));

        // Local root should be detected and be a valid path
        assert!(constraints.local_root.is_some());
        let local_root = constraints.local_root.unwrap();
        assert!(!local_root.is_empty());
    }

    #[test]
    fn test_query_constraints_auto_scope_resolution() {
        // Test 'auto' scope resolution
        let constraints_with_project = QueryConstraints::with_project_context(
            Some("auto".to_string()),
            None,
            Some("test-project".to_string()),
            None,
            None,
            None,
            None,
            None,
        );

        // Auto should resolve to "project" when project label is available
        assert_eq!(constraints_with_project.scope, "project");

        let constraints_without_project = QueryConstraints::with_project_context(
            Some("auto".to_string()),
            None,
            None, // No project label
            None,
            None,
            None,
            None,
            None,
        );

        // Auto should resolve to "global" when no project label is available
        assert_eq!(constraints_without_project.scope, "global");
    }

    #[test]
    fn test_query_constraints_from_mcp_params() {
        // Test the MCP parameter constructor
        let constraints = QueryConstraints::from_mcp_params(
            "test query",
            Some("project".to_string()),
            Some(15),
            Some("user-project".to_string()),
            Some("/user/path".to_string()),
        );

        assert_eq!(constraints.scope, "project");
        assert_eq!(constraints.max_results, Some(15));
        assert_eq!(constraints.project_label, Some("user-project".to_string()));
        assert_eq!(constraints.local_root, Some("/user/path".to_string()));
        assert_eq!(constraints.graph_required, false); // Should use defaults
        assert_eq!(constraints.allow_hopgraph, true);  // Should use defaults
        assert_eq!(constraints.allow_raggraph, true); // Should use defaults
        assert_eq!(constraints.allow_vector, true);   // Should use defaults
    }

    #[test]
    fn test_query_constraints_effective_namespace() {
        // Test effective namespace derivation
        let constraints_with_project = QueryConstraints {
            project_label: Some("test-project".to_string()),
            ..Default::default()
        };

        assert_eq!(
            constraints_with_project.get_effective_namespace(),
            Some("test-project".to_string())
        );

        let constraints_without_project = QueryConstraints::default();
        assert_eq!(constraints_without_project.get_effective_namespace(), None);
    }

    #[test]
    fn test_query_constraints_is_project_scoped() {
        // Test project scoping detection
        let constraints_project = QueryConstraints {
            scope: "project".to_string(),
            project_label: Some("test".to_string()),
            ..Default::default()
        };
        assert!(constraints_project.is_project_scoped());

        let constraints_file = QueryConstraints {
            scope: "file".to_string(),
            ..Default::default()
        };
        assert!(constraints_file.is_project_scoped());

        let constraints_global = QueryConstraints {
            scope: "global".to_string(),
            project_label: None,
            ..Default::default()
        };
        assert!(!constraints_global.is_project_scoped());
    }

    #[test]
    fn test_query_constraints_effective_local_root() {
        // Test effective local root for file scope
        let constraints_file_with_root = QueryConstraints {
            scope: "file".to_string(),
            local_root: Some("/explicit/root".to_string()),
            ..Default::default()
        };
        assert_eq!(
            constraints_file_with_root.get_effective_local_root(),
            Some("/explicit/root".to_string())
        );

        let constraints_file_without_root = QueryConstraints {
            scope: "file".to_string(),
            local_root: None,
            ..Default::default()
        };
        // Should auto-detect current directory
        assert!(constraints_file_without_root.get_effective_local_root().is_some());

        let constraints_project_scope = QueryConstraints {
            scope: "project".to_string(),
            local_root: None,
            ..Default::default()
        };
        // Should return None for non-file scopes without explicit root
        assert_eq!(constraints_project_scope.get_effective_local_root(), None);
    }
}
