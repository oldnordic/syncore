//! Real Executor Application Tools Tests
//!
//! Phase 6.10 - TDD tests for application tracking tools
//! These tests validate that RealExecutor returns proper envelope-wrapped responses.
//!
//! Tools covered:
//! 1. application_record
//! 2. application_get
//! 3. application_history

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
    let db_path = format!(":memory:_application_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);

    RealExecutor::new(Arc::new(state))
}

// ============================================================================
// Test 1: application_record real execution
// ============================================================================

#[test]
fn test_application_record_real() {
    let executor = create_test_executor("record");

    let params = json!({
        "file_path": "/src/main.rs",
        "change_type": "modification",
        "old_content": "fn main() {}",
        "new_content": "fn main() { println!(\"Hello\"); }",
        "line_start": 1,
        "line_end": 1,
        "description": "Added hello world print",
        "task_id": 1,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt
        .block_on(async { executor.execute_real_tool_async("application_record", &params).await });

    // Should succeed
    assert!(result.is_ok(), "Real application_record should succeed: {:?}", result.err());
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("change_id").is_some()
            || data.get("success").and_then(|s| s.as_bool()).unwrap_or(false),
        "Data should indicate recording success: {:?}",
        data
    );
}

// ============================================================================
// Test 2: application_record respects dry_run
// ============================================================================

#[test]
fn test_application_record_respects_dry_run() {
    let executor = create_test_executor("record_dry");

    let params = json!({
        "file_path": "/src/test.rs",
        "change_type": "addition",
        "new_content": "// new test",
        "line_start": 1,
        "line_end": 1,
        "description": "Test change",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt
        .block_on(async { executor.execute_real_tool_async("application_record", &params).await });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run indication
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some() || data.to_string().contains("DRY RUN"),
        "Data should indicate dry run mode: {:?}",
        data
    );
}

// ============================================================================
// Test 3: application_get real execution
// ============================================================================

#[test]
fn test_application_get_real() {
    let executor = create_test_executor("get");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // First record 3 changes for task_id 42
    for i in 1..=3 {
        let record_params = json!({
            "file_path": format!("/src/file{}.rs", i),
            "change_type": "modification",
            "new_content": format!("content {}", i),
            "line_start": 1,
            "line_end": 1,
            "description": format!("Change {}", i),
            "task_id": 42,
            "dry_run": false
        });

        rt.block_on(async {
            executor.execute_real_tool_async("application_record", &record_params).await
        })
        .expect("Record should succeed");
    }

    // Now get all changes for task_id 42
    let get_params = json!({
        "task_id": 42,
        "dry_run": false
    });

    let result = rt
        .block_on(async { executor.execute_real_tool_async("application_get", &get_params).await });

    // Should succeed
    assert!(result.is_ok(), "Real application_get should succeed: {:?}", result.err());
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("changes").is_some() || data.get("count").is_some(),
        "Data should have changes information: {:?}",
        data
    );
}

// ============================================================================
// Test 4: application_get respects dry_run
// ============================================================================

#[test]
fn test_application_get_respects_dry_run() {
    let executor = create_test_executor("get_dry");

    let params = json!({
        "task_id": 1,
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("application_get", &params).await });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run indication
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some() || data.to_string().contains("DRY RUN"),
        "Data should indicate dry run mode: {:?}",
        data
    );
}

// ============================================================================
// Test 5: application_history real execution
// ============================================================================

#[test]
fn test_application_history_real() {
    let executor = create_test_executor("history");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Record multiple changes to the same file
    let file_path = "/src/utils.rs";

    for i in 1..=3 {
        let record_params = json!({
            "file_path": file_path,
            "change_type": "modification",
            "old_content": format!("version {}", i - 1),
            "new_content": format!("version {}", i),
            "line_start": 1,
            "line_end": 10,
            "description": format!("Update {}", i),
            "task_id": i,
            "dry_run": false
        });

        rt.block_on(async {
            executor.execute_real_tool_async("application_record", &record_params).await
        })
        .expect("Record should succeed");
    }

    // Get file history
    let history_params = json!({
        "file_path": file_path,
        "dry_run": false
    });

    let result = rt.block_on(async {
        executor.execute_real_tool_async("application_history", &history_params).await
    });

    // Should succeed
    assert!(result.is_ok(), "Real application_history should succeed: {:?}", result.err());
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("history").is_some() || data.get("count").is_some(),
        "Data should have history information: {:?}",
        data
    );
}

// ============================================================================
// Test 6: application_history respects dry_run
// ============================================================================

#[test]
fn test_application_history_respects_dry_run() {
    let executor = create_test_executor("history_dry");

    let params = json!({
        "file_path": "/src/test.rs",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt
        .block_on(async { executor.execute_real_tool_async("application_history", &params).await });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run indication
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some() || data.to_string().contains("DRY RUN"),
        "Data should indicate dry run mode: {:?}",
        data
    );
}

// ============================================================================
// Test 7: Error handling - missing required parameters
// ============================================================================

#[test]
fn test_application_tools_error_handling() {
    let executor = create_test_executor("errors");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Test application_record without 'file_path'
    let params = json!({
        "change_type": "modification",
        "description": "test",
        "line_start": 1,
        "line_end": 1
        // Missing 'file_path' - should error
    });

    let result = rt
        .block_on(async { executor.execute_real_tool_async("application_record", &params).await });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(result.is_ok(), "RealExecutor should return Ok(Value) even for errors");
    let envelope = result.unwrap();
    assert_error_envelope(&envelope);
    let error = unwrap_error(&envelope);
    assert_error_fields(error);

    // Test application_record without 'change_type'
    let params2 = json!({
        "file_path": "/src/test.rs",
        "description": "test",
        "line_start": 1,
        "line_end": 1
        // Missing 'change_type' - should error
    });

    let result2 = rt
        .block_on(async { executor.execute_real_tool_async("application_record", &params2).await });

    assert!(result2.is_ok(), "RealExecutor should return Ok(Value) even for errors");
    let envelope2 = result2.unwrap();
    assert_error_envelope(&envelope2);
    let error2 = unwrap_error(&envelope2);
    assert_error_fields(error2);

    // Test application_get without 'task_id'
    let params3 = json!({
        "dry_run": false
        // Missing 'task_id' - should error
    });

    let result3 =
        rt.block_on(async { executor.execute_real_tool_async("application_get", &params3).await });

    assert!(result3.is_ok(), "RealExecutor should return Ok(Value) even for errors");
    let envelope3 = result3.unwrap();
    assert_error_envelope(&envelope3);
    let error3 = unwrap_error(&envelope3);
    assert_error_fields(error3);

    // Test application_history without 'file_path'
    let params4 = json!({
        "dry_run": false
        // Missing 'file_path' - should error
    });

    let result4 = rt.block_on(async {
        executor.execute_real_tool_async("application_history", &params4).await
    });

    assert!(result4.is_ok(), "RealExecutor should return Ok(Value) even for errors");
    let envelope4 = result4.unwrap();
    assert_error_envelope(&envelope4);
    let error4 = unwrap_error(&envelope4);
    assert_error_fields(error4);
}
