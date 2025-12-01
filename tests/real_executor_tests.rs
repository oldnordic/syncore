//! Real Executor Integration Tests
//!
//! Tests for Phase 6 - Full Real Executor Wiring
//! Tests validate that RealExecutor returns proper envelope-wrapped responses.

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

/// Helper to create a RealExecutor with fresh state
fn create_test_executor(suffix: &str) -> RealExecutor {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let db_path = format!(":memory:_real_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(":memory:").expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);

    RealExecutor::new(Arc::new(state))
}

// ============================================================================
// Memory Tool Tests
// ============================================================================

#[test]
fn test_memory_store_real_execution() {
    let executor = create_test_executor("mem_store");
    let params = json!({
        "key": "test_key_real",
        "value": "test_value_real",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("memory_store", &params).await });

    // Should succeed
    assert!(result.is_ok(), "Real memory_store should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert_eq!(data["stored"], true);

    // Verify side effect: value should be in state
    let verify_params = json!({
        "key": "test_key_real",
        "dry_run": false
    });

    let verify_result = rt
        .block_on(async { executor.execute_real_tool_async("memory_query", &verify_params).await });

    assert!(verify_result.is_ok());
    let verify_envelope = verify_result.unwrap();
    assert_success_envelope(&verify_envelope);
    let verify_data = unwrap_data(&verify_envelope);
    assert_eq!(verify_data["value"], "test_value_real");
    assert_eq!(verify_data["found"], true);
}

#[test]
fn test_memory_store_dry_run() {
    let executor = create_test_executor("mem_store_dry");
    let params = json!({
        "key": "test_key_dry",
        "value": "test_value_dry",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("memory_store", &params).await });

    // Should succeed but not persist
    assert!(result.is_ok());
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run indication
    let data = unwrap_data(&envelope);
    assert!(data.to_string().contains("DRY RUN") || data.get("dry_run").is_some());

    // Verify NO side effect: value should NOT be in state
    let verify_params = json!({
        "key": "test_key_dry",
        "dry_run": false
    });

    let verify_result = rt
        .block_on(async { executor.execute_real_tool_async("memory_query", &verify_params).await });

    assert!(verify_result.is_ok());
    let verify_envelope = verify_result.unwrap();
    assert_success_envelope(&verify_envelope);
    let verify_data = unwrap_data(&verify_envelope);
    assert_eq!(
        verify_data["found"], false,
        "Dry run should not persist value. Found: {:?}",
        verify_data
    );
}

#[test]
fn test_memory_query_real_execution() {
    let executor = create_test_executor("mem_query");

    // First store a value
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store_params = json!({
        "key": "query_test_key",
        "value": "query_test_value",
        "dry_run": false
    });

    let store_result = rt
        .block_on(async { executor.execute_real_tool_async("memory_store", &store_params).await });
    assert!(store_result.is_ok(), "Store should succeed");

    // Now query it
    let query_params = json!({
        "key": "query_test_key",
        "dry_run": false
    });

    let result = rt
        .block_on(async { executor.execute_real_tool_async("memory_query", &query_params).await });

    assert!(result.is_ok());
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert_eq!(data["value"], "query_test_value");
    assert_eq!(data["found"], true);
}

#[test]
fn test_memory_query_not_found() {
    let executor = create_test_executor("mem_query_nf");
    let params = json!({
        "key": "nonexistent_key",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("memory_query", &params).await });

    assert!(result.is_ok());
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert_eq!(data["found"], false);
    assert!(data["value"].is_null() || data["value"] == json!(null));
}

#[test]
fn test_memory_store_error_handling() {
    let executor = create_test_executor("mem_store_err");
    let params = json!({
        "key": "test_key"
        // Missing 'value' parameter - should error
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("memory_store", &params).await });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(result.is_ok(), "RealExecutor should return Ok(Value) even for errors");
    let envelope = result.unwrap();
    assert_error_envelope(&envelope);
    let error = unwrap_error(&envelope);
    assert_error_fields(error);
}

// ============================================================================
// Placeholder tests for other tool groups
// ============================================================================

#[test]
#[ignore] // Will implement after memory tests pass
fn test_vector_insert_real_execution() {
    // TDD: Implement after memory tools wired
    panic!("Not yet implemented");
}

#[test]
#[ignore] // Will implement after memory tests pass
fn test_vector_search_real_execution() {
    // TDD: Implement after memory tools wired
    panic!("Not yet implemented");
}

#[test]
#[ignore] // Will implement after vector tests pass
fn test_task_create_real_execution() {
    // TDD: Implement after vector tools wired
    panic!("Not yet implemented");
}

// Add more placeholder tests for remaining 44 tools...
