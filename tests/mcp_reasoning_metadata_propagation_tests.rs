//! TDD Tests for reasoning metadata propagation in unified MCP reasoning tools
//!
//! These tests MUST FAIL before implementing ReasoningMetadata functionality
//! and PASS after complete implementation.

use anyhow::Result;
use syncore::mcp_server::server::MCPServerHandler;
use syncore::mcp_server::types::{RagGraphQueryRequest, RagGraphMultihopRequest};
use syncore::router::SynCoreState;
use std::sync::Arc;
use rmcp::model::CallToolResult;
use serde_json::{json, Value};

/// Test that metadata field exists in raggraph_query response
#[tokio::test]
async fn test_raggraph_query_metadata_exists() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query: "find authentication functions".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        scope: None,
        project_label: None,
        local_root: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await;

    assert!(result.is_ok(), "raggraph_query should succeed");

    let response_text = result.unwrap().content[0].text.as_ref().unwrap();
    let response: Value = serde_json::from_str(response_text)?;

    // This test will FAIL before implementation because metadata field doesn't exist
    assert!(
        response.get("metadata").is_some(),
        "Response should contain 'metadata' field"
    );

    Ok(())
}

/// Test that metadata field exists in raggraph_multihop response
#[tokio::test]
async fn test_raggraph_multihop_metadata_exists() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphMultihopRequest {
        seed_nodes: vec![1, 2, 3],
    };

    let result = handler.raggraph_multihop(syncore::mcp_server::Parameters(request)).await;

    assert!(result.is_ok(), "raggraph_multihop should succeed");

    let response_text = result.unwrap().content[0].text.as_ref().unwrap();
    let response: Value = serde_json::from_str(response_text)?;

    // This test will FAIL before implementation because metadata field doesn't exist
    assert!(
        response.get("metadata").is_some(),
        "Response should contain 'metadata' field"
    );

    Ok(())
}

/// Test that metadata field exists in code_graph_fusion_query response
#[tokio::test]
async fn test_code_graph_fusion_query_metadata_exists() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query: "database connection patterns".to_string(),
        namespace: Some("src/db".to_string()),
        mode_hint: Some("reasoning".to_string()),
        top_k: Some(10),
        scope: Some("project".to_string()),
        project_label: Some("syncore".to_string()),
        local_root: None,
    };

    let result = handler.code_graph_fusion_query(syncore::mcp_server::Parameters(request)).await;

    assert!(result.is_ok(), "code_graph_fusion_query should succeed");

    let response_text = result.unwrap().content[0].text.as_ref().unwrap();
    let response: Value = serde_json::from_str(response_text)?;

    // This test will FAIL before implementation because metadata field doesn't exist
    assert!(
        response.get("metadata").is_some(),
        "Response should contain 'metadata' field"
    );

    Ok(())
}

/// Test that metadata backend_used matches SQLiteGraph when configured
#[tokio::test]
async fn test_metadata_backend_used_sqlitegraph() -> Result<()> {
    // Configure environment to use SQLiteGraph
    std::env::set_var("GRAPH_BACKEND", "sqlite");

    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query: "test query for metadata".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        scope: None,
        project_label: None,
        local_root: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await;

    assert!(result.is_ok(), "raggraph_query should succeed with SQLiteGraph");

    let response_text = result.unwrap().content[0].text.as_ref().unwrap();
    let response: Value = serde_json::from_str(response_text)?;

    let metadata = response.get("metadata").unwrap().as_object().unwrap();
    let backend_used = metadata.get("backend_used").unwrap().as_str().unwrap();

    // This test will FAIL before implementation because backend_used field doesn't exist
    assert_eq!(
        backend_used, "SQLiteGraph",
        "metadata.backend_used should be 'SQLiteGraph' when configured for sqlite"
    );

    // Cleanup
    std::env::remove_var("GRAPH_BACKEND");

    Ok(())
}

/// Test that metadata parameters contain parsed arguments with defaults
#[tokio::test]
async fn test_metadata_parameters_with_defaults() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test request with minimal parameters
    let request = RagGraphQueryRequest {
        query: "test parameters".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: None, // Should use default
        scope: None,
        project_label: None,
        local_root: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await;

    assert!(result.is_ok(), "raggraph_query should succeed");

    let response_text = result.unwrap().content[0].text.as_ref().unwrap();
    let response: Value = serde_json::from_str(response_text)?;

    let metadata = response.get("metadata").unwrap().as_object().unwrap();
    let parameters = metadata.get("parameters").unwrap().as_object().unwrap();

    // This test will FAIL before implementation because parameters field doesn't exist
    assert!(
        parameters.contains_key("query"),
        "metadata.parameters should contain 'query' key"
    );

    // Should have default values applied
    assert!(
        parameters.get("top_k").is_some(),
        "metadata.parameters should contain default 'top_k' value"
    );

    Ok(())
}

/// Test that metadata timestamps are valid and increasing
#[tokio::test]
async fn test_metadata_timestamps_valid() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query: "test timestamps".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        scope: None,
        project_label: None,
        local_root: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await;

    assert!(result.is_ok(), "raggraph_query should succeed");

    let response_text = result.unwrap().content[0].text.as_ref().unwrap();
    let response: Value = serde_json::from_str(response_text)?;

    let metadata = response.get("metadata").unwrap().as_object().unwrap();
    let start_time = metadata.get("start_time_ms").unwrap().as_u64().unwrap();
    let end_time = metadata.get("end_time_ms").unwrap().as_u64().unwrap();

    // This test will FAIL before implementation because timing fields don't exist
    assert!(
        start_time < end_time,
        "metadata.start_time_ms should be less than metadata.end_time_ms"
    );

    // Reasonable timing bounds (should not be in the future, should be recent)
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    assert!(
        start_time <= current_time,
        "metadata.start_time_ms should not be in the future"
    );

    Ok(())
}

