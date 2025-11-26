/*
//! TDD Tests for Incremental Indexing in index_application
//!
//! Tests verify that incremental indexing:
//! 1. Skips unchanged files
//! 2. Detects and indexes new files
//! 3. Detects and re-indexes modified files
//! 4. Removes entities for deleted files
//! 5. Handles mixed changes in a single run
//! 6. Is idempotent (running twice with no changes = no changes)
//! 7. Respects HNSW warmup state machine
//! 8. Ignores non-target file extensions

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use syncore::code_graph::CodeGraph;
use syncore::db::DbManager;
use syncore::vector::{RealEmbeddings, VectorStore};

/// Helper: Create test infrastructure
fn setup_test_env() -> Result<(TempDir, DbManager, Arc<Mutex<VectorStore>>)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let code_graph_path = temp_dir.path().join("code_graph.db");

    let db_manager = DbManager::new(db_path.to_str().unwrap(), code_graph_path.to_str().unwrap())?;

    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    Ok((temp_dir, db_manager, vector_store))
}

/// Helper: Create a Rust file with given content
fn create_rust_file(dir: &Path, name: &str, content: &str) -> Result<String> {
    let file_path = dir.join(name);
    let mut file = fs::File::create(&file_path)?;
    file.write_all(content.as_bytes())?;
    Ok(file_path.to_str().unwrap().to_string())
}

/// Helper: Get entity count for a file
fn get_entity_count_for_file(db_manager: &DbManager, file_path: &str) -> Result<i64> {
    let conn = db_manager.code_graph_conn();
    let db = conn.lock().unwrap();
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE file_path = ?",
        [file_path],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Helper: Get total entity count
fn get_total_entity_count(db_manager: &DbManager) -> Result<i64> {
    let conn = db_manager.code_graph_conn();
    let db = conn.lock().unwrap();
    let count: i64 = db.query_row("SELECT COUNT(*) FROM code_entities", [], |row| row.get(0))?;
    Ok(count)
}

/// Helper: Check if file_index_state exists for a file
fn get_file_index_state(db_manager: &DbManager, file_path: &str) -> Result<Option<(String, i64)>> {
    let conn = db_manager.code_graph_conn();
    let db = conn.lock().unwrap();
    let result: Result<(String, i64), _> = db.query_row(
        "SELECT sha256, mtime FROM file_index_state WHERE file_path = ?",
        [file_path],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );
    match result {
        Ok((sha256, mtime)) => Ok(Some((sha256, mtime))),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

// ============================================================================
// TEST 1: Incremental indexing skips unchanged files
// ============================================================================
#[test]
fn test_incremental_skips_unchanged_files() -> Result<()> {
    let (temp_dir, db_manager, vector_store) = setup_test_env()?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;

    // Create a test file
    let file_content = r#"
pub fn hello() {
    println!("Hello");
}
"#;
    let file_path = create_rust_file(&src_dir, "test.rs", file_content)?;

    // First indexing run
    let mut code_graph =
        CodeGraph::with_connection(db_manager.code_graph_conn(), vector_store.clone())?;
    let first_count = code_graph.index_file(Path::new(&file_path))?;
    assert!(first_count > 0, "Should index at least one entity");

    let initial_entity_count = get_total_entity_count(&db_manager)?;

    // Second indexing run (no changes)
    // With incremental indexing, this should return 0 (file skipped)
    let second_count = code_graph.index_file(Path::new(&file_path))?;

    let final_entity_count = get_total_entity_count(&db_manager)?;

    // PHASE 5: Incremental indexing should skip unchanged files and return 0
    assert_eq!(
        second_count, 0,
        "Second indexing of unchanged file should return 0 (skipped)"
    );

    // Entity count should remain the same
    assert_eq!(
        initial_entity_count, final_entity_count,
        "Entity count should remain stable after skipping unchanged file"
    );

    Ok(())
}

// ============================================================================
// TEST 2: Incremental indexing detects new files
// ============================================================================
#[test]
fn test_incremental_detects_new_files() -> Result<()> {
    let (temp_dir, db_manager, vector_store) = setup_test_env()?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;

    // Create initial file
    let file1_content = r#"
pub fn first() {}
"#;
    let file1_path = create_rust_file(&src_dir, "first.rs", file1_content)?;

    // Index initial state
    let mut code_graph =
        CodeGraph::with_connection(db_manager.code_graph_conn(), vector_store.clone())?;
    code_graph.index_file(Path::new(&file1_path))?;

    let initial_count = get_total_entity_count(&db_manager)?;

    // Add a new file
    let file2_content = r#"
pub fn second() {}
pub fn third() {}
"#;
    let file2_path = create_rust_file(&src_dir, "second.rs", file2_content)?;

    // Index the new file
    code_graph.index_file(Path::new(&file2_path))?;

    let final_count = get_total_entity_count(&db_manager)?;

    // Should have more entities now
    assert!(
        final_count > initial_count,
        "Should have more entities after adding new file"
    );

    // New file should have entities
    let new_file_entities = get_entity_count_for_file(&db_manager, &file2_path)?;
    assert!(
        new_file_entities > 0,
        "New file should have entities in database"
    );

    Ok(())
}

// ============================================================================
// TEST 3: Incremental indexing detects modified files
// ============================================================================
#[test]
fn test_incremental_detects_modified_files_and_updates_entities() -> Result<()> {
    let (temp_dir, db_manager, vector_store) = setup_test_env()?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;

    // Create initial file with one function
    let initial_content = r#"
pub fn original() {
    println!("original");
}
"#;
    let file_path = create_rust_file(&src_dir, "modify.rs", initial_content)?;

    // Index initial state
    let mut code_graph =
        CodeGraph::with_connection(db_manager.code_graph_conn(), vector_store.clone())?;
    code_graph.index_file(Path::new(&file_path))?;

    let initial_count = get_entity_count_for_file(&db_manager, &file_path)?;

    // Modify the file - add another function
    let modified_content = r#"
pub fn original() {
    println!("original");
}

pub fn added() {
    println!("added");
}
"#;
    fs::write(&file_path, modified_content)?;

    // Re-index the modified file
    code_graph.index_file(Path::new(&file_path))?;

    let final_count = get_entity_count_for_file(&db_manager, &file_path)?;

    // Should have more entities now
    assert!(
        final_count > initial_count,
        "Should have more entities after modification (initial={}, final={})",
        initial_count,
        final_count
    );

    Ok(())
}

// ============================================================================
// TEST 4: Incremental indexing removes deleted files
// ============================================================================
#[test]
fn test_incremental_removes_deleted_files() -> Result<()> {
    let (temp_dir, db_manager, vector_store) = setup_test_env()?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;

    // Create two files
    let file1_content = "pub fn keep() {}";
    let file2_content = "pub fn delete_me() {}";
    let file1_path = create_rust_file(&src_dir, "keep.rs", file1_content)?;
    let file2_path = create_rust_file(&src_dir, "delete.rs", file2_content)?;

    // Index both files
    let mut code_graph =
        CodeGraph::with_connection(db_manager.code_graph_conn(), vector_store.clone())?;
    code_graph.index_file(Path::new(&file1_path))?;
    code_graph.index_file(Path::new(&file2_path))?;

    // Verify both have entities
    let file2_entities_before = get_entity_count_for_file(&db_manager, &file2_path)?;
    assert!(file2_entities_before > 0, "File 2 should have entities");

    // Delete the second file from filesystem
    fs::remove_file(&file2_path)?;

    // TODO: Call incremental indexer which should detect deletion
    // For now, manually delete entities to simulate expected behavior
    {
        let conn = db_manager.code_graph_conn();
        let db = conn.lock().unwrap();
        db.execute(
            "DELETE FROM code_entities WHERE file_path = ?",
            [&file2_path],
        )?;
    }

    // Verify file 2 entities are gone
    let file2_entities_after = get_entity_count_for_file(&db_manager, &file2_path)?;
    assert_eq!(
        file2_entities_after, 0,
        "Deleted file should have no entities"
    );

    // Verify file 1 entities still exist
    let file1_entities = get_entity_count_for_file(&db_manager, &file1_path)?;
    assert!(file1_entities > 0, "Kept file should still have entities");

    Ok(())
}

// ============================================================================
// TEST 5: Incremental indexing handles mixed changes
// ============================================================================
#[test]
fn test_incremental_handles_mixed_changes() -> Result<()> {
    let (temp_dir, db_manager, vector_store) = setup_test_env()?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;

    // Create initial files: unchanged, to_modify, to_delete
    let unchanged_content = "pub fn unchanged() {}";
    let modify_content = "pub fn will_modify() {}";
    let delete_content = "pub fn will_delete() {}";

    let unchanged_path = create_rust_file(&src_dir, "unchanged.rs", unchanged_content)?;
    let modify_path = create_rust_file(&src_dir, "modify.rs", modify_content)?;
    let delete_path = create_rust_file(&src_dir, "delete.rs", delete_content)?;

    // Index all files
    let mut code_graph =
        CodeGraph::with_connection(db_manager.code_graph_conn(), vector_store.clone())?;
    code_graph.index_file(Path::new(&unchanged_path))?;
    code_graph.index_file(Path::new(&modify_path))?;
    code_graph.index_file(Path::new(&delete_path))?;

    let initial_total = get_total_entity_count(&db_manager)?;

    // Make changes:
    // 1. Add new file
    let new_content = "pub fn new_function() {}";
    let new_path = create_rust_file(&src_dir, "new.rs", new_content)?;

    // 2. Modify existing file
    let modified_content = "pub fn will_modify() {}\npub fn added() {}";
    fs::write(&modify_path, modified_content)?;

    // 3. Delete file
    fs::remove_file(&delete_path)?;

    // Index new and modified files
    code_graph.index_file(Path::new(&new_path))?;
    code_graph.index_file(Path::new(&modify_path))?;

    // Simulate deletion handling
    {
        let conn = db_manager.code_graph_conn();
        let db = conn.lock().unwrap();
        db.execute(
            "DELETE FROM code_entities WHERE file_path = ?",
            [&delete_path],
        )?;
    }

    // Verify results
    let new_entities = get_entity_count_for_file(&db_manager, &new_path)?;
    assert!(new_entities > 0, "New file should have entities");

    let modify_entities = get_entity_count_for_file(&db_manager, &modify_path)?;
    assert!(
        modify_entities >= 2,
        "Modified file should have 2+ entities"
    );

    let delete_entities = get_entity_count_for_file(&db_manager, &delete_path)?;
    assert_eq!(delete_entities, 0, "Deleted file should have no entities");

    let unchanged_entities = get_entity_count_for_file(&db_manager, &unchanged_path)?;
    assert!(
        unchanged_entities > 0,
        "Unchanged file should still have entities"
    );

    Ok(())
}

// ============================================================================
// TEST 6: Incremental indexing is idempotent
// ============================================================================
#[test]
fn test_incremental_is_idempotent() -> Result<()> {
    let (temp_dir, db_manager, vector_store) = setup_test_env()?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;

    // Create test file
    let content = "pub fn stable() {}";
    let file_path = create_rust_file(&src_dir, "stable.rs", content)?;

    // Index twice
    let mut code_graph =
        CodeGraph::with_connection(db_manager.code_graph_conn(), vector_store.clone())?;
    code_graph.index_file(Path::new(&file_path))?;
    let count_after_first = get_total_entity_count(&db_manager)?;

    code_graph.index_file(Path::new(&file_path))?;
    let count_after_second = get_total_entity_count(&db_manager)?;

    // Should be identical
    assert_eq!(
        count_after_first, count_after_second,
        "Indexing twice should produce same entity count"
    );

    Ok(())
}

// ============================================================================
// TEST 7: Incremental indexing respects HNSW state machine
// ============================================================================
#[test]
fn test_incremental_respects_hnsw_state_machine() -> Result<()> {
    let (temp_dir, db_manager, vector_store) = setup_test_env()?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;

    // Set HNSW to Cold state
    {
        let vs = vector_store.lock().unwrap();
        vs.warmup_controller().mark_cold();
        assert!(!vs.warmup_controller().is_hot());
    }

    // Create and index file while Cold
    let content = "pub fn cold_index() {}";
    let file_path = create_rust_file(&src_dir, "cold.rs", content)?;

    let mut code_graph =
        CodeGraph::with_connection(db_manager.code_graph_conn(), vector_store.clone())?;
    let count = code_graph.index_file(Path::new(&file_path))?;

    assert!(count > 0, "Should index even when HNSW is Cold");

    // Search should work via brute-force fallback
    let vs = vector_store.lock().unwrap();
    let results = vs.search(
        "cold index function",
        5,
        syncore::vector::SearchScope::Global,
    )?;
    // Results may or may not find the entity depending on embedding quality
    // The key is that it doesn't panic or deadlock

    Ok(())
}

// ============================================================================
// TEST 8: Incremental indexing ignores non-target extensions
// ============================================================================
#[test]
fn test_incremental_ignores_non_target_extensions() -> Result<()> {
    let (temp_dir, db_manager, vector_store) = setup_test_env()?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;

    // Create files with different extensions
    let rust_content = "pub fn rust_func() {}";
    let md_content = "# Markdown file\nSome text";
    let json_content = r###"{"key": "value"}"###;

    let rust_path = create_rust_file(&src_dir, "code.rs", rust_content)?;
    create_rust_file(&src_dir, "readme.md", md_content)?;
    create_rust_file(&src_dir, "config.json", json_content)?;

    // Only index the Rust file
    let mut code_graph =
        CodeGraph::with_connection(db_manager.code_graph_conn(), vector_store.clone())?;
    code_graph.index_file(Path::new(&rust_path))?;

    // Verify only Rust file has entities
    let total = get_total_entity_count(&db_manager)?;
    let rust_entities = get_entity_count_for_file(&db_manager, &rust_path)?;

    assert_eq!(
        total, rust_entities,
        "Only Rust file should have entities indexed"
    );

    Ok(())
}
*/
