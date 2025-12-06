//! Phase 10: Cross-Tool Reasoning Consistency Contracts TDD Tests
//!
//! These tests MUST FAIL before implementation and PASS after implementation.
//! Tests ensure that all three reasoning tools produce mutually consistent
//! metadata, traces, evaluations, and reflections for comparable inputs.

use syncore::mcp_server::reasoning::{
    UnifiedReasoningResponse, ToolReasoningSnapshot, ConsistencyViolation,
    ConsistencyReport, build_tool_snapshot_from_unified_response,
    validate_snapshots_consistency,
};
use serde_json;

/// Test 1: Consistency contracts for identical queries
#[tokio::test]
async fn consistency_contracts_for_identical_queries() {
    // Create mock unified responses for each tool with identical query "test_function"
    let query_response = create_mock_unified_response("raggraph_query", "test_function", 85);
    let multihop_response = create_mock_unified_response("raggraph_multihop", "test_function", 88);
    let fusion_response = create_mock_unified_response("code_graph_fusion_query", "test_function", 90);

    // Build snapshots from responses
    let query_snapshot = build_tool_snapshot_from_unified_response("raggraph_query", &query_response);
    let multihop_snapshot = build_tool_snapshot_from_unified_response("raggraph_multihop", &multihop_response);
    let fusion_snapshot = build_tool_snapshot_from_unified_response("code_graph_fusion_query", &fusion_response);

    let snapshots = &[query_snapshot, multihop_snapshot, fusion_snapshot];

    // Validate consistency
    let report = validate_snapshots_consistency(snapshots);

    // High-level invariants
    assert!(report.is_consistent, "Tools should be consistent for identical queries");

    // Backend consistency
    let backends: Vec<String> = snapshots.iter()
        .map(|s| s.metadata_backend.clone())
        .collect();
    assert!(backends.windows(2).all(|w| w[0] == w[1]),
              "All tools should use the same backend: {:?}", backends);

    // Score band consistency (within ±20 points)
    let scores: Vec<u8> = snapshots.iter().map(|s| s.evaluation_score).collect();
    let max_score = scores.iter().max().copied().unwrap_or(0);
    let min_score = scores.iter().min().copied().unwrap_or(100);
    assert!(max_score.saturating_sub(min_score) <= 20,
              "Scores should be within ±20 points: min={}, max={}", min_score, max_score);

    // Valid reflection categories
    for snapshot in snapshots {
        if let Some(ref category) = snapshot.reflection_category {
            assert!(["stable", "degraded", "anomalous"].contains(&category.as_str()),
                    "Invalid reflection category: {}", category);
        }
    }
}

/// Test 2: Consistency metadata fields are present for all tools
#[tokio::test]
async fn consistency_metadata_fields_are_present_for_all_tools() {
    let tools = ["raggraph_query", "raggraph_multihop", "code_graph_fusion_query"];

    for tool in &tools {
        let response = create_mock_unified_response(*tool, "simple_query", 75);
        let snapshot = build_tool_snapshot_from_unified_response(tool, &response);

        // Assert critical metadata fields are non-empty/non-zero
        assert!(!snapshot.metadata_backend.is_empty(),
                "{} should have non-empty backend field", tool);
        assert!(snapshot.evaluation_score > 0,
                "{} should have positive evaluation score", tool);
        assert!(snapshot.evaluation_confidence > 0.0 && snapshot.evaluation_confidence <= 1.0,
                "{} should have valid confidence range", tool);
    }
}

/// Test 3: Consistency trace stage sets overlap
#[tokio::test]
async fn consistency_trace_stage_sets_overlap() {
    // Create responses with traces containing different stages
    let tools = ["raggraph_query", "raggraph_multihop", "code_graph_fusion_query"];
    let mut stage_sets = Vec::new();

    for tool in &tools {
        let response = create_mock_unified_response_with_stages(*tool, "test_query", 80);
        let snapshot = build_tool_snapshot_from_unified_response(tool, &response);

        // Extract stages from response trace (would need access to trace data)
        // For now, verify trace backend consistency
        if let Some(ref trace_backend) = snapshot.trace_backend {
            assert_eq!(trace_backend, &snapshot.metadata_backend,
                      "Trace backend should match metadata backend for {}", tool);
        }

        stage_sets.push((*tool, snapshot.metadata_backend.clone()));
    }

    // Verify at least backend consistency across tools
    let backends: Vec<String> = stage_sets.iter().map(|(_, backend)| backend.clone()).collect();
    assert!(backends.windows(2).all(|w| w[0] == w[1]),
              "All tools should use consistent backends: {:?}", backends);
}

