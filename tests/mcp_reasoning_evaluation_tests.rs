//! Phase 8 TDD Tests for Reasoning Evaluation Contracts
//!
//! These tests MUST FAIL before implementing Phase 8 evaluation features
//! and PASS after complete implementation.

use anyhow::Result;
use syncore::mcp_server::server::MCPServerHandler;
use syncore::mcp_server::types::{RagGraphQueryRequest, RagGraphMultihopRequest};
use syncore::router::SynCoreState;
use std::sync::Arc;
use rmcp::model::CallToolResult;
use serde_json::{json, Value};

/// Test 8.1: evaluation_score_is_deterministic
#[tokio::test]
async fn evaluation_score_is_deterministic() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Create identical requests
    let request1 = RagGraphQueryRequest {
        query_text: "deterministic test query".to_string(),
        namespace: Some("src".to_string()),
        mode_hint: Some("reasoning".to_string()),
        top_k: Some(10),
        project_label: Some("test_project".to_string()),
        scope: Some("project".to_string()),
    };

    let request2 = RagGraphQueryRequest {
        query_text: "deterministic test query".to_string(),
        namespace: Some("src".to_string()),
        mode_hint: Some("reasoning".to_string()),
        top_k: Some(10),
        project_label: Some("test_project".to_string()),
        scope: Some("project".to_string()),
    };

    let result1 = handler.raggraph_query(syncore::mcp_server::Parameters(request1)).await?;
    let result2 = handler.raggraph_query(syncore::mcp_server::Parameters(request2)).await?;

    let response1: Value = serde_json::from_str(&result1.content[0].text.as_ref().unwrap())?;
    let response2: Value = serde_json::from_str(&result2.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 8 implementation because:
    // 1. evaluation field doesn't exist yet
    // 2. ReasoningEvaluation struct not implemented
    // 3. Deterministic scoring logic not implemented

    assert!(
        response1.get("evaluation").is_some(),
        "Response should contain 'evaluation' field"
    );
    assert!(
        response2.get("evaluation").is_some(),
        "Response should contain 'evaluation' field"
    );

    let eval1 = response1.get("evaluation").unwrap().as_object().unwrap();
    let eval2 = response2.get("evaluation").unwrap().as_object().unwrap();

    let score1 = eval1.get("score").unwrap().as_u64().unwrap();
    let score2 = eval2.get("score").unwrap().as_u64().unwrap();

    assert_eq!(
        score1, score2,
        "Identical requests should produce identical evaluation scores: {} vs {}",
        score1, score2
    );

    // Score should be in valid range
    assert!(
        score1 >= 0 && score1 <= 100,
        "Evaluation score should be between 0 and 100, got {}",
        score1
    );

    Ok(())
}

