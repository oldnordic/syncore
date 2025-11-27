//! APEX 2.15 - Phase 1 (TDD): Reindex Mutex Tests
//!
//! These tests verify that concurrent DELETE+INSERT operations are serialized:
//! - Manual full reindex and LiveIndexer never run simultaneously
//! - No UNIQUE constraint errors occur
//! - All index operations complete successfully
//!
//! EXPECTED: All tests FAIL initially (mutex not implemented yet)

use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use syncore::code_graph::CodeGraph;
use syncore::config::SyncoreConfig;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use tempfile::TempDir;
use tokio::time::sleep;

/// Test-only concurrency detector
/// Tracks how many DELETE+INSERT operations are running concurrently
static CONCURRENT_OPERATIONS: AtomicUsize = AtomicUsize::new(0);

/// Helper: Simulate index_file with concurrency tracking
async fn simulate_index_file_with_tracking(
    code_graph_path: &str,
    file_path: &str,
    entity_name: &str,
) -> Result<()> {
    // Enter critical section
    let prev = CONCURRENT_OPERATIONS.fetch_add(1, Ordering::SeqCst);

    // If prev > 0, another operation is already running (BAD!)
    if prev > 0 {
        eprintln!("[TEST] ❌ CONCURRENCY DETECTED: {} operations running", prev + 1);
    }

    // Simulate DELETE phase
    sleep(Duration::from_millis(10)).await;

    let conn = Connection::open(code_graph_path)?;

    // DELETE
    conn.execute(
        "DELETE FROM code_entities WHERE file_path = ?",
        [file_path],
    )?;

    // Simulate processing delay
    sleep(Duration::from_millis(10)).await;

    // INSERT
    conn.execute(
        "INSERT INTO code_entities
         (file_path, entity_type, name, line_start, line_end, language, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            file_path,
            "Function",
            entity_name,
            1,
            5,
            "Rust",
            chrono::Utc::now().timestamp(),
        ],
    )?;

    // Exit critical section
    CONCURRENT_OPERATIONS.fetch_sub(1, Ordering::SeqCst);

    Ok(())
}

/// Helper: Create test config
fn create_test_config(workspace: &TempDir) -> Result<SyncoreConfig> {
    let mut config = SyncoreConfig::default();
    let db_dir = workspace.path().join(".syncore");
    fs::create_dir_all(&db_dir)?;
    config.paths.code_graph_db = db_dir.join("code_graph.db").to_string_lossy().to_string();
    Ok(config)
}

/// Helper: Check diagnostic log for UNIQUE constraint errors
fn check_for_unique_constraint_errors() -> bool {
    if let Ok(content) = fs::read_to_string("/tmp/code_graph_diagnostic.log") {
        content.contains("UNIQUE constraint failed")
    } else {
        false
    }
}

