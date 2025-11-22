//! HNSW Warmup State Machine
//!
//! Provides a state machine for tracking HNSW index warmup status.
//! This enables non-blocking warmup with brute-force fallback.
//!
//! State transitions:
//! - Cold -> WarmingUp: When warmup begins
//! - WarmingUp -> Hot: When warmup completes
//! - Hot -> Cold: For testing/reset only
//!
//! Behavior by state:
//! - Cold: Use brute-force search (vectors in memory)
//! - WarmingUp: Use brute-force search (HNSW building)
//! - Hot: Use HNSW search (fast approximate)

use std::sync::atomic::{AtomicU8, Ordering};

/// HNSW warmup states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HnswWarmupState {
    /// HNSW not loaded; use brute-force fallback
    Cold = 0,
    /// Background rebuild in progress; use brute-force fallback
    WarmingUp = 1,
    /// HNSW ready; use HNSW for search
    Hot = 2,
}

impl From<u8> for HnswWarmupState {
    fn from(value: u8) -> Self {
        match value {
            0 => HnswWarmupState::Cold,
            1 => HnswWarmupState::WarmingUp,
            2 => HnswWarmupState::Hot,
            _ => HnswWarmupState::Cold, // Default to Cold for safety
        }
    }
}

/// Controller for HNSW warmup state transitions
///
/// Thread-safe state management using AtomicU8.
/// All state changes are logged for debugging.
#[derive(Debug)]
pub struct WarmupController {
    state: AtomicU8,
}

impl Default for WarmupController {
    fn default() -> Self {
        Self::new()
    }
}

impl WarmupController {
    /// Create a new controller in Cold state
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(HnswWarmupState::Cold as u8),
        }
    }

    /// Get current warmup state
    pub fn state(&self) -> HnswWarmupState {
        HnswWarmupState::from(self.state.load(Ordering::SeqCst))
    }

    /// Check if HNSW is ready (Hot state)
    pub fn is_hot(&self) -> bool {
        self.state() == HnswWarmupState::Hot
    }

    /// Check if warmup is in progress
    pub fn is_warming_up(&self) -> bool {
        self.state() == HnswWarmupState::WarmingUp
    }

    /// Mark state as Cold
    ///
    /// Used for testing or reset scenarios.
    pub fn mark_cold(&self) {
        let old = self.state.swap(HnswWarmupState::Cold as u8, Ordering::SeqCst);
        eprintln!(
            "[SynCore] HNSW warmup state: {:?} -> Cold",
            HnswWarmupState::from(old)
        );
    }

    /// Mark state as WarmingUp
    ///
    /// Called at the start of HNSW warmup/rebuild.
    pub fn mark_warming_up(&self) {
        let old = self.state.swap(HnswWarmupState::WarmingUp as u8, Ordering::SeqCst);
        eprintln!(
            "[SynCore] HNSW warmup state: {:?} -> WarmingUp",
            HnswWarmupState::from(old)
        );
    }

    /// Mark state as Hot
    ///
    /// Called when HNSW warmup completes and index is ready.
    pub fn mark_hot(&self) {
        let old = self.state.swap(HnswWarmupState::Hot as u8, Ordering::SeqCst);
        eprintln!(
            "[SynCore] HNSW warmup state: {:?} -> Hot",
            HnswWarmupState::from(old)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_cold() {
        let ctrl = WarmupController::new();
        assert_eq!(ctrl.state(), HnswWarmupState::Cold);
        assert!(!ctrl.is_hot());
        assert!(!ctrl.is_warming_up());
    }

    #[test]
    fn test_state_transitions() {
        let ctrl = WarmupController::new();

        ctrl.mark_warming_up();
        assert_eq!(ctrl.state(), HnswWarmupState::WarmingUp);
        assert!(ctrl.is_warming_up());

        ctrl.mark_hot();
        assert_eq!(ctrl.state(), HnswWarmupState::Hot);
        assert!(ctrl.is_hot());

        ctrl.mark_cold();
        assert_eq!(ctrl.state(), HnswWarmupState::Cold);
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let ctrl = Arc::new(WarmupController::new());
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let ctrl = ctrl.clone();
                thread::spawn(move || {
                    for _ in 0..100 {
                        match i % 3 {
                            0 => ctrl.mark_cold(),
                            1 => ctrl.mark_warming_up(),
                            _ => ctrl.mark_hot(),
                        }
                        let _ = ctrl.state();
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // Should not panic - state is one of the valid values
        let state = ctrl.state();
        assert!(matches!(
            state,
            HnswWarmupState::Cold | HnswWarmupState::WarmingUp | HnswWarmupState::Hot
        ));
    }
}
