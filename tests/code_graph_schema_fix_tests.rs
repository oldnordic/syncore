//! APEX 2.4-CG-SCHEMA-FIX: Tests for CodeGraph schema initialization
//!
//! These tests verify that:
//! 1. Schema initializes without "duplicate column" errors
//! 2. Schema columns match expected CodeEntity fields
//! 3. Schema migration is idempotent

use anyhow::Result;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use syncore::code_graph::CodeGraph;
use syncore::vector::{StubEmbeddings, VectorStore};

#[tokio::test]
async fn test_schema_initializes_without_duplicate_columns() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_graph.db");

    // Initialize CodeGraph - should NOT error with "duplicate column"
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let result = CodeGraph::new(db_path.to_str().unwrap(), vector_store);

    assert!(
        result.is_ok(),
        "CodeGraph initialization should not fail with duplicate column error: {:?}",
        result.err()
    );

    let code_graph = result?;

    // Verify table_info shows no 'summary' field in code_entities
    let db = code_graph.db_for_testing().lock().unwrap();

    let mut stmt = db.prepare("PRAGMA table_info(code_entities)")?;
    let columns: Vec<String> =
        stmt.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>()?;

    // Count 'summary' columns - should be 0
    let summary_count = columns.iter().filter(|c| c == &"summary").count();
    assert_eq!(
        summary_count, 0,
        "code_entities should not have a 'summary' column, found {} occurrences",
        summary_count
    );

    Ok(())
}

#[tokio::test]
async fn test_schema_fields_match_expected_columns() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_graph.db");

    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    // Query pragma table_info for code_entities
    let db = code_graph.db_for_testing().lock().unwrap();
    let mut stmt = db.prepare("PRAGMA table_info(code_entities)")?;
    let columns: Vec<String> =
        stmt.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>()?;

    // Expected columns based on CodeEntity struct
    let expected_columns = vec![
        "id",
        "file_path",
        "entity_type",
        "name",
        "signature",
        "line_start",
        "line_end",
        "docstring",
        "language",
        "indexed_at",
        "created_at",
        "last_modified_at",
        "change_count",
        "author_count",
        "body_snippet",
    ];

    // Check all expected columns exist
    for expected in &expected_columns {
        assert!(columns.contains(&expected.to_string()), "Missing expected column: {}", expected);
    }

    // Check no duplicate columns
    let mut sorted_columns = columns.clone();
    sorted_columns.sort();
    sorted_columns.dedup();

    assert_eq!(
        columns.len(),
        sorted_columns.len(),
        "Found duplicate columns in code_entities table"
    );

    Ok(())
}

#[tokio::test]
async fn test_schema_migration_idempotent() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_graph.db");

    // Run schema creation first time
    let embeddings1 = Box::new(StubEmbeddings::new(384)?);
    let vector_store1 = Arc::new(Mutex::new(VectorStore::new(embeddings1)));
    let code_graph1 = CodeGraph::new(db_path.to_str().unwrap(), vector_store1)?;

    // Get column count after first initialization
    let db1 = code_graph1.db_for_testing().lock().unwrap();
    let mut stmt1 = db1.prepare("PRAGMA table_info(code_entities)")?;
    let columns_first: Vec<String> =
        stmt1.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>()?;
    drop(stmt1);
    drop(db1);
    drop(code_graph1);

    // Run schema creation second time on same database
    let embeddings2 = Box::new(StubEmbeddings::new(384)?);
    let vector_store2 = Arc::new(Mutex::new(VectorStore::new(embeddings2)));
    let code_graph2 = CodeGraph::new(db_path.to_str().unwrap(), vector_store2)?;

    // Get column count after second initialization
    let db2 = code_graph2.db_for_testing().lock().unwrap();
    let mut stmt2 = db2.prepare("PRAGMA table_info(code_entities)")?;
    let columns_second: Vec<String> =
        stmt2.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>()?;

    // Verify second run didn't add duplicate columns
    assert_eq!(
        columns_first.len(),
        columns_second.len(),
        "Second schema initialization should not add new columns"
    );

    assert_eq!(
        columns_first, columns_second,
        "Columns should be identical after idempotent migration"
    );

    Ok(())
}
