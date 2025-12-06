//! Phase 7 TDD Tests for Reasoning Trace Contracts + Introspection Layer
//!
//! These tests MUST FAIL before implementing Phase 7 trace features
//! and PASS after complete implementation.

use anyhow::Result;
use syncore::mcp_server::server::MCPServerHandler;
use syncore::mcp_server::types::{RagGraphQueryRequest, RagGraphMultihopRequest};
use syncore::router::SynCoreState;
use std::sync::Arc;
use rmcp::model::CallToolResult;
use serde_json::{json, Value};

/// Test 7.1: test_trace_structure_present
#[tokio::test]
async fn test_trace_structure_present() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query_text: "test trace structure".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 7 implementation because:
    // 1. trace field doesn't exist yet
    // 2. ReasoningTrace struct not implemented
    // 3. Response format hasn't been extended

    assert!(
        response.get("trace").is_some(),
        "Response should contain 'trace' field"
    );

    let trace = response.get("trace").unwrap().as_object().unwrap();

    // Check trace structure
    assert!(
        trace.contains_key("stages"),
        "trace should contain 'stages' field"
    );
    assert!(
        trace.contains_key("summary"),
        "trace should contain 'summary' field"
    );
    assert!(
        trace.contains_key("backend"),
        "trace should contain 'backend' field"
    );
    assert!(
        trace.contains_key("timing_breakdown"),
        "trace should contain 'timing_breakdown' field"
    );
    assert!(
        trace.contains_key("parameters_hash"),
        "trace should contain 'parameters_hash' field"
    );

    // Check stages is an array
    assert!(
        trace.get("stages").unwrap().is_array(),
        "trace.stages should be an array"
    );

    Ok(())
}

/// Test 7.2: test_trace_stages_in_deterministic_order
#[tokio::test]
async fn test_trace_stages_in_deterministic_order() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query_text: "test deterministic order".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(3),
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 7 because:
    // 1. trace field doesn't exist
    // 2. Stage order not defined yet
    // 3. Deterministic ordering not implemented

    let trace = response.get("trace").unwrap().as_object().unwrap();
    let stages = trace.get("stages").unwrap().as_array().unwrap();

    // Should have expected stages in deterministic order
    let expected_order = vec![
        "parsing",
        "backend_selection",
        "vector_search",
        "graph_traversal",
        "formatting"
    ];

    assert_eq!(
        stages.len(),
        expected_order.len(),
        "Should have exactly {} stages", expected_order.len()
    );

    for (i, stage) in stages.iter().enumerate() {
        let stage_obj = stage.as_object().unwrap();
        let stage_name = stage_obj.get("stage").unwrap().as_str().unwrap();

        assert_eq!(
            stage_name,
            expected_order[i],
            "Stage {} should be '{}' but found '{}'",
            i, expected_order[i], stage_name
        );

        // Each stage should have required fields
        assert!(
            stage_obj.get("ok").is_some(),
            "Stage '{}' should have 'ok' field", stage_name
        );
        assert!(
            stage_obj.get("detail").is_some(),
            "Stage '{}' should have 'detail' field", stage_name
        );
        assert!(
            stage_obj.get("timestamp_ms").is_some(),
            "Stage '{}' should have 'timestamp_ms' field", stage_name
        );
    }

    Ok(())
}

/// Test 7.3: test_trace_parameters_hash_consistency
#[tokio::test]
async fn test_trace_parameters_hash_consistency() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test identical requests produce identical hashes
    let request1 = RagGraphQueryRequest {
        query_text: "test hash consistency".to_string(),
        namespace: Some("src".to_string()),
        mode_hint: Some("reasoning".to_string()),
        top_k: Some(10),
        project_label: Some("syncore".to_string()),
        scope: Some("project".to_string()),
    };

    let request2 = RagGraphQueryRequest {
        query_text: "test hash consistency".to_string(),
        namespace: Some("src".to_string()),
        mode_hint: Some("reasoning".to_string()),
        top_k: Some(10),
        project_label: Some("syncore".to_string()),
        scope: Some("project".to_string()),
    };

    let result1 = handler.raggraph_query(syncore::mcp_server::Parameters(request1)).await?;
    let result2 = handler.raggraph_query(syncore::mcp_server::Parameters(request2)).await?;

    let response1: Value = serde_json::from_str(&result1.content[0].text.as_ref().unwrap())?;
    let response2: Value = serde_json::from_str(&result2.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 7 because:
    // 1. trace field doesn't exist
    // 2. parameters_hash not implemented
    // 3. SHA256 hashing not added

    let trace1 = response1.get("trace").unwrap().as_object().unwrap();
    let trace2 = response2.get("trace").unwrap().as_object().unwrap();

    let hash1 = trace1.get("parameters_hash").unwrap().as_str().unwrap();
    let hash2 = trace2.get("parameters_hash").unwrap().as_str().unwrap();

    assert_eq!(
        hash1, hash2,
        "Identical requests should produce identical parameters hashes"
    );

    // Hash should be SHA256 format (64 hex characters)
    assert_eq!(
        hash1.len(),
        64,
        "Parameters hash should be 64 characters (SHA256), got {}",
        hash1.len()
    );

    // Should only contain hex characters
    assert!(
        hash1.chars().all(|c| c.is_ascii_hexdigit()),
        "Parameters hash should contain only hex characters"
    );

    Ok(())
}

