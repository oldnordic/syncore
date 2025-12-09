//! Reasoning evaluation layer
//!
//! Provides deterministic, machine-auditable evaluation scores, anomaly flags,
//! and confidence ratings based solely on metadata and trace analysis.

use serde::{Deserialize, Serialize};

/// Evaluation result for reasoning execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReasoningEvaluation {
    /// Deterministic score (0-100) - higher is better
    pub score: u8,
    /// Confidence rating (0.0-1.0) - higher is more reliable
    pub confidence: f32,
    /// Anomaly flags indicating issues found
    pub anomaly_flags: Vec<String>,
    /// Brief 1-2 line summary of evaluation
    pub summary: String,
}

impl ReasoningEvaluation {
    /// Create a new evaluation with default values
    pub fn new() -> Self {
        Self {
            score: 100,
            confidence: 1.0,
            anomaly_flags: Vec::new(),
            summary: "Perfect execution".to_string(),
        }
    }

    /// Clamp score to valid range [0, 100]
    fn clamp_score(score: i32) -> u8 {
        if score < 0 {
            0
        } else if score > 100 {
            100
        } else {
            score as u8
        }
    }

    /// Clamp confidence to valid range [0.0, 1.0]
    fn clamp_confidence(confidence: f32) -> f32 {
        if confidence < 0.0 {
            0.0
        } else if confidence > 1.0 {
            1.0
        } else {
            confidence
        }
    }

    /// Generate summary based on score and anomaly flags
    fn generate_summary(score: u8, anomaly_flags: &[String]) -> String {
        if anomaly_flags.is_empty() {
            match score {
                95..=100 => "Excellent execution with no anomalies".to_string(),
                80..=94 => "Good execution with minor issues".to_string(),
                60..=79 => "Acceptable execution with notable issues".to_string(),
                40..=59 => "Poor execution with significant issues".to_string(),
                _ => "Failed execution with critical issues".to_string(),
            }
        } else {
            let issue_count = anomaly_flags.len();
            if score >= 60 {
                format!("Execution completed with {} issues", issue_count)
            } else {
                format!("Execution failed with {} critical issues", issue_count)
            }
        }
    }

    /// Detect missing required stages
    fn detect_missing_stages(trace: &super::trace::ReasoningTrace) -> Vec<String> {
        let trace_stages: std::collections::HashSet<&str> =
            trace.stages.iter().map(|s| s.stage.as_str()).collect();

        let required_stages = vec!["parsing", "vector_search", "graph_traversal", "formatting"];
        let mut flags = Vec::new();

        for stage in required_stages {
            if !trace_stages.contains(stage) {
                flags.push(format!("missing_stage:{}", stage));
            }
        }

        flags
    }

    /// Detect unordered stages (stages not in expected order)
    fn detect_unordered_stages(trace: &super::trace::ReasoningTrace) -> Vec<String> {
        let _expected_order = vec!["parsing", "vector_search", "graph_traversal", "formatting"];
        let mut flags = Vec::new();

        // Only check for unordered stages if we have at least 2 stages
        if trace.stages.len() < 2 {
            return flags;
        }

        // Check if fusion stage appears in wrong position (if present)
        let mut fusion_found = false;
        let mut graph_traversal_found = false;

        for (_i, stage) in trace.stages.iter().enumerate() {
            match stage.stage.as_str() {
                "fusion" => {
                    fusion_found = true;
                    // Fusion should come after graph_traversal
                    if !graph_traversal_found {
                        flags.push(format!("unordered_stage:fusion_before_graph_traversal"));
                    }
                }
                "graph_traversal" => {
                    graph_traversal_found = true;
                    // If we've already seen fusion and now seeing graph_traversal, it's unordered
                    if fusion_found {
                        flags.push(format!("unordered_stage:graph_traversal_after_fusion"));
                    }
                }
                _ => {}
            }
        }

        flags
    }

    /// Detect invalid timing values
    fn detect_invalid_timing(
        metadata: &super::metadata::ReasoningMetadata,
        trace: &super::trace::ReasoningTrace,
    ) -> Vec<String> {
        let mut flags = Vec::new();
        let total_duration = metadata.end_time_ms.saturating_sub(metadata.start_time_ms);

        // Check timing breakdown against total duration
        for (stage_name, timing) in &trace.timing_breakdown {
            if *timing > total_duration {
                flags.push(format!("invalid_timing:{}:greater_than_total", stage_name));
            }
        }

        // Check vector timing against metadata
        if let (Some(vector_ms), Some(trace_vector_ms)) =
            (metadata.vector_search_ms, trace.timing_breakdown.get("vector_search"))
        {
            let trace_vector_ms = *trace_vector_ms as u128;
            let ratio = if vector_ms > 0 {
                trace_vector_ms as f64 / vector_ms as f64
            } else {
                0.0
            };

            // Allow some tolerance for timing differences (within 50%)
            if ratio > 1.5 || ratio < 0.5 {
                flags.push("invalid_timing:vector_search_mismatch_with_metadata".to_string());
            }
        }

        flags
    }

