//! Phase 9: Reasoning Reflection TDD Tests
//!
//! These tests MUST FAIL before implementation and PASS after implementation.
//! Tests ensure reflection is deterministic, passive, and additive only.

use syncore::mcp_server::reasoning::{
    ReasoningMetadata, ReasoningTrace, ReasoningTraceStage, ReasoningEvaluation,
    build_reflection, normalize_reflection, ReasoningReflection,
};
use serde_json;

/// Test 1: Reflection is deterministic for same input
#[tokio::test]
async fn reflection_is_deterministic_for_same_input() {
    let metadata = create_test_metadata();
    let trace = create_test_trace();
    let evaluation = create_test_evaluation();

    // Build reflection twice with identical inputs
    let reflection1 = build_reflection(&metadata, &trace, &evaluation);
    let reflection2 = build_reflection(&metadata, &trace, &evaluation);

    // Should be identical (deterministic)
    assert_eq!(reflection1, reflection2, "Reflection should be deterministic for identical inputs");

    // After normalization should still be identical
    let normalized1 = normalize_reflection(reflection1);
    let normalized2 = normalize_reflection(reflection2);
    assert_eq!(normalized1, normalized2, "Normalized reflection should be deterministic");
}

/// Test 2: Reflection classifies stable reasoning correctly
#[tokio::test]
async fn reflection_classifies_stable_reasoning_correctly() {
    let metadata = create_test_metadata();
    let trace = create_test_trace();
    let evaluation = ReasoningEvaluation {
        score: 95,
        confidence: 0.98,
        anomaly_flags: vec![],
        summary: "Excellent execution".to_string(),
    };

    let reflection = build_reflection(&metadata, &trace, &evaluation);

    // High score, no anomalies should be "stable"
    assert_eq!(reflection.category, "stable", "High score with no anomalies should be stable");
    assert!(reflection.regression_risk <= 0.1, "Stable reasoning should have low regression risk");
    assert_eq!(reflection.recommended_top_k_delta, 0, "Stable reasoning should recommend no top_k changes");
    assert!(reflection.improvement_hints.is_empty(), "Stable reasoning should have no improvement hints");
}

/// Test 3: Reflection flags high risk patterns
#[tokio::test]
async fn reflection_flags_high_risk_patterns() {
    let metadata = create_test_metadata();
    let trace = create_test_trace();
    let evaluation = ReasoningEvaluation {
        score: 45,
        confidence: 0.3,
        anomaly_flags: vec![
            "missing_stage:graph_traversal".to_string(),
            "erratic_timing:graph_traversal_much_slower".to_string(),
            "failed_stage:processing".to_string(),
        ],
        summary: "Poor execution".to_string(),
    };

    let reflection = build_reflection(&metadata, &trace, &evaluation);

    // Low score with multiple anomalies should be "anomalous"
    assert_eq!(reflection.category, "anomalous", "Low score with anomalies should be anomalous");
    assert!(reflection.regression_risk >= 0.7, "Anomalous reasoning should have high regression risk");
    assert!(!reflection.improvement_hints.is_empty(), "Anomalous reasoning should provide improvement hints");

    // Should suggest specific improvements based on anomalies
    let hints_text = reflection.improvement_hints.join(" ");
    assert!(hints_text.contains("verify") || hints_text.contains("check"),
              "Should suggest verification or checking");
}

/// Test 4: Reflection recommends scope adjustments when appropriate
#[tokio::test]
async fn reflection_recommends_scope_adjustments_when_appropriate() {
    let metadata = create_test_metadata();
    let trace = create_test_trace();
    let evaluation = ReasoningEvaluation {
        score: 75,
        confidence: 0.8,
        anomaly_flags: vec!["sparse_results:few_entities".to_string()],
        summary: "Acceptable execution".to_string(),
    };

    let reflection = build_reflection(&metadata, &trace, &evaluation);

    // Medium score with sparse results should suggest broader search
    assert_eq!(reflection.recommended_top_k_delta, 5, "Sparse results should recommend +5 top_k");
    assert!(reflection.recommended_scope_hint.is_some(), "Should recommend scope adjustment");

    let scope_hint = reflection.recommended_scope_hint.as_ref().unwrap();
    assert!(scope_hint.contains("widen") || scope_hint.contains("expand"),
              "Should recommend widening scope");
}

/// Test 5: Reflection proposes top_k adjustments for sparse results
#[tokio::test]
async fn reflection_proposes_top_k_adjustments_for_sparse_results() {
    let metadata = create_test_metadata_with_few_results(3); // Only 3 results
    let trace = create_test_trace();
    let evaluation = ReasoningEvaluation {
        score: 80,
        confidence: 0.85,
        anomaly_flags: vec!["sparse_results:few_entities".to_string()],
        summary: "Good execution with limited results".to_string(),
    };

    let reflection = build_reflection(&metadata, &trace, &evaluation);

    // Should recommend increasing top_k for sparse results
    assert!(reflection.recommended_top_k_delta > 0, "Sparse results should recommend positive top_k delta");
    assert!(reflection.recommended_top_k_delta <= 10, "Top_k delta should be reasonable");
}

