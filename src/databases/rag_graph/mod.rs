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

pub mod schema;
pub mod writer;
pub mod reader;

// Re-export main types for convenience
pub use schema::{
    NodeLabel,
    RelationType,
    EmbeddingProperties,
    RAG_PROJECT_LABEL,
    rag_namespace,
};

pub use writer::{
    upsert_embedding,
    create_relationship,
    batch_upsert_embeddings,
    delete_embedding,
};

pub use reader::{
    EmbeddingResult,
    NeighborResult,
    get_embedding_by_id,
    get_neighbors,
    get_embedding_text,
    count_embeddings,
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