    /// Detect fusion-specific issues
    fn detect_fusion_issues(
        metadata: &super::metadata::ReasoningMetadata,
        trace: &super::trace::ReasoningTrace,
    ) -> Vec<String> {
        let mut flags = Vec::new();

        // If fusion was performed (metadata has fusion_ms), trace should have fusion stage
        if metadata.fusion_ms.is_some() {
            let has_fusion_stage = trace.stages.iter().any(|s| s.stage == "fusion");
            if !has_fusion_stage {
                flags.push("missing_stage:fusion".to_string());
            }
        }

        flags
    }

    /// Detect timing-based confidence issues
    fn detect_timing_confidence_issues(
        metadata: &super::metadata::ReasoningMetadata,
    ) -> Vec<String> {
        let mut flags = Vec::new();

        // If graph traversal took significantly longer than vector search, mark as erratic
        if let (Some(vector_ms), Some(graph_ms)) =
            (metadata.vector_search_ms, metadata.graph_traversal_ms)
        {
            if vector_ms > 0 {
                let ratio = graph_ms as f64 / vector_ms as f64;
                if ratio > 2.0 {
                    flags.push("erratic_timing:graph_traversal_much_slower".to_string());
                }
            }
        }

        flags
    }
}

/// Evaluate reasoning execution based on metadata and trace
///
/// This function provides deterministic, machine-auditable evaluation without
/// influencing execution. It is purely analytical → metadata_in → evaluation_out.
pub fn evaluate_reasoning(
    metadata: &super::metadata::ReasoningMetadata,
    trace: &super::trace::ReasoningTrace,
) -> ReasoningEvaluation {
    let mut evaluation = ReasoningEvaluation::new();
    let mut score_adjustment = 0i32;
    let mut confidence_adjustment = 0.0f32;

    // Detect various anomaly types
    evaluation.anomaly_flags.extend(ReasoningEvaluation::detect_missing_stages(trace));
    evaluation.anomaly_flags.extend(ReasoningEvaluation::detect_unordered_stages(trace));
    evaluation.anomaly_flags.extend(ReasoningEvaluation::detect_invalid_timing(metadata, trace));
    evaluation.anomaly_flags.extend(ReasoningEvaluation::detect_fusion_issues(metadata, trace));
    evaluation.anomaly_flags.extend(ReasoningEvaluation::detect_timing_confidence_issues(metadata));

    // Apply scoring penalties for detected anomalies
    for flag in &evaluation.anomaly_flags {
        if flag.starts_with("missing_stage:") {
            score_adjustment -= 25;
        } else if flag.starts_with("unordered_stage:") {
            score_adjustment -= 15;
        } else if flag.starts_with("invalid_timing:") {
            score_adjustment -= 10;
        } else if flag.starts_with("erratic_timing:") {
            confidence_adjustment -= 0.1;
        }
    }

    // Special handling for fusion requested but missing
    if metadata.fusion_ms.is_some() && !trace.stages.iter().any(|s| s.stage == "fusion") {
        score_adjustment -= 10;
    }

    // Apply execution failure caps
    let failed_execution = trace.stages.iter().any(|s| !s.ok);
    if failed_execution {
        evaluation.score = evaluation.score.saturating_sub(40); // Cap at 60
        evaluation.confidence = (evaluation.confidence - 0.5).max(0.0); // Cap at 0.5
    }

    // Apply adjustments
    let raw_score = 100i32.saturating_sub(score_adjustment);
    evaluation.score = ReasoningEvaluation::clamp_score(raw_score);

    let raw_confidence = (1.0f32 - confidence_adjustment).max(0.0);
    evaluation.confidence = ReasoningEvaluation::clamp_confidence(raw_confidence);

    // Ensure failure caps are respected
    if failed_execution {
        evaluation.score = evaluation.score.min(60);
        evaluation.confidence = evaluation.confidence.min(0.5);
    }

    // Generate summary
    evaluation.summary =
        ReasoningEvaluation::generate_summary(evaluation.score, &evaluation.anomaly_flags);

    evaluation
}