/// Test 6: Reflection handles failed executions gracefully
#[tokio::test]
async fn reflection_handles_failed_executions_gracefully() {
    let metadata = create_test_metadata();
    let mut trace = create_test_trace();
    // Add a failed stage
    trace.stages.push(ReasoningTraceStage {
        stage: "processing".to_string(),
        ok: false,
        detail: "Execution failed".to_string(),
        timestamp_ms: 2000,
    });

    let evaluation = ReasoningEvaluation {
        score: 30, // Low due to failure caps
        confidence: 0.4, // Low due to failure caps
        anomaly_flags: vec!["execution_failure".to_string()],
        summary: "Failed execution".to_string(),
    };

    let reflection = build_reflection(&metadata, &trace, &evaluation);

    // Failed execution should be marked as anomalous
    assert_eq!(reflection.category, "anomalous", "Failed execution should be anomalous");
    assert!(reflection.regression_risk >= 0.9, "Failed execution should have very high regression risk");

    // Should provide actionable hints for failed executions
    let hints_text = reflection.improvement_hints.join(" ");
    assert!(hints_text.contains("verify") || hints_text.contains("check") || hints_text.contains("query"),
              "Failed execution should provide actionable verification hints");
}

/// Test 7: Reflection serialization roundtrip
#[tokio::test]
async fn reflection_serialization_roundtrip() {
    let metadata = create_test_metadata();
    let trace = create_test_trace();
    let evaluation = create_test_evaluation();

    let reflection = build_reflection(&metadata, &trace, &evaluation);
    let normalized_reflection = normalize_reflection(reflection);

    // Serialize and deserialize
    let reflection_json = serde_json::to_string_pretty(&normalized_reflection).unwrap();
    let deserialized_reflection: ReasoningReflection = serde_json::from_str(&reflection_json).unwrap();

    assert_eq!(normalized_reflection, deserialized_reflection,
              "Reflection should serialize/deserialize correctly");

    // All required fields should be present
    assert!(!deserialized_reflection.category.is_empty(), "Category should not be empty");
    assert!(deserialized_reflection.regression_risk >= 0.0 && deserialized_reflection.regression_risk <= 1.0,
              "Regression risk should be in valid range");
}

/// Test 8: Reflection does not modify original results
#[tokio::test]
async fn reflection_does_not_modify_original_results() {
    let metadata = create_test_metadata();
    let trace = create_test_trace();
    let evaluation = create_test_evaluation();

    // Clone originals for comparison
    let original_metadata = metadata.clone();
    let original_trace = trace.clone();
    let original_evaluation = evaluation.clone();

    // Build reflection (this should not modify inputs)
    let _reflection = build_reflection(&metadata, &trace, &evaluation);
    let _normalized_reflection = normalize_reflection(_reflection);

    // Verify originals are unchanged
    assert_eq!(metadata.request_id, original_metadata.request_id);
    assert_eq!(trace.stages.len(), original_trace.stages.len());
    assert_eq!(evaluation.score, original_evaluation.score);
    assert_eq!(evaluation.confidence, original_evaluation.confidence);
}

// Helper functions for creating test data

fn create_test_metadata() -> ReasoningMetadata {
    ReasoningMetadata {
        request_id: "test_request_123".to_string(),
        backend_used: "SQLiteGraph".to_string(),
        start_time_ms: 1000,
        end_time_ms: 1500,
        vector_search_ms: Some(200),
        graph_traversal_ms: Some(300),
        fusion_ms: None,
        parameters: serde_json::json!({"query": "test"}),
        debug_flags: vec!["parsing:ok".to_string(), "execution:ok".to_string()],
    }
}

fn create_test_metadata_with_few_results(result_count: usize) -> ReasoningMetadata {
    let mut metadata = create_test_metadata();
    metadata.parameters = serde_json::json!({
        "query": "test",
        "result_count": result_count
    });
    metadata
}

fn create_test_trace() -> ReasoningTrace {
    ReasoningTrace {
        stages: vec![
            ReasoningTraceStage {
                stage: "parsing".to_string(),
                ok: true,
                detail: "Request parsed successfully".to_string(),
                timestamp_ms: 1050,
            },
            ReasoningTraceStage {
                stage: "vector_search".to_string(),
                ok: true,
                detail: "Vector search completed".to_string(),
                timestamp_ms: 1250,
            },
            ReasoningTraceStage {
                stage: "graph_traversal".to_string(),
                ok: true,
                detail: "Graph traversal completed".to_string(),
                timestamp_ms: 1400,
            },
            ReasoningTraceStage {
                stage: "formatting".to_string(),
                ok: true,
                detail: "Response formatted".to_string(),
                timestamp_ms: 1450,
            },
        ],
        summary: "All stages completed successfully".to_string(),
        backend: "SQLiteGraph".to_string(),
        timing_breakdown: std::collections::HashMap::from([
            ("parsing".to_string(), 50),
            ("vector_search".to_string(), 200),
            ("graph_traversal".to_string(), 150),
            ("formatting".to_string(), 50),
        ]),
        parameters_hash: "test_hash_123".to_string(),
    }
}

fn create_test_evaluation() -> ReasoningEvaluation {
    ReasoningEvaluation {
        score: 85,
        confidence: 0.92,
        anomaly_flags: vec![],
        summary: "Good execution with minor issues".to_string(),
    }
}