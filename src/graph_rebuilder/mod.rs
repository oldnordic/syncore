//! Graph Rebuilder Module - Phase B Implementation
//!
//! This module handles graph validation, relationship extraction, and batch Neo4j operations:
//! - `validate.rs` - GraphValidator for checking graph health (connectivity, duplicates, orphans)
//! - `extractor.rs` - RelationshipExtractor using tree-sitter AST parsing
//! - `neo4j_push.rs` - BatchEdgePusher for efficient idempotent edge MERGE operations

pub mod extractor;
pub mod neo4j_push;
pub mod validate;

// Re-export main types for convenient access
pub use extractor::RelationshipExtractor;
pub use neo4j_push::BatchEdgePusher;
pub use validate::{ConnectivityStats, GraphValidator};
