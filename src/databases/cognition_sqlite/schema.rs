//! Cognition SQLite Schema - Reasoning Episode Tracking
//!
//! Defines SQLite schema for cognitive reasoning episodes.
//! Separate from code entity and portfolio schemas for clarity.

use serde::{Deserialize, Serialize};

/// Node Labels for cognition tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningEpisodeProperties {
    pub id: i64,
    pub timestamp: i64,
    pub user_query: String,
    pub selected_mode: String,
    pub outcome: String,
    pub notes: Option<String>,
    pub namespace: String,
    pub graph_domain: String,
}

/// Properties for ReasoningSession node (Tree-of-Thought)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningSessionProperties {
    pub id: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
    pub namespace: String,
    pub graph_domain: String,
}

/// Properties for ThoughtNode node (Tree-of-Thought)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtNodeProperties {
    pub id: i64,
    pub session_id: String,
    pub parent_id: Option<i64>,
    pub content: String,
    pub thought_type: String,
    pub depth: i64,
    pub breadth: i64,
    pub confidence: f64,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub namespace: String,
    pub graph_domain: String,
}

/// Result type for reasoning sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResult {
    pub id: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
    pub namespace: String,
    pub graph_domain: String,
}

/// Result type for thought nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtNodeResult {
    pub id: i64,
    pub session_id: String,
    pub parent_id: Option<i64>,
    pub content: String,
    pub thought_type: String,
    pub depth: i64,
    pub breadth: i64,
    pub confidence: f64,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub namespace: String,
    pub graph_domain: String,
}

/// Metrics for reasoning sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub total_nodes: i64,
    pub max_depth: i64,
    pub avg_confidence: f64,
    pub node_types: std::collections::HashMap<String, i64>,
}

/// Project label for double-labeling pattern
pub const COGNITION_PROJECT_LABEL: &str = "CognitionGraph";

/// Graph domain identifier stored on cognition nodes
pub const GRAPH_DOMAIN: &str = "cognition";

/// Type aliases for consistency with existing patterns
pub type SessionProperties = ReasoningSessionProperties;

/// Get namespace with domain-specific prefix.
pub fn cognition_namespace(base_namespace: &str) -> String {
    format!("cognition_{}", base_namespace)
}
