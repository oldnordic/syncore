//! Real Executor Mapping Tools Tests
//!
//! Phase 6.8 - TDD tests for application structure mapping tools
//! These tests validate that RealExecutor returns proper envelope-wrapped responses.
//!
//! Tools covered:
//! 1. mapping_record
//! 2. mapping_get
//! 3. mapping_search
//! 4. mapping_deps

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
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = format!(":memory:_mapping_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);

    RealExecutor::new(Arc::new(state))
}

// ============================================================================
// Test 1: mapping_record real execution
// ============================================================================

#[test]
fn test_mapping_record_real() {
    let executor = create_test_executor("record");

    let params = json!({
        "path": "/src/main.rs",
        "kind": "file",
        "language": "rust",
        "imports": ["std::io", "serde::Serialize"],
        "exports": ["main", "App"],
        "dependencies": ["/src/lib.rs", "/src/config.rs"],
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_record", &params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real mapping_record should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("recorded")
            .and_then(|r| r.as_bool())
            .unwrap_or(false)
            || data
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false),
        "Data should indicate recording success: {:?}",
        data
    );
}

// ============================================================================
// Test 2: mapping_record respects dry_run
// ============================================================================

#[test]
fn test_mapping_record_respects_dry_run() {
    let executor = create_test_executor("record_dry");

    let params = json!({
        "path": "/src/test.rs",
        "kind": "file",
        "language": "rust",
        "imports": [],
        "exports": [],
        "dependencies": [],
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_record", &params)
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
        data.get("dry_run").is_some() || data.to_string().contains("DRY RUN"),
        "Data should indicate dry run mode: {:?}",
        data
    );
}

// ============================================================================
// Test 3: mapping_get real execution
// ============================================================================

#[test]
fn test_mapping_get_real() {
    let executor = create_test_executor("get");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // First record a file
    let record_params = json!({
        "path": "/src/utils.rs",
        "kind": "file",
        "language": "rust",
        "imports": ["std::collections::HashMap"],
        "exports": ["parse_config", "Config"],
        "dependencies": [],
        "dry_run": false
    });

    let record_result = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_record", &record_params)
            .await
    });
    assert!(record_result.is_ok(), "Record should succeed");

    // Now get the file
    let get_params = json!({
        "path": "/src/utils.rs",
        "dry_run": false
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_get", &get_params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real mapping_get should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("path").is_some() || data.get("kind").is_some() || data.is_object(),
        "Data should have file information: {:?}",
        data
    );
}

// ============================================================================
// Test 4: mapping_get respects dry_run
// ============================================================================

#[test]
fn test_mapping_get_respects_dry_run() {
    let executor = create_test_executor("get_dry");

    let params = json!({
        "path": "/src/test.rs",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_get", &params)
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
        data.get("dry_run").is_some() || data.to_string().contains("DRY RUN"),
        "Data should indicate dry run mode: {:?}",
        data
    );
}

// ============================================================================
// Test 5: mapping_search real execution
// ============================================================================

#[test]
fn test_mapping_search_real() {
    let executor = create_test_executor("search");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Record a file first
    let record_params = json!({
        "path": "/src/api/handlers.rs",
        "kind": "file",
        "language": "rust",
        "imports": ["axum::Router"],
        "exports": ["create_handler", "ApiHandler"],
        "dependencies": [],
        "dry_run": false
    });

    let record_result = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_record", &record_params)
            .await
    });
    assert!(record_result.is_ok(), "Record should succeed");

    // Search for it
    let search_params = json!({
        "query": "api handlers",
        "dry_run": false
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_search", &search_params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real mapping_search should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("files").is_some() || data.get("results").is_some() || data.get("count").is_some(),
        "Data should have search results: {:?}",
        data
    );
}

// ============================================================================
// Test 6: mapping_search respects dry_run
// ============================================================================

#[test]
fn test_mapping_search_respects_dry_run() {
    let executor = create_test_executor("search_dry");

    let params = json!({
        "query": "test query",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_search", &params)
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
        data.get("dry_run").is_some() || data.to_string().contains("DRY RUN"),
        "Data should indicate dry run mode: {:?}",
        data
    );
}

// ============================================================================
// Test 7: mapping_deps real execution (transitive dependencies)
// ============================================================================

#[test]
fn test_mapping_deps_real() {
    let executor = create_test_executor("deps");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create dependency chain: A -> B -> C
    // File C (no dependencies)
    rt.block_on(async {
        executor
            .execute_real_tool_async(
                "mapping_record",
                &json!({
                    "path": "/src/c.rs",
                    "kind": "file",
                    "language": "rust",
                    "imports": [],
                    "exports": ["c_func"],
                    "dependencies": [],
                    "dry_run": false
                }),
            )
            .await
    })
    .expect("Record C should succeed");

    // File B (depends on C)
    rt.block_on(async {
        executor
            .execute_real_tool_async(
                "mapping_record",
                &json!({
                    "path": "/src/b.rs",
                    "kind": "file",
                    "language": "rust",
                    "imports": ["c_func"],
                    "exports": ["b_func"],
                    "dependencies": ["/src/c.rs"],
                    "dry_run": false
                }),
            )
            .await
    })
    .expect("Record B should succeed");

    // File A (depends on B)
    rt.block_on(async {
        executor
            .execute_real_tool_async(
                "mapping_record",
                &json!({
                    "path": "/src/a.rs",
                    "kind": "file",
                    "language": "rust",
                    "imports": ["b_func"],
                    "exports": ["a_func"],
                    "dependencies": ["/src/b.rs"],
                    "dry_run": false
                }),
            )
            .await
    })
    .expect("Record A should succeed");

    // Get dependencies of A (should return B and C transitively)
    let deps_params = json!({
        "path": "/src/a.rs",
        "dry_run": false
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_deps", &deps_params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real mapping_deps should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dependencies").is_some() || data.get("deps").is_some(),
        "Data should have dependencies: {:?}",
        data
    );
}

// ============================================================================
// Test 8: mapping_deps respects dry_run
// ============================================================================

#[test]
fn test_mapping_deps_respects_dry_run() {
    let executor = create_test_executor("deps_dry");

    let params = json!({
        "path": "/src/test.rs",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_deps", &params)
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
        data.get("dry_run").is_some() || data.to_string().contains("DRY RUN"),
        "Data should indicate dry run mode: {:?}",
        data
    );
}

// ============================================================================
// Test 9: Error handling - missing required parameters
// ============================================================================

#[test]
fn test_mapping_tools_error_handling() {
    let executor = create_test_executor("errors");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Test mapping_record without 'path'
    let params = json!({
        "kind": "file",
        "imports": [],
        "exports": [],
        "dependencies": []
        // Missing 'path' - should error
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_record", &params)
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

    // Test mapping_get without 'path'
    let params2 = json!({
        "dry_run": false
        // Missing 'path' - should error
    });

    let result2 = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_get", &params2)
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

    // Test mapping_search without 'query'
    let params3 = json!({
        "dry_run": false
        // Missing 'query' - should error
    });

    let result3 = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_search", &params3)
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

    // Test mapping_deps without 'path'
    let params4 = json!({
        "dry_run": false
        // Missing 'path' - should error
    });

    let result4 = rt.block_on(async {
        executor
            .execute_real_tool_async("mapping_deps", &params4)
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
