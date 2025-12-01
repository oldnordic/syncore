//! TOON Result Types

use super::ram_cache::MemoryEntry;

/// Results from TOON instruction execution
#[derive(Debug, Clone)]
pub enum ToonResult {
    /// Retrieved memory entries from query
    Retrieved(Vec<MemoryEntry>),

    /// Context was folded into new memory entry
    Folded {
        new_id: String,
    },

    /// Pointer token emitted
    Pointer(String),

    /// Memory entry loaded
    Loaded(MemoryEntry),

    /// Execution completed
    Completed,
}

/// Result from a single TOON execution step
#[derive(Debug, Clone)]
pub struct ToonStepResult {
    pub node_id: String,
    pub result: ToonResult,
}

impl ToonStepResult {
    pub fn new(node_id: String, result: ToonResult) -> Self {
        Self {
            node_id,
            result,
        }
    }
}
