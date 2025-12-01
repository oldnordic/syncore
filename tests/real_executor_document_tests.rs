//! Real Executor Document Tools Tests
//!
//! Phase 6.5 - TDD tests for document indexing and search
//! These tests validate that RealExecutor returns proper envelope-wrapped responses.
//!
//! Tools covered:
//! 1. document_index
//! 2. document_search

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
#[allow(deprecated)]
fn create_test_executor(suffix: &str) -> RealExecutor {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let db_path = format!(":memory:_doc_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);

    RealExecutor::new(Arc::new(state))
}

/// Create a temporary text file for testing
fn create_temp_text_file(suffix: &str, content: &str) -> (String, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join(format!("doc_{}.txt", suffix));

    let mut file = fs::File::create(&file_path).expect("Failed to create file");
    file.write_all(content.as_bytes()).expect("Failed to write file");

    (file_path.to_string_lossy().to_string(), temp_dir)
}

// ============================================================================
// Test 1: document_index with single file
// ============================================================================

#[test]
fn test_document_index_file_real() {
    let executor = create_test_executor("doc_index_file");
    let (file_path, _temp_dir) = create_temp_text_file(
        "test1",
        "This is a test document about artificial intelligence and machine learning.",
    );

    // Get parent directory
    let dir_path = std::path::Path::new(&file_path).parent().unwrap().to_string_lossy().to_string();

    let params = json!({
        "directory": dir_path,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("document_index", &params).await });

    // Should succeed
    assert!(result.is_ok(), "Real document_index should succeed: {:?}", result.err());
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("indexed").is_some()
            || data.get("chunk_count").is_some()
            || data.get("success").is_some(),
        "Data should indicate indexing success: {:?}",
        data
    );
}

// ============================================================================
// Test 2: document_index with directory of multiple files
// ============================================================================

#[test]
fn test_document_index_directory_real() {
    let executor = create_test_executor("doc_index_dir");
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create multiple files
    for i in 1..=3 {
        let file_path = temp_dir.path().join(format!("doc{}.txt", i));
        let mut file = fs::File::create(&file_path).expect("Failed to create file");
        file.write_all(format!("Document {} content about topic {}", i, i).as_bytes())
            .expect("Failed to write");
    }

    let params = json!({
        "directory": temp_dir.path().to_string_lossy().to_string(),
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("document_index", &params).await });

    assert!(result.is_ok(), "Real document_index_directory should succeed: {:?}", result.err());
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("indexed").is_some() || data.get("chunk_count").is_some(),
        "Data should have indexed count: {:?}",
        data
    );
}

// ============================================================================
// Test 3: document_search after indexing
// ============================================================================

#[test]
fn test_document_search_real() {
    let executor = create_test_executor("doc_search");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // First index a document
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("search_doc.txt");
    let mut file = fs::File::create(&file_path).expect("Failed to create file");
    file.write_all(b"Machine learning and artificial intelligence are transforming technology.")
        .expect("Failed to write");

    let index_params = json!({
        "directory": temp_dir.path().to_string_lossy().to_string(),
        "dry_run": false
    });

    rt.block_on(async { executor.execute_real_tool_async("document_index", &index_params).await })
        .expect("Index should succeed");

    // Now search
    let search_params = json!({
        "query": "machine learning",
        "limit": 5,
        "dry_run": false
    });

    let result = rt.block_on(async {
        executor.execute_real_tool_async("document_search", &search_params).await
    });

    assert!(result.is_ok(), "Real document_search should succeed");
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
// Test 4: document_index respects dry_run
// ============================================================================

#[test]
fn test_document_index_respects_dry_run() {
    let executor = create_test_executor("doc_index_dry");
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let params = json!({
        "directory": temp_dir.path().to_string_lossy().to_string(),
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("document_index", &params).await });

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
// Test 5: document_search respects dry_run
// ============================================================================

#[test]
fn test_document_search_respects_dry_run() {
    let executor = create_test_executor("doc_search_dry");

    let params = json!({
        "query": "test query",
        "limit": 5,
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("document_search", &params).await });

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
        "Data should indicate dry run mode: {:?}",
        data
    );
}

// ============================================================================
// Test 6: Error handling - missing required parameters
// ============================================================================

#[test]
fn test_document_tools_error_handling() {
    let executor = create_test_executor("doc_errors");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Test document_index without directory
    let params = json!({
        "dry_run": false
        // Missing 'directory' - should error
    });

    let result =
        rt.block_on(async { executor.execute_real_tool_async("document_index", &params).await });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(result.is_ok(), "RealExecutor should return Ok(Value) even for errors");
    let envelope = result.unwrap();
    assert_error_envelope(&envelope);
    let error = unwrap_error(&envelope);
    assert_error_fields(error);

    // Test document_search without query
    let params2 = json!({
        "limit": 10
        // Missing 'query' - should error
    });

    let result2 =
        rt.block_on(async { executor.execute_real_tool_async("document_search", &params2).await });

    assert!(result2.is_ok(), "RealExecutor should return Ok(Value) even for errors");
    let envelope2 = result2.unwrap();
    assert_error_envelope(&envelope2);
    let error2 = unwrap_error(&envelope2);
    assert_error_fields(error2);
}
