//! TDD Tests for Bug #3: Vector Store Snapshot Loading on Startup
//!
//! ISSUE: mcp_stdio_main.rs creates VectorStore but never calls load_snapshot()
//!
//! Root Cause:
//! - VectorStore::new() creates EMPTY in-memory store
//! - set_index_path() is called but load_snapshot() is MISSING
//! - Database has 10,558 embeddings but in-memory vectors array is empty
//! - Result: search_code() fails, poisoned locks, all semantic search broken
//!
//! Evidence from Production:
//! - Database: 10,558 embeddings (SELECT COUNT(*) FROM code_embeddings)
//! - Snapshot files exist: ~/.config/syncore/syncore_general.index.vectors
//! - VectorStore created at mcp_stdio_main.rs:81,90 but no load_snapshot()
//!
//! EXPECTED: These tests FAIL initially, then PASS after fix

use anyhow::Result;
use std::sync::{Arc, Mutex};
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use tempfile::TempDir;

/// Test that VectorStore without load_snapshot() is empty
#[test]
fn test_vector_store_without_load_snapshot_is_empty() -> Result<()> {
    // Arrange: Create vector store with snapshot files present
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("test_index");

    // Create and populate a vector store
    let embeddings1 = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store1 = VectorStore::new(embeddings1);
    store1.set_index_path(index_path.to_string_lossy().to_string());

    // Insert test vectors
    store1.insert_text(1, None, "test function one", "code_entity")?;
    store1.insert_text(2, None, "test function two", "code_entity")?;
    store1.insert_text(3, None, "test function three", "code_entity")?;

    // Verify vectors exist
    assert_eq!(store1.len(), 3, "First store should have 3 vectors");

    // Save snapshot to disk
    store1.save_snapshot()?;
    drop(store1);

    // Act: Simulate mcp_stdio_main.rs behavior (WITHOUT load_snapshot)
    let embeddings2 = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store2 = VectorStore::new(embeddings2);
    store2.set_index_path(index_path.to_string_lossy().to_string());
    // BUG: load_snapshot() NOT called here (mimics production bug)

    // Assert: Store is EMPTY (snapshot exists but not loaded)
    assert_eq!(
        store2.len(),
        0,
        "Store without load_snapshot() should be EMPTY despite snapshot existing"
    );

    Ok(())
}

/// Test that VectorStore WITH load_snapshot() restores state
#[test]
fn test_vector_store_with_load_snapshot_restores_state() -> Result<()> {
    // Arrange: Create and save snapshot
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("test_index");

    let embeddings1 = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store1 = VectorStore::new(embeddings1);
    store1.set_index_path(index_path.to_string_lossy().to_string());

    store1.insert_text(1, None, "function alpha", "code_entity")?;
    store1.insert_text(2, None, "function beta", "code_entity")?;
    store1.insert_text(3, None, "function gamma", "code_entity")?;

    store1.save_snapshot()?;
    drop(store1);

    // Act: Correct behavior (WITH load_snapshot)
    let embeddings2 = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store2 = VectorStore::new(embeddings2);
    store2.set_index_path(index_path.to_string_lossy().to_string());
    store2.load_snapshot()?; // FIX: This line is missing in production

    // Assert: Store restored with 3 vectors
    assert_eq!(
        store2.len(),
        3,
        "Store WITH load_snapshot() should restore all vectors from snapshot"
    );

    Ok(())
}

