use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};
use syncore::code_graph::CodeGraph;
use syncore::vector::{RealEmbeddings, VectorStore};
use tempfile::TempDir;

fn create_test_code_graph() -> Result<(TempDir, CodeGraph)> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("code_graph.db");
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid temp db path"))?
        .to_string();

    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(&db_path_str, vector_store)?;

    Ok((temp_dir, code_graph))
}

#[test]
fn test_index_same_file_twice_is_idempotent() -> Result<()> {
    let (_temp_dir, mut code_graph) = create_test_code_graph()?;
    let file_to_index = Path::new("src/code_graph/indexer.rs");
    assert!(file_to_index.exists(), "Test file must exist in repo");

    let canonical_path = file_to_index.canonicalize()?;
    let canonical_str = canonical_path.to_string_lossy().to_string();

    let first_entities = code_graph.index_file(file_to_index)?;
    assert!(first_entities > 0, "Initial indexing should produce entities");

    let count_after_first: i64 = {
        let db = code_graph.db_for_testing().lock().unwrap();
        db.query_row(
            "SELECT COUNT(*) FROM code_entities WHERE file_path = ?",
            params![canonical_str.clone()],
            |row| row.get(0),
        )?
    };

    {
        let db = code_graph.db_for_testing().lock().unwrap();
        db.execute(
            "DELETE FROM file_index_state WHERE file_path = ?",
            params![canonical_str.clone()],
        )?;
    }

    let second_entities = code_graph.index_file(file_to_index)?;
    assert!(second_entities > 0, "Reindexing should still produce entities");

    let count_after_second: i64 = {
        let db = code_graph.db_for_testing().lock().unwrap();
        db.query_row(
            "SELECT COUNT(*) FROM code_entities WHERE file_path = ?",
            params![canonical_str.clone()],
            |row| row.get(0),
        )?
    };

    assert_eq!(
        count_after_first, count_after_second,
        "Entity count should remain stable after reindexing"
    );

    Ok(())
}

#[test]
fn test_file_path_normalization_uses_canonical_form() -> Result<()> {
    let (_temp_dir, mut code_graph) = create_test_code_graph()?;
    let file_to_index = Path::new("src/code_graph/indexer.rs");
    assert!(file_to_index.exists(), "Test file must exist in repo");

    code_graph.index_file(file_to_index)?;

    let canonical_path = file_to_index.canonicalize()?;
    let canonical_str = canonical_path.to_string_lossy().to_string();

    let stored_path: Option<String> = {
        let db = code_graph.db_for_testing().lock().unwrap();
        db.query_row(
            "SELECT file_path FROM code_entities WHERE file_path = ? LIMIT 1",
            params![canonical_str.clone()],
            |row| row.get(0),
        )
        .optional()?
    };

    assert!(stored_path.is_some(), "Canonical path should be stored in code_entities");
    let stored_path = stored_path.unwrap();
    assert!(stored_path.starts_with('/'), "Stored path should be absolute");
    assert_eq!(stored_path, canonical_str, "Stored path must match canonical path");

    Ok(())
}
