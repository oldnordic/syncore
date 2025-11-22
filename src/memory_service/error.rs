//! Memory Service Error Types

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum MemoryError {
    /// Embedding dimension doesn't match cache dimension
    DimensionMismatch,
    /// Cache capacity exceeded
    CapacityExceeded,
    /// Invalid importance value (must be 0.0-1.0)
    InvalidImportance,
    /// Internal error with message
    Internal(String),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::DimensionMismatch => {
                write!(f, "Embedding dimension mismatch")
            }
            MemoryError::CapacityExceeded => {
                write!(f, "Cache capacity exceeded")
            }
            MemoryError::InvalidImportance => {
                write!(f, "Invalid importance value (must be between 0.0 and 1.0)")
            }
            MemoryError::Internal(msg) => {
                write!(f, "Internal memory error: {}", msg)
            }
        }
    }
}

impl std::error::Error for MemoryError {}