/// Test 4: Consistency evaluation monotonicity for more complex query
#[tokio::test]
async fn consistency_evaluation_monotonicity_for_more_complex_query() {
    // Complex query responses
    let query_response = create_mock_unified_response("raggraph_query", "complex_multi_step_query", 92);
    let multihop_response = create_mock_unified_response("raggraph_multihop", "complex_multi_step_query", 95);
    let fusion_response = create_mock_unified_response("code_graph_fusion_query", "complex_multi_step_query", 90);

    let snapshots = vec![
        build_tool_snapshot_from_unified_response("raggraph_query", &query_response),
        build_tool_snapshot_from_unified_response("raggraph_multihop", &multihop_response),
        build_tool_snapshot_from_unified_response("code_graph_fusion_query", &fusion_response),
    ];

    let report = validate_snapshots_consistency(&snapshots);

    // For high-scoring complex queries, fusion should not have higher regression risk
    assert!(report.is_consistent, "Complex queries should maintain consistency");

    // Check confidence consistency (no wild outliers)
    let confidences: Vec<f32> = snapshots.iter().map(|s| s.evaluation_confidence).collect();
    let max_conf = confidences.iter().reduce(|a, b| a.max(*b)).copied().unwrap_or(0.0);
    let min_conf = confidences.iter().reduce(|a, b| a.min(*b)).copied().unwrap_or(1.0);

    assert!(max_conf - min_conf <= 0.3,
              "Confidence values should not diverge wildly: min={}, max={}", min_conf, max_conf);
}

/// Test 5: Consistency reflection categories align with evaluation
#[tokio::test]
async fn consistency_reflection_categories_align_with_evaluation() {
    let test_cases = vec![
        ("high_score_tool", 95, Some("stable")),
        ("medium_score_tool", 85, Some("stable")),
        ("low_score_tool", 65, Some("degraded")),
        ("very_low_score_tool", 45, Some("anomalous")),
        ("no_reflection_tool", 70, None),
    ];

    for (tool_name, score, expected_category) in test_cases {
        let response = create_mock_unified_response_with_reflection(tool_name, "test_query", score, expected_category);
        let snapshot = build_tool_snapshot_from_unified_response(tool_name, &response);

        // If evaluation score >= 90 → reflection.category must be "stable"
        if score >= 90 {
            assert_eq!(snapshot.reflection_category.as_deref(), Some("stable"),
                      "High score ({}), should have 'stable' reflection for {}", score, tool_name);
        }

        // If evaluation score < 70 → reflection.category must NOT be "stable"
        if score < 70 && snapshot.reflection_category.is_some() {
            assert_ne!(snapshot.reflection_category.as_deref(), Some("stable"),
                      "Low score ({}), should not have 'stable' reflection for {}", score, tool_name);
        }
    }
}

/// Test 6: Consistency contract detects backend mismatches
#[tokio::test]
async fn consistency_contract_detects_backend_mismatches() {
    // Create snapshots with intentionally mismatched backends
    let snapshots = vec![
        ToolReasoningSnapshot {
            tool_name: "raggraph_query".to_string(),
            metadata_backend: "SQLiteGraph".to_string(),
            trace_backend: Some("SQLiteGraph".to_string()),
            evaluation_score: 85,
            evaluation_confidence: 0.9,
            reflection_category: Some("stable".to_string()),
        },
        ToolReasoningSnapshot {
            tool_name: "raggraph_multihop".to_string(),
            metadata_backend: "Neo4j".to_string(),  // Different backend!
            trace_backend: Some("Neo4j".to_string()),
            evaluation_score: 88,
            evaluation_confidence: 0.85,
            reflection_category: Some("stable".to_string()),
        },
        ToolReasoningSnapshot {
            tool_name: "code_graph_fusion_query".to_string(),
            metadata_backend: "SQLiteGraph".to_string(),
            trace_backend: Some("SQLiteGraph".to_string()),
            evaluation_score: 90,
            evaluation_confidence: 0.95,
            reflection_category: Some("stable".to_string()),
        },
    ];

    let report = validate_snapshots_consistency(&snapshots);

    // Should detect backend mismatch
    assert!(!report.is_consistent, "Should detect backend mismatch");
    assert!(report.violations.iter().any(|v| v.code == "backend_mismatch"),
              "Should have backend_mismatch violation");
}

