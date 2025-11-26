//! Refactor Patterns and Heuristics
//!
//! Priority calculation and pattern matching for refactor suggestions.

use crate::project_analysis::{ProjectAnalysisEngine, RefactorKind, RefactorSuggestion};

impl ProjectAnalysisEngine {
    /// Calculate priority score for a refactor suggestion
    pub fn calculate_suggestion_priority(&self, suggestion: &RefactorSuggestion) -> f32 {
        match suggestion.kind {
            RefactorKind::SplitFile => {
                // High priority for very large files
                if let Some(loc) = suggestion.metrics.get("loc") {
                    loc.as_u64().unwrap_or(0) as f32 * 0.1
                } else {
                    50.0
                }
            }
            RefactorKind::ExtractFacade => {
                // Medium-high priority for architectural issues
                if let Some(fan_in) = suggestion.metrics.get("fan_in") {
                    fan_in.as_u64().unwrap_or(0) as f32 * 2.0
                } else {
                    40.0
                }
            }
            RefactorKind::ReduceCycle => {
                // High priority for circular dependencies
                if let Some(strength) = suggestion.metrics.get("cycle_strength") {
                    strength.as_u64().unwrap_or(0) as f32 * 5.0
                } else {
                    60.0
                }
            }
            RefactorKind::PruneDeadCode => {
                // Lower priority for cleanup
                if let Some(count) = suggestion.metrics.get("dead_entities") {
                    count.as_u64().unwrap_or(0) as f32 * 1.0
                } else {
                    20.0
                }
            }
            RefactorKind::SimplifyDependency => {
                // Medium priority for complexity
                if let Some(deps) = suggestion.metrics.get("distinct_dependencies") {
                    deps.as_u64().unwrap_or(0) as f32 * 1.5
                } else {
                    30.0
                }
            }
        }
    }
}
