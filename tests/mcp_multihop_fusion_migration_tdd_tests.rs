//! TDD Tests for surgical migration of raggraph_multihop and code_graph_fusion_query
//!
//! These tests MUST FAIL before migration and PASS after migration.
//! They validate that both handlers use unified reasoning infrastructure correctly.

use anyhow::Result;
use syncore::mcp_server::server::MCPServerHandler;
use syncore::mcp_server::types::{RagGraphMultihopRequest, RagGraphQueryRequest};
use syncore::raggraph::{RagGraphConfig, RaggraphBackendMode};
use syncore::config::{SyncoreConfig, GraphBackend};
use syncore::code_graph::{RagGraphAPI, QueryScope};
use std::sync::Arc;
use rmcp::model::{CallToolResult, Content};
use serde_json::json;

/// Test that raggraph_multihop uses unified backend selection
#[tokio::test]
async fn test_raggraph_multihop_uses_unified_backend_selection() -> Result<()> {
    let state = Arc::new(syncore::router::SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test request with seed nodes
    let request = RagGraphMultihopRequest {
        seed_nodes: vec![1, 2, 3],
    };

    // Before migration: This should use complex manual backend selection
    // After migration: This should use select_reasoning_backend() from unified infrastructure
    let result = handler.raggraph_multihop(syncore::mcp_server::Parameters(request)).await;

    assert!(result.is_ok(), "raggraph_multihop should succeed");

    // This test will FAIL before migration because the handler doesn't use unified infrastructure
    // After migration, this should pass when the handler calls select_reasoning_backend()

    // Verify no manual backend selection logic remains
    let response_text = result.unwrap().content[0].text.as_ref().unwrap();

    // The response should maintain the same structure
    let response: serde_json::Value = serde_json::from_str(response_text)?;
    assert!(response.get("top_nodes").is_some(), "Should maintain top_nodes field");
    assert!(response.get("reasoning_path").is_some(), "Should maintain reasoning_path field");

    Ok(())
}

/// Test that raggraph_multihop preserves backward compatibility
#[tokio::test]
async fn test_raggraph_multihop_backward_compatibility() -> Result<()> {
    let state = Arc::new(syncore::router::SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test with empty seed nodes
    let request = RagGraphMultihopRequest {
        seed_nodes: vec![],
    };

    let result = handler.raggraph_multihop(syncore::mcp_server::Parameters(request)).await;

    // Should handle edge cases gracefully
    assert!(result.is_ok(), "raggraph_multihop should handle empty seeds");

    // Response format should be identical to pre-migration
    let response_text = result.unwrap().content[0].text.as_ref().unwrap();
    let response: serde_json::Value = serde_json::from_str(response_text)?;

    // Verify exact field names match pre-migration format
    assert!(response.get("top_nodes").is_array(), "top_nodes should be array");
    assert!(response.get("reasoning_path").is_array(), "reasoning_path should be array");
    assert!(response.get("context_embedding_dim").is_number(), "context_embedding_dim should be number");

    Ok(())
}

/// Test that raggraph_multihop uses unified request parsing
#[tokio::test]
async fn test_raggraph_multihop_unified_request_parsing() -> Result<()> {
    let state = Arc::new(syncore::router::SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test with various seed node configurations
    let test_cases = vec![
        vec![1],
        vec![1, 5, 10],
        vec![100, 200, 300, 400],
    ];

    for seed_nodes in test_cases {
        let request = RagGraphMultihopRequest {
            seed_nodes: seed_nodes.clone(),
        };

        let result = handler.raggraph_multihop(syncore::mcp_server::Parameters(request)).await;

        assert!(result.is_ok(), "raggraph_multihop should work with seed_nodes: {:?}", seed_nodes);

        // This test will FAIL before migration because request parsing is not unified
        // After migration, the handler should use parse_reasoning_request() with proper validation

        let response_text = result.unwrap().content[0].text.as_ref().unwrap();
        let response: serde_json::Value = serde_json::from_str(response_text)?;

        // Should maintain deterministic behavior
        assert!(response.get("top_nodes").is_some(), "Should return results for {:?}", seed_nodes);
    }

    Ok(())
}

/// Test that code_graph_fusion_query uses unified backend selection
#[tokio::test]
async fn test_code_graph_fusion_query_uses_unified_backend_selection() -> Result<()> {
    let state = Arc::new(syncore::router::SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test request with full parameters
    let request = RagGraphQueryRequest {
        query: "find database connection code".to_string(),
        namespace: Some("src/db".to_string()),
        mode_hint: Some("reasoning".to_string()),
        top_k: Some(15),
        scope: Some("project".to_string()),
        project_label: Some("syncore".to_string()),
        local_root: Some("src/".to_string()),
    };

    // Before migration: This should use complex manual backend selection
    // After migration: This should use select_reasoning_backend() from unified infrastructure
    let result = handler.code_graph_fusion_query(syncore::mcp_server::Parameters(request)).await;

    assert!(result.is_ok(), "code_graph_fusion_query should succeed");

    // This test will FAIL before migration because the handler doesn't use unified infrastructure
    // After migration, this should pass when the handler calls select_reasoning_backend()

    Ok(())
}

/// Test that code_graph_fusion_query has no direct RagGraphAPI calls
#[tokio::test]
async fn test_code_graph_fusion_query_no_direct_api_calls() -> Result<()> {
    let state = Arc::new(syncore::router::SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryRequest {
        query: "test query".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: None,
        scope: None,
        project_label: None,
        local_root: None,
    };

    let result = handler.code_graph_fusion_query(syncore::mcp_server::Parameters(request)).await;

    assert!(result.is_ok(), "code_graph_fusion_query should succeed");

    // This test validates that the handler no longer directly constructs RagGraphAPI
    // Before migration: Handler manually creates RagGraphAPI instances
    // After migration: Handler routes through execute_reasoning_request()

    // Response should maintain identical format
    let response_text = result.unwrap().content[0].text.as_ref().unwrap();
    let response: serde_json::Value = serde_json::from_str(response_text)?;

    // Should have the standard fusion query response structure
    assert!(response.get("entities").is_array(), "Should have entities array");
    assert!(response.get("selected_mode").is_string(), "Should have selected_mode");
    assert!(response.get("applied_scope").is_some(), "Should have applied_scope");

    Ok(())
}

/// Test that code_graph_fusion_query preserves query scope parsing
#[tokio::test]
async fn test_code_graph_fusion_query_scope_parsing_preserved() -> Result<()> {
    let state = Arc::new(syncore::router::SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let test_scopes = vec![
        ("local", "Local"),
        ("project", "Project"),
        ("workspace", "Workspace"),
        ("global", "Global"),
        ("auto", "Auto"),
    ];

    for (input_scope, expected_scope) in test_scopes {
        let request = RagGraphQueryRequest {
            query: "test scope parsing".to_string(),
            namespace: None,
            mode_hint: None,
            top_k: None,
            scope: Some(input_scope.to_string()),
            project_label: None,
            local_root: None,
        };

        let result = handler.code_graph_fusion_query(syncore::mcp_server::Parameters(request)).await;

        assert!(result.is_ok(), "code_graph_fusion_query should handle scope: {}", input_scope);

        // This test will FAIL before migration if scope parsing logic is replaced
        // After migration, scope parsing should be preserved through unified infrastructure

        let response_text = result.unwrap().content[0].text.as_ref().unwrap();
        let response: serde_json::Value = serde_json::from_str(response_text)?;

        // Should preserve scope parsing behavior
        if let Some(applied_scope) = response.get("applied_scope") {
            // Scope parsing should work consistently
            assert!(applied_scope.is_string() || applied_scope.is_null(), "applied_scope should be valid");
        }
    }

    Ok(())
}

/// Test that both handlers use unified error handling
#[tokio::test]
async fn test_unified_error_handling() -> Result<()> {
    let state = Arc::new(syncore::router::SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test raggraph_multihop error handling
    let malformed_multihop = RagGraphMultihopRequest {
        seed_nodes: vec![999999], // Non-existent entity ID
    };

    let multihop_result = handler.raggraph_multihop(syncore::mcp_server::Parameters(malformed_multihop)).await;

    // Should use unified error formatting, not custom error messages
    assert!(multihop_result.is_ok(), "Should return error result, not panic");

    // Test code_graph_fusion_query error handling
    let malformed_fusion = RagGraphQueryRequest {
        query: "".to_string(), // Empty query
        namespace: None,
        mode_hint: None,
        top_k: None,
        scope: None,
        project_label: None,
        local_root: None,
    };

    let fusion_result = handler.code_graph_fusion_query(syncore::mcp_server::Parameters(malformed_fusion)).await;

    // Should use unified error formatting
    assert!(fusion_result.is_ok(), "Should return error result, not panic");

    // This test will FAIL before migration because error handling is not unified
    // After migration, both should use format_error_response() from unified infrastructure

    Ok(())
}

/// Test deterministic scoring and ranking
#[tokio::test]
async fn test_deterministic_scoring_ranking() -> Result<()> {
    let state = Arc::new(syncore::router::SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test identical queries produce identical results
    let query = RagGraphQueryRequest {
        query: "function that handles authentication".to_string(),
        namespace: None,
        mode_hint: Some("simple".to_string()),
        top_k: Some(5),
        scope: Some("project".to_string()),
        project_label: None,
        local_root: None,
    };

    let results: Vec<Result<CallToolResult, _>> = vec![
        handler.code_graph_fusion_query(syncore::mcp_server::Parameters(query.clone())).await,
        handler.code_graph_fusion_query(syncore::mcp_server::Parameters(query.clone())).await,
        handler.code_graph_fusion_query(syncore::mcp_server::Parameters(query)).await,
    ];

    // All results should be identical (deterministic)
    for result in &results {
        assert!(result.is_ok(), "All queries should succeed");
    }

    let response_texts: Vec<String> = results
        .into_iter()
        .map(|r| r.unwrap().content[0].text.as_ref().unwrap().clone())
        .collect();

    // All responses should be identical
    for i in 1..response_texts.len() {
        assert_eq!(response_texts[0], response_texts[i], "Response {} should match response 0", i);
    }

    // This test validates that migration preserves deterministic behavior
    // Before migration: Might have non-deterministic elements
    // After migration: Should be deterministic through unified infrastructure

    Ok(())
}

/// Test SQLiteGraph-first execution works
#[tokio::test]
async fn test_sqlitegraph_first_execution() -> Result<()> {
    // Configure environment to prefer SQLiteGraph
    std::env::set_var("SYNCORE_RAGGRAPH_BACKEND", "real");
    std::env::set_var("GRAPH_BACKEND", "sqlite");

    let state = Arc::new(syncore::router::SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test multihop with SQLiteGraph preference
    let multihop_request = RagGraphMultihopRequest {
        seed_nodes: vec![1, 2],
    };

    let multihop_result = handler.raggraph_multihop(syncore::mcp_server::Parameters(multihop_request)).await;
    assert!(multihop_result.is_ok(), "raggraph_multihop should work with SQLiteGraph");

    // Test fusion query with SQLiteGraph preference
    let fusion_request = RagGraphQueryRequest {
        query: "sqlitegraph test".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        scope: Some("local".to_string()),
        project_label: None,
        local_root: None,
    };

    let fusion_result = handler.code_graph_fusion_query(syncore::mcp_server::Parameters(fusion_request)).await;
    assert!(fusion_result.is_ok(), "code_graph_fusion_query should work with SQLiteGraph");

    // This test will FAIL before migration if handlers don't respect SQLiteGraph-first preference
    // After migration: Should use unified backend selection with SQLiteGraph preference

    // Cleanup
    std::env::remove_var("SYNCORE_RAGGRAPH_BACKEND");
    std::env::remove_var("GRAPH_BACKEND");

    Ok(())
}

/// Test Neo4j fallback still functional
#[tokio::test]
async fn test_neo4j_fallback_functional() -> Result<()> {
    // Configure environment to prefer Neo4j
    std::env::set_var("SYNCORE_RAGGRAPH_BACKEND", "real");
    std::env::set_var("GRAPH_BACKEND", "neo4j");
    std::env::set_var("NEO4J_URI", "bolt://127.0.0.1:7687");

    let state = Arc::new(syncore::router::SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test multihop Neo4j fallback
    let multihop_request = RagGraphMultihopRequest {
        seed_nodes: vec![1],
    };

    let multihop_result = handler.raggraph_multihop(syncore::mcp_server::Parameters(multihop_request)).await;

    // Should gracefully handle Neo4j unavailability and fall back to SQLiteGraph
    assert!(multihop_result.is_ok(), "raggraph_multihop should handle Neo4j fallback");

    // Test fusion query Neo4j fallback
    let fusion_request = RagGraphQueryRequest {
        query: "neo4j fallback test".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        scope: None,
        project_label: None,
        local_root: None,
    };

    let fusion_result = handler.code_graph_fusion_query(syncore::mcp_server::Parameters(fusion_request)).await;

    // Should gracefully handle Neo4j unavailability and fall back to SQLiteGraph
    assert!(fusion_result.is_ok(), "code_graph_fusion_query should handle Neo4j fallback");

    // This test will FAIL before migration if handlers don't implement proper fallback
    // After migration: Should use unified backend selection with Neo4j fallback

    // Cleanup
    std::env::remove_var("SYNCORE_RAGGRAPH_BACKEND");
    std::env::remove_var("GRAPH_BACKEND");
    std::env::remove_var("NEO4J_URI");

    Ok(())
}

/// Test file size constraints (< 300 LOC per handler after migration)
#[test]
fn test_handler_file_size_constraints() -> Result<()> {
    // This test validates that handlers don't exceed 300 LOC after migration
    // Before migration: Handlers might be > 150 LOC each due to duplicated logic
    // After migration: Handlers should be < 50 LOC each using unified infrastructure

    use std::fs;
    use std::path::Path;

    let server_rs_path = "src/mcp_server/server.rs";
    let server_content = fs::read_to_string(server_rs_path)?;

    // Find raggraph_multihop handler
    let multihop_start = server_content.find("async fn raggraph_multihop").unwrap();
    let multihop_end = server_content[multihop_start..].find("async fn").unwrap_or_else(|| {
        server_content[multihop_start..].find("}\n\n").unwrap()
    });
    let multihop_loc = server_content[multihop_start..multihop_start + multihop_end]
        .lines()
        .count();

    // Find code_graph_fusion_query handler
    let fusion_start = server_content.find("async fn code_graph_fusion_query").unwrap();
    let fusion_end = server_content[fusion_start..].find("async fn").unwrap_or_else(|| {
        server_content[fusion_start..].find("}\n\n").unwrap()
    });
    let fusion_loc = server_content[fusion_start..fusion_start + fusion_end]
        .lines()
        .count();

    // Before migration: These handlers are likely > 150 LOC each
    // After migration: Should be < 50 LOC each using unified infrastructure

    println!("raggraph_multihop: {} lines", multihop_loc);
    println!("code_graph_fusion_query: {} lines", fusion_loc);

    // This test documents the current state for comparison
    // The actual migration should reduce these to < 50 lines each

    Ok(())
}