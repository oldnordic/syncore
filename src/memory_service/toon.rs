//! TOON Memory Operation Types
//!
//! Defines the schema for TOON (internal reasoning) memory operations.
//! Phase 1: Type definitions only (no parser implementation).

/// TOON memory operation types
#[derive(Debug, Clone, PartialEq)]
pub enum ToonMemoryOp {
    /// Store a new memory entry
    Store {
        summary: String,
        importance: f32,
        tags: Vec<String>,
        raw_text: String,
    },

    /// Retrieve memory entries by query
    Retrieve { query: String, k: usize },

    /// Reference a memory node by ID (pointer)
    Pointer { id: String },
}

impl ToonMemoryOp {
    /// Check if this is a Store operation
    pub fn is_store(&self) -> bool {
        matches!(self, ToonMemoryOp::Store { .. })
    }

    /// Check if this is a Retrieve operation
    pub fn is_retrieve(&self) -> bool {
        matches!(self, ToonMemoryOp::Retrieve { .. })
    }

    /// Check if this is a Pointer operation
    pub fn is_pointer(&self) -> bool {
        matches!(self, ToonMemoryOp::Pointer { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_store() {
        let op = ToonMemoryOp::Store {
            summary: "test".to_string(),
            importance: 0.5,
            tags: vec![],
            raw_text: "text".to_string(),
        };
        assert!(op.is_store());
        assert!(!op.is_retrieve());
        assert!(!op.is_pointer());
    }

    #[test]
    fn test_is_retrieve() {
        let op = ToonMemoryOp::Retrieve {
            query: "test".to_string(),
            k: 5,
        };
        assert!(!op.is_store());
        assert!(op.is_retrieve());
        assert!(!op.is_pointer());
    }

    #[test]
    fn test_is_pointer() {
        let op = ToonMemoryOp::Pointer {
            id: "N123".to_string(),
        };
        assert!(!op.is_store());
        assert!(!op.is_retrieve());
        assert!(op.is_pointer());
    }
}
