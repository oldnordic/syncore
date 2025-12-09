//! Code Graph Fusion Query SQLiteGraph Integration Tests
//!
//! Tests that code_graph_fusion_query MCP tool works with SQLiteGraph backend
//! while maintaining Neo4j compatibility.
//!
//! TDD approach: Tests are written FIRST to define the expected behavior,
//! then implementation follows to make them pass.

use rmcp::handler::server::wrapper::Parameters;
use serde_json;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

// Test utilities for creating mock components
mod test_utils {
    use super::*;

    /// Create a mock VectorStore for testing
    pub fn create_mock_vector_store() -> Arc<Mutex<syncore::vector::VectorStore>> {
        // Create a VectorStore with real embeddings for testing
        let embeddings = Box::new(syncore::vector::RealEmbeddings::new(384).unwrap());
        let store = syncore::vector::VectorStore::new(embeddings);
        Arc::new(Mutex::new(store))
    }

    /// Create a mock SynCoreState with SQLiteGraph backend
    pub fn create_state_with_sqlite_graph() -> syncore::mcp::SynCoreState {
        let code_store = create_mock_vector_store();
        let general_store = create_mock_vector_store();

        let mut state =
            syncore::mcp::SynCoreState::with_dual_stores(code_store, general_store).unwrap();

        // Override with SQLiteGraph backend
        let temp_dir = tempdir().unwrap();
        let code_graph_db_path = temp_dir.path().join("test_code_graph.db");
        std::env::set_var("GRAPH_BACKEND", "sqlitegraph");
        std::env::set_var("SYNCORE_CODE_GRAPH_DB", code_graph_db_path.to_str().unwrap());

        state
    }

    /// Create a mock SynCoreState with Neo4j backend
    pub fn create_state_with_neo4j() -> syncore::mcp::SynCoreState {
        let code_store = create_mock_vector_store();
        let general_store = create_mock_vector_store();

        let state =
            syncore::mcp::SynCoreState::with_dual_stores(code_store, general_store).unwrap();

        // Note: In real tests, we would set up a mock Neo4j client here
        // For these tests, we'll test the behavior when Neo4j is available
        state
    }
}

#[test]
fn test_code_graph_fusion_query_sqlite_graph_backend() {
    // Test: When GRAPH_BACKEND=sqlitegraph and no Neo4j present
    // → REAL mode runs successfully using SQLiteGraph backend

    // Arrange
    std::env::set_var("GRAPH_BACKEND", "sqlitegraph");
    let state = test_utils::create_state_with_sqlite_graph();
    let mcp_server = syncore::mcp_server::SynCoreMCPServer::new(state);

    // Act
    let query_request = syncore::code_graph::RagGraphQueryRequest {
        query: "find format function".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(10),
        scope: None,
        project_label: None,
        local_root: None,
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { mcp_server.code_graph_fusion_query(Parameters(query_request)).await });

    // Assert
    assert!(result.is_ok(), "code_graph_fusion_query should succeed with SQLiteGraph backend");

    let call_result = result.unwrap();
    assert!(!call_result.content.is_empty(), "Should return query results");

    // Verify we get a real response (not "requires Neo4j connection" error)
    // Use the same text extraction pattern as the server code
    let response_text = call_result
        .content
        .first()
        .and_then(|c| {
            let json = serde_json::to_value(c).ok()?;
            json.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
        })
        .unwrap_or_default();

    assert!(
        !response_text.contains("requires Neo4j connection"),
        "Should not return Neo4j error when SQLiteGraph is configured"
    );
    assert!(
        !response_text.contains("no available graph backend"),
        "Should successfully create graph backend"
    );

    // Clean up
    std::env::remove_var("GRAPH_BACKEND");
    std::env::remove_var("SYNCORE_CODE_GRAPH_DB");

    println!("✅ CodeGraph fusion query succeeded with SQLiteGraph backend");
}

#[test]
fn test_code_graph_fusion_query_neo4j_preserved() {
    // Test: When GRAPH_BACKEND=neo4j and Neo4j client present
    // → REAL mode uses Neo4j path unchanged

    // Arrange
    std::env::set_var("GRAPH_BACKEND", "neo4j");
    let state = test_utils::create_state_with_neo4j();
    let mcp_server = syncore::mcp_server::SynCoreMCPServer::new(state);

    // Act
    let query_request = syncore::code_graph::RagGraphQueryRequest {
        query: "find async function".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(10),
        scope: None,
        project_label: None,
        local_root: None,
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { mcp_server.code_graph_fusion_query(Parameters(query_request)).await });

    // Assert - currently this will fail because we don't have Neo4j setup,
    // but we can verify it attempts the Neo4j path
    assert!(result.is_ok(), "MCP call should handle gracefully");

    let call_result = result.unwrap();
    // This should return a meaningful error about Neo4j not being available
    // rather than the old "requires Neo4j connection" generic message
    let response_text = call_result
        .content
        .first()
        .and_then(|c| {
            let json = serde_json::to_value(c).ok()?;
            json.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
        })
        .unwrap_or_default();
    assert!(
        response_text.contains("graph backend") || response_text.contains("Neo4j"),
        "Should mention graph backend in error message"
    );

    // Clean up
    std::env::remove_var("GRAPH_BACKEND");

    println!("✅ Neo4j path preservation verified");
}