/// Test 8.2: evaluation_flags_detect_missing_stages
#[tokio::test]
async fn evaluation_flags_detect_missing_stages() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Request that should result in complete execution
    let request = RagGraphQueryRequest {
        query_text: "complete execution test".to_string(),
        namespace: Some("src".to_string()),
        mode_hint: Some("reasoning".to_string()),
        top_k: Some(5),
        project_label: Some("test".to_string()),
        scope: Some("project".to_string()),
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 8 because:
    // 1. evaluation field doesn't exist
    // 2. anomaly_flags field not implemented
    // 3. Missing stage detection logic not implemented

    let evaluation = response.get("evaluation").unwrap().as_object().unwrap();
    let anomaly_flags = evaluation.get("anomaly_flags").unwrap().as_array().unwrap();

    // Should have no missing stage flags for successful execution
    let missing_flags: Vec<&String> = anomaly_flags
        .iter()
        .filter_map(|f| {
            let flag_str = f.as_str().unwrap();
            if flag_str.starts_with("missing_stage:") {
                Some(&flag_str.to_string())
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        missing_flags.len(),
        0,
        "Successful execution should have no missing stage flags, but found: {:?}",
        missing_flags
    );

    Ok(())
}

/// Test 8.3: evaluation_flags_detect_unordered_stages
#[tokio::test]
async fn evaluation_flags_detect_unordered_stages() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query_text: "order validation test".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(3),
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 8 because:
    // 1. evaluation field doesn't exist
    // 2. Stage ordering validation not implemented
    // 3. Unordered stage detection logic not implemented

    let evaluation = response.get("evaluation").unwrap().as_object().unwrap();
    let trace = response.get("trace").unwrap().as_object().unwrap();
    let stages = trace.get("stages").unwrap().as_array().unwrap();

    // Verify expected deterministic order
    let expected_order = vec!["parsing", "vector_search", "graph_traversal", "formatting"];
    assert_eq!(
        stages.len(),
        expected_order.len(),
        "Should have expected number of stages"
    );

    for (i, expected_stage) in expected_order.iter().enumerate() {
        let stage_obj = stages.get(i).unwrap().as_object().unwrap();
        let actual_stage = stage_obj.get("stage").unwrap().as_str().unwrap();
        assert_eq!(
            actual_stage, *expected_stage,
            "Stage {} should be '{}' but found '{}'",
            i, expected_stage, actual_stage
        );
    }

    let anomaly_flags = evaluation.get("anomaly_flags").unwrap().as_array().unwrap();

    // Should have no unordered stage flags for correct execution
    let unordered_flags: Vec<&String> = anomaly_flags
        .iter()
        .filter_map(|f| {
            let flag_str = f.as_str().unwrap();
            if flag_str.starts_with("unordered_stage:") {
                Some(&flag_str.to_string())
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        unordered_flags.len(),
        0,
        "Correctly ordered execution should have no unordered stage flags, but found: {:?}",
        unordered_flags
    );

    Ok(())
}

/// Test 8.4: evaluation_confidence_reduces_if_timings_erratic
#[tokio::test]
async fn evaluation_confidence_reduces_if_timings_erratic() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Request with potentially erratic timing behavior
    let request = RagGraphQueryRequest {
        query_text: "timing erratic test".to_string(),
        namespace: Some("large_module".to_string()),
        mode_hint: Some("fusion".to_string()), // Should trigger fusion stage
        top_k: Some(50), // Large top_k might cause timing issues
        project_label: Some("complex_project".to_string()),
        scope: Some("project".to_string()),
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 8 because:
    // 1. evaluation field doesn't exist
    // 2. confidence scoring not implemented
    // 3. Timing-based confidence reduction not implemented

    let evaluation = response.get("evaluation").unwrap().as_object().unwrap();
    let trace = response.get("trace").unwrap().as_object().unwrap();
    let timing_breakdown = trace.get("timing_breakdown").unwrap().as_object().unwrap();

    let confidence = evaluation.get("confidence").unwrap().as_f64().unwrap();

    // Confidence should be in valid range
    assert!(
        confidence >= 0.0 && confidence <= 1.0,
        "Confidence should be between 0.0 and 1.0, got {}",
        confidence
    );

    // If graph traversal took significantly longer than vector search, confidence should be reduced
    if let (Some(vector_ms), Some(graph_ms)) = (
        timing_breakdown.get("vector_search").and_then(|v| v.as_u64()),
        timing_breakdown.get("graph_traversal").and_then(|v| v.as_u64()),
    ) {
        let ratio = graph_ms as f64 / vector_ms as f64;
        if ratio > 2.0 {
            assert!(
                confidence < 0.9,
                "Confidence should be reduced when graph traversal > 2x vector search (ratio: {:.2}, confidence: {:.2})",
                ratio, confidence
            );
        }
    }

    Ok(())
}

/// Test 8.5: evaluation_handles_failed_executions
#[tokio::test]
async fn evaluation_handles_failed_executions() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Create a request that should cause execution failure
    let request = RagGraphQueryRequest {
        query_text: "".to_string(), // Empty query should cause failure
        namespace: None,
        mode_hint: None,
        top_k: Some(0), // Invalid top_k
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 8 because:
    // 1. evaluation field doesn't exist in error responses
    // 2. Failed execution scoring rules not implemented
    // 3. Score/confidence caps for failures not implemented

    assert!(
        response.get("evaluation").is_some(),
        "Error responses should also contain evaluation"
    );

    let evaluation = response.get("evaluation").unwrap().as_object().unwrap();

    let score = evaluation.get("score").unwrap().as_u64().unwrap();
    let confidence = evaluation.get("confidence").unwrap().as_f64().unwrap();

    // Failed execution should have score capped at 60 and confidence capped at 0.5
    assert!(
        score <= 60,
        "Failed execution score should be capped at 60, got {}",
        score
    );

    assert!(
        confidence <= 0.5,
        "Failed execution confidence should be capped at 0.5, got {}",
        confidence
    );

    // Should still have valid ranges
    assert!(score >= 0, "Score should be non-negative");
    assert!(confidence >= 0.0, "Confidence should be non-negative");

    Ok(())
}

/// Test 8.6: evaluation_ignores_non_deterministic_fields
#[tokio::test]
async fn evaluation_ignores_non_deterministic_fields() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Create requests that are semantically identical but might have non-deterministic fields
    let request1 = RagGraphQueryRequest {
        query_text: "deterministic evaluation test".to_string(),
        namespace: Some("src".to_string()),
        mode_hint: Some("reasoning".to_string()),
        top_k: Some(10),
        project_label: Some("test".to_string()),
        scope: Some("project".to_string()),
    };

    let request2 = RagGraphQueryRequest {
        query_text: "deterministic evaluation test".to_string(),
        namespace: Some("src".to_string()),
        mode_hint: Some("reasoning".to_string()),
        top_k: Some(10),
        project_label: Some("test_different".to_string()), // Different project label (non-deterministic)
        scope: Some("project".to_string()),
    };

    let result1 = handler.raggraph_query(syncore::mcp_server::Parameters(request1)).await?;
    let result2 = handler.raggraph_query(syncore::mcp_server::Parameters(request2)).await?;

    let response1: Value = serde_json::from_str(&result1.content[0].text.as_ref().unwrap())?;
    let response2: Value = serde_json::from_str(&result2.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 8 because:
    // 1. evaluation field doesn't exist
    // 2. Evaluation logic not implemented
    // 3. Non-deterministic field handling not defined

    let eval1 = response1.get("evaluation").unwrap().as_object().unwrap();
    let eval2 = response2.get("evaluation").unwrap().as_object().unwrap();

    // Core evaluation should be based only on deterministic fields (metadata + trace structure)
    let score1 = eval1.get("score").unwrap().as_u64().unwrap();
    let score2 = eval2.get("score").unwrap().as_u64().unwrap();

    let confidence1 = eval1.get("confidence").unwrap().as_f64().unwrap();
    let confidence2 = eval2.get("confidence").unwrap().as_f64().unwrap();

    // Project label differences should not affect the core evaluation score
    assert_eq!(
        score1, score2,
        "Core evaluation score should ignore non-deterministic fields like project_label: {} vs {}",
        score1, score2
    );

    // Anomaly flags should be identical for structurally identical execution
    let flags1 = eval1.get("anomaly_flags").unwrap().as_array().unwrap();
    let flags2 = eval2.get("anomaly_flags").unwrap().as_array().unwrap();

    // Sort flags for comparison to handle potential ordering differences
    let mut sorted_flags1: Vec<String> = flags1.iter()
        .map(|f| f.as_str().unwrap().to_string())
        .collect();
    let mut sorted_flags2: Vec<String> = flags2.iter()
        .map(|f| f.as_str().unwrap().to_string())
        .collect();
    sorted_flags1.sort();
    sorted_flags2.sort();

    assert_eq!(
        sorted_flags1, sorted_flags2,
        "Anomaly flags should be identical for structurally identical executions"
    );

    Ok(())
}

/// Test 8.7: evaluation_serialization_roundtrip
#[tokio::test]
async fn evaluation_serialization_roundtrip() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query_text: "serialization roundtrip test".to_string(),
        namespace: Some("test".to_string()),
        mode_hint: Some("evaluation".to_string()),
        top_k: Some(7),
        project_label: Some("test_project".to_string()),
        scope: Some("test".to_string()),
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 8 because:
    // 1. evaluation field doesn't exist
    // 2. ReasoningEvaluation struct not implemented
    // 3. Serialization roundtrip consistency not validated

    let original_evaluation = response.get("evaluation").unwrap();

    // Serialize to string and deserialize back
    let eval_json = serde_json::to_string_pretty(original_evaluation)?;
    let deserialized_evaluation: Value = serde_json::from_str(&eval_json)?;

    // Should be identical after roundtrip
    assert_eq!(
        original_evaluation, &deserialized_evaluation,
        "Evaluation should be identical after JSON serialization roundtrip"
    );

    // All required fields should survive roundtrip
    let eval_obj = deserialized_evaluation.as_object().unwrap();
    let required_fields = vec!["score", "confidence", "anomaly_flags", "summary"];

    for field in required_fields {
        assert!(
            eval_obj.contains_key(field),
            "Field '{}' should survive serialization roundtrip", field
        );
    }

    // Verify field types after roundtrip
    assert!(eval_obj.get("score").unwrap().is_number(), "Score should be a number");
    assert!(eval_obj.get("confidence").unwrap().is_number(), "Confidence should be a number");
    assert!(eval_obj.get("anomaly_flags").unwrap().is_array(), "Anomaly flags should be an array");
    assert!(eval_obj.get("summary").unwrap().is_string(), "Summary should be a string");

    Ok(())
}

/// Test 8.8: evaluation_integrates_into_unified_response
#[tokio::test]
async fn evaluation_integrates_into_unified_response() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test all three reasoning tools to ensure evaluation integration
    let query_request = RagGraphQueryRequest {
        query_text: "evaluation integration test".to_string(),
        namespace: Some("src".to_string()),
        mode_hint: Some("reasoning".to_string()),
        top_k: Some(5),
        project_label: Some("syncore".to_string()),
        scope: Some("project".to_string()),
    };

    let multihop_request = RagGraphMultihopRequest {
        seed_nodes: vec![1, 2, 3, 4, 5],
    };

    let fusion_request = RagGraphQueryRequest {
        query_text: "evaluation fusion test".to_string(),
        namespace: Some("src".to_string()),
        mode_hint: Some("fusion".to_string()),
        top_k: Some(10),
        project_label: Some("syncore".to_string()),
        scope: Some("project".to_string()),
    };

    // Execute all three reasoning tools
    let query_result = handler.raggraph_query(syncore::mcp_server::Parameters(query_request)).await?;
    let multihop_result = handler.raggraph_multihop(syncore::mcp_server::Parameters(multihop_request)).await?;
    let fusion_result = handler.code_graph_fusion_query(syncore::mcp_server::Parameters(fusion_request)).await?;

    let query_response: Value = serde_json::from_str(&query_result.content[0].text.as_ref().unwrap())?;
    let multihop_response: Value = serde_json::from_str(&multihop_result.content[0].text.as_ref().unwrap())?;
    let fusion_response: Value = serde_json::from_str(&fusion_result.content[0].text.as_ref().unwrap())?;

    // This test will FAIL before Phase 8 because:
    // 1. evaluation field doesn't exist in any responses
    // 2. Evaluation integration not implemented for any reasoning tool
    // 3. Consistent evaluation format across tools not enforced

    // All responses should contain evaluation
    assert!(
        query_response.get("evaluation").is_some(),
        "Query response should contain evaluation"
    );
    assert!(
        multihop_response.get("evaluation").is_some(),
        "Multihop response should contain evaluation"
    );
    assert!(
        fusion_response.get("evaluation").is_some(),
        "Fusion response should contain evaluation"
    );

    // All evaluations should have the same structure
    let query_eval = query_response.get("evaluation").unwrap().as_object().unwrap();
    let multihop_eval = multihop_response.get("evaluation").unwrap().as_object().unwrap();
    let fusion_eval = fusion_response.get("evaluation").unwrap().as_object().unwrap();

    let evaluation_fields = vec!["score", "confidence", "anomaly_flags", "summary"];

    for field in evaluation_fields {
        assert!(
            query_eval.contains_key(field),
            "Query evaluation should contain field: {}",
            field
        );
        assert!(
            multihop_eval.contains_key(field),
            "Multihop evaluation should contain field: {}",
            field
        );
        assert!(
            fusion_eval.contains_key(field),
            "Fusion evaluation should contain field: {}",
            field
        );
    }

    // All evaluations should be in valid ranges
    for (tool_name, eval) in [
        ("query", query_eval),
        ("multihop", multihop_eval),
        ("fusion", fusion_eval),
    ] {
        let score = eval.get("score").unwrap().as_u64().unwrap();
        let confidence = eval.get("confidence").unwrap().as_f64().unwrap();

        assert!(
            score >= 0 && score <= 100,
            "{} evaluation score should be between 0 and 100, got {}",
            tool_name, score
        );
        assert!(
            confidence >= 0.0 && confidence <= 1.0,
            "{} evaluation confidence should be between 0.0 and 1.0, got {}",
            tool_name, confidence
        );
    }

    Ok(())
}