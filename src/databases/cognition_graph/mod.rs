//! Cognition Graph Canonical Module - Reasoning Episode Tracking
//!
//! This module provides type-safe Neo4j operations for cognitive reasoning episodes:
//! - ReasoningEpisode: Cognitive reasoning sessions with outcomes
//! - CodeEntity: Reference nodes linking episodes to code (lightweight, ID-only)
//!
//! Separate from code entity, RAG, and portfolio schemas for clear domain boundaries.
//!
//! Architecture:
//! - schema.rs: Defines :ReasoningEpisode, :CodeEntity nodes and USES relationships
//! - writer.rs: All write operations for cognition entities
//! - reader.rs: All read operations for cognition entities
//!
//! Rules (same as other canonical modules):
//! 1. No ad-hoc Cypher queries outside this module
//! 2. No string concatenation for Cypher
//! 3. No runtime-generated schema
//! 4. Namespace from Neo4jClient (defaults to "syncore_default")
//! 5. All writes use MERGE (idempotent)
//! 6. All queries parameterized
//! 7. All operations namespace-aware
//! 8. All entities use double label pattern: `:ReasoningEpisode:SynCore`

pub mod schema;
pub mod writer;
pub mod reader;

// Re-export main types for convenience
pub use schema::{
    NodeLabel,
    RelationType,
    ReasoningEpisodeProperties,
    COGNITION_PROJECT_LABEL,
    cognition_namespace,
};

pub use writer::{
    upsert_reasoning_episode,
    create_uses_relationship,
    delete_reasoning_episode,
};

pub use reader::{
    ReasoningEpisodeResult,
    get_reasoning_episode_by_id,
    fetch_related_episodes,
    count_reasoning_episodes,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Compile-time check: All expected types are exported
        let _label: NodeLabel = NodeLabel::ReasoningEpisode;
        let _rel: RelationType = RelationType::Uses;
        let _label_str: &str = COGNITION_PROJECT_LABEL;
        assert_eq!(_label_str, "SynCore");
    }
}
