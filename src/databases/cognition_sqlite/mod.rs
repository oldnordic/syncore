//! Cognition SQLite Module - Reasoning Episode Tracking
//!
//! This module provides SQLite-based operations for cognitive reasoning episodes:
//! - ReasoningEpisode: Cognitive reasoning sessions with outcomes (legacy)
//! - ReasoningSession: Tree-of-Thought reasoning sessions
//! - ThoughtNode: Individual thoughts in reasoning tree
//! - CodeReference: Reference nodes linking episodes to code (lightweight, ID-only)
//!
//! Separate from code entity, RAG, and portfolio schemas for clear domain boundaries.
//!
//! Architecture:
//! - schema.rs: Defines schema for reasoning sessions, nodes, and edges
//! - writer.rs: All write operations for cognition entities
//! - reader.rs: All read operations for cognition entities
//!
//! Rules:
//! 1. No ad-hoc SQL queries outside this module
//! 2. No string concatenation for SQL
//! 3. No runtime-generated schema
//! 4. Namespace-aware operations
//! 5. All writes use INSERT OR REPLACE (idempotent)
//! 6. All queries parameterized
//! 7. All operations namespace-aware
//! 8. All entities use proper foreign key constraints

pub mod reader;
pub mod schema;
pub mod writer;

// Re-export main types for convenience
pub use schema::{
    cognition_namespace, NodeLabel, ReasoningEpisodeProperties, ReasoningSessionProperties,
    RelationType, SessionMetrics, SessionProperties, ThoughtNodeProperties, ThoughtNodeResult,
    COGNITION_PROJECT_LABEL, GRAPH_DOMAIN,
};

pub use writer::CognitionSqliteWriter;

pub use reader::{CognitionSqliteReader, ReasoningEpisodeResult};
pub use schema::SessionResult;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Compile-time check: All expected types are exported
        let _label: NodeLabel = NodeLabel::ReasoningSession;
        let _rel: RelationType = RelationType::BelongsTo;
        let _label_str: &str = COGNITION_PROJECT_LABEL;
        assert_eq!(_label_str, "CognitionGraph");
        assert_eq!(GRAPH_DOMAIN, "cognition");
    }
}
