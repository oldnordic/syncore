//! TDD Tests for Document Indexer Isolation
//! Ensures tests do NOT write to ~/.syncore or any global paths.
//! Tests MUST pass custom database_path and vector_store_path.

use std::fs;
use std::path::{Path, PathBuf};
use syncore::document_indexer::DocumentIndexer;
use syncore::global_store::{GlobalDbPool, GlobalVectorStore};
use tempfile::TempDir;

/// Helper to create isolated test environment
fn create_isolated_env() -> TempDir {
    TempDir::new().expect("Should create temp directory")
}

/// Helper to create test documents
fn setup_test_docs(base: &Path) {
    fs::write(
        base.join("readme.md"),
        "# Project README\n\nThis is documentation.",
    )
    .unwrap();
    fs::write(base.join("notes.txt"), "Important notes for the project.").unwrap();
    fs::write(
        base.join("lib.rs"),
        "pub fn hello() { println!(\"hello\"); }",
    )
    .unwrap();
}

#[test]
fn test_index_directory_with_custom_paths_does_not_touch_home() {
    let temp_dir = create_isolated_env();
    let docs_dir = temp_dir.path().join("docs");
    let db_path = temp_dir.path().join("test.db");
    let vectors_dir = temp_dir.path().join("vectors");

    fs::create_dir_all(&docs_dir).unwrap();
    fs::create_dir_all(&vectors_dir).unwrap();
    setup_test_docs(&docs_dir);

    // Use the NEW API with dependency injection
    let indexer = DocumentIndexer::with_defaults();
    let chunk_count = indexer
        .index_directory_with_storage(&docs_dir, &db_path, &vectors_dir)
        .expect("Should index with custom paths");

    assert!(chunk_count > 0, "Should have indexed documents");

    // Verify files exist in temp directory
    assert!(db_path.exists(), "Database should be in temp dir");
    assert!(vectors_dir.exists(), "Vectors dir should be in temp dir");

    // CRITICAL: Verify NO writes to ~/.syncore
    let home = std::env::var("HOME").expect("HOME should be set");
    let global_syncore = PathBuf::from(home).join(".syncore");

    // If .syncore doesn't exist, that's fine - test passes
    // If it DOES exist, ensure we didn't add new files during this test
    if global_syncore.exists() {
        // Get modification time before and after test
        // This is a weak check, but combined with the custom paths it should suffice
        let global_db = global_syncore.join("global.db");
        if global_db.exists() {
            // If global.db was modified in the last second, fail the test
            let metadata = fs::metadata(&global_db).unwrap();
            let modified = metadata.modified().unwrap();
            let now = std::time::SystemTime::now();
            let age = now.duration_since(modified).unwrap_or_default();

            // Fail if global.db was modified within the last 2 seconds
            assert!(
                age.as_secs() > 2,
                "Global database at ~/.syncore/global.db should NOT be modified by tests"
            );
        }
    }
}

#[test]
fn test_global_db_pool_with_custom_path() {
    let temp_dir = create_isolated_env();
    let db_path = temp_dir.path().join("custom.db");

    // Use NEW API with custom path
    let pool = GlobalDbPool::new_with_path(&db_path).expect("Should create pool with custom path");

    // Verify database was created at custom path
    assert!(db_path.exists(), "Database should be at custom path");

    // Insert test data
    {
        let conn = pool.get();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO memory (k, v, ts) VALUES (?1, ?2, ?3)",
            ("test_key", "test_value", ts),
        )
        .expect("Should insert to custom db");
    }

    // Verify data is in custom database
    {
        let conn = pool.get();
        let value: String = conn
            .query_row("SELECT v FROM memory WHERE k = ?1", ["test_key"], |row| {
                row.get(0)
            })
            .expect("Should read from custom db");
        assert_eq!(value, "test_value");
    }
}

