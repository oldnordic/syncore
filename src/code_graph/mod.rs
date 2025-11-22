//! Code graph module for indexing and searching code
//!
//! This module provides functionality to:
//! - Parse code files and extract entities (functions, classes, etc.)
//! - Store entities in SQLite with full-text search capabilities
//! - Create embeddings for semantic code search
//! - Build relationship graphs between code entities
//! - Search code using both semantic (vector) and structural (graph) methods

// Submodules
pub mod edge_extractor;
mod edge_persistence; // PHASE 2: Semantic edge persistence (SQLite + Neo4j)
pub mod fusion_attention;
pub mod fusion_quality;
pub mod fusion_reasoning;
pub mod fusion_router;
pub mod fusion_simple;
mod graph;
pub mod incremental; // PHASE 5: Incremental indexing support
mod indexer;
pub mod multi_hop; // PHASE 4: Multi-hop graph reasoning
pub mod neo4j_relationships;
pub mod neo4j_sync;
pub mod neo4j_writer;
pub mod rag_graph_api;
mod searcher;
pub mod semantic_extractor; // PHASE 1: Semantic edge extraction
pub mod temporal_extractor; // PHASE 3: Temporal metadata extraction
mod types;
mod utils;

// Re-export public API
pub use graph::CodeGraph;
pub use incremental::{
    classify_file, classify_files, compute_file_sha256, get_file_mtime, FileChangeStatus,
    FileIndexState, IncrementalClassification,
}; // PHASE 5
pub use multi_hop::{MultiHopNode, MultiHopResult}; // PHASE 4
pub use rag_graph_api::{RagGraphAPI, RagGraphQueryRequest, RagGraphQueryResponse, RankedEntity};
pub use types::{CodeEdge, CodeEntity, CodeMatch, EdgeType, EntityType, MatchType, QueryScope};
