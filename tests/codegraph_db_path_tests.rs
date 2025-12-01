//! Tests for CodeGraph database path validation
//!
//! Ensures CodeGraph NEVER uses :memory: database accidentally

use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use syncore::code_graph::CodeGraph;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

#[test]
fn test_codegraph_new_rejects_memory_db() {
    // FIX 1: CodeGraph::new must reject :memory: database path
    let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Attempt to create CodeGraph with :memory: should fail
    let result = CodeGraph::new(":memory:", vector_store);

    assert!(result.is_err(), "CodeGraph::new should reject :memory: database");
    if let Err(e) = result {
        let err_msg = format!("{}", e);
        assert!(
            err_msg.contains("memory") || err_msg.contains(":memory:"),
            "Error message should mention :memory: database: {}",
            err_msg
        );
    }
}

#[test]
fn test_codegraph_with_connection_detects_memory_db() {
    // FIX 1: CodeGraph::with_connection should detect if connection is :memory:
    let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Create in-memory connection
    let conn = Connection::open(":memory:").unwrap();
    let db = Arc::new(Mutex::new(conn));

    // Attempt to create CodeGraph with :memory: connection should fail
    let result = CodeGraph::with_connection(db, vector_store);

    assert!(result.is_err(), "CodeGraph::with_connection should reject :memory: database");
    if let Err(e) = result {
        let err_msg = format!("{}", e);
        assert!(
            err_msg.contains("memory") || err_msg.contains(":memory:"),
            "Error message should mention :memory: database: {}",
            err_msg
        );
    }
}

#[test]
fn test_codegraph_accepts_file_db() {
    // CodeGraph should accept valid file paths
    let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let temp_db = "/tmp/test_codegraph_file.db";
    let _ = std::fs::remove_file(temp_db); // Clean up if exists

    let result = CodeGraph::new(temp_db, vector_store);

    assert!(result.is_ok(), "CodeGraph::new should accept file database path: {:?}", result.err());

    // Cleanup
    let _ = std::fs::remove_file(temp_db);
}
