//! Real Executor Code Tools Tests
//!
//! Phase 6.4 - TDD tests for code analysis/search/indexing tools
//! These tests validate that RealExecutor returns proper envelope-wrapped responses.
//!
//! Tools covered:
//! 1. parser_analyze
//! 2. parser_search
//! 3. code_index
//! 4. code_index_directory
//! 5. code_search

mod real_executor_test_helpers;
use real_executor_test_helpers::{
    assert_error_envelope, assert_error_fields, assert_success_envelope, unwrap_data, unwrap_error,
};

use serde_json::json;
use std::fs;
use std::io::Write as IoWrite;
use std::sync::{Arc, Mutex};
use syncore::macro_tools::executor_real::RealExecutor;
use syncore::memory::Memory;
use syncore::router::SynCoreState;
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};

/// Helper to create a RealExecutor with fresh state
fn create_test_executor(suffix: &str) -> RealExecutor {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = format!(":memory:_code_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);

    RealExecutor::new(Arc::new(state))
}

/// Create a temporary Rust file for testing
fn create_temp_rust_file(suffix: &str) -> (String, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join(format!("test_{}.rs", suffix));

    let content = r#"
pub struct TestStruct {
    pub field: i32,
}

impl TestStruct {
    pub fn new(field: i32) -> Self {
        Self { field }
    }

    pub fn get_field(&self) -> i32 {
        self.field
    }
}

pub fn helper_function() -> String {
    "helper".to_string()
}
"#;

    let mut file = fs::File::create(&file_path).expect("Failed to create file");
    file.write_all(content.as_bytes())
        .expect("Failed to write file");

    (file_path.to_string_lossy().to_string(), temp_dir)
}

// ============================================================================
// Test 1: parser_analyze real execution
// ============================================================================

#[test]
fn test_parser_analyze_real() {
    let executor = create_test_executor("parser_analyze");
    let (file_path, _temp_dir) = create_temp_rust_file("analyze");

    let params = json!({
        "file_path": file_path,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("parser_analyze", &params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real parser_analyze should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("entities").is_some()
            || data.get("functions").is_some()
            || data.get("structs").is_some(),
        "Data should have structural information: {:?}",
        data
    );
}

// ============================================================================
// Test 2: parser_analyze respects dry_run
// ============================================================================

#[test]
fn test_parser_analyze_respects_dry_run() {
    let executor = create_test_executor("parser_dry");
    let (file_path, _temp_dir) = create_temp_rust_file("dry");

    let params = json!({
        "file_path": file_path,
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("parser_analyze", &params)
            .await
    });

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
// Test 3: parser_search real execution
// ============================================================================

#[test]
fn test_parser_search_real() {
    let executor = create_test_executor("parser_search");
    let (_file_path, temp_dir) = create_temp_rust_file("search");
    let search_path = temp_dir.path().to_string_lossy().to_string();

    let params = json!({
        "pattern": "pub fn",
        "path": search_path,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("parser_search", &params)
            .await
    });

    assert!(
        result.is_ok(),
        "Real parser_search should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("matches").is_some(),
        "Data should have matches: {:?}",
        data
    );
}

// ============================================================================
// Test 4: parser_search respects dry_run
// ============================================================================

#[test]
fn test_parser_search_respects_dry_run() {
    let executor = create_test_executor("search_dry");

    let params = json!({
        "pattern": "test",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("parser_search", &params)
            .await
    });

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
// Test 5: code_index real execution
// ============================================================================

#[test]
fn test_code_index_real() {
    let executor = create_test_executor("code_index");
    let (file_path, _temp_dir) = create_temp_rust_file("index");

    let params = json!({
        "file_path": file_path,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("code_index", &params)
            .await
    });

    assert!(
        result.is_ok(),
        "Real code_index should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("indexed").is_some() || data.get("success").is_some(),
        "Data should indicate indexing success: {:?}",
        data
    );
}

// ============================================================================
// Test 6: code_index respects dry_run
// ============================================================================

#[test]
fn test_code_index_respects_dry_run() {
    let executor = create_test_executor("index_dry");
    let (file_path, _temp_dir) = create_temp_rust_file("dry_index");

    let params = json!({
        "file_path": file_path,
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("code_index", &params)
            .await
    });

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
// Test 7: code_index_directory real execution
// ============================================================================

#[test]
fn test_code_index_directory_real() {
    let executor = create_test_executor("index_dir");
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create multiple files
    for i in 1..=3 {
        let file_path = temp_dir.path().join(format!("file{}.rs", i));
        let mut file = fs::File::create(&file_path).expect("Failed to create file");
        file.write_all(b"pub fn test() {}")
            .expect("Failed to write");
    }

    let params = json!({
        "directory": temp_dir.path().to_string_lossy().to_string(),
        "pattern": "*.rs",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("code_index_directory", &params)
            .await
    });

    assert!(
        result.is_ok(),
        "Real code_index_directory should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("indexed_files").is_some() || data.get("count").is_some(),
        "Data should have indexed file count: {:?}",
        data
    );
}

// ============================================================================
// Test 8: code_index_directory respects dry_run
// ============================================================================

#[test]
fn test_code_index_directory_respects_dry_run() {
    let executor = create_test_executor("dir_dry");
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let params = json!({
        "directory": temp_dir.path().to_string_lossy().to_string(),
        "pattern": "*.rs",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("code_index_directory", &params)
            .await
    });

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
// Test 9: code_search real execution
// ============================================================================

#[test]
fn test_code_search_real() {
    let executor = create_test_executor("code_search");

    let params = json!({
        "query": "function test",
        "limit": 5,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("code_search", &params)
            .await
    });

    assert!(
        result.is_ok(),
        "Real code_search should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("results").is_some() || data.get("matches").is_some(),
        "Data should have search results: {:?}",
        data
    );
}

// ============================================================================
// Test 10: code_search respects dry_run
// ============================================================================

#[test]
fn test_code_search_respects_dry_run() {
    let executor = create_test_executor("search_dry_code");

    let params = json!({
        "query": "test",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("code_search", &params)
            .await
    });

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
// Test 11: Error handling - missing required parameters
// ============================================================================

#[test]
fn test_code_tools_error_handling() {
    let executor = create_test_executor("code_errors");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Test parser_analyze without file_path
    let params = json!({
        "dry_run": false
        // Missing 'file_path' - should error
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("parser_analyze", &params)
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

    // Test parser_search without pattern
    let params2 = json!({
        "dry_run": false
        // Missing 'pattern' - should error
    });

    let result2 = rt.block_on(async {
        executor
            .execute_real_tool_async("parser_search", &params2)
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

    // Test code_search without query
    let params3 = json!({
        "limit": 10
        // Missing 'query' - should error
    });

    let result3 = rt.block_on(async {
        executor
            .execute_real_tool_async("code_search", &params3)
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
}
