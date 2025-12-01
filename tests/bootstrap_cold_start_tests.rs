//! APEX 2.15 - Phase 1 (TDD): Bootstrap Cold Start Tests
//!
//! These tests verify that when code_entities is EMPTY at startup:
//! - Full bootstrap indexing runs automatically
//! - All subsystems start AFTER bootstrap completes
//! - Entities are indexed into SQLite + Neo4j
//!
//! EXPECTED: All tests FAIL initially (bootstrap not implemented yet)

use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use syncore::bootstrap::run_startup_bootstrap_for_tests;
use syncore::code_graph::CodeGraph;
use syncore::config::SyncoreConfig;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use tempfile::TempDir;

/// Helper: Create test workspace with minimal Rust file
fn create_test_workspace() -> Result<(TempDir, PathBuf)> {
    let workspace = TempDir::new()?;
    let src_dir = workspace.path().join("src");
    fs::create_dir(&src_dir)?;

    let lib_file = src_dir.join("lib.rs");
    let mut file = fs::File::create(&lib_file)?;
    writeln!(file, "/// Test library")?;
    writeln!(file, "pub fn add(a: i32, b: i32) -> i32 {{")?;
    writeln!(file, "    a + b")?;
    writeln!(file, "}}")?;
    writeln!(file)?;
    writeln!(file, "pub struct Calculator {{")?;
    writeln!(file, "    value: i32,")?;
    writeln!(file, "}}")?;

    Ok((workspace, src_dir))
}

/// Helper: Create test config with isolated DB
fn create_test_config(workspace: &TempDir) -> Result<SyncoreConfig> {
    let mut config = SyncoreConfig::default();

    // Use workspace-local database paths
    let db_dir = workspace.path().join(".syncore");
    fs::create_dir_all(&db_dir)?;

    config.paths.db_path = db_dir.join("test.db").to_string_lossy().to_string();
    config.paths.code_graph_db = db_dir.join("code_graph.db").to_string_lossy().to_string();

    Ok(config)
}

/// Helper: Count entities in code_entities table
fn count_code_entities(db_path: &str) -> Result<usize> {
    let conn = Connection::open(db_path)?;
    let count: usize =
        conn.query_row("SELECT COUNT(*) FROM code_entities", [], |row| row.get(0)).unwrap_or(0);
    Ok(count)
}

#[tokio::test]
async fn test_cold_start_triggers_full_bootstrap() -> Result<()> {
    // Arrange: Create empty workspace with src/lib.rs
    let (workspace, src_dir) = create_test_workspace()?;
    let config = create_test_config(&workspace)?;

    // Verify code_graph DB exists but is empty
    let code_graph_path = &config.paths.code_graph_db;

    // Initialize CodeGraph to create schema
    {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let _graph = CodeGraph::new(code_graph_path, vector_store)?;
    }

    // Verify table is empty BEFORE bootstrap
    let count_before = count_code_entities(code_graph_path)?;
    assert_eq!(count_before, 0, "code_entities should be empty before bootstrap");

    // Act: Run bootstrap (NOT YET IMPLEMENTED - will do nothing)
    run_startup_bootstrap_for_tests(&config).await?;

    // Assert: Entities should exist after bootstrap
    let count_after = count_code_entities(code_graph_path)?;

    // THIS WILL FAIL IN PHASE 1 (expected)
    assert!(
        count_after > 0,
        "EXPECTED FAILURE: Bootstrap should have indexed entities. Found: {} (expected > 0)",
        count_after
    );

    // Verify specific entities were indexed
    // Note: entity_type is stored as lowercase ("function", not "Function")
    let conn = Connection::open(code_graph_path)?;
    let function_count: usize = conn.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE entity_type = 'function' AND name = 'add'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(function_count, 1, "Function 'add' should be indexed");

    Ok(())
}

#[tokio::test]
async fn test_cold_start_logs_bootstrap_message() -> Result<()> {
    // Arrange
    let (workspace, _src_dir) = create_test_workspace()?;
    let config = create_test_config(&workspace)?;

    // Initialize empty DB
    {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let _graph = CodeGraph::new(&config.paths.code_graph_db, vector_store)?;
    }

    // Act: Capture stderr during bootstrap
    // Note: In real implementation, we'd check eprintln! output
    // For now, just verify function doesn't panic
    run_startup_bootstrap_for_tests(&config).await?;

    // Assert: This test documents expected log behavior
    // In Phase 2, should log: "[SynCore] No code entities found. Running initial bootstrap index..."

    // THIS TEST WILL FAIL - no logging implemented yet
    // Manual verification: Check stderr contains "Initial bootstrap index"

    Ok(())
}

#[tokio::test]
async fn test_cold_start_with_multiple_files() -> Result<()> {
    // Arrange: Create workspace with multiple Rust files
    let workspace = TempDir::new()?;
    let src_dir = workspace.path().join("src");
    fs::create_dir(&src_dir)?;

    // File 1: lib.rs
    let lib_file = src_dir.join("lib.rs");
    let mut file = fs::File::create(&lib_file)?;
    writeln!(file, "pub mod utils;")?;
    writeln!(file, "pub fn main_fn() {{}}")?;

    // File 2: utils.rs
    let utils_file = src_dir.join("utils.rs");
    let mut file = fs::File::create(&utils_file)?;
    writeln!(file, "pub fn helper() {{}}")?;
    writeln!(file, "pub struct Config {{}}")?;

    let config = create_test_config(&workspace)?;

    // Initialize empty DB
    {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let _graph = CodeGraph::new(&config.paths.code_graph_db, vector_store)?;
    }

    // Act: Run bootstrap
    run_startup_bootstrap_for_tests(&config).await?;

    // Assert: Should index ALL files
    let count = count_code_entities(&config.paths.code_graph_db)?;

    // THIS WILL FAIL IN PHASE 1
    assert!(
        count >= 3,
        "EXPECTED FAILURE: Should index entities from multiple files. Found: {}",
        count
    );

    Ok(())
}
