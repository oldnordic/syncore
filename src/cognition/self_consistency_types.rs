//! Self-Consistency Checker Types
//!
//! Type definitions for the self-consistency evaluation system

use serde::{Deserialize, Serialize};

/// Result of self-consistency evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfConsistencyResult {
    pub score: f32,
    pub issues: Vec<SelfConsistencyIssue>,
    pub suggested_plan: Option<SuggestedPlan>,
}

/// Type of self-consistency issue
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelfConsistencyIssueKind {
    RepeatedFailedSequence,
    ConflictingPattern,
    GraphInconsistency,
    NamespaceMismatch,
    ToolOrderSuspicious,
    MissingRequiredStep,
    PotentialLoop,
    Other(String),
}

/// Self-consistency issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfConsistencyIssue {
    pub kind: SelfConsistencyIssueKind,
    pub description: String,
    pub severity: SelfConsistencySeverity,
}

/// Severity level for issues
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelfConsistencySeverity {
    Info,
    Warning,
    Error,
}

/// Suggested corrected plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedPlan {
    pub recommended_tool_sequence: Vec<String>,
    pub recommended_mode: Option<String>,
    pub notes: Option<String>,
}
