//! Cognition Graph Schema - Reasoning Episode Tracking
//!
//! Defines Neo4j schema for cognitive reasoning episodes.
//! Separate from code entity and portfolio schemas for clarity.

/// Node Labels for cognition tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeLabel {
    ReasoningEpisode,  // Cognitive reasoning session
    CodeReference,     // Reference to code entity (lightweight, ID-only)
}

impl NodeLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReasoningEpisode => "ReasoningEpisode",
            Self::CodeReference => "CodeReference",
        }
    }
}

/// Relationship Types for cognition graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationType {
    Uses,  // ReasoningEpisode USES CodeReference
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
pub const COGNITION_PROJECT_LABEL: &str = "CognitionGraph";

/// Graph domain identifier stored on cognition nodes
pub const GRAPH_DOMAIN: &str = "cognition";

/// Get namespace from client with domain-specific prefix.
pub fn cognition_namespace(client: &crate::graph::Neo4jClient) -> String {
    format!("cognition_{}", client.namespace())
}
