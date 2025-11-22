//! Real Executor Logs Tools Tests
//!
//! Phase 6.11 - TDD tests for logs_tail tool
//! These tests validate that RealExecutor returns proper envelope-wrapped responses.
//!
//! Tool covered:
//! 1. logs_tail

mod real_executor_test_helpers;
use real_executor_test_helpers::{
    assert_error_envelope, assert_error_fields, assert_success_envelope, unwrap_data, unwrap_error,
};

use serde_json::json;
use std::fs::File;
use std::io::Write as IoWrite;
use std::sync::{Arc, Mutex};
use syncore::macro_tools::executor_real::RealExecutor;
use syncore::memory::Memory;
use syncore::router::SynCoreState;
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};
use tempfile::TempDir;

/// Helper to create a RealExecutor with fresh state
fn create_test_executor(suffix: &str) -> RealExecutor {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = format!(":memory:_logs_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);

    RealExecutor::new(Arc::new(state))
}

/// Helper to create a temp log file with test data
fn create_temp_log_file(lines: &[&str]) -> (TempDir, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let log_path = temp_dir.path().join("test.log");
    let mut file = File::create(&log_path).expect("Failed to create log file");

    for line in lines {
        writeln!(file, "{}", line).expect("Failed to write log line");
    }

    (temp_dir, log_path.to_str().unwrap().to_string())
}

// ============================================================================
// Test 1: logs_tail real execution with limit
// ============================================================================

#[test]
fn test_logs_tail_real() {
    let executor = create_test_executor("tail");

    // Create temp log file with 5 lines
    let (_temp_dir, log_path) = create_temp_log_file(&[
        "[INFO] Line 1",
        "[INFO] Line 2",
        "[INFO] Line 3",
        "[INFO] Line 4",
        "[INFO] Line 5",
    ]);

    let params = json!({
        "file_path": log_path,
        "n": 3,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("logs_tail", &params).await });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real logs_tail should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    let lines = data.get("lines").expect("Should have 'lines' field");
    assert!(lines.is_array(), "Lines should be an array");

    let lines_arr = lines.as_array().unwrap();
    assert_eq!(lines_arr.len(), 3, "Should return exactly 3 lines");

    // Verify it's the last 3 lines
    assert!(lines_arr[0].as_str().unwrap().contains("Line 3"));
    assert!(lines_arr[1].as_str().unwrap().contains("Line 4"));
    assert!(lines_arr[2].as_str().unwrap().contains("Line 5"));
}

// ============================================================================
// Test 2: logs_tail respects dry_run
// ============================================================================

#[test]
fn test_logs_tail_respects_dry_run() {
    let executor = create_test_executor("tail_dry");

    let params = json!({
        "file_path": "/tmp/nonexistent.log",
        "n": 10,
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("logs_tail", &params).await });

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
// Test 3: logs_tail default limit
// ============================================================================

#[test]
fn test_logs_tail_default_limit() {
    let executor = create_test_executor("tail_default");

    // Create log file with 100 lines
    let lines: Vec<String> = (1..=100).map(|i| format!("[INFO] Line {}", i)).collect();
    let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    let (_temp_dir, log_path) = create_temp_log_file(&line_refs);

    // Call without 'n' parameter - should default to 50
    let params = json!({
        "file_path": log_path,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("logs_tail", &params).await });

    assert!(result.is_ok(), "Should succeed with default limit");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    let lines = data.get("lines").expect("Should have 'lines' field");
    let lines_arr = lines.as_array().unwrap();

    // Default limit should be 50
    assert_eq!(lines_arr.len(), 50, "Default limit should be 50");

    // Should be last 50 lines (51-100)
    assert!(lines_arr[0].as_str().unwrap().contains("Line 51"));
    assert!(lines_arr[49].as_str().unwrap().contains("Line 100"));
}

// ============================================================================
// Test 4: logs_tail error handling - nonexistent file
// ============================================================================

#[test]
fn test_logs_tail_nonexistent_file() {
    let executor = create_test_executor("tail_nonexistent");

    let params = json!({
        "file_path": "/tmp/definitely_does_not_exist_12345.log",
        "n": 10,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("logs_tail", &params).await });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(
        result.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope = result.unwrap();
    assert_error_envelope(&envelope);
    let error = unwrap_error(&envelope);
    assert_error_fields(error);
}

// ============================================================================
// Test 5: logs_tail zero lines (empty file)
// ============================================================================

#[test]
fn test_logs_tail_empty_file() {
    let executor = create_test_executor("tail_empty");

    // Create empty log file
    let (_temp_dir, log_path) = create_temp_log_file(&[]);

    let params = json!({
        "file_path": log_path,
        "n": 10,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("logs_tail", &params).await });

    // Should succeed
    assert!(result.is_ok(), "Should succeed with empty file");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    let lines = data.get("lines").expect("Should have 'lines' field");
    let lines_arr = lines.as_array().unwrap();

    assert_eq!(lines_arr.len(), 0, "Empty file should return empty array");
}

// ============================================================================
// Test 6: logs_tail missing file_path parameter
// ============================================================================

#[test]
fn test_logs_tail_missing_file_path() {
    let executor = create_test_executor("tail_missing_path");

    let params = json!({
        "n": 10,
        "dry_run": false
        // Missing 'file_path' - should error
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("logs_tail", &params).await });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(
        result.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope = result.unwrap();
    assert_error_envelope(&envelope);
    let error = unwrap_error(&envelope);
    assert_error_fields(error);
}

// ============================================================================
// Test 7: logs_tail with n=0
// ============================================================================

#[test]
fn test_logs_tail_zero_limit() {
    let executor = create_test_executor("tail_zero");

    let (_temp_dir, log_path) =
        create_temp_log_file(&["[INFO] Line 1", "[INFO] Line 2", "[INFO] Line 3"]);

    let params = json!({
        "file_path": log_path,
        "n": 0,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("logs_tail", &params).await });

    assert!(result.is_ok(), "Should succeed with n=0");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    let lines = data.get("lines").expect("Should have 'lines' field");
    let lines_arr = lines.as_array().unwrap();

    assert_eq!(lines_arr.len(), 0, "n=0 should return empty array");
}
