//! Cognition Graph Schema - Reasoning Episode Tracking
//!
//! Defines Neo4j schema for cognitive reasoning episodes.
//! Separate from code entity and portfolio schemas for clarity.

/// Node Labels for cognition tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeLabel {
    ReasoningEpisode, // Cognitive reasoning session (legacy)
    ReasoningSession, // Tree-of-Thought reasoning session
    ThoughtNode,      // Individual thought in reasoning tree
    CodeReference,    // Reference to code entity (lightweight, ID-only)
}

impl NodeLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReasoningEpisode => "ReasoningEpisode",
            Self::ReasoningSession => "ReasoningSession",
            Self::ThoughtNode => "ThoughtNode",
            Self::CodeReference => "CodeReference",
        }
    }
}

/// Relationship Types for cognition graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationType {
    Uses,      // ReasoningEpisode USES CodeReference
    HasChild,  // ReasoningSession HAS_CHILD ThoughtNode
    BelongsTo, // ThoughtNode BELONGS_TO ReasoningSession
}

impl RelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uses => "USES",
            Self::HasChild => "HAS_CHILD",
            Self::BelongsTo => "BELONGS_TO",
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

/// Properties for ReasoningSession node (Tree-of-Thought)
#[derive(Debug, Clone)]
pub struct ReasoningSessionProperties {
    pub id: String,
    pub task_id: Option<String>,
    pub metadata: Option<String>,
    pub created_at: i64,
    // PHASE ST-6: Circuit breaker session counters
    pub total_nodes: i64,
    pub depth: i64,
    pub breadth: i64,
    pub identical_expansions: i64,
    pub consecutive_errors: i64,
}

/// Properties for ThoughtNode node (Tree-of-Thought)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThoughtNodeProperties {
    pub id: String,
    pub session_id: String,
    pub parent_id: Option<String>,
    pub step_index: i64,
    pub content: String,
    pub score: Option<f64>,
}

/// Project label for double-labeling pattern
pub const COGNITION_PROJECT_LABEL: &str = "CognitionGraph";

/// Graph domain identifier stored on cognition nodes
pub const GRAPH_DOMAIN: &str = "cognition";

/// Type aliases for consistency with existing patterns
pub type SessionProperties = ReasoningSessionProperties;

/// Get namespace from client with domain-specific prefix.
pub fn cognition_namespace(client: &crate::graph::Neo4jClient) -> String {
    format!("cognition_{}", client.namespace())
}
