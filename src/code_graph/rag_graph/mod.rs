//! RAGGraph API - Modular Implementation
//!
//! High-level interface combining vector search, graph expansion, and tri-mode fusion.
//! Split into modules for better organization while maintaining identical public API.
//!
//! Module Structure:
//! - mod.rs: Main types, re-exports, and API struct definition
//! - backend.rs: Backend operations and graph interactions
//! - query.rs: Query processing and coordination
//! - multihop.rs: Multi-hop graph reasoning and expansion
//! - fusion_bridge.rs: Fusion mode integration and score combination

use super::fusion_router::FusionRouter;
use super::graph::CodeGraph;
use super::types::QueryScope;
use crate::graph::GraphBackend;
use std::sync::Arc;

// Declare submodules
mod backend;
mod query;
pub mod multihop;
mod fusion_bridge;

// Re-export all the split modules
pub use backend::*;
pub use query::*;
pub use multihop::*;
pub use fusion_bridge::*;

// Re-export fusion types for compatibility
pub use super::fusion_router::FusionMode;

/// Request structure for RAGGraph queries
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RagGraphQueryRequest {
    /// The text query to search for
    pub query: String,
    /// Optional namespace for scoped search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Optional fusion mode hint ("simple", "attention", "reasoning")
    /// If None, router auto-selects based on query characteristics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode_hint: Option<String>,
    /// Maximum number of results to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Query scope: "local" | "project" | "workspace" | "global" | "auto"
    /// Controls search breadth across projects. Default: "project"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Project label for filtering (e.g., "SynCore", "OdinCode")
    /// Required for Project scope, optional for others
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_label: Option<String>,
    /// Local root path for Local scope filtering (e.g., "src/code_graph/")
    /// Only used when scope is "local"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_root: Option<String>,
}

/// Response structure for RAGGraph queries
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RagGraphQueryResponse {
    /// Ranked list of code entities with scores
    pub entities: Vec<RankedEntity>,
    /// Selected fusion mode used for this query
    pub selected_mode: String,
    /// Applied query scope
    pub applied_scope: QueryScope,
    /// Debug information about fusion processing
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub debug_info: std::collections::HashMap<String, String>,
    /// Original query that was processed
    pub query: String,
}

/// Ranked code entity with relevance scores
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RankedEntity {
    /// Entity ID from database
    pub entity_id: i64,
    /// Combined relevance score (0.0 to 1.0)
    pub relevance_score: f32,
    /// Entity type (function, struct, enum, etc.)
    pub entity_type: String,
    /// File path where entity is located
    pub file_path: String,
    /// Entity name
    pub name: String,
    /// Function signature or struct definition
    pub signature: Option<String>,
    /// Temporal relevance score (recency factor)
    pub temporal_score: Option<f32>,
    /// Graph connectivity score
    pub graph_score: Option<f32>,
    /// Graph embedding score (GRAPH domain: GraphBERT or SimpleFeatureCombiner)
    pub graph_embedding_score: Option<f32>,
}

impl Default for RagGraphQueryResponse {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
            selected_mode: "simple".to_string(),
            applied_scope: QueryScope::Global,
            debug_info: std::collections::HashMap::new(),
            query: String::new(),
        }
    }
}

impl Default for RankedEntity {
    fn default() -> Self {
        Self {
            entity_id: 0,
            relevance_score: 0.0,
            entity_type: String::new(),
            file_path: String::new(),
            name: String::new(),
            signature: None,
            temporal_score: None,
            graph_score: None,
            graph_embedding_score: None,
        }
    }
}

/// High-level RAGGraph API
///
/// Main entry point for RAGGraph queries, integrating vector search,
/// graph expansion, and tri-mode fusion for intelligent code exploration.
pub struct RagGraphAPI {
    pub(crate) code_graph: CodeGraph,
    pub(crate) graph_backend: Arc<dyn GraphBackend>,
    pub(crate) router: FusionRouter,
}

impl RagGraphAPI {
    /// Create new RAGGraph API instance
    ///
    /// # Arguments
    /// * `code_graph` - CodeGraph instance for entity storage and vector search
    /// * `graph_backend` - GraphBackend for graph traversal (Neo4j or SQLiteGraph)
    pub fn new(code_graph: CodeGraph, graph_backend: Arc<dyn GraphBackend>) -> Self {
        Self {
            code_graph,
            graph_backend,
            router: FusionRouter::new(),
        }
    }

    /// Create new RAGGraph API instance with any GraphBackend implementation
    ///
    /// This constructor accepts any backend that implements the GraphBackend trait,
    /// including SQLiteGraph, Neo4j, or future backends.
    ///
    /// # Arguments
    /// * `code_graph` - CodeGraph instance for entity storage and vector search
    /// * `graph_backend` - Any GraphBackend implementation (SQLiteGraph, Neo4j, etc.)
    ///
    /// # Returns
    /// New RagGraphAPI instance with the specified backend
    pub fn new_with_backend(
        code_graph: CodeGraph,
        graph_backend: Arc<dyn GraphBackend>,
    ) -> Self {
        Self {
            code_graph,
            graph_backend,
            router: FusionRouter::new(),
        }
    }
}

