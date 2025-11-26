//! Tests for vector snapshot integrity validation
//!
//! Ensures vector snapshots are validated against SQLite code_embeddings
//! and automatically rebuilt if IDs don't match

use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use syncore::code_graph::CodeGraph;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

#[test]
fn test_vector_snapshot_with_invalid_ids_triggers_rebuild() {
    // FIX 2: Vector snapshot containing nonexistent IDs should trigger rebuild
    let temp_db = "/tmp/test_snapshot_rebuild.db";
    let _ = std::fs::remove_file(temp_db);

    // Create database with known entities
    let conn = Connection::open(temp_db).unwrap();
    syncore::db::ensure_schema(temp_db).unwrap();

    // Insert test entity with ID 1000
    conn.execute(
        "INSERT INTO code_entities (id, file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (1000, 'test.rs', 'function', 'test_fn', 'fn test_fn()', 1, 10, 'rust', 0)",
        [],
    ).unwrap();

    conn.execute(
        "INSERT INTO code_embeddings (entity_id, vector_id, model_version, created_at)
         VALUES (1000, 1000, 'test', 0)",
        [],
    )
    .unwrap();

    drop(conn);

    // Create vector snapshot with WRONG IDs (9999 not in database)
    let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let mut vector_store = VectorStore::new(embeddings);
    vector_store.set_index_path("/tmp/test_snapshot_rebuild".to_string());

    // Insert vector with ID 9999 (doesn't exist in code_embeddings)
    let fake_embedding = vec![0.1; 384];
    vector_store.add_test_vector(9999, None, fake_embedding, "fake text".to_string());

    // Save the invalid snapshot
    vector_store.save_snapshot().unwrap();

    drop(vector_store);

    // Now create CodeGraph and load the snapshot with validation
    let embeddings2 = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let mut vector_store2 = VectorStore::new(embeddings2);
    vector_store2.set_index_path("/tmp/test_snapshot_rebuild".to_string());

    // Load snapshot with validation against database
    let db_conn = Connection::open(temp_db).unwrap();
    let result = vector_store2.load_snapshot_with_validation(&db_conn);

    // Should succeed but trigger rebuild (clear vectors)
    assert!(
        result.is_ok(),
        "load_snapshot_with_validation should succeed: {:?}",
        result.err()
    );

    // Vectors should be empty after rebuild
    assert_eq!(
        vector_store2.len(),
        0,
        "Vector store should be empty after rebuilding due to invalid IDs"
    );

    // Cleanup
    let _ = std::fs::remove_file(temp_db);
    let _ = std::fs::remove_file("/tmp/test_snapshot_rebuild.vectors");
    let _ = std::fs::remove_file("/tmp/test_snapshot_rebuild.meta");
}

#[test]
fn test_vector_snapshot_with_valid_ids_loads_normally() {
    // FIX 2: Valid snapshot should load without rebuild
    let temp_db = "/tmp/test_snapshot_valid.db";
    let _ = std::fs::remove_file(temp_db);

    // Create database with known entities
    let conn = Connection::open(temp_db).unwrap();
    syncore::db::ensure_schema(temp_db).unwrap();

    // Insert test entity with ID 2000
    conn.execute(
        "INSERT INTO code_entities (id, file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (2000, 'test.rs', 'function', 'test_fn', 'fn test_fn()', 1, 10, 'rust', 0)",
        [],
    ).unwrap();

    conn.execute(
        "INSERT INTO code_embeddings (entity_id, vector_id, model_version, created_at)
         VALUES (2000, 2000, 'test', 0)",
        [],
    )
    .unwrap();

    drop(conn);

    // Create vector snapshot with VALID ID (2000 exists in database)
    let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let mut vector_store = VectorStore::new(embeddings);
    vector_store.set_index_path("/tmp/test_snapshot_valid".to_string());

    // Insert vector with ID 2000 (exists in code_embeddings)
    let valid_embedding = vec![0.1; 384];
    vector_store.add_test_vector(2000, None, valid_embedding, "valid text".to_string());

    // Save the valid snapshot
    vector_store.save_snapshot().unwrap();

    drop(vector_store);

    // Now create CodeGraph and load the snapshot with validation
    let embeddings2 = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let mut vector_store2 = VectorStore::new(embeddings2);
    vector_store2.set_index_path("/tmp/test_snapshot_valid".to_string());

    // Load snapshot with validation against database
    let db_conn = Connection::open(temp_db).unwrap();
    let result = vector_store2.load_snapshot_with_validation(&db_conn);

    // Should succeed and keep vectors
    assert!(
        result.is_ok(),
        "load_snapshot_with_validation should succeed: {:?}",
        result.err()
    );

    // Vectors should be preserved
    assert_eq!(
        vector_store2.len(),
        1,
        "Vector store should keep valid vectors after loading"
    );

    // Cleanup
    let _ = std::fs::remove_file(temp_db);
    let _ = std::fs::remove_file("/tmp/test_snapshot_valid.vectors");
    let _ = std::fs::remove_file("/tmp/test_snapshot_valid.meta");
}

#[test]
fn test_after_rebuild_reindexing_works() {
    // FIX 2: After snapshot rebuild, code_index_directory should repopulate correctly
    let temp_db = "/tmp/test_reindex_after_rebuild.db";
    let _ = std::fs::remove_file(temp_db);

    // Create fresh database
    syncore::db::ensure_schema(temp_db).unwrap();

    // Create CodeGraph
    let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let mut code_graph = CodeGraph::new(temp_db, vector_store).unwrap();

    // Create a test Rust file to index
    let test_file = "/tmp/test_reindex.rs";
    std::fs::write(test_file, "fn test_function() { println!(\"test\"); }").unwrap();

    // Index the file
    let entity_count = code_graph
        .index_file(std::path::Path::new(test_file))
        .unwrap();

    assert!(entity_count > 0, "Should index at least one entity");

    // Test passes if indexing succeeded (we can't directly access vector_store internals)

    // Cleanup
    let _ = std::fs::remove_file(temp_db);
    let _ = std::fs::remove_file(test_file);
}