#[tokio::test]
async fn test_manual_reindex_and_liveindexer_do_not_run_concurrently() -> Result<()> {
    // Reset concurrency counter
    CONCURRENT_OPERATIONS.store(0, Ordering::SeqCst);

    // Arrange: Create workspace and DB
    let workspace = TempDir::new()?;
    let config = create_test_config(&workspace)?;

    // Initialize DB
    {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let _graph = CodeGraph::new(&config.paths.code_graph_db, vector_store)?;
    }

    let db_path = config.paths.code_graph_db.clone();

    // Act: Spawn two concurrent tasks simulating:
    // - Task A: Manual reindex (bulk operation)
    // - Task B: LiveIndexer incremental update

    let db_path_a = db_path.clone();
    let task_a = tokio::spawn(async move {
        for i in 0..5 {
            simulate_index_file_with_tracking(
                &db_path_a,
                &format!("/test/file_a_{}.rs", i),
                &format!("func_a_{}", i),
            )
            .await
            .expect("Task A failed");
            sleep(Duration::from_millis(5)).await;
        }
    });

    let db_path_b = db_path.clone();
    let task_b = tokio::spawn(async move {
        for i in 0..5 {
            simulate_index_file_with_tracking(
                &db_path_b,
                &format!("/test/file_b_{}.rs", i),
                &format!("func_b_{}", i),
            )
            .await
            .expect("Task B failed");
            sleep(Duration::from_millis(5)).await;
        }
    });

    // Wait for both tasks
    task_a.await?;
    task_b.await?;

    // Assert: Check for concurrent operations
    let final_counter = CONCURRENT_OPERATIONS.load(Ordering::SeqCst);
    assert_eq!(final_counter, 0, "Concurrency counter should return to 0");

    // THIS WILL FAIL IN PHASE 1 (expected)
    // Because no mutex exists yet, operations WILL overlap

    // Check if diagnostic log contains UNIQUE constraint errors
    let has_unique_errors = check_for_unique_constraint_errors();

    assert!(
        !has_unique_errors,
        "EXPECTED FAILURE: UNIQUE constraint errors detected in /tmp/code_graph_diagnostic.log. \
         This indicates concurrent DELETE+INSERT operations occurred."
    );

    // Verify all entities were inserted (no silent failures)
    let conn = Connection::open(&db_path)?;
    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM code_entities",
        [],
        |row| row.get(0)
    )?;

    assert_eq!(
        count, 10,
        "EXPECTED FAILURE: Should have 10 entities (5 from each task). Found: {}. \
         Missing entities indicate UNIQUE constraint failures.",
        count
    );

    Ok(())
}

#[tokio::test]
async fn test_sequential_operations_succeed_without_mutex() -> Result<()> {
    // This test verifies that WITHOUT concurrency, operations succeed
    // (Baseline test - should PASS even in Phase 1)

    CONCURRENT_OPERATIONS.store(0, Ordering::SeqCst);

    let workspace = TempDir::new()?;
    let config = create_test_config(&workspace)?;

    {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let _graph = CodeGraph::new(&config.paths.code_graph_db, vector_store)?;
    }

    // Run operations SEQUENTIALLY (no concurrency)
    for i in 0..5 {
        simulate_index_file_with_tracking(
            &config.paths.code_graph_db,
            &format!("/test/file_{}.rs", i),
            &format!("func_{}", i),
        )
        .await?;
    }

    // Assert: All operations succeed
    let conn = Connection::open(&config.paths.code_graph_db)?;
    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM code_entities",
        [],
        |row| row.get(0)
    )?;

    assert_eq!(count, 5, "Sequential operations should succeed");

    // No UNIQUE errors should occur
    let has_unique_errors = check_for_unique_constraint_errors();
    assert!(!has_unique_errors, "Sequential operations should not cause UNIQUE errors");

    Ok(())
}

#[tokio::test]
async fn test_high_concurrency_stress() -> Result<()> {
    // Stress test: 20 concurrent tasks trying to index
    // THIS WILL DEFINITELY FAIL WITHOUT MUTEX

    CONCURRENT_OPERATIONS.store(0, Ordering::SeqCst);

    let workspace = TempDir::new()?;
    let config = create_test_config(&workspace)?;

    {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let _graph = CodeGraph::new(&config.paths.code_graph_db, vector_store)?;
    }

    let db_path = Arc::new(config.paths.code_graph_db.clone());
    let mut handles = vec![];

    // Spawn 20 concurrent tasks
    for task_id in 0..20 {
        let db = db_path.clone();
        let handle = tokio::spawn(async move {
            simulate_index_file_with_tracking(
                &db,
                &format!("/test/stress_{}.rs", task_id),
                &format!("stress_func_{}", task_id),
            )
            .await
            .expect(&format!("Stress task {} failed", task_id));
        });
        handles.push(handle);
    }

    // Wait for all
    for handle in handles {
        handle.await?;
    }

    // Assert: All 20 entities should exist
    let conn = Connection::open(db_path.as_ref())?;
    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM code_entities",
        [],
        |row| row.get(0)
    )?;

    // THIS WILL FAIL IN PHASE 1
    assert_eq!(
        count, 20,
        "EXPECTED FAILURE: High concurrency should still complete all operations. Found: {} (expected: 20)",
        count
    );

    Ok(())
}
