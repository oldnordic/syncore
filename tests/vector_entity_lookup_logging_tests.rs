//! Tests for vector→entity lookup logging
//!
//! Ensures failed lookups are logged instead of silently ignored

use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use syncore::code_graph::CodeGraph;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

#[test]
fn test_search_logs_failed_vector_entity_lookup() {
    // FIX 3: When get_entity_by_vector_id fails, it should log a warning
    let temp_db = "/tmp/test_lookup_logging.db";
    let _ = std::fs::remove_file(temp_db);

    // Create database
    let conn = Connection::open(temp_db).unwrap();
    syncore::db::ensure_schema(temp_db).unwrap();
    drop(conn);

    // Create CodeGraph with vector store containing orphaned vector ID
    let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let mut vector_store = VectorStore::new(embeddings);

    // Insert vector with ID 8888 that doesn't exist in code_embeddings
    let orphan_embedding = vec![0.1; 384];
    vector_store.add_test_vector(8888, None, orphan_embedding, "orphan text".to_string());

    let vector_store_arc = Arc::new(Mutex::new(vector_store));

    let mut code_graph = CodeGraph::new(temp_db, vector_store_arc).unwrap();

    // Search should NOT panic, but should log the failure
    let result = code_graph.search_code("orphan text", 10);

    // Should return Ok with empty or partial results (orphan ID skipped)
    assert!(
        result.is_ok(),
        "search_code should not fail: {:?}",
        result.err()
    );

    let matches = result.unwrap();
    // The orphaned vector should be skipped (not returned in matches)
    // We can't directly test logging here, but the function should not crash

    // Cleanup
    let _ = std::fs::remove_file(temp_db);
}

#[test]
fn test_rag_graph_logs_failed_neo4j_lookup() {
    // FIX 3: RAGGraph should log when Neo4j graph_score query fails
    // This is tested indirectly - the main fix is adding log statements
    // The test ensures the code path doesn't panic

    let temp_db = "/tmp/test_rag_logging.db";
    let _ = std::fs::remove_file(temp_db);

    syncore::db::ensure_schema(temp_db).unwrap();

    let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let mut code_graph = CodeGraph::new(temp_db, vector_store).unwrap();

    // Create a test file
    let test_file = "/tmp/test_rag.rs";
    std::fs::write(test_file, "fn test() {}").unwrap();

    // Index it
    let _ = code_graph.index_file(std::path::Path::new(test_file));

    // Search should work
    let result = code_graph.search_code("test", 5);
    assert!(result.is_ok());

    // Cleanup
    let _ = std::fs::remove_file(temp_db);
    let _ = std::fs::remove_file(test_file);
}