/// Test 7.4: test_trace_stage_failure_propagation
#[tokio::test]
async fn test_trace_stage_failure_propagation() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Create a request that should cause a parsing failure
    let request = RagGraphQueryRequest {
        query_text: "".to_string(), // Empty query should cause parsing failure
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 7 because:
    // 1. trace field doesn't exist
    // 2. Error responses don't include traces yet
    // 3. Stage failure tracking not implemented

    assert!(
        response.get("trace").is_some(),
        "Error responses should also contain traces"
    );

    let trace = response.get("trace").unwrap().as_object().unwrap();
    let stages = trace.get("stages").unwrap().as_array().unwrap();

    // Should have at least parsing stage
    let parsing_stage = stages.iter().find(|s| {
        s.as_object()
            .unwrap()
            .get("stage")
            .unwrap()
            .as_str()
            .unwrap() == "parsing"
    });

    assert!(
        parsing_stage.is_some(),
        "Should have parsing stage even in error responses"
    );

    let parsing_obj = parsing_stage.unwrap().as_object().unwrap();

    // Parsing should have failed
    assert_eq!(
        parsing_obj.get("ok").unwrap().as_bool().unwrap(),
        false,
        "Parsing stage should be marked as failed"
    );

    // Should have detail explaining the failure
    let detail = parsing_obj.get("detail").unwrap().as_str().unwrap();
    assert!(
        !detail.is_empty(),
        "Failed stage should have non-empty detail explaining failure"
    );
    assert!(
        detail.to_lowercase().contains("empty") || detail.to_lowercase().contains("invalid"),
        "Error detail should mention the cause of failure: {}",
        detail
    );

    Ok(())
}

/// Test 7.5: test_trace_backend_matches_metadata
#[tokio::test]
async fn test_trace_backend_matches_metadata() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query_text: "test backend consistency".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 7 because:
    // 1. trace field doesn't exist
    // 2. Backend tracking not implemented
    // 3. Metadata/backend consistency not enforced

    let trace = response.get("trace").unwrap().as_object().unwrap();
    let metadata = response.get("metadata").unwrap().as_object().unwrap();

    let trace_backend = trace.get("backend").unwrap().as_str().unwrap();
    let metadata_backend = metadata.get("backend_used").unwrap().as_str().unwrap();

    assert_eq!(
        trace_backend, metadata_backend,
        "Trace backend should match metadata backend: '{}' vs '{}'",
        trace_backend, metadata_backend
    );

    // Should be one of the expected backends
    assert!(
        trace_backend == "SQLiteGraph" || trace_backend == "Neo4j",
        "Backend should be SQLiteGraph or Neo4j, got '{}'",
        trace_backend
    );

    Ok(())
}

/// Test 7.6: test_trace_timing_alignment
#[tokio::test]
async fn test_trace_timing_alignment() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query_text: "test timing alignment".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 7 because:
    // 1. trace field doesn't exist
    // 2. timing_breakdown not implemented
    // 3. Stage timing not tracked

    let trace = response.get("trace").unwrap().as_object().unwrap();
    let metadata = response.get("metadata").unwrap().as_object().unwrap();
    let timing_breakdown = trace.get("timing_breakdown").unwrap().as_object().unwrap();

    // Total duration should align
    let trace_start = metadata.get("start_time_ms").unwrap().as_u64().unwrap() as u128;
    let trace_end = metadata.get("end_time_ms").unwrap().as_u64().unwrap() as u128;
    let total_duration = trace_end - trace_start;

    // Sum of stage durations should equal total duration (within small tolerance)
    let mut stage_sum = 0u128;
    for (_stage_name, duration) in timing_breakdown {
        stage_sum += duration.as_u64().unwrap() as u128;
    }

    // Allow small tolerance for timing differences
    let tolerance = total_duration / 100; // 1% tolerance
    assert!(
        stage_sum >= total_duration.saturating_sub(tolerance) && stage_sum <= total_duration + tolerance,
        "Stage timing sum ({}) should approximately equal total duration ({})",
        stage_sum, total_duration
    );

    // Should have timing entries for major stages
    let expected_stages = vec!["vector_search", "graph_traversal", "fusion"];
    for stage in expected_stages {
        if metadata.get(&format!("{}_ms", stage)).is_some() {
            assert!(
                timing_breakdown.get(stage).is_some(),
                "Should have timing breakdown for stage: {}", stage
            );
        }
    }

    Ok(())
}

