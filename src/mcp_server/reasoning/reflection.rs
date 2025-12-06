//! Reasoning reflection module
//!
//! Provides passive reflection and adaptation suggestions for reasoning execution.
//! This layer analyzes ReasoningMetadata, ReasoningTrace, and ReasoningEvaluation
//! to provide structured insights without modifying the actual reasoning results.

use serde::{Deserialize, Serialize};

/// Reflection result for reasoning execution
///
/// Contains deterministic analysis of reasoning execution quality, risk assessment,
/// and actionable improvement suggestions. This is PASSIVE - it does not change
/// any reasoning results, only adds metadata for analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReasoningReflection {
    /// Reflection category: "stable", "degraded", "anomalous"
    pub category: String,
    /// Regression risk assessment (0.0-1.0, higher is riskier)
    pub regression_risk: f32,
    /// Recommended adjustment to top_k parameter (0 if no change recommended)
    pub recommended_top_k_delta: i32,
    /// Recommended scope adjustment (e.g., "narrow:local", "widen:project")
    pub recommended_scope_hint: Option<String>,
    /// Human-readable improvement hints
    pub improvement_hints: Vec<String>,
}

impl ReasoningReflection {
    /// Create a new reflection with default values
    pub fn new() -> Self {
        Self {
            category: "stable".to_string(),
            regression_risk: 0.0,
            recommended_top_k_delta: 0,
            recommended_scope_hint: None,
            improvement_hints: Vec::new(),
        }
    }

    /// Create a reflection with the specified category and risk
    pub fn with_category_risk(category: &str, regression_risk: f32) -> Self {
        Self {
            category: category.to_string(),
            regression_risk: regression_risk.clamp(0.0, 1.0),
            recommended_top_k_delta: 0,
            recommended_scope_hint: None,
            improvement_hints: Vec::new(),
        }
    }
}

/// Build reflection from reasoning metadata, trace, and evaluation
///
/// Analyzes the reasoning execution to determine stability, risk factors,
/// and improvement suggestions. All outputs are deterministic based on inputs.
///
/// # Arguments
/// * `metadata` - Reasoning execution metadata
/// * `trace` - Reasoning execution trace
/// * `evaluation` - Reasoning evaluation results
///
/// # Returns
/// Deterministic ReasoningReflection based on analysis
pub fn build_reflection(
    metadata: &super::ReasoningMetadata,
    trace: &super::ReasoningTrace,
    evaluation: &super::ReasoningEvaluation,
) -> ReasoningReflection {
    let mut reflection = ReasoningReflection::new();

    // Determine category based on score and anomalies
    if evaluation.score >= 90 && evaluation.anomaly_flags.is_empty() {
        // Stable: high score, no anomalies
        reflection.category = "stable".to_string();
        reflection.regression_risk = 0.05; // Low risk
    } else if evaluation.score >= 70 && evaluation.anomaly_flags.len() <= 2 {
        // Degraded: medium score or minor issues
        reflection.category = "degraded".to_string();
        reflection.regression_risk = 0.4 + (evaluation.anomaly_flags.len() as f32 * 0.1);
    } else {
        // Anomalous: low score or major issues
        reflection.category = "anomalous".to_string();
        reflection.regression_risk = 0.7 + (evaluation.anomaly_flags.len() as f32 * 0.05);
        reflection.regression_risk = reflection.regression_risk.min(1.0);
    }

    // Check for execution failures in trace
    let has_failed_stage = trace.stages.iter().any(|stage| !stage.ok);
    if has_failed_stage {
        reflection.category = "anomalous".to_string();
        reflection.regression_risk = reflection.regression_risk.max(0.9);
        reflection.improvement_hints.push("verify query constraints".to_string());
        reflection.improvement_hints.push("check backend health".to_string());
    }

    // Analyze timing patterns for scope hints
    let has_timing_issues = evaluation.anomaly_flags.iter()
        .any(|flag| flag.contains("timing") || flag.contains("slow"));

    if has_timing_issues {
        // Check if graph traversal was much slower than vector search
        if let (Some(vector_ms), Some(graph_ms)) = (metadata.vector_search_ms, metadata.graph_traversal_ms) {
            if vector_ms > 0 && graph_ms > vector_ms * 3 {
                reflection.recommended_scope_hint = Some("narrow:local".to_string());
                reflection.improvement_hints.push("consider local-only search".to_string());
            }
        }
    }

    // Analyze result count from metadata if available
    if let Some(result_count) = extract_result_count_from_metadata(metadata) {
        if result_count < 5 {
            reflection.recommended_top_k_delta = 5;
            reflection.recommended_scope_hint = Some("widen:project".to_string());
            reflection.improvement_hints.push("increase search scope".to_string());
        }
    }

    // Generate specific improvement hints based on anomalies
    for flag in &evaluation.anomaly_flags {
        if flag.contains("missing_stage") {
            reflection.improvement_hints.push(format!("verify pipeline completeness for {}", flag));
        } else if flag.contains("unordered_stage") {
            reflection.improvement_hints.push("review stage execution order".to_string());
        } else if flag.contains("invalid_timing") {
            reflection.improvement_hints.push("profile timing bottlenecks".to_string());
        } else if flag.contains("fusion") {
            reflection.improvement_hints.push("check fusion configuration".to_string());
        }
    }

    // Add confidence-based hints
    if evaluation.confidence < 0.5 {
        reflection.improvement_hints.push("increase result confidence with broader queries".to_string());
    }

    reflection
}

