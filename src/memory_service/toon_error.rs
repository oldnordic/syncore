//! TOON Error Types

use super::error::MemoryError;
use std::fmt;

/// Errors that can occur during TOON execution
#[derive(Debug, Clone, PartialEq)]
pub enum ToonError {
    /// Node was not found in the graph
    NodeNotFound(String),

    /// Invalid pointer reference
    InvalidPointer(String),

    /// Execution loop detected (visited same node twice)
    ExecutionLoopDetected(String),

    /// Memory operation failed
    MemoryFailure(MemoryError),

    /// Internal error
    Internal(String),
}

impl fmt::Display for ToonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToonError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            ToonError::InvalidPointer(id) => write!(f, "Invalid pointer: {}", id),
            ToonError::ExecutionLoopDetected(id) => {
                write!(f, "Execution loop detected at node: {}", id)
            }
            ToonError::MemoryFailure(err) => write!(f, "Memory failure: {}", err),
            ToonError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ToonError {}

impl From<MemoryError> for ToonError {
    fn from(err: MemoryError) -> Self {
        ToonError::MemoryFailure(err)
    }
}
