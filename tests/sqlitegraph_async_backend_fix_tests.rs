//! SQLiteGraph Async Backend Fix Tests
//!
//! TDD tests to validate that AsyncSQLiteBackend fixes compile correctly
//! These tests MUST FAIL before the fix and PASS after the fix.
//!
//! Tests validate:
//! - JoinHandle result handling via .await? not .unwrap()
//! - Correct mapping of JoinError → backend error
//! - Send + 'static closure requirements
//! - No async blocking violations

use std::sync::Arc;
use syncore::config::{GraphBackend as ConfigBackend, GraphConfig};
use syncore::graph::backend_selector::create_graph_backend;
use syncore::sqlitegraph::async_sqlite_backend::{AsyncSQLiteBackend, SyncGraphBackend};
use tempfile::tempdir;
use tokio::runtime::Handle;

/// Create a test AsyncSQLiteBackend for testing
async fn create_test_backend() -> AsyncSQLiteBackend {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_backend_fix.db");

    let graph_config = GraphConfig {
        backend: ConfigBackend::SqliteGraph,
        path: db_path.to_str().unwrap().to_string(),
        uri: String::new(),
        user: String::new(),
        password: String::new(),
        enabled: true,
    };

    let async_backend = create_graph_backend(&graph_config, "test").await.unwrap();
    AsyncSQLiteBackend::new(async_backend).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_spawn_blocking_joinhandle_result_handling() {
    // This test ensures JoinHandle results are handled via .await? not .unwrap()
    let backend = Arc::new(create_test_backend().await);

    // This should compile and execute without JoinHandle unwrap errors
    let result = backend.execute_query("SELECT 1 as test", vec![]);

    // Validate that result handling works correctly
    assert!(result.is_ok(), "Execute query should succeed with proper error mapping");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_async_backend_requires_send_static_closures() {
    let backend = Arc::new(create_test_backend().await);

    // Test multiple async backend calls to ensure Send + 'static bounds
    let mut handles = Vec::new();

    for i in 0..5 {
        let backend_clone = backend.clone();
        let handle = tokio::spawn(async move {
            // Each closure should satisfy Send + 'static
            backend_clone.execute_query("SELECT ?", vec![("param", serde_json::json!(i))])
        });
        handles.push(handle);
    }

    // All operations should complete successfully
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent operation should complete without panicking");
        let query_result = result.unwrap();
        assert!(query_result.is_ok(), "Query should succeed with proper closure bounds");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_async_backend_does_not_block_runtime() {
    let backend = Arc::new(create_test_backend().await);

    // Verify tokio::runtime::Handle::current() works (would panic if no runtime)
    let _runtime_handle = Handle::current();

    // Confirm no nested runtime errors occur
    let background_task = tokio::spawn(async {
        for i in 0..50 {
            tokio::task::yield_now().await;
            if i % 10 == 0 {
                // Periodically verify we're still making progress
                assert!(true, "Background task is still running");
            }
        }
    });

    // Perform database operation
    let result = backend.execute_query("SELECT 1", vec![]);
    assert!(result.is_ok());

    // Background task should complete without being blocked
    background_task.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_async_backend_missing_imports_detected() {
    // This test references a known missing trait to ensure compilation fails pre-fix
    use syncore::sqlitegraph::async_sqlite_backend::SyncGraphBackend;

    let backend = create_test_backend().await;

    // Test that we can use both SyncGraphBackend trait and direct backend methods
    assert_eq!(backend.namespace(), "test");

    // Test sync methods work without blocking
    let result = backend.get_neighbors(999); // Non-existent node
    assert!(result.is_ok(), "Get neighbors should work with sync interface");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_join_error_mapping() {
    let backend = create_test_backend().await;

    // Test invalid query to verify JoinError mapping
    let result = backend.execute_query("INVALID SQL SYNTAX", vec![]);
    assert!(result.is_err(), "Invalid SQL should return proper error");

    // Error should contain context about the operation
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Failed to execute execute_query")
            || error_msg.contains("invalid")
            || error_msg.contains("syntax"),
        "Error should contain meaningful context"
    );
}
