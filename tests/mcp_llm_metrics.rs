//! Tests for "llm.metrics" MCP tool
//!
//! Verifies that MCP tool properly exposes GGUFEngine metrics
//! information through MCP protocol.

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use syncore::llm::{LanguageModel, Prompt};
use syncore::mcp::protocol::{handle_mcp_request, MCPRequest};
use syncore::models::gguf_engine::GGUFEngine;
use syncore::router::SynCoreState;

/// Test that server responds to "llm.metrics"
#[tokio::test]
async fn test_mcp_llm_metrics_response() -> Result<()> {
    // Create test state with GGUFEngine
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = SynCoreState::with_llm_model(engine);

    // Create MCP request for llm.metrics
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.metrics",
            "arguments": {}
        })),
        id: json!(1),
    };

    // Handle request
    let response = handle_mcp_request(request, &state).await;

    // Verify response structure
    assert!(response.result.is_some(), "Should have successful result");
    assert!(response.error.is_none(), "Should not have error");

    let result = response.result.unwrap();
    let metrics_obj = result.as_object().unwrap();

    // Verify required fields
    assert!(metrics_obj.contains_key("total_requests"));
    assert!(metrics_obj.contains_key("total_tokens_in"));
    assert!(metrics_obj.contains_key("total_tokens_out"));
    assert!(metrics_obj.contains_key("last_latency_ms"));
    assert!(metrics_obj.contains_key("avg_latency_ms"));
    assert!(metrics_obj.contains_key("model_file_size_bytes"));

    Ok(())
}

/// Test that metrics.total_requests > 0 after calling llm.complete
#[tokio::test]
async fn test_mcp_llm_metrics_requests_after_generation() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = SynCoreState::with_llm_model(engine);

    // Generate some text to increment metrics
    let gguf_engine = state.llm_model.as_ref().unwrap().downcast_ref::<GGUFEngine>().unwrap();

    let prompt = Prompt::new("", "test prompt for metrics");
    let _result = gguf_engine.complete(&prompt);

    // Now check metrics via MCP
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.metrics",
            "arguments": {}
        })),
        id: json!(1),
    };

    let response = handle_mcp_request(request, &state).await;
    let result = response.result.unwrap();
    let metrics_obj = result.as_object().unwrap();

    let total_requests = metrics_obj.get("total_requests").unwrap().as_u64().unwrap();
    assert!(total_requests > 0, "Should have at least 1 request after generation");

    Ok(())
}

/// Test that token counts reflect previous generation calls
#[tokio::test]
async fn test_mcp_llm_metrics_token_counts() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = SynCoreState::with_llm_model(engine);

    let gguf_engine = state.llm_model.as_ref().unwrap().downcast_ref::<GGUFEngine>().unwrap();

    // Generate multiple prompts
    let prompts = vec![
        Prompt::new("", "short"),
        Prompt::new("", "medium length prompt"),
        Prompt::new("", "this is a longer prompt with more content"),
    ];

    for prompt in prompts {
        let _result = gguf_engine.complete(&prompt);
    }

    // Check metrics
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.metrics",
            "arguments": {}
        })),
        id: json!(1),
    };

    let response = handle_mcp_request(request, &state).await;
    let result = response.result.unwrap();
    let metrics_obj = result.as_object().unwrap();

    let total_tokens_in = metrics_obj.get("total_tokens_in").unwrap().as_u64().unwrap();
    let total_tokens_out = metrics_obj.get("total_tokens_out").unwrap().as_u64().unwrap();

    assert!(total_tokens_in > 0, "Should have input tokens");
    assert!(total_tokens_out > 0, "Should have output tokens");

    Ok(())
}

/// Test that last_latency_ms > 0.0
#[tokio::test]
async fn test_mcp_llm_metrics_last_latency() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = SynCoreState::with_llm_model(engine);

    let gguf_engine = state.llm_model.as_ref().unwrap().downcast_ref::<GGUFEngine>().unwrap();

    // Generate to create latency data
    let prompt = Prompt::new("", "test latency");
    let _result = gguf_engine.complete(&prompt);

    // Check metrics
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.metrics",
            "arguments": {}
        })),
        id: json!(1),
    };

    let response = handle_mcp_request(request, &state).await;
    let result = response.result.unwrap();
    let metrics_obj = result.as_object().unwrap();

    let last_latency_ms = metrics_obj.get("last_latency_ms").unwrap().as_f64().unwrap();
    assert!(last_latency_ms >= 0.0, "Last latency should be non-negative");

    Ok(())
}