/// Test that Arc<Mutex<VectorStore>> pattern works with load_snapshot
#[test]
fn test_arc_mutex_vector_store_load_snapshot() -> Result<()> {
    // Arrange: Simulates exact pattern from mcp_stdio_main.rs
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("code_index");

    // Create and populate first store
    let embeddings1 = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store1 = VectorStore::new(embeddings1);
    store1.set_index_path(index_path.to_string_lossy().to_string());

    store1.insert_text(100, None, "pub fn main()", "code_entity")?;
    store1.insert_text(101, None, "pub fn setup()", "code_entity")?;

    store1.save_snapshot()?;
    drop(store1);

    // Act: Exact pattern from mcp_stdio_main.rs:80-85
    let code_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut code_store = VectorStore::new(code_embeddings);
    code_store.set_index_path(index_path.to_string_lossy().to_string());

    // THIS IS THE FIX WE'RE TESTING FOR:
    code_store.load_snapshot()?;

    let code_store = Arc::new(Mutex::new(code_store));

    // Assert: Can search after loading
    let store_locked = code_store.lock().unwrap();
    assert_eq!(
        store_locked.len(),
        2,
        "Arc<Mutex<VectorStore>> should have loaded vectors"
    );

    Ok(())
}

/// Test search_code() fails with empty vector store
#[test]
fn test_search_code_fails_without_loaded_vectors() -> Result<()> {
    use rusqlite::Connection;
    use syncore::code_graph::CodeGraph;

    // Arrange: Create database with entities
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let index_path = temp_dir.path().join("vectors");

    // Create database with code entities
    let conn = Connection::open(&db_path)?;
    syncore::db::ensure_schema(db_path.to_str().unwrap())?;

    conn.execute(
        "INSERT INTO code_entities (id, file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (1, 'test.rs', 'function', 'search_test', 'fn search_test()', 1, 10, 'rust', 0)",
        [],
    )?;

    conn.execute(
        "INSERT INTO code_embeddings (entity_id, vector_id, model_version, created_at)
         VALUES (1, 1, 'test', 0)",
        [],
    )?;

    drop(conn);

    // Create vector store WITHOUT loading snapshot
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut vector_store = VectorStore::new(embeddings);
    vector_store.set_index_path(index_path.to_string_lossy().to_string());
    // BUG: No load_snapshot() called

    let vector_store = Arc::new(Mutex::new(vector_store));

    // Act: Try to search (should return 0 results with empty vectors)
    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;
    let results = code_graph.search_code("search_test", 10)?;

    // Assert: Search returns empty (database has entity but vector store is empty)
    assert_eq!(
        results.len(),
        0,
        "search_code() with unloaded vector store should return 0 results"
    );

    Ok(())
}

/// Test search_code() succeeds WITH loaded vectors
#[test]
fn test_search_code_succeeds_with_loaded_vectors() -> Result<()> {
    use rusqlite::Connection;
    use syncore::code_graph::CodeGraph;

    // Arrange: Create database and vector snapshot
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let index_path = temp_dir.path().join("vectors");

    let conn = Connection::open(&db_path)?;
    syncore::db::ensure_schema(db_path.to_str().unwrap())?;

    conn.execute(
        "INSERT INTO code_entities (id, file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (1, 'test.rs', 'function', 'working_search', 'fn working_search()', 1, 10, 'rust', 0)",
        [],
    )?;

    conn.execute(
        "INSERT INTO code_embeddings (entity_id, vector_id, model_version, created_at)
         VALUES (1, 1, 'bge-small-en-v1.5', 0)",
        [],
    )?;

    drop(conn);

    // Create and populate vector store
    let embeddings1 = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store1 = VectorStore::new(embeddings1);
    store1.set_index_path(index_path.to_string_lossy().to_string());
    store1.insert_text(1, None, "fn working_search()", "code_entity")?;
    store1.save_snapshot()?;
    drop(store1);

    // Act: Load vector store WITH snapshot loading (THE FIX)
    let embeddings2 = Box::new(HuggingFaceEmbeddings::new()?);
    let mut vector_store = VectorStore::new(embeddings2);
    vector_store.set_index_path(index_path.to_string_lossy().to_string());
    vector_store.load_snapshot()?; // FIX: This line is missing in production

    let vector_store = Arc::new(Mutex::new(vector_store));

    // Search should work now
    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;
    let results = code_graph.search_code("search", 10)?;

    // Assert: Search returns results
    assert!(
        results.len() > 0,
        "search_code() WITH loaded vectors should return results"
    );

    Ok(())
}
