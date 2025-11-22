//! Real Executor Graph Tools Tests
//!
//! Phase 6.6 - TDD tests for graph query/insert/relate tools
//! These tests validate that RealExecutor returns proper envelope-wrapped responses.
//!
//! Tools covered:
//! 1. graph_query
//! 2. graph_insert
//! 3. graph_relate

mod real_executor_test_helpers;
use real_executor_test_helpers::{
    assert_error_envelope, assert_error_fields, assert_success_envelope, unwrap_data, unwrap_error,
};

use serde_json::json;
use std::sync::{Arc, Mutex};
use syncore::macro_tools::executor_real::RealExecutor;
use syncore::memory::Memory;
use syncore::router::SynCoreState;
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};

/// Helper to create a RealExecutor with fresh state (neo4j = None)
fn create_test_executor(suffix: &str) -> RealExecutor {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = format!(":memory:_graph_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);
    // Note: state.neo4j will be None, so all real operations should fail with GraphUnavailable

    RealExecutor::new(Arc::new(state))
}

// ============================================================================
// Test 1: graph_query with neo4j = None → GraphUnavailable
// ============================================================================

#[test]
fn test_graph_query_real() {
    let executor = create_test_executor("graph_query");

    let params = json!({
        "cypher": "MATCH (n) RETURN n LIMIT 10",
        "params": null,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("graph_query", &params)
            .await
    });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(
        result.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope = result.unwrap();
    assert_error_envelope(&envelope);
    let error = unwrap_error(&envelope);
    assert_error_fields(error);

    // Verify error message indicates graph unavailable
    let err_msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("");
    assert!(
        err_msg.contains("Graph")
            || err_msg.contains("unavailable")
            || err_msg.contains("disabled"),
        "Error should indicate graph unavailable: {:?}",
        err_msg
    );
}

// ============================================================================
// Test 2: graph_query respects dry_run
// ============================================================================

#[test]
fn test_graph_query_respects_dry_run() {
    let executor = create_test_executor("graph_query_dry");

    let params = json!({
        "cypher": "MATCH (n) RETURN n LIMIT 10",
        "params": null,
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("graph_query", &params)
            .await
    });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run indication
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some()
            || data.to_string().contains("DRY RUN")
            || data.get("results").is_some(), // Synthetic results acceptable
        "Dry run should return valid response: {:?}",
        data
    );
}

// ============================================================================
// Test 3: graph_insert with neo4j = None → GraphUnavailable
// ============================================================================

#[test]
fn test_graph_insert_real() {
    let executor = create_test_executor("graph_insert");

    let params = json!({
        "cypher": "CREATE (n:TestNode {name: 'test'}) RETURN n",
        "params": null,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("graph_insert", &params)
            .await
    });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(
        result.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope = result.unwrap();
    assert_error_envelope(&envelope);
    let error = unwrap_error(&envelope);
    assert_error_fields(error);

    // Verify error message indicates graph unavailable
    let err_msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("");
    assert!(
        err_msg.contains("Graph")
            || err_msg.contains("unavailable")
            || err_msg.contains("disabled"),
        "Error should indicate graph unavailable: {:?}",
        err_msg
    );
}

// ============================================================================
// Test 4: graph_insert respects dry_run
// ============================================================================

#[test]
fn test_graph_insert_respects_dry_run() {
    let executor = create_test_executor("graph_insert_dry");

    let params = json!({
        "cypher": "CREATE (n:TestNode {name: 'test'}) RETURN n",
        "params": null,
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("graph_insert", &params)
            .await
    });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run indication
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some()
            || data.to_string().contains("DRY RUN")
            || data.get("created").is_some(), // Synthetic result acceptable
        "Dry run should return valid response: {:?}",
        data
    );
}

// ============================================================================
// Test 5: graph_relate with neo4j = None → GraphUnavailable
// ============================================================================

#[test]
fn test_graph_relate_real() {
    let executor = create_test_executor("graph_relate");

    let params = json!({
        "from_id": 1,
        "to_id": 2,
        "rel_type": "DEPENDS_ON",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("graph_relate", &params)
            .await
    });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(
        result.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope = result.unwrap();
    assert_error_envelope(&envelope);
    let error = unwrap_error(&envelope);
    assert_error_fields(error);

    // Verify error message indicates graph unavailable
    let err_msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("");
    assert!(
        err_msg.contains("Graph")
            || err_msg.contains("unavailable")
            || err_msg.contains("disabled"),
        "Error should indicate graph unavailable: {:?}",
        err_msg
    );
}

// ============================================================================
// Test 6: graph_relate respects dry_run
// ============================================================================

#[test]
fn test_graph_relate_respects_dry_run() {
    let executor = create_test_executor("graph_relate_dry");

    let params = json!({
        "from_id": 1,
        "to_id": 2,
        "rel_type": "DEPENDS_ON",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("graph_relate", &params)
            .await
    });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run indication
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some()
            || data.to_string().contains("DRY RUN")
            || data.get("success").is_some(), // Synthetic result acceptable
        "Dry run should return valid response: {:?}",
        data
    );
}

// ============================================================================
// Test 7: Error handling - missing required parameters
// ============================================================================

#[test]
fn test_graph_tools_error_handling() {
    let executor = create_test_executor("graph_errors");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Test graph_query without cypher
    let params = json!({
        "dry_run": false
        // Missing 'cypher' - should error
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("graph_query", &params)
            .await
    });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(
        result.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope = result.unwrap();
    assert_error_envelope(&envelope);
    let error = unwrap_error(&envelope);
    assert_error_fields(error);

    // Test graph_insert without cypher
    let params2 = json!({
        "params": null
        // Missing 'cypher' - should error
    });

    let result2 = rt.block_on(async {
        executor
            .execute_real_tool_async("graph_insert", &params2)
            .await
    });

    assert!(
        result2.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope2 = result2.unwrap();
    assert_error_envelope(&envelope2);
    let error2 = unwrap_error(&envelope2);
    assert_error_fields(error2);

    // Test graph_relate without from_id
    let params3 = json!({
        "to_id": 2,
        "rel_type": "DEPENDS_ON"
        // Missing 'from_id' - should error
    });

    let result3 = rt.block_on(async {
        executor
            .execute_real_tool_async("graph_relate", &params3)
            .await
    });

    assert!(
        result3.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope3 = result3.unwrap();
    assert_error_envelope(&envelope3);
    let error3 = unwrap_error(&envelope3);
    assert_error_fields(error3);

    // Test graph_relate without rel_type
    let params4 = json!({
        "from_id": 1,
        "to_id": 2
        // Missing 'rel_type' - should error
    });

    let result4 = rt.block_on(async {
        executor
            .execute_real_tool_async("graph_relate", &params4)
            .await
    });

    assert!(
        result4.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope4 = result4.unwrap();
    assert_error_envelope(&envelope4);
    let error4 = unwrap_error(&envelope4);
    assert_error_fields(error4);
}
