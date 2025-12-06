//! Agent Planning & Reasoning Engine (APRE) - PHASE 4
//!
//! Deterministic, memory-driven multi-step reasoning system that extends
//! the existing ToT engine with full PLAN → ACT → REFLECT capabilities.
//!
//! Architecture:
//! - planner.rs: PlanningEngine with PlanNode, PlanTree, and deterministic planning
//! - reflection.rs: ReflectionEngine for failure analysis and self-correction
//! - session_state.rs: Agent session state integration with reasoning sessions
//! - mod.rs: Module exports and common types

pub mod planner;
pub mod reflection;
pub mod session_state;

// Re-export main types for convenience
pub use planner::{PlanNode, PlanTree, PlanningEngine};
pub use reflection::{FailureAnalysis, ReflectionEngine, ReflectionReport};
pub use session_state::{AgentSessionState, PlanExecutionState};

use anyhow::Result;
use std::time::SystemTime;

/// Common error types for APRE
#[derive(thiserror::Error, Debug)]
pub enum ApreError {
    #[error("Planning failed: {0}")]
    PlanningFailed(String),

    #[error("Reflection failed: {0}")]
    ReflectionFailed(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Plan execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Deadlock detected: {0}")]
    DeadlockDetected(String),

    #[error("Circuit breaker activated: {0}")]
    CircuitBreakerActivated(String),

    #[error("Memory operation failed: {0}")]
    MemoryError(#[from] anyhow::Error),

    #[error("Graph operation failed: {0}")]
    GraphError(String),
}

/// Result type alias for APRE operations
pub type ApreResult<T> = Result<T, ApreError>;

/// Get current timestamp as Unix epoch milliseconds
pub fn current_timestamp_ms() -> i64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_timestamp_generation() {
        let ts1 = current_timestamp_ms();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let ts2 = current_timestamp_ms();

        assert!(ts2 > ts1);
        assert!(ts1 > 0);
    }

    #[test]
    fn test_apre_error_display() {
        let err = ApreError::PlanningFailed("test failure".to_string());
        assert_eq!(err.to_string(), "Planning failed: test failure");
    }
}
