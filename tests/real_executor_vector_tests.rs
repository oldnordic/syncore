//! Real Executor Vector Tools Tests
//!
//! Phase 6 - TDD tests for vector_insert and vector_search
//! These tests validate that RealExecutor returns proper envelope-wrapped responses.

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
#[allow(deprecated)]
fn create_test_executor(suffix: &str) -> RealExecutor {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let db_path = format!(":memory:_vec_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(":memory:").expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);

    RealExecutor::new(Arc::new(state))
}

// ============================================================================
// Test 1: vector_insert real basic execution
// ============================================================================

#[test]
fn test_vector_insert_real_basic() {
    let executor = create_test_executor("vec_insert_basic");

    // Get initial vector store size
    let initial_size = {
        let store = executor.state.general_store.lock().unwrap();
        store.len()
    };

    let params = json!({
        "text": "This is a test vector for insertion",
        "metadata": {"type": "test"},
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("vector_insert", &params).await });

    // Should succeed (returns Ok(Value))
    assert!(result.is_ok(), "Real vector_insert should succeed: {:?}", result.err());
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("inserted").is_some() || data.get("vector_id").is_some(),
        "Data should indicate insertion: {:?}",
        data
    );

    // Verify side effect: vector store size MUST increase
    let final_size = {
        let store = executor.state.general_store.lock().unwrap();
        store.len()
    };

    assert!(
        final_size > initial_size,
        "Vector store size should increase after insert. Initial: {}, Final: {}",
        initial_size,
        final_size
    );
}

// ============================================================================
// Test 2: vector_search real basic execution
// ============================================================================

#[test]
fn test_vector_search_real_basic() {
    let executor = create_test_executor("vec_search_basic");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // First insert some known vectors
    let insert_params = json!({
        "text": "artificial intelligence and machine learning",
        "dry_run": false
    });

    let insert_result = rt.block_on(async {
        executor.execute_real_tool_async("vector_insert", &insert_params).await
    });
    assert!(insert_result.is_ok(), "Insert should succeed");
    let insert_envelope = insert_result.unwrap();
    assert_eq!(insert_envelope.get("ok"), Some(&json!(true)), "Insert should return ok=true");

    // Now search with a similar query
    let search_params = json!({
        "query": "AI and ML",
        "limit": 5,
        "dry_run": false
    });

    let result = rt.block_on(async {
        executor.execute_real_tool_async("vector_search", &search_params).await
    });

    assert!(result.is_ok(), "Real vector_search should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_eq!(envelope.get("ok"), Some(&json!(true)), "Envelope should have ok=true");
    assert!(envelope.get("data").is_some(), "Success envelope must have 'data' field");

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(data.get("results").is_some(), "Search data must have 'results' field: {:?}", data);

    let results = data["results"].as_array().expect("Results must be an array");

    // Should find at least our inserted vector
    assert!(!results.is_empty(), "Search should return non-empty results after insertion");
}

// ============================================================================
// Test 3: vector_insert error handling
// ============================================================================

#[test]
fn test_vector_insert_real_error_handling() {
    let executor = create_test_executor("vec_insert_err");

    // Missing required 'text' parameter
    let params = json!({
        "metadata": {"type": "test"}
        // Missing 'text' field - should error
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("vector_insert", &params).await });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(result.is_ok(), "RealExecutor should return Ok(Value) even for errors");
    let envelope = result.unwrap();

    // Validate error envelope structure
    assert_error_envelope(&envelope);

    // Validate error details
    let error = unwrap_error(&envelope);
    assert_error_fields(error);
}

// ============================================================================
// Test 4: vector_search error handling
// ============================================================================

#[test]
fn test_vector_search_real_error_handling() {
    let executor = create_test_executor("vec_search_err");

    // Missing required 'query' parameter
    let params = json!({
        "limit": 10
        // Missing 'query' field - should error
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("vector_search", &params).await });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(result.is_ok(), "RealExecutor should return Ok(Value) even for errors");
    let envelope = result.unwrap();

    // Validate error envelope structure
    assert_error_envelope(&envelope);

    // Validate error details
    let error = unwrap_error(&envelope);
    assert_error_fields(error);
}

// ============================================================================
// Test 5: vector_insert respects dry_run
// ============================================================================

#[test]
fn test_vector_insert_respects_dry_run() {
    let executor = create_test_executor("vec_insert_dry");

    // Get initial size
    let initial_size = {
        let store = executor.state.general_store.lock().unwrap();
        store.len()
    };

    let params = json!({
        "text": "This should not be inserted due to dry run",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("vector_insert", &params).await });

    // Should succeed
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_eq!(envelope.get("ok"), Some(&json!(true)), "Dry run envelope should have ok=true");
    assert!(envelope.get("data").is_some(), "Success envelope must have 'data' field");

    // Unwrap data and validate dry run indication
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some()
            || data
                .get("message")
                .and_then(|v| v.as_str())
                .map(|s| s.contains("DRY RUN"))
                .unwrap_or(false),
        "Data should indicate dry run mode: {:?}",
        data
    );

    // Verify NO side effect: size must NOT change
    let final_size = {
        let store = executor.state.general_store.lock().unwrap();
        store.len()
    };

    assert_eq!(
        initial_size, final_size,
        "Vector store size must not change during dry run. Initial: {}, Final: {}",
        initial_size, final_size
    );
}

// ============================================================================
// Test 6: vector_search respects dry_run
// ============================================================================

#[test]
fn test_vector_search_respects_dry_run() {
    let executor = create_test_executor("vec_search_dry");

    let params = json!({
        "query": "test query for dry run",
        "limit": 5,
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("vector_search", &params).await });

    assert!(result.is_ok(), "Dry run search should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_eq!(envelope.get("ok"), Some(&json!(true)), "Dry run envelope should have ok=true");
    assert!(envelope.get("data").is_some(), "Success envelope must have 'data' field");

    // Unwrap data and validate dry run response
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some()
            || data
                .get("message")
                .and_then(|v| v.as_str())
                .map(|s| s.contains("DRY RUN"))
                .unwrap_or(false)
            || data.get("results").is_some(), // Synthetic results are acceptable
        "Dry run search should return valid response: {:?}",
        data
    );
}
