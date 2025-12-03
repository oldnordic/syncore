//! Tests for "llm.health" MCP tool
//!
//! Verifies that the MCP tool properly exposes GGUFEngine health
//! information through the MCP protocol.

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use syncore::intellitask::IntelliTask;
use syncore::mcp::protocol::{handle_mcp_request, MCPRequest};
use syncore::models::gguf_engine::GGUFEngine;
use syncore::router::SynCoreState;

/// Create test state with GGUFEngine for MCP testing
fn create_test_state_with_llm() -> SynCoreState {
    // Use in-memory databases to avoid file cleanup issues
    let memory = syncore::memory::Memory::new(":memory:").unwrap();
    let tasks = syncore::tasks::Tasks::new(":memory:").unwrap();
    let embeddings = Box::new(syncore::vector::RealEmbeddings::new(384).unwrap());
    let vector_store =
        std::sync::Arc::new(std::sync::Mutex::new(syncore::vector::VectorStore::new(embeddings)));

    let mut state = SynCoreState::new(memory, tasks, vector_store);

    // Add GGUFEngine via IntelliTask
    let engine = Arc::new(GGUFEngine::new_test());
    let intellitask = Arc::new(IntelliTask::new(engine));
    state.intellitask = Some(intellitask);

    state
}

/// Cleanup test database files
fn cleanup_test_db(db_name: &str) {
    let _ = std::fs::remove_file(db_name);
    let _ = std::fs::remove_file(&format!("{}_tasks", db_name));
}

/// Test that server responds to "llm.health"
#[tokio::test]
async fn test_mcp_llm_health_response() -> Result<()> {
    // Create test state with GGUFEngine
    let state = create_test_state_with_llm();

    // Create MCP request for llm.health
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.health",
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
    let health_obj = result.as_object().unwrap();

    // Verify required fields
    assert_eq!(health_obj.get("backend_name").unwrap().as_str().unwrap(), "gguf_engine");
    assert!(health_obj.contains_key("status"));
    assert!(health_obj.contains_key("device"));
    assert!(health_obj.contains_key("model_path"));
    assert!(health_obj.contains_key("model_loaded"));
    assert!(health_obj.contains_key("tokenizer_loaded"));
    assert!(health_obj.contains_key("arch"));
    assert!(health_obj.contains_key("last_error"));

    Ok(())
}

/// Test that backend_name is always "gguf_engine"
#[tokio::test]
async fn test_mcp_llm_health_backend_name() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = create_test_state_with_llm();

    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.health",
            "arguments": {}
        })),
        id: json!(1),
    };

    let response = handle_mcp_request(request, &state).await;
    let result = response.result.unwrap();
    let health_obj = result.as_object().unwrap();

    assert_eq!(health_obj.get("backend_name").unwrap().as_str().unwrap(), "gguf_engine");

    Ok(())
}

/// Test that model_loaded is a boolean
#[tokio::test]
async fn test_mcp_llm_health_model_loaded_bool() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = create_test_state_with_llm();

    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.health",
            "arguments": {}
        })),
        id: json!(1),
    };

    let response = handle_mcp_request(request, &state).await;
    let result = response.result.unwrap();
    let health_obj = result.as_object().unwrap();

    let model_loaded = health_obj.get("model_loaded").unwrap().as_bool();
    assert!(model_loaded.is_some());

    Ok(())
}

/// Test that device is one of expected values
#[tokio::test]
async fn test_mcp_llm_health_device_values() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = create_test_state_with_llm();

    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.health",
            "arguments": {}
        })),
        id: json!(1),
    };

    let response = handle_mcp_request(request, &state).await;
    let result = response.result.unwrap();
    let health_obj = result.as_object().unwrap();

    let device = health_obj.get("device").unwrap().as_str().unwrap();
    let valid_devices = ["cpu", "gpu_vulkan", "cpu_fallback"];
    assert!(valid_devices.contains(&device), "Device '{}' should be one of valid devices", device);

    Ok(())
}

/// Test that status is one of expected values
#[tokio::test]
async fn test_mcp_llm_health_status_values() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = create_test_state_with_llm();

    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.health",
            "arguments": {}
        })),
        id: json!(1),
    };

    let response = handle_mcp_request(request, &state).await;
    let result = response.result.unwrap();
    let health_obj = result.as_object().unwrap();

    let status = health_obj.get("status").unwrap().as_str().unwrap();
    let valid_statuses = ["Ok", "Degraded", "Error"];
    assert!(
        valid_statuses.contains(&status),
        "Status '{}' should be one of valid statuses",
        status
    );

    Ok(())
}

/// Test that last_error is optional
#[tokio::test]
async fn test_mcp_llm_health_last_error_optional() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = create_test_state_with_llm();

    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.health",
            "arguments": {}
        })),
        id: json!(1),
    };

    let response = handle_mcp_request(request, &state).await;
    let result = response.result.unwrap();
    let health_obj = result.as_object().unwrap();

    // last_error should be present but can be null
    assert!(health_obj.contains_key("last_error"));
    let last_error = health_obj.get("last_error").unwrap();
    // It can be null or a string
    assert!(last_error.is_null() || last_error.is_string());

    Ok(())
}

/// Test that calling llm.health does not mutate engine state
#[tokio::test]
async fn test_mcp_llm_health_no_mutation() -> Result<()> {
    let engine = Arc::new(GGUFEngine::new_test()) as Arc<dyn std::any::Any + Send + Sync>;
    let state = create_test_state_with_llm();

    // Call llm.health multiple times
    for i in 0..5 {
        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            method: "mcp.call_tool".to_string(),
            params: Some(json!({
                "name": "llm.health",
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
            "name": "llm.health",
            "arguments": {}
        })),
        id: json!(100),
    };

    let response1 = handle_mcp_request(request1, &state).await;
    let request2 = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "llm.health",
            "arguments": {}
        })),
        id: json!(101),
    };

    let response2 = handle_mcp_request(request2, &state).await;

    // Responses should be identical
    assert_eq!(response1.result, response2.result);

    Ok(())
}
