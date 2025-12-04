//! Tree-of-Thought Reasoning Engine - ST-3 Core Implementation
//!
//! This module provides the core ToT reasoning engine for SynCore.
//! Implements session management, node expansion, and tree structure maintenance.
//!
//! Architecture:
//! - session.rs: ReasoningSession management and context
//! - node.rs: ThoughtNode context and evaluation
//! - engine.rs: ToTEngine orchestrator (main entry point)
//!
//! Phase ST-3 Scope:
//! - Basic session management with root nodes
//! - Node expansion with deterministic stub branches
//! - Active node selection (most recent leaf)
//! - Session isolation and tree structure invariants

pub mod branch_manager;
pub mod engine;
pub mod llm_adapter;
pub mod metrics;
pub mod node;
pub mod reasoning_session;
pub mod session;
pub mod session_sqlite;
pub mod tree_logger;

// Re-export main types for convenience
pub use engine::ToTEngine;
pub use node::ReasoningNodeContext;
pub use reasoning_session::ReasoningSession;
pub use session::ReasoningSessionManager;
pub use session_sqlite::ReasoningSessionManagerSqlite;
pub use tree_logger::TreeLogger;

use anyhow::Result;

/// Common error types for reasoning engine
#[derive(thiserror::Error, Debug)]
pub enum ReasoningError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Invalid parent reference: {0}")]
    InvalidParent(String),

    #[error("Database error: {0}")]
    Database(#[from] anyhow::Error),

    #[error("Neo4j error: {0}")]
    Neo4j(String),

    // PHASE ST-6 Circuit Breaker Errors
    #[error("Branch limit exceeded: {0}")]
    BranchLimitExceeded(String),

    #[error("Depth limit exceeded: {0}")]
    DepthLimitExceeded(String),

    #[error("Breadth limit exceeded: {0}")]
    BreadthLimitExceeded(String),

    #[error("Repetitive thought pattern detected: {0}")]
    RepetitiveThoughtPattern(String),

    #[error("Loop detected: {0}")]
    LoopDetected(String),

    #[error("Too many consecutive errors: {0}")]
    TooManyErrors(String),

    #[error("Safety invariant failure: {0}")]
    SafetyInvariantFailure(String),
}

/// Result type alias for reasoning operations
pub type ReasoningResult<T> = Result<T, ReasoningError>;

/// Generate UUID-based session IDs
pub fn generate_session_id() -> String {
    format!("session_{}", uuid::Uuid::new_v4())
}

/// Generate UUID-based node IDs
pub fn generate_node_id() -> String {
    format!("node_{}", uuid::Uuid::new_v4())
}

/// Get current timestamp as Unix epoch milliseconds
pub fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_generation() {
        let session_id = generate_session_id();
        let node_id = generate_node_id();

        assert!(session_id.starts_with("session_"));
        assert!(node_id.starts_with("node_"));
        assert!(session_id.len() > 10);
        assert!(node_id.len() > 10);
    }

    #[test]
    fn test_timestamp_generation() {
        let ts1 = current_timestamp();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let ts2 = current_timestamp();

        assert!(ts2 > ts1);
        assert!(ts1 > 0);
    }

    #[test]
    fn test_reasoning_error_display() {
        let err = ReasoningError::SessionNotFound("test_session".to_string());
        assert_eq!(err.to_string(), "Session not found: test_session");
    }
}
