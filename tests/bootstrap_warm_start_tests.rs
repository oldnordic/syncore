//! APEX 2.15 - Phase 1 (TDD): Bootstrap Warm Start Tests
//!
//! These tests verify that when code_entities is NOT EMPTY at startup:
//! - Bootstrap is SKIPPED
//! - Existing entities are PRESERVED
//! - Only incremental Live Indexer starts
//!
//! EXPECTED: All tests FAIL initially (warm start logic not implemented yet)

use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::io::Write;
use std::sync::{Arc, Mutex};
use syncore::bootstrap::run_startup_bootstrap_for_tests;
use syncore::code_graph::CodeGraph;
use syncore::config::SyncoreConfig;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use tempfile::TempDir;

/// Helper: Create test config with isolated DB
fn create_test_config(workspace: &TempDir) -> Result<SyncoreConfig> {
    let mut config = SyncoreConfig::default();

    let db_dir = workspace.path().join(".syncore");
    fs::create_dir_all(&db_dir)?;

    config.paths.db_path = db_dir.join("test.db").to_string_lossy().to_string();
    config.paths.code_graph_db = db_dir.join("code_graph.db").to_string_lossy().to_string();

    Ok(config)
}

/// Helper: Insert a fake entity directly into code_entities
fn insert_fake_entity(db_path: &str) -> Result<()> {
    let conn = Connection::open(db_path)?;

    conn.execute(
        "INSERT INTO code_entities
         (file_path, entity_type, name, line_start, line_end, language, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            "/fake/test.rs",
            "Function",
            "fake_function",
            1,
            5,
            "Rust",
            chrono::Utc::now().timestamp(),
        ],
    )?;

    Ok(())
}

/// Helper: Count entities in code_entities table
fn count_code_entities(db_path: &str) -> Result<usize> {
    let conn = Connection::open(db_path)?;
    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM code_entities",
        [],
        |row| row.get(0)
    )?;
    Ok(count)
}

#[tokio::test]
async fn test_warm_start_skips_bootstrap() -> Result<()> {
    // Arrange: Create workspace and DB with PRE-EXISTING entity
    let workspace = TempDir::new()?;
    let config = create_test_config(&workspace)?;

    // Initialize DB and insert 1 fake entity
    {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let _graph = CodeGraph::new(&config.paths.code_graph_db, vector_store)?;
    }

    insert_fake_entity(&config.paths.code_graph_db)?;

    let count_before = count_code_entities(&config.paths.code_graph_db)?;
    assert_eq!(count_before, 1, "Should have 1 pre-existing entity");

    // Act: Run bootstrap (should detect existing entities and SKIP)
    run_startup_bootstrap_for_tests(&config).await?;

    // Assert: Entity count should remain EXACTLY 1 (no reindex, no delete)
    let count_after = count_code_entities(&config.paths.code_graph_db)?;

    // THIS WILL FAIL IN PHASE 1 (expected)
    assert_eq!(
        count_after, 1,
        "EXPECTED FAILURE: Warm start should preserve existing entities. Found: {} (expected: 1)",
        count_after
    );

    // Verify the original entity still exists
    let conn = Connection::open(&config.paths.code_graph_db)?;
    let name: String = conn.query_row(
        "SELECT name FROM code_entities WHERE entity_type = 'Function'",
        [],
        |row| row.get(0)
    )?;

    assert_eq!(name, "fake_function", "Original entity should be preserved");

    Ok(())
}

#[tokio::test]
async fn test_warm_start_logs_skip_message() -> Result<()> {
    // Arrange
    let workspace = TempDir::new()?;
    let config = create_test_config(&workspace)?;

    // Initialize DB with entity
    {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let _graph = CodeGraph::new(&config.paths.code_graph_db, vector_store)?;
    }

    insert_fake_entity(&config.paths.code_graph_db)?;

    // Act: Run bootstrap (should skip and log)
    run_startup_bootstrap_for_tests(&config).await?;

    // Assert: This test documents expected log behavior
    // In Phase 2, should log: "[SynCore] Existing code entities found (count = N), skipping bootstrap."

    // THIS TEST DOCUMENTS EXPECTED BEHAVIOR
    // Manual verification: Check stderr contains "skipping bootstrap"

    Ok(())
}

#[tokio::test]
async fn test_warm_start_with_many_entities() -> Result<()> {
    // Arrange: Create DB with multiple pre-existing entities
    let workspace = TempDir::new()?;
    let config = create_test_config(&workspace)?;

    {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let _graph = CodeGraph::new(&config.paths.code_graph_db, vector_store)?;
    }

    // Insert 10 fake entities
    let conn = Connection::open(&config.paths.code_graph_db)?;
    for i in 0..10 {
        conn.execute(
            "INSERT INTO code_entities
             (file_path, entity_type, name, line_start, line_end, language, indexed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                format!("/fake/test{}.rs", i),
                "Function",
                format!("func_{}", i),
                1,
                5,
                "Rust",
                chrono::Utc::now().timestamp(),
            ],
        )?;
    }

    let count_before = count_code_entities(&config.paths.code_graph_db)?;
    assert_eq!(count_before, 10, "Should have 10 pre-existing entities");

    // Act: Run bootstrap
    run_startup_bootstrap_for_tests(&config).await?;

    // Assert: All 10 entities should be preserved
    let count_after = count_code_entities(&config.paths.code_graph_db)?;

    // THIS WILL FAIL IN PHASE 1
    assert_eq!(
        count_after, 10,
        "EXPECTED FAILURE: Warm start should preserve all entities. Found: {}",
        count_after
    );

    Ok(())
}

#[tokio::test]
async fn test_warm_start_does_not_delete_existing_data() -> Result<()> {
    // Arrange: Setup with existing entity
    let workspace = TempDir::new()?;
    let config = create_test_config(&workspace)?;

    {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let _graph = CodeGraph::new(&config.paths.code_graph_db, vector_store)?;
    }

    insert_fake_entity(&config.paths.code_graph_db)?;

    // Get the entity ID before bootstrap
    let conn = Connection::open(&config.paths.code_graph_db)?;
    let entity_id_before: i64 = conn.query_row(
        "SELECT id FROM code_entities WHERE name = 'fake_function'",
        [],
        |row| row.get(0)
    )?;

    // Act: Run bootstrap
    run_startup_bootstrap_for_tests(&config).await?;

    // Assert: Same entity ID should still exist (no DELETE happened)
    let entity_id_after: Result<i64, _> = conn.query_row(
        "SELECT id FROM code_entities WHERE name = 'fake_function'",
        [],
        |row| row.get(0)
    );

    // THIS WILL FAIL IN PHASE 1
    assert!(
        entity_id_after.is_ok(),
        "EXPECTED FAILURE: Original entity should not be deleted"
    );

    assert_eq!(
        entity_id_after.unwrap(),
        entity_id_before,
        "Entity ID should remain unchanged (no DELETE+INSERT)"
    );

    Ok(())
}