/// Test that metadata timing fields are present when vector search is performed
#[tokio::test]
async fn test_metadata_vector_search_timing_present() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // This query should trigger vector search
    let request = RagGraphQueryRequest {
        query: "semantic search test".to_string(),
        namespace: None,
        mode_hint: Some("simple".to_string()),
        top_k: Some(5),
        scope: Some("local".to_string()),
        project_label: None,
        local_root: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await;

    assert!(result.is_ok(), "raggraph_query should succeed");

    let response_text = result.unwrap().content[0].text.as_ref().unwrap();
    let response: Value = serde_json::from_str(response_text)?;

    let metadata = response.get("metadata").unwrap().as_object().unwrap();

    // This test will FAIL before implementation because timing fields don't exist
    assert!(
        metadata.get("vector_search_ms").is_some(),
        "metadata.vector_search_ms should be present when vector search is performed"
    );

    let vector_search_time = metadata.get("vector_search_ms").unwrap().as_u64().unwrap();

    assert!(
        vector_search_time > 0,
        "metadata.vector_search_ms should be greater than 0"
    );

    Ok(())
}

/// Test deterministic behavior - identical requests produce identical metadata except timestamps
#[tokio::test]
async fn test_metadata_deterministic_behavior() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query: "deterministic test".to_string(),
        namespace: None,
        mode_hint: Some("attention".to_string()),
        top_k: Some(5),
        scope: Some("project".to_string()),
        project_label: None,
        local_root: None,
    };

    // Execute the same request twice
    let result1 = handler.raggraph_query(syncore::mcp_server::Parameters(request.clone())).await;
    let result2 = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await;

    assert!(result1.is_ok(), "First request should succeed");
    assert!(result2.is_ok(), "Second request should succeed");

    let response1_text = result1.unwrap().content[0].text.as_ref().unwrap();
    let response2_text = result2.unwrap().content[0].text.as_ref().unwrap();

    let response1: Value = serde_json::from_str(response1_text)?;
    let response2: Value = serde_json::from_str(response2_text)?;

    let metadata1 = response1.get("metadata").unwrap().as_object().unwrap();
    let metadata2 = response2.get("metadata").unwrap().as_object().unwrap();

    // This test will FAIL before implementation because metadata doesn't exist
    // All fields except timestamps should be identical
    let backend1 = metadata1.get("backend_used").unwrap();
    let backend2 = metadata2.get("backend_used").unwrap();
    assert_eq!(
        backend1, backend2,
        "metadata.backend_used should be identical for identical requests"
    );

    let parameters1 = metadata1.get("parameters").unwrap();
    let parameters2 = metadata2.get("parameters").unwrap();
    assert_eq!(
        parameters1, parameters2,
        "metadata.parameters should be identical for identical requests"
    );

    let debug_flags1 = metadata1.get("debug_flags").unwrap().as_array().unwrap();
    let debug_flags2 = metadata2.get("debug_flags").unwrap().as_array().unwrap();
    assert_eq!(
        debug_flags1, debug_flags2,
        "metadata.debug_flags should be identical for identical requests"
    );

    Ok(())
}

/// Test that metadata appears in error responses
#[tokio::test]
async fn test_metadata_in_error_responses() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Create a request that should cause an error (empty query)
    let request = RagGraphQueryRequest {
        query: "".to_string(), // Empty query should cause error
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        scope: None,
        project_label: None,
        local_root: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await;

    // The request should return an error result, but still include metadata
    assert!(result.is_ok(), "Should return error result, not panic");

    let response_text = result.unwrap().content[0].text.as_ref().unwrap();
    let response: Value = serde_json::from_str(response_text)?;

    // This test will FAIL before implementation because metadata doesn't exist in error responses
    assert!(
        response.get("metadata").is_some(),
        "Error responses should also contain 'metadata' field"
    );

    // Should also have error information
    assert!(
        response.get("error").is_some(),
        "Error responses should contain 'error' field"
    );

    Ok(())
}

/// Test that multihop request has specific timing fields
#[tokio::test]
async fn test_multihop_metadata_timing_fields() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphMultihopRequest {
        seed_nodes: vec![1, 5, 10],
    };

    let result = handler.raggraph_multihop(syncore::mcp_server::Parameters(request)).await;

    assert!(result.is_ok(), "raggraph_multihop should succeed");

    let response_text = result.unwrap().content[0].text.as_ref().unwrap();
    let response: Value = serde_json::from_str(response_text)?;

    let metadata = response.get("metadata").unwrap().as_object().unwrap();

    // This test will FAIL before implementation because metadata doesn't exist
    assert!(
        metadata.get("graph_traversal_ms").is_some(),
        "multihop metadata should contain 'graph_traversal_ms'"
    );

    let graph_traversal_time = metadata.get("graph_traversal_ms").unwrap().as_u64().unwrap();

    assert!(
        graph_traversal_time > 0,
        "metadata.graph_traversal_ms should be greater than 0"
    );

    Ok(())
}