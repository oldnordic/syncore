//! Integration test: Syncore MCP server startup with corrupted HNSW snapshots
//! Tests that the server can start up even when HNSW snapshots are corrupted

use anyhow::Result;
use std::fs;
use syncore::vector::{SearchScope, VectorStore};
use tempfile::TempDir;

/// Test that VectorStore (used by MCP server) handles corrupted HNSW snapshots on startup
/// This simulates the real-world scenario: abrupt shutdown leaves corrupted snapshots,
/// server restarts should handle this gracefully without panicking.
#[tokio::test]
async fn test_vectorstore_startup_corrupted_snapshot_resilience() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Create the scenario: corrupted HNSW snapshot files from previous crash
    let hnsw_path = root.join("test.hnsw");
    let vectors_path = root.join("test.vectors");
    let meta_path = root.join("test.meta");

    // Create corrupted HNSW files (simulate abrupt shutdown)
    let dir = hnsw_path.parent().unwrap();
    let basename = hnsw_path.file_stem().and_then(|s| s.to_str()).unwrap();

    // Create truncated/corrupted HNSW files that would cause UnexpectedEof
    let graph_file = dir.join(format!("{}.hnsw.graph", basename));
    let data_file = dir.join(format!("{}.hnsw.data", basename));
    let layer_file = dir.join(format!("{}.hnsw.layer", basename));

    // These files will trigger the exact panic we fixed
    fs::write(&graph_file, vec![0u8; 5])?; // Too short - causes UnexpectedEof
    fs::write(&data_file, vec![0u8; 5])?; // Too short - causes UnexpectedEof
    fs::write(&layer_file, vec![0u8; 5])?; // Too short - causes UnexpectedEof

    // This is the critical test: VectorStore startup should NOT panic
    // It should detect corrupted HNSW, fall back to rebuild from vectors, and work normally
    let embeddings = Box::new(syncore::vector::RealEmbeddings::new(3)?);
    let mut store = VectorStore::new(embeddings);
    store.set_index_path(hnsw_path.to_string_lossy().to_string());

    // Insert data (this would panic before our fix if corrupted snapshot loading failed)
    let insert_result = store.insert_text(1, None, "test", "code_entity");

    // Should succeed (corrupted snapshots handled gracefully)
    assert!(insert_result.is_ok());

    // Verify the store is functional after handling corrupted snapshot
    assert_eq!(store.len(), 1); // Should have the vector we just inserted

    // Test that search still works
    let results = store.search("test", 5, SearchScope::Global)?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);

    Ok(())
}

/// Test that creating VectorStore with zero-length HNSW files works
#[tokio::test]
async fn test_vectorstore_zero_length_hnsw_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    let hnsw_path = root.join("test.hnsw");

    // Create zero-length HNSW files
    let dir = hnsw_path.parent().unwrap();
    let basename = hnsw_path.file_stem().and_then(|s| s.to_str()).unwrap();

    let graph_file = dir.join(format!("{}.hnsw.graph", basename));
    let data_file = dir.join(format!("{}.hnsw.data", basename));

    fs::write(&graph_file, vec![])?; // Empty file
    fs::write(&data_file, vec![])?; // Empty file

    // Should not panic when creating VectorStore with empty HNSW files
    let embeddings = Box::new(syncore::vector::RealEmbeddings::new(3)?);
    let mut store = VectorStore::new(embeddings);
    store.set_index_path(hnsw_path.to_string_lossy().to_string());

    // Should be able to insert and search normally
    store.insert_text(1, None, "test content", "code_entity")?;

    let results = store.search("test", 5, SearchScope::Global)?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);

    Ok(())
}
