//! Cognition Graph Schema - Reasoning Episode Tracking
//!
//! Defines Neo4j schema for cognitive reasoning episodes.
//! Separate from code entity and portfolio schemas for clarity.

/// Node Labels for cognition tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeLabel {
    ReasoningEpisode,  // Cognitive reasoning session
    CodeEntity,        // Reference to code entity (lightweight, ID-only)
}

impl NodeLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReasoningEpisode => "ReasoningEpisode",
            Self::CodeEntity => "CodeEntity",
        }
    }
}

/// Relationship Types for cognition graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationType {
    Uses,  // ReasoningEpisode USES CodeEntity
}

impl RelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uses => "USES",
        }
    }
}

/// Properties for ReasoningEpisode node
#[derive(Debug, Clone)]
pub struct ReasoningEpisodeProperties {
    pub id: i64,
    pub timestamp: i64,
    pub user_query: String,
    pub selected_mode: String,
    pub outcome: String,
    pub notes: Option<String>,
}

/// Project label for double-labeling pattern
pub const COGNITION_PROJECT_LABEL: &str = "SynCore";

/// Get namespace from client (defaults to syncore_default)
pub fn cognition_namespace(client: &crate::graph::Neo4jClient) -> String {
    client.namespace().to_string()
}
