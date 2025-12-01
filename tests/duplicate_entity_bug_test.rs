//! Test for duplicate entity insertion bug
//!
//! ISSUE: indexer.rs extracts duplicate entities from the same file,
//! causing UNIQUE constraint violations.
//!
//! Bug observed in: src/code_graph/edge_extractor.rs
//! Error: UNIQUE constraint failed: code_entities.file_path, entity_type, name, line_start
//!
//! Root cause: Parser extracts same entity multiple times:
//! - Traits from imports (lines 240-265)
//! - Traits from AST traversal
//! - Constants from multiple sources
//!
//! EXPECTED: This test should FAIL initially, then PASS after fix

use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::io::Write as IoWrite;
use std::sync::{Arc, Mutex};
use syncore::code_graph::CodeGraph;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use tempfile::TempDir;

/// Test that reproduces the exact UNIQUE constraint error from production
#[test]
fn test_duplicate_entity_rejection() -> Result<()> {
    // Arrange: Create a test file that triggers duplicate entity extraction
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("duplicate_trigger.rs");

    // This Rust code pattern is known to cause duplicate entity extraction:
    // 1. Trait in use statement
    // 2. Same trait referenced in impl block
    let mut file = fs::File::create(&test_file)?;
    writeln!(file, "use std::fmt::Debug;")?;
    writeln!(file)?;
    writeln!(file, "pub struct MyStruct {{}}")?;
    writeln!(file)?;
    writeln!(file, "impl Debug for MyStruct {{")?;
    writeln!(file, "    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{")?;
    writeln!(file, "        write!(f, \"MyStruct\")")?;
    writeln!(file, "    }}")?;
    writeln!(file, "}}")?;
    drop(file);

    // Create CodeGraph with real database
    let db_path = temp_dir.path().join("test.db");
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    // Act: Index the file (should not fail)
    let result = code_graph.index_file(&test_file);

    // Assert: Should succeed without UNIQUE constraint error
    match result {
        Ok(count) => {
            assert!(count > 0, "Should have indexed at least 1 entity");

            // Verify no duplicate entities in database
            let conn = Connection::open(&db_path)?;
            let duplicate_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM (
                    SELECT file_path, entity_type, name, line_start, COUNT(*) as cnt
                    FROM code_entities
                    GROUP BY file_path, entity_type, name, line_start
                    HAVING cnt > 1
                )",
                [],
                |row| row.get(0),
            )?;

            assert_eq!(
                duplicate_count, 0,
                "Found {} duplicate entities with same (file_path, entity_type, name, line_start)",
                duplicate_count
            );

            Ok(())
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("UNIQUE constraint failed") {
                panic!(
                    "EXPECTED FAILURE: UNIQUE constraint error detected!\n\
                     Error: {}\n\
                     This test documents the bug. After fix, this should pass.",
                    error_msg
                );
            } else {
                // Some other error - propagate it
                Err(e)
            }
        }
    }
}

/// Test edge_extractor.rs specifically (the file from the production error)
#[test]
fn test_edge_extractor_file_no_duplicates() -> Result<()> {
    // Arrange: Index the actual edge_extractor.rs file that failed in production
    let edge_extractor_path =
        "/home/feanor/Projects/SynCore/syncore/src/code_graph/edge_extractor.rs";

    if !std::path::Path::new(edge_extractor_path).exists() {
        // Skip test if file doesn't exist (e.g., in CI)
        eprintln!("Skipping test: edge_extractor.rs not found");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    // Act: Index edge_extractor.rs
    let result = code_graph.index_file(std::path::Path::new(edge_extractor_path));

    // Assert: Should succeed
    match result {
        Ok(count) => {
            eprintln!("Successfully indexed {} entities from edge_extractor.rs", count);

            // Verify no duplicates
            let conn = Connection::open(&db_path)?;
            let duplicate_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM (
                    SELECT file_path, entity_type, name, line_start, COUNT(*) as cnt
                    FROM code_entities
                    GROUP BY file_path, entity_type, name, line_start
                    HAVING cnt > 1
                )",
                [],
                |row| row.get(0),
            )?;

            assert_eq!(
                duplicate_count, 0,
                "Found {} duplicate entities in edge_extractor.rs",
                duplicate_count
            );

            Ok(())
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("UNIQUE constraint failed") {
                panic!(
                    "PRODUCTION BUG REPRODUCED: edge_extractor.rs triggers UNIQUE constraint!\n\
                     Error: {}\n\
                     This is the exact file that failed in production.",
                    error_msg
                );
            } else {
                Err(e)
            }
        }
    }
}

/// Regression test: Verify deduplication doesn't break normal indexing
#[test]
fn test_normal_file_indexing_still_works() -> Result<()> {
    // Arrange: Create a simple Rust file with no duplicates
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("normal.rs");

    let mut file = fs::File::create(&test_file)?;
    writeln!(file, "pub fn add(a: i32, b: i32) -> i32 {{")?;
    writeln!(file, "    a + b")?;
    writeln!(file, "}}")?;
    writeln!(file)?;
    writeln!(file, "pub fn sub(a: i32, b: i32) -> i32 {{")?;
    writeln!(file, "    a - b")?;
    writeln!(file, "}}")?;
    drop(file);

    let db_path = temp_dir.path().join("test.db");
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    // Act: Index the file
    let count = code_graph.index_file(&test_file)?;

    // Assert: Should find 2 functions
    assert_eq!(count, 2, "Should have indexed 2 functions (add and sub)");

    // Verify entities in database
    let conn = Connection::open(&db_path)?;
    let db_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_entities WHERE entity_type = 'function'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(db_count, 2, "Database should contain 2 function entities");

    Ok(())
}