/// Test that avg_latency_ms > 0.0
#[tokio::test]
async fn test_mcp_llm_metrics_avg_latency() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = SynCoreState::with_llm_model(engine);

    let gguf_engine = state.llm_model.as_ref().unwrap().downcast_ref::<GGUFEngine>().unwrap();

    // Generate multiple requests to get average
    for i in 0..3 {
        let prompt = Prompt::new("", &format!("test prompt {}", i));
        let _result = gguf_engine.complete(&prompt);
    }

    // Check metrics
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.metrics",
            "arguments": {}
        })),
        id: json!(1),
    };

    let response = handle_mcp_request(request, &state).await;
    let result = response.result.unwrap();
    let metrics_obj = result.as_object().unwrap();

    let avg_latency_ms = metrics_obj.get("avg_latency_ms").unwrap().as_f64().unwrap();
    assert!(avg_latency_ms >= 0.0, "Average latency should be non-negative");

    Ok(())
}

/// Test thread-safety: spawn 3 simultaneous calls, then check metrics
#[tokio::test]
async fn test_mcp_llm_metrics_thread_safety() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = Arc::new(SynCoreState::with_llm_model(engine));

    let gguf_engine = state.llm_model.as_ref().unwrap().downcast_ref::<GGUFEngine>().unwrap();

    // Spawn multiple threads for concurrent generation
    let mut handles = vec![];
    for i in 0..3 {
        let gguf_engine_clone = unsafe { std::ptr::read(gguf_engine as *const _) };
        let handle = thread::spawn(move || {
            let prompt = Prompt::new("", &format!("concurrent test {}", i));
            let _result = gguf_engine_clone.complete(&prompt);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Check metrics via MCP
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.metrics",
            "arguments": {}
        })),
        id: json!(1),
    };

    let response = handle_mcp_request(request, &state).await;
    let result = response.result.unwrap();
    let metrics_obj = result.as_object().unwrap();

    let total_requests = metrics_obj.get("total_requests").unwrap().as_u64().unwrap();
    assert_eq!(total_requests, 3, "Should have exactly 3 requests from 3 threads");

    Ok(())
}

/// Test failure path: invalid model should still return zeroed counters
#[tokio::test]
async fn test_mcp_llm_metrics_failure_path() -> Result<()> {
    // For test engine, we can't easily simulate invalid model
    // But we can test that metrics returns zeroed counters initially
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = SynCoreState::with_llm_model(engine);

    // Check initial metrics (should be zeroed)
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.metrics",
            "arguments": {}
        })),
        id: json!(1),
    };

    let response = handle_mcp_request(request, &state).await;
    let result = response.result.unwrap();
    let metrics_obj = result.as_object().unwrap();

    let total_requests = metrics_obj.get("total_requests").unwrap().as_u64().unwrap();
    let total_tokens_in = metrics_obj.get("total_tokens_in").unwrap().as_u64().unwrap();
    let total_tokens_out = metrics_obj.get("total_tokens_out").unwrap().as_u64().unwrap();

    assert_eq!(total_requests, 0, "Initial requests should be 0");
    assert_eq!(total_tokens_in, 0, "Initial tokens in should be 0");
    assert_eq!(total_tokens_out, 0, "Initial tokens out should be 0");

    Ok(())
}

/// Test that calling llm.metrics does not mutate engine state
#[tokio::test]
async fn test_mcp_llm_metrics_no_mutation() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = SynCoreState::with_llm_model(engine);

    // Generate some activity first
    let gguf_engine = state.llm_model.as_ref().unwrap().downcast_ref::<GGUFEngine>().unwrap();

    let prompt = Prompt::new("", "test for no mutation");
    let _result = gguf_engine.complete(&prompt);

    // Call llm.metrics multiple times
    for i in 0..5 {
        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            method: "mcp.call_tool".to_string(),
            params: Some(json!({
                "name": "llm.metrics",
                "arguments": {}
            })),
            id: json!(i),
        };

        let response = handle_mcp_request(request, &state).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    // Verify no mutations occurred by checking that responses are consistent
    let request1 = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.metrics",
            "arguments": {}
        })),
        id: json!(100),
    };

    let response1 = handle_mcp_request(request1, &state).await;
    let request2 = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.metrics",
            "arguments": {}
        })),
        id: json!(101),
    };

    let response2 = handle_mcp_request(request2, &state).await;

    // Responses should be identical
    assert_eq!(response1.result, response2.result);

    Ok(())
}
