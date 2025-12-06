//! Planning Engine - PHASE 4 Core Component
//!
//! Deterministic planning engine that creates and refines multi-step plans
//! using existing memory, vector, and graph APIs. Enforces circuit breaker
//! patterns and uses only existing infrastructure.

pub mod types;
pub mod cycle;
pub mod engine;
#[cfg(test)]
mod tests;

// Re-export the main public types
pub use types::{PlanNode, PlanNodeStatus, PlanTree};
pub use engine::PlanningEngine;