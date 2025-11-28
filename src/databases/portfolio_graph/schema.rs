//! Portfolio Graph Schema Definitions
//!
//! This module defines the schema for portfolio tracking entities:
//! - Code changes (Patches)
//! - Sequential reasoning steps
//! - Task management
//!
//! Separate from code entity and RAG schemas to maintain clear domain boundaries.

use crate::graph::Neo4jClient;

/// Portfolio Graph project label (for double-label pattern)
pub const PORTFOLIO_PROJECT_LABEL: &str = "PortfolioGraph";

/// Graph domain identifier for portfolio nodes
pub const GRAPH_DOMAIN: &str = "portfolio";

/// Portfolio node types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeLabel {
    /// Code change/patch tracking
    Patch,
    /// Sequential reasoning step
    Step,
    /// Task tracking
    Task,
}

impl NodeLabel {
    /// Convert to Neo4j label string
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeLabel::Patch => "Patch",
            NodeLabel::Step => "Step",
            NodeLabel::Task => "Task",
        }
    }
}

/// Portfolio relationship types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    /// Patch/Step is for a specific task
    ForTask,
    /// Patch applies to a file
    AppliesTo,
    /// Step follows another step (sequential)
    Follows,
}

impl RelationType {
    /// Convert to Neo4j relationship type string
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationType::ForTask => "FOR_TASK",
            RelationType::AppliesTo => "APPLIES_TO",
            RelationType::Follows => "FOLLOWS",
        }
    }

    /// Parse from string (for database queries)
    pub fn try_parse(s: &str) -> Option<Self> {
        match s {
            "FOR_TASK" => Some(RelationType::ForTask),
            "APPLIES_TO" => Some(RelationType::AppliesTo),
            "FOLLOWS" => Some(RelationType::Follows),
            _ => None,
        }
    }
}

/// Properties for Patch nodes
#[derive(Debug, Clone)]
pub struct PatchProperties {
    /// Patch ID (from patches table in SQLite)
    pub id: i64,
    /// Optional patch metadata as JSON string
    pub metadata: Option<String>,
}

/// Properties for Step nodes
#[derive(Debug, Clone)]
pub struct StepProperties {
    /// Step ID (from steps table in SQLite)
    pub id: i64,
    /// Step number in sequence
    pub step_number: i64,
    /// Optional step metadata as JSON string
    pub metadata: Option<String>,
}

/// Properties for Task nodes
#[derive(Debug, Clone)]
pub struct TaskProperties {
    /// Task ID (from tasks table in SQLite)
    pub id: i64,
    /// Optional task metadata as JSON string
    pub metadata: Option<String>,
}

/// Get namespace from Neo4j client (for consistent namespace isolation)
pub fn portfolio_namespace(client: &Neo4jClient) -> String {
    format!("portfolio_{}", client.namespace())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_label_roundtrip() {
        assert_eq!(NodeLabel::Patch.as_str(), "Patch");
        assert_eq!(NodeLabel::Step.as_str(), "Step");
        assert_eq!(NodeLabel::Task.as_str(), "Task");
    }

    #[test]
    fn test_relation_type_roundtrip() {
        assert_eq!(RelationType::ForTask.as_str(), "FOR_TASK");
        assert_eq!(
            RelationType::try_parse("FOR_TASK"),
            Some(RelationType::ForTask)
        );
        assert_eq!(
            RelationType::try_parse("APPLIES_TO"),
            Some(RelationType::AppliesTo)
        );
        assert_eq!(RelationType::try_parse("UNKNOWN"), None);
    }

    #[test]
    fn test_project_label() {
        assert_eq!(PORTFOLIO_PROJECT_LABEL, "PortfolioGraph");
    }
}