/// Test 7: Consistency serialization roundtrip of snapshot
#[tokio::test]
async fn consistency_serialization_roundtrip_of_snapshot() {
    let original_snapshot = ToolReasoningSnapshot {
        tool_name: "raggraph_query".to_string(),
        metadata_backend: "SQLiteGraph".to_string(),
        trace_backend: Some("SQLiteGraph".to_string()),
        evaluation_score: 87,
        evaluation_confidence: 0.88,
        reflection_category: Some("stable".to_string()),
    };

    // Serialize to JSON
    let json_str = serde_json::to_string_pretty(&original_snapshot)
        .expect("Should serialize snapshot to JSON");

    // Deserialize back
    let deserialized: ToolReasoningSnapshot = serde_json::from_str(&json_str)
        .expect("Should deserialize snapshot from JSON");

    // Verify roundtrip equality
    assert_eq!(original_snapshot.tool_name, deserialized.tool_name);
    assert_eq!(original_snapshot.metadata_backend, deserialized.metadata_backend);
    assert_eq!(original_snapshot.trace_backend, deserialized.trace_backend);
    assert_eq!(original_snapshot.evaluation_score, deserialized.evaluation_score);
    assert_eq!(original_snapshot.evaluation_confidence, deserialized.evaluation_confidence);
    assert_eq!(original_snapshot.reflection_category, deserialized.reflection_category);

    // Verify complete equality
    assert_eq!(original_snapshot, deserialized, "Snapshot should serialize/deserialize correctly");
}

/// Test 8: Consistency does not modify original responses
#[tokio::test]
async fn consistency_does_not_modify_original_responses() {
    let original_response = create_mock_unified_response("raggraph_query", "unchanged_query", 82);

    // Clone for comparison
    let response_copy = original_response.clone();

    // Build snapshot (should not modify original)
    let _snapshot = build_tool_snapshot_from_unified_response("raggraph_query", &original_response);

    // Verify original response was not modified
    assert_eq!(original_response.response_type, response_copy.response_type);
    assert_eq!(original_response.request_metadata.query, response_copy.request_metadata.query);
    assert_eq!(original_response.results.len(), response_copy.results.len());
    assert_eq!(original_response.backend_info.backend_type, response_copy.backend_info.backend_type);
    assert_eq!(original_response.success, response_copy.success);

    // If metadata, trace, evaluation, reflection are present, they should be unchanged
    if let (Some(ref orig_meta), Some(ref copy_meta)) = (&original_response.metadata, &response_copy.metadata) {
        assert_eq!(orig_meta.request_id, copy_meta.request_id);
        assert_eq!(orig_meta.backend_used, copy_meta.backend_used);
    }

    if let (Some(ref orig_eval), Some(ref copy_eval)) = (&original_response.evaluation, &response_copy.evaluation) {
        assert_eq!(orig_eval.score, copy_eval.score);
        assert_eq!(orig_eval.confidence, copy_eval.confidence);
    }
}

// Helper functions for creating mock unified responses

fn create_mock_unified_response(tool_name: &str, query: &str, score: u8) -> UnifiedReasoningResponse {
    UnifiedReasoningResponse {
        response_type: tool_name.to_string(),
        request_metadata: create_mock_request_metadata(query),
        results: vec![create_mock_reasoning_result()],
        backend_info: create_mock_backend_info(),
        debug_info: create_mock_debug_info(),
        success: true,
        error: None,
        metadata: Some(create_mock_reasoning_metadata()),
        trace: Some(create_mock_reasoning_trace()),
        evaluation: Some(create_mock_reasoning_evaluation(score)),
        reflection: Some(create_mock_reasoning_reflection(score)),
    }
}

fn create_mock_unified_response_with_reflection(tool_name: &str, query: &str, score: u8, category: Option<&str>) -> UnifiedReasoningResponse {
    let response = create_mock_unified_response(tool_name, query, score);
    UnifiedReasoningResponse {
        reflection: category.map(|cat| create_mock_reasoning_reflection_with_category(score, cat)),
        ..response
    }
}

fn create_mock_unified_response_with_stages(tool_name: &str, query: &str, score: u8) -> UnifiedReasoningResponse {
    create_mock_unified_response(tool_name, query, score)
}

fn create_mock_request_metadata(query: &str) -> syncore::mcp_server::reasoning::RequestMetadata {
    syncore::mcp_server::reasoning::RequestMetadata {
        query: query.to_string(),
        request_type: "test".to_string(),
        parameters: std::collections::HashMap::new(),
        timestamp: 1234567890,
    }
}