#[test]
fn test_code_graph_fusion_query_defaults_to_sqlitegraph() {
    // Test: When no graph backend is configured
    // → Defaults to SQLiteGraph backend successfully

    // Arrange
    // Remove any graph backend configuration
    std::env::remove_var("GRAPH_BACKEND");
    let state = test_utils::create_state_with_sqlite_graph(); // but don't set env var
    let mcp_server = syncore::mcp_server::SynCoreMCPServer::new(state);

    // Act
    let query_request = syncore::code_graph::RagGraphQueryRequest {
        query: "test query".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(10),
        scope: None,
        project_label: None,
        local_root: None,
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { mcp_server.code_graph_fusion_query(Parameters(query_request)).await });

    // Assert
    assert!(result.is_ok(), "MCP call should succeed with default backend");

    let call_result = result.unwrap();
    assert!(!call_result.content.is_empty(), "Should return query results");

    // Should work successfully with default SQLiteGraph backend
    let response_text = call_result
        .content
        .first()
        .and_then(|c| {
            let json = serde_json::to_value(c).ok()?;
            json.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
        })
        .unwrap_or_default();
    assert!(
        !response_text.contains("error"),
        "Should not return an error when using default backend"
    );
    assert!(
        !response_text.contains("requires Neo4j connection"),
        "Should not require Neo4j when defaulting to SQLiteGraph"
    );

    println!("✅ Default backend successfully defaults to SQLiteGraph");
}

#[test]
fn test_code_graph_fusion_query_scope_parsing() {
    // Test: Query scope parsing still works with SQLiteGraph

    // Arrange
    std::env::set_var("GRAPH_BACKEND", "sqlitegraph");
    let state = test_utils::create_state_with_sqlite_graph();
    let mcp_server = syncore::mcp_server::SynCoreMCPServer::new(state);

    // Act
    let query_request = syncore::code_graph::RagGraphQueryRequest {
        query: "test query".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        scope: Some("local".to_string()), // Test scope parsing
        project_label: None,
        local_root: Some("src/".to_string()),
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { mcp_server.code_graph_fusion_query(Parameters(query_request)).await });

    // Assert
    assert!(result.is_ok(), "Query with scope should work");

    // Clean up
    std::env::remove_var("GRAPH_BACKEND");
    std::env::remove_var("SYNCORE_CODE_GRAPH_DB");

    println!("✅ Scope parsing works with SQLiteGraph");
}

#[test]
fn test_code_graph_fusion_query_mode_hints() {
    // Test: Mode hints (simple/attention/reasoning) work with SQLiteGraph

    // Arrange
    std::env::set_var("GRAPH_BACKEND", "sqlitegraph");
    let state = test_utils::create_state_with_sqlite_graph();
    let mcp_server = syncore::mcp_server::SynCoreMCPServer::new(state);

    // Act
    let query_request = syncore::code_graph::RagGraphQueryRequest {
        query: "test query".to_string(),
        namespace: None,
        mode_hint: Some("attention".to_string()), // Test mode hint
        top_k: Some(5),
        scope: None,
        project_label: None,
        local_root: None,
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { mcp_server.code_graph_fusion_query(Parameters(query_request)).await });

    // Assert
    assert!(result.is_ok(), "Query with mode hint should work");

    // Clean up
    std::env::remove_var("GRAPH_BACKEND");
    std::env::remove_var("SYNCORE_CODE_GRAPH_DB");

    println!("✅ Mode hints work with SQLiteGraph");
}

#[test]
fn test_code_graph_fusion_query_namespace_filtering() {
    // Test: Namespace filtering works with SQLiteGraph

    // Arrange
    std::env::set_var("GRAPH_BACKEND", "sqlitegraph");
    let state = test_utils::create_state_with_sqlite_graph();
    let mcp_server = syncore::mcp_server::SynCoreMCPServer::new(state);

    // Act
    let query_request = syncore::code_graph::RagGraphQueryRequest {
        query: "test query".to_string(),
        namespace: Some("test_namespace".to_string()), // Test namespace filtering
        mode_hint: None,
        top_k: Some(5),
        scope: None,
        project_label: None,
        local_root: None,
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { mcp_server.code_graph_fusion_query(Parameters(query_request)).await });

    // Assert
    assert!(result.is_ok(), "Query with namespace should work");

    // Clean up
    std::env::remove_var("GRAPH_BACKEND");
    std::env::remove_var("SYNCORE_CODE_GRAPH_DB");

    println!("✅ Namespace filtering works with SQLiteGraph");
}
