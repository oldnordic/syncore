//! Reflection Engine - PHASE 4 Core Component
//!
//! Analyzes failures, updates memory, and prevents infinite loops through
//! graph-driven reasoning validation. Uses existing ToT engine and memory APIs.

pub mod types;
pub mod classifier;
pub mod analysis;
pub mod engine;
#[cfg(test)]
mod tests;

// Re-export the main public types
pub use types::{
    FailureCategory, RootCause, RecoveryAction, FailureAnalysis,
    RetryPlan, EmergentBehavior, ReflectionReport
};
pub use engine::ReflectionEngine;