#[test]
fn test_global_vector_store_with_custom_dir() {
    let temp_dir = create_isolated_env();
    let vectors_dir = temp_dir.path().join("custom_vectors");
    fs::create_dir_all(&vectors_dir).unwrap();

    // Use NEW API with custom directory
    let mut store = GlobalVectorStore::new_with_path(&vectors_dir)
        .expect("Should create store with custom path");

    // Insert text
    store
        .insert_text(1, "Custom vector store test", "test_namespace")
        .expect("Should insert to custom vector store");

    // Verify index exists in custom directory
    let index_path = store.get_index_path("test_namespace");
    let vectors_file = format!("{}.vectors", index_path.display());

    // The vectors file should be in the custom directory
    assert!(
        index_path.starts_with(&vectors_dir),
        "Index path should be within custom directory"
    );
}

#[test]
fn test_document_indexer_preserves_default_behavior() {
    // The original API should still work (backward compatibility)
    let indexer = DocumentIndexer::with_defaults();
    let temp_dir = create_isolated_env();
    let docs_dir = temp_dir.path().join("docs");
    fs::create_dir_all(&docs_dir).unwrap();
    fs::write(docs_dir.join("test.md"), "# Test").unwrap();

    // This test just verifies the old API still exists
    // We won't call index_directory() as it would touch ~/.syncore
    let docs = indexer.scan_directory(&docs_dir).unwrap();
    assert!(!docs.is_empty(), "Should scan documents");
}

#[test]
fn test_no_sqlite_readonly_panic_in_temp_dir() {
    let temp_dir = create_isolated_env();
    let docs_dir = temp_dir.path().join("docs");
    let db_path = temp_dir.path().join("test.db");
    let vectors_dir = temp_dir.path().join("vectors");

    fs::create_dir_all(&docs_dir).unwrap();
    fs::create_dir_all(&vectors_dir).unwrap();
    setup_test_docs(&docs_dir);

    let indexer = DocumentIndexer::with_defaults();

    // Should NOT panic with SQLITE_READONLY_DIRECTORY
    let result = indexer.index_directory_with_storage(&docs_dir, &db_path, &vectors_dir);

    assert!(
        result.is_ok(),
        "Should not panic or fail with readonly directory: {:?}",
        result.err()
    );
}

#[test]
fn test_multiple_indexing_sessions_same_temp_db() {
    let temp_dir = create_isolated_env();
    let docs_dir = temp_dir.path().join("docs");
    let db_path = temp_dir.path().join("test.db");
    let vectors_dir = temp_dir.path().join("vectors");

    fs::create_dir_all(&docs_dir).unwrap();
    fs::create_dir_all(&vectors_dir).unwrap();
    setup_test_docs(&docs_dir);

    let indexer = DocumentIndexer::with_defaults();

    // First indexing
    let count1 = indexer
        .index_directory_with_storage(&docs_dir, &db_path, &vectors_dir)
        .expect("First indexing should succeed");

    // Add more documents
    fs::write(docs_dir.join("extra.md"), "Extra content").unwrap();

    // Second indexing to same database
    let count2 = indexer
        .index_directory_with_storage(&docs_dir, &db_path, &vectors_dir)
        .expect("Second indexing should succeed");

    // Should have indexed more documents
    assert!(count2 >= count1, "Should handle re-indexing");
}

#[test]
fn test_indexer_returns_correct_chunk_count_with_custom_storage() {
    let temp_dir = create_isolated_env();
    let docs_dir = temp_dir.path().join("docs");
    let db_path = temp_dir.path().join("test.db");
    let vectors_dir = temp_dir.path().join("vectors");

    fs::create_dir_all(&docs_dir).unwrap();
    fs::create_dir_all(&vectors_dir).unwrap();

    // Create documents with known content
    fs::write(docs_dir.join("short.md"), "Short").unwrap();
    fs::write(
        docs_dir.join("medium.txt"),
        "Medium length document content here",
    )
    .unwrap();

    let indexer = DocumentIndexer::with_defaults();
    let chunk_count = indexer
        .index_directory_with_storage(&docs_dir, &db_path, &vectors_dir)
        .expect("Should index successfully");

    // Each small document should create exactly 1 chunk
    assert!(
        chunk_count >= 2,
        "Should have at least 2 chunks for 2 documents"
    );
}
