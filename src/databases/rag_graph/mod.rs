//! RAG Graph Canonical Module - Embedding Storage & Retrieval
//!
//! This module provides type-safe Neo4j operations for RAG (Retrieval-Augmented Generation) graph entities.
//! Separate from code entity schema to maintain clear domain boundaries.
//!
//! Architecture:
//! - schema.rs: Defines :Embedding nodes and semantic relationships
//! - writer.rs: All write operations for embeddings
//! - reader.rs: All read operations for embeddings
//!
//! Rules (same as canonical Neo4j module):
//! 1. No ad-hoc Cypher queries outside this module
//! 2. No string concatenation for Cypher
//! 3. No runtime-generated schema
//! 4. Namespace from Neo4jClient (defaults to "syncore_default")
//! 5. All writes use MERGE (idempotent)
//! 6. All queries parameterized (no SQL injection)
//! 7. All operations namespace-aware
//! 8. All entities use double label pattern: `:Embedding:SynCore`

pub mod reader;
pub mod schema;
pub mod writer;

// Re-export main types for convenience
pub use schema::{rag_namespace, EmbeddingProperties, NodeLabel, RelationType, RAG_PROJECT_LABEL};

pub use writer::{
    batch_upsert_embeddings, create_relationship, delete_embedding, upsert_embedding,
};

pub use reader::{
    count_embeddings, get_embedding_by_id, get_embedding_text, get_neighbors, EmbeddingResult,
    NeighborResult,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Compile-time check: All expected types are exported
        let _label: NodeLabel = NodeLabel::Embedding;
        let _rel: RelationType = RelationType::SimilarTo;
        let _label_str: &str = RAG_PROJECT_LABEL;
        assert_eq!(_label_str, "SynCore");
    }
}