/// Normalize evaluation to ensure consistent bounds and format
pub fn normalize_evaluation(mut evaluation: ReasoningEvaluation) -> ReasoningEvaluation {
    // Ensure score is in valid range
    evaluation.score = ReasoningEvaluation::clamp_score(evaluation.score as i32);

    // Ensure confidence is in valid range
    evaluation.confidence = ReasoningEvaluation::clamp_confidence(evaluation.confidence);

    // Deduplicate and sort anomaly flags for consistency
    evaluation.anomaly_flags.sort();
    evaluation.anomaly_flags.dedup();

    // Ensure summary is not empty and is reasonably sized
    if evaluation.summary.is_empty() {
        evaluation.summary = "Evaluation completed".to_string();
    } else if evaluation.summary.len() > 200 {
        evaluation.summary.truncate(197);
        evaluation.summary.push_str("...");
    }

    evaluation
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_server::reasoning::metadata::ReasoningMetadata;
    use crate::mcp_server::reasoning::trace::{ReasoningTrace, ReasoningTraceStage};
    use std::collections::HashMap;

    fn create_test_metadata() -> ReasoningMetadata {
        ReasoningMetadata {
            request_id: "test".to_string(),
            backend_used: "SQLiteGraph".to_string(),
            start_time_ms: 1000,
            end_time_ms: 2000,
            vector_search_ms: Some(200),
            graph_traversal_ms: Some(300),
            fusion_ms: None,
            parameters: serde_json::json!({}),
            debug_flags: vec!["parsing:ok".to_string()],
        }
    }

    fn create_test_trace() -> ReasoningTrace {
        ReasoningTrace {
            stages: vec![
                ReasoningTraceStage::success("parsing", "parsed"),
                ReasoningTraceStage::success("vector_search", "found"),
                ReasoningTraceStage::success("graph_traversal", "traversed"),
                ReasoningTraceStage::success("formatting", "formatted"),
            ],
            summary: "Test trace".to_string(),
            backend: "SQLiteGraph".to_string(),
            timing_breakdown: {
                let mut map = HashMap::new();
                map.insert("vector_search".to_string(), 200u128);
                map.insert("graph_traversal".to_string(), 300u128);
                map
            },
            parameters_hash: "test_hash".to_string(),
        }
    }

    #[test]
    fn test_perfect_execution_evaluation() {
        let metadata = create_test_metadata();
        let trace = create_test_trace();

        let evaluation = evaluate_reasoning(&metadata, &trace);

        assert_eq!(evaluation.score, 100);
        assert_eq!(evaluation.confidence, 1.0);
        assert!(evaluation.anomaly_flags.is_empty());
        assert!(evaluation.summary.contains("Excellent"));
    }

    #[test]
    fn test_missing_stage_detection() {
        let mut trace = create_test_trace();
        // Remove a stage
        trace.stages.remove(2); // Remove graph_traversal

        let metadata = create_test_metadata();
        let evaluation = evaluate_reasoning(&metadata, &trace);

        assert!(evaluation.score < 100);
        assert!(evaluation
            .anomaly_flags
            .iter()
            .any(|f| f.contains("missing_stage:graph_traversal")));
    }

    #[test]
    fn test_fusion_missing_penalty() {
        let mut metadata = create_test_metadata();
        metadata.fusion_ms = Some(400); // Request fusion

        let trace = create_test_trace();
        // Don't add fusion stage

        let evaluation = evaluate_reasoning(&metadata, &trace);

        // Should have penalty for missing fusion stage and missing_stage flag
        assert!(evaluation.score < 90); // Should have -10 penalty
        assert!(evaluation.anomaly_flags.iter().any(|f| f.contains("missing_stage:fusion")));
    }

    #[test]
    fn test_failed_execution_caps() {
        let mut trace = create_test_trace();
        trace.stages.push(ReasoningTraceStage::failure("processing", "failed to process"));

        let metadata = create_test_metadata();
        let evaluation = evaluate_reasoning(&metadata, &trace);

        assert_eq!(evaluation.score, 60); // Should be capped at 60
        assert_eq!(evaluation.confidence, 0.5); // Should be capped at 0.5
        assert!(evaluation.summary.contains("Failed"));
    }

    #[test]
    fn test_normalize_evaluation() {
        let evaluation = ReasoningEvaluation {
            score: 150,      // Invalid > 100
            confidence: 1.5, // Invalid > 1.0
            anomaly_flags: vec!["flag1".to_string(), "flag2".to_string(), "flag1".to_string()], // Duplicate
            summary: "".to_string(), // Empty
        };

        let normalized = normalize_evaluation(evaluation);

        assert_eq!(normalized.score, 100); // Should be clamped
        assert_eq!(normalized.confidence, 1.0); // Should be clamped
        assert_eq!(normalized.anomaly_flags.len(), 2); // Should deduplicate
        assert_eq!(normalized.summary, "Evaluation completed"); // Should have default
    }

    #[test]
    fn test_deterministic_evaluation() {
        let metadata = create_test_metadata();
        let trace = create_test_trace();

        let eval1 = evaluate_reasoning(&metadata, &trace);
        let eval2 = evaluate_reasoning(&metadata, &trace);

        assert_eq!(eval1, eval2, "Identical inputs should produce identical evaluations");
    }

    #[test]
    fn test_timing_confidence_reduction() {
        let mut metadata = create_test_metadata();
        // Set graph traversal much slower than vector search
        metadata.graph_traversal_ms = Some(1000); // 5x slower than vector search (200)

        let trace = create_test_trace();

        let evaluation = evaluate_reasoning(&metadata, &trace);

        assert!(evaluation.confidence < 0.9, "Confidence should be reduced for erratic timing");
        assert!(evaluation.anomaly_flags.iter().any(|f| f.contains("erratic_timing")));
    }
}
