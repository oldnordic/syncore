//! Code graph module for indexing and searching code
//!
//! This module provides functionality to:
//! - Parse code files and extract entities (functions, classes, etc.)
//! - Store entities in SQLite with full-text search capabilities
//! - Create embeddings for semantic code search
//! - Build relationship graphs between code entities
//! - Search code using both semantic (vector) and structural (graph) methods

// Submodules
pub mod body_extractor; // APEX v1.7 Phase 3: Function body extraction for semantic search
pub mod delta; // APEX 2.6-CG-GRAPH-DELTA: Delta-based selective reindexing
pub mod edge_extractor;
mod edge_persistence; // PHASE 2: Semantic edge persistence (SQLite + Neo4j)
pub mod entity_boost; // STEP C: Entity type boosting for relevance
pub mod explain; // STEP D: Function explanation with callers/callees/complexity
pub mod fusion_attention;
pub mod fusion_quality;
pub mod fusion_reasoning;
pub mod fusion_router;
pub mod fusion_simple;
mod graph;
pub mod incremental; // PHASE 5: Incremental indexing support
pub mod index_application; // Multilanguage index application
mod indexer;
pub mod language_parser; // Multilanguage parser trait
pub mod multi_hop; // PHASE 4: Multi-hop graph reasoning
pub mod neo4j_relationships;
pub mod neo4j_sync;
pub mod neo4j_writer;
pub mod parsers; // Language-specific parser implementations
pub mod rag_graph_api;
mod searcher;
pub mod semantic_extractor; // PHASE 1: Semantic edge extraction
pub mod temporal_extractor; // PHASE 3: Temporal metadata extraction
mod types;
pub mod update_service; // APEX 2.3-CG: Incremental code graph updates
mod utils;

// Re-export public API
pub use graph::CodeGraph;
pub use incremental::{
    classify_file, classify_files, compute_file_sha256, get_file_mtime, FileChangeStatus,
    FileIndexState, IncrementalClassification,
}; // PHASE 5
pub use index_application::IndexApplication; // Multilanguage index application
pub use language_parser::LanguageParser; // Multilanguage parser trait
pub use multi_hop::{MultiHopNode, MultiHopResult}; // PHASE 4
pub use parsers::{python_parser::PythonLanguageParser, rust_parser::RustLanguageParser}; // Language implementations
pub use rag_graph_api::{RagGraphAPI, RagGraphQueryRequest, RagGraphQueryResponse, RankedEntity};
pub use types::{CodeEdge, CodeEntity, CodeMatch, EdgeType, EntityType, MatchType, QueryScope};