/// Test 7.7: test_trace_serialization_roundtrip
#[tokio::test]
async fn test_trace_serialization_roundtrip() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query_text: "test serialization roundtrip".to_string(),
        namespace: Some("serialization".to_string()),
        mode_hint: Some("test".to_string()),
        top_k: Some(7),
        project_label: Some("test_project".to_string()),
        scope: Some("test".to_string()),
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 7 because:
    // 1. trace field doesn't exist
    // 2. ReasoningTrace serialization not implemented
    // 3. Roundtrip consistency not validated

    let original_trace = response.get("trace").unwrap();

    // Serialize to string and deserialize back
    let trace_json = serde_json::to_string_pretty(original_trace)?;
    let deserialized_trace: Value = serde_json::from_str(&trace_json)?;

    // Should be identical after roundtrip
    assert_eq!(
        original_trace, &deserialized_trace,
        "Trace should be identical after JSON serialization roundtrip"
    );

    // All required fields should survive roundtrip
    let trace_obj = deserialized_trace.as_object().unwrap();
    let required_fields = vec!["stages", "summary", "backend", "timing_breakdown", "parameters_hash"];

    for field in required_fields {
        assert!(
            trace_obj.contains_key(field),
            "Field '{}' should survive serialization roundtrip", field
        );
    }

    // Verify stages structure after roundtrip
    let stages = trace_obj.get("stages").unwrap().as_array().unwrap();
    for (i, stage) in stages.iter().enumerate() {
        let stage_obj = stage.as_object().unwrap();
        let stage_fields = vec!["stage", "ok", "detail", "timestamp_ms"];

        for field in stage_fields {
            assert!(
                stage_obj.contains_key(field),
                "Stage {} field '{}' should survive roundtrip", i, field
            );
        }
    }

    Ok(())
}

/// Test 7.8: test_trace_for_error_responses
#[tokio::test]
async fn test_trace_for_error_responses() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test raggraph_query with invalid parameters
    let query_request = RagGraphQueryRequest {
        query_text: "".to_string(), // Invalid - empty query
        namespace: None,
        mode_hint: None,
        top_k: Some(0), // Invalid - zero top_k
        project_label: None,
        scope: None,
    };

    let query_result = handler.raggraph_query(syncore::mcp_server::Parameters(query_request)).await?;
    let query_response: Value = serde_json::from_str(&query_result.content[0].text.as_ref().unwrap())?;

    // Test raggraph_multihop with invalid parameters
    let multihop_request = RagGraphMultihopRequest {
        seed_nodes: vec![], // Invalid - empty seed nodes
    };

    let multihop_result = handler.raggraph_multihop(syncore::mcp_server::Parameters(multihop_request)).await?;
    let multihop_response: Value = serde_json::from_str(&multihop_result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 7 because:
    // 1. trace field doesn't exist in error responses
    // 2. Error handling not integrated with tracing
    // 3. Different error types may not be traced consistently

    // Both error responses should contain traces
    assert!(
        query_response.get("trace").is_some(),
        "Query error response should contain trace"
    );
    assert!(
        multihop_response.get("trace").is_some(),
        "Multihop error response should contain trace"
    );

    // Both should have error information
    assert!(
        query_response.get("error").is_some(),
        "Query response should contain error information"
    );
    assert!(
        multihop_response.get("error").is_some(),
        "Multihop response should contain error information"
    );

    // Traces should be structurally consistent
    let query_trace = query_response.get("trace").unwrap().as_object().unwrap();
    let multihop_trace = multihop_response.get("trace").unwrap().as_object().unwrap();

    let required_fields = vec!["stages", "summary", "backend", "timing_breakdown", "parameters_hash"];

    for field in required_fields {
        assert!(
            query_trace.contains_key(field),
            "Query trace should contain field: {}", field
        );
        assert!(
            multihop_trace.contains_key(field),
            "Multihop trace should contain field: {}", field
        );
    }

    // Summary should indicate failure
    let query_summary = query_trace.get("summary").unwrap().as_str().unwrap();
    let multihop_summary = multihop_trace.get("summary").unwrap().as_str().unwrap();

    assert!(
        query_summary.to_lowercase().contains("error") || query_summary.to_lowercase().contains("failed"),
        "Query trace summary should indicate error: {}", query_summary
    );
    assert!(
        multihop_summary.to_lowercase().contains("error") || multihop_summary.to_lowercase().contains("failed"),
        "Multihop trace summary should indicate error: {}", multihop_summary
    );

    Ok(())
}