/// Normalize reflection values to ensure consistency
///
/// Ensures all values are within valid ranges and sorts improvement hints.
///
/// # Arguments
/// * `reflection` - Reflection to normalize
///
/// # Returns
/// Normalized ReasoningReflection
pub fn normalize_reflection(mut reflection: ReasoningReflection) -> ReasoningReflection {
    // Clamp regression risk to valid range
    reflection.regression_risk = reflection.regression_risk.clamp(0.0, 1.0);

    // Limit recommended top_k delta to reasonable range
    reflection.recommended_top_k_delta = reflection.recommended_top_k_delta.clamp(-20, 50);

    // Sort and deduplicate improvement hints
    reflection.improvement_hints.sort();
    reflection.improvement_hints.dedup();

    // Limit improvement hints to prevent overly verbose output
    if reflection.improvement_hints.len() > 8 {
        reflection.improvement_hints.truncate(8);
    }

    // Ensure category is valid
    match reflection.category.as_str() {
        "stable" | "degraded" | "anomalous" => {
            // Valid category
        }
        _ => {
            // Default to stable for unknown categories
            reflection.category = "stable".to_string();
            reflection.regression_risk = reflection.regression_risk.min(0.1);
        }
    }

    reflection
}

/// Extract result count from metadata parameters if available
///
/// Helper function to parse result count from ReasoningMetadata parameters.
/// This is used to determine if results were sparse.
///
/// # Arguments
/// * `metadata` - Reasoning metadata containing parameters
///
/// # Returns
/// Result count if found, None if not available
fn extract_result_count_from_metadata(metadata: &super::ReasoningMetadata) -> Option<usize> {
    // Try to extract result_count from parameters
    if let Some(result_count_value) = metadata.parameters.get("result_count") {
        if let Some(count) = result_count_value.as_u64() {
            return Some(count as usize);
        }
    }

    // Check for top_k parameter as a proxy for result count
    if let Some(top_k_value) = metadata.parameters.get("top_k") {
        if let Some(top_k) = top_k_value.as_u64() {
            return Some(top_k as usize);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_server::reasoning::{ReasoningMetadata, ReasoningTrace, ReasoningTraceStage, ReasoningEvaluation};

    #[test]
    fn test_reflection_new() {
        let reflection = ReasoningReflection::new();
        assert_eq!(reflection.category, "stable");
        assert_eq!(reflection.regression_risk, 0.0);
        assert_eq!(reflection.recommended_top_k_delta, 0);
        assert!(reflection.recommended_scope_hint.is_none());
        assert!(reflection.improvement_hints.is_empty());
    }

    #[test]
    fn test_reflection_with_category_risk() {
        let reflection = ReasoningReflection::with_category_risk("anomalous", 0.8);
        assert_eq!(reflection.category, "anomalous");
        assert_eq!(reflection.regression_risk, 0.8);
    }

    #[test]
    fn test_reflection_clamping() {
        let reflection = ReasoningReflection::with_category_risk("stable", 1.5);
        assert_eq!(reflection.regression_risk, 1.0);
    }
}