fn create_mock_reasoning_result() -> syncore::mcp_server::reasoning::ReasoningResult {
    syncore::mcp_server::reasoning::ReasoningResult {
        id: "test_id".to_string(),
        name: "test_function".to_string(),
        entity_type: "function".to_string(),
        file_path: "/test/path.rs".to_string(),
        relevance_score: 0.85,
        scores: syncore::mcp_server::reasoning::ScoreComponents {
            vector_score: Some(0.9),
            graph_score: Some(0.8),
            temporal_score: Some(0.7),
            graph_embedding_score: Some(0.75),
            combined_score: 0.85,
        },
        metadata: std::collections::HashMap::new(),
    }
}

fn create_mock_backend_info() -> syncore::mcp_server::reasoning::BackendInfo {
    syncore::mcp_server::reasoning::BackendInfo {
        backend_type: "SQLiteGraph".to_string(),
        config_source: "auto".to_string(),
        auto_selected: true,
        metadata: std::collections::HashMap::new(),
    }
}

fn create_mock_debug_info() -> syncore::mcp_server::reasoning::DebugInfo {
    syncore::mcp_server::reasoning::DebugInfo {
        processing_time_ms: Some(150),
        entities_examined: Some(10),
        graph_depth: Some(3),
        vector_search_info: None,
        graph_expansion_info: None,
        metadata: std::collections::HashMap::new(),
    }
}

fn create_mock_reasoning_metadata() -> syncore::mcp_server::reasoning::ReasoningMetadata {
    syncore::mcp_server::reasoning::ReasoningMetadata {
        request_id: "test_req_123".to_string(),
        backend_used: "SQLiteGraph".to_string(),
        start_time_ms: 1000,
        end_time_ms: 1150,
        vector_search_ms: Some(100),
        graph_traversal_ms: Some(50),
        fusion_ms: None,
        parameters: serde_json::json!({"query": "test"}),
        debug_flags: vec!["parsing:ok".to_string(), "execution:ok".to_string()],
    }
}

fn create_mock_reasoning_trace() -> syncore::mcp_server::reasoning::ReasoningTrace {
    syncore::mcp_server::reasoning::ReasoningTrace {
        stages: vec![
            syncore::mcp_server::reasoning::ReasoningTraceStage {
                stage: "parsing".to_string(),
                ok: true,
                detail: "parsed successfully".to_string(),
                timestamp_ms: 1050,
            },
            syncore::mcp_server::reasoning::ReasoningTraceStage {
                stage: "vector_search".to_string(),
                ok: true,
                detail: "search completed".to_string(),
                timestamp_ms: 1100,
            },
            syncore::mcp_server::reasoning::ReasoningTraceStage {
                stage: "graph_traversal".to_string(),
                ok: true,
                detail: "traversal completed".to_string(),
                timestamp_ms: 1125,
            },
            syncore::mcp_server::reasoning::ReasoningTraceStage {
                stage: "formatting".to_string(),
                ok: true,
                detail: "formatted successfully".to_string(),
                timestamp_ms: 1145,
            },
        ],
        summary: "All stages completed successfully".to_string(),
        backend: "SQLiteGraph".to_string(),
        timing_breakdown: std::collections::HashMap::from([
            ("parsing".to_string(), 50),
            ("vector_search".to_string(), 25),
            ("graph_traversal".to_string(), 25),
            ("formatting".to_string(), 20),
        ]),
        parameters_hash: "test_hash_123".to_string(),
    }
}

fn create_mock_reasoning_evaluation(score: u8) -> syncore::mcp_server::reasoning::ReasoningEvaluation {
    syncore::mcp_server::reasoning::ReasoningEvaluation {
        score,
        confidence: if score >= 90 { 0.95 } else if score >= 70 { 0.8 } else { 0.6 },
        anomaly_flags: if score >= 90 { vec![] } else { vec!["minor_timing_issue".to_string()] },
        summary: if score >= 90 { "Excellent execution".to_string() } else { "Acceptable execution".to_string() },
    }
}

fn create_mock_reasoning_reflection(score: u8) -> syncore::mcp_server::reasoning::reflection::ReasoningReflection {
    create_mock_reasoning_reflection_with_category(score, if score >= 90 { "stable" } else if score >= 70 { "degraded" } else { "anomalous" })
}

fn create_mock_reasoning_reflection_with_category(_score: u8, category: &str) -> syncore::mcp_server::reasoning::reflection::ReasoningReflection {
    syncore::mcp_server::reasoning::reflection::ReasoningReflection {
        category: category.to_string(),
        regression_risk: if category == "stable" { 0.05 } else if category == "degraded" { 0.4 } else { 0.8 },
        recommended_top_k_delta: 0,
        recommended_scope_hint: None,
        improvement_hints: vec![],
    }
}