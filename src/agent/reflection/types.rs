//! Reflection Core Types
//!
//! Defines the fundamental data structures for reflection functionality

use crate::agent::current_timestamp_ms;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Failure category classification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FailureCategory {
    /// Network or connectivity issues
    Network,
    /// Database or storage issues
    Database,
    /// Authentication or authorization issues
    Authentication,
    /// Resource constraints (memory, CPU, etc.)
    Resource,
    /// Logic or algorithmic errors
    Logic,
    /// External service failures
    ExternalService,
    /// Timeout or performance issues
    Performance,
    /// Unknown or uncategorized failure
    Unknown,
}

/// Root cause analysis for a failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCause {
    /// Description of the root cause
    pub description: String,

    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,

    /// Supporting evidence
    pub evidence: Vec<String>,

    /// Recommended actions
    pub recommendations: Vec<String>,
}

/// Recovery action suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    /// Action description
    pub action: String,

    /// Priority level (1-10, higher = more important)
    pub priority: i32,

    /// Estimated success probability (0.0 to 1.0)
    pub success_probability: f64,

    /// Resources required
    pub resources: Vec<String>,

    /// Prerequisites for this action
    pub prerequisites: Vec<String>,
}

/// Failure analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureAnalysis {
    /// Original action that failed
    pub original_action: String,

    /// Error message
    pub error_message: String,

    /// Failure category
    pub category: FailureCategory,

    /// Root causes identified
    pub root_causes: Vec<RootCause>,

    /// Recovery actions suggested
    pub recovery_actions: Vec<RecoveryAction>,

    /// Severity level (1-10)
    pub severity: i32,

    /// Whether this failure is recoverable
    pub is_recoverable: bool,

    /// Estimated time to recovery (seconds)
    pub estimated_recovery_time: i32,

    /// Analysis timestamp
    pub timestamp: i64,
}

/// Retry plan generated from reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPlan {
    /// Plan ID
    pub id: String,

    /// Maximum retry attempts
    pub max_retries: i32,

    /// Initial delay in milliseconds
    pub initial_delay_ms: i32,

    /// Backoff multiplier
    pub backoff_multiplier: f64,

    /// Maximum delay in milliseconds
    pub max_delay_ms: i32,

    /// Retry actions to attempt
    pub retry_actions: Vec<String>,

    /// Conditions for giving up
    pub abort_conditions: Vec<String>,

    /// Created timestamp
    pub created_at: i64,
}

/// Emergent behavior pattern detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergentBehavior {
    /// Behavior type identifier
    pub behavior_type: String,

    /// Confidence in detection (0.0 to 1.0)
    pub confidence: f64,

    /// Actions affected by this behavior
    pub affected_actions: Vec<String>,

    /// Pattern description
    pub description: String,

    /// Mitigation strategies
    pub mitigation_strategies: Vec<String>,

    /// Detection timestamp
    pub timestamp: i64,
}

/// Reflection report containing analysis and recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionReport {
    /// Report ID
    pub id: String,

    /// Associated plan ID
    pub plan_id: String,

    /// Whether a failure was detected
    pub failure_detected: bool,

    /// Failure analysis (if applicable)
    pub failure_analysis: Option<FailureAnalysis>,

    /// Root causes identified
    pub root_causes: Vec<String>,

    /// Recovery actions suggested
    pub recovery_actions: Vec<String>,

    /// Failure category
    pub failure_category: Option<String>,

    /// Retry plan (if applicable)
    pub retry_plan: Option<RetryPlan>,

    /// Emergent behaviors detected
    pub emergent_behaviors: Vec<EmergentBehavior>,

    /// Key insights from reflection
    pub insights: Vec<String>,

    /// Memory keys for storing this reflection
    pub memory_keys: Vec<String>,

    /// Report creation timestamp
    pub created_at: i64,

    /// Summary of the reflection
    pub summary: String,

    /// Original action description
    pub action_description: String,

    /// Summary of the error
    pub error_summary: String,

    /// Recommendations from the reflection
    pub recommendations: Vec<String>,
}

impl ReflectionReport {
    /// Create a new reflection report
    pub fn new(plan_id: String) -> Self {
        let id = format!("reflection_{}", Uuid::new_v4());
        Self {
            id,
            plan_id,
            failure_detected: false,
            failure_analysis: None,
            root_causes: Vec::new(),
            recovery_actions: Vec::new(),
            failure_category: None,
            retry_plan: None,
            emergent_behaviors: Vec::new(),
            insights: Vec::new(),
            memory_keys: Vec::new(),
            created_at: current_timestamp_ms(),
            summary: String::new(),
            action_description: String::new(),
            error_summary: String::new(),
            recommendations: Vec::new(),
        }
    }
}