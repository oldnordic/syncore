//! RAG Graph Schema Definitions
//!
//! This module defines the schema for RAG (Retrieval-Augmented Generation) graph entities.
//! Separate from code entity schema to maintain clear domain boundaries.
//!
//! Schema components:
//! - NodeLabel: :Embedding (text embeddings for semantic search)
//! - NodeProperties: Embedding-specific properties
//! - RelationType: Semantic relationships between embeddings
//! - Namespace isolation: Uses same namespace pattern as code entities

use crate::graph::Neo4jClient;

/// RAG Graph project label (for double-label pattern)
pub const RAG_PROJECT_LABEL: &str = "SynCore";

/// RAG node types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeLabel {
    /// Text embedding for semantic search
    Embedding,
}

impl NodeLabel {
    /// Convert to Neo4j label string
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeLabel::Embedding => "Embedding",
        }
    }
}

/// RAG relationship types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    /// Semantic similarity relationship with weight
    SimilarTo,
    /// Part-of relationship (chunk to document)
    PartOf,
    /// References relationship (embedding to entity)
    References,
}

impl RelationType {
    /// Convert to Neo4j relationship type string
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationType::SimilarTo => "SIMILAR_TO",
            RelationType::PartOf => "PART_OF",
            RelationType::References => "REFERENCES",
        }
    }

    /// Parse from string (for database queries)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "SIMILAR_TO" => Some(RelationType::SimilarTo),
            "PART_OF" => Some(RelationType::PartOf),
            "REFERENCES" => Some(RelationType::References),
            _ => None,
        }
    }
}

/// Properties for Embedding nodes
#[derive(Debug, Clone)]
pub struct EmbeddingProperties {
    /// Node ID (from embeddings table in SQLite)
    pub id: i64,
    /// Original text that was embedded
    pub text: String,
    /// Optional metadata as JSON string
    pub metadata: Option<String>,
}

/// Get namespace from Neo4j client (for consistent namespace isolation)
pub fn rag_namespace(client: &Neo4jClient) -> String {
    client.namespace().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_label_roundtrip() {
        assert_eq!(NodeLabel::Embedding.as_str(), "Embedding");
    }

    #[test]
    fn test_relation_type_roundtrip() {
        assert_eq!(RelationType::SimilarTo.as_str(), "SIMILAR_TO");
        assert_eq!(
            RelationType::from_str("SIMILAR_TO"),
            Some(RelationType::SimilarTo)
        );
        assert_eq!(RelationType::from_str("UNKNOWN"), None);
    }

    #[test]
    fn test_project_label() {
        assert_eq!(RAG_PROJECT_LABEL, "SynCore");
    }
}
