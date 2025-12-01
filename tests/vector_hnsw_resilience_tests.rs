//! HNSW + VectorStore resilience tests for Phase 4
//! Tests that HNSW snapshot loading never panics on missing/corrupt files

use anyhow::Result;
use std::fs;

use syncore::vector::{HuggingFaceEmbeddings, SearchScope, VectorStore};
use tempfile::TempDir;

/// Test that HNSW load_missing_snapshot_does_not_panic
#[tokio::test]
async fn test_hnsw_load_missing_snapshot_does_not_panic() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Ensure no snapshot files exist
    let hnsw_path = root.join("test.hnsw");
    let vectors_path = root.join("test.vectors");
    let meta_path = root.join("test.meta");

    assert!(!hnsw_path.exists());
    assert!(!vectors_path.exists());
    assert!(!meta_path.exists());

    // Create VectorStore with HuggingFace embeddings
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store = VectorStore::new(embeddings);
    store.set_index_path(hnsw_path.to_string_lossy().to_string());

    // This should NOT panic, even with missing snapshot files
    // Verify store is in a valid, empty state
    assert_eq!(store.len(), 0);

    // Verify we can still use the store (insert/search should work)
    store.insert_text(1, None, "test", "code_entity")?;

    let results = store.search("test", 5, SearchScope::Global)?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);

    Ok(())
}

/// Test that HNSW load_truncated_snapshot_does_not_panic
#[tokio::test]
async fn test_hnsw_load_truncated_snapshot_does_not_panic() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Create truncated/bogus snapshot files
    let hnsw_path = root.join("test.hnsw");
    let vectors_path = root.join("test.vectors");
    let meta_path = root.join("test.meta");

    // Write truncated files (just a few bytes)
    fs::write(&hnsw_path, b"truncated")?;
    fs::write(&vectors_path, b"truncated")?;
    fs::write(&meta_path, b"truncated")?;

    // Create VectorStore with HuggingFace embeddings
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store = VectorStore::new(embeddings);
    store.set_index_path(hnsw_path.to_string_lossy().to_string());

    // This should NOT panic, even with truncated snapshot files
    // Verify store is in a valid, empty state (fallback to empty)
    assert_eq!(store.len(), 0);

    // Verify we can still use the store
    store.insert_text(1, None, "test", "code_entity")?;

    let results = store.search("test", 5, SearchScope::Global)?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);

    Ok(())
}

/// Test that VectorStore startup handles corrupt snapshot
#[tokio::test]
async fn test_vectorstore_startup_handles_corrupt_snapshot() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Create corrupt snapshot files (invalid binary data)
    let hnsw_path = root.join("test.hnsw");
    let vectors_path = root.join("test.vectors");
    let meta_path = root.join("test.meta");

    // Write invalid binary data that will cause deserialization to fail
    let corrupt_data = vec![0xFF; 1024]; // All 255 bytes
    fs::write(&hnsw_path, corrupt_data.clone())?;
    fs::write(&vectors_path, corrupt_data.clone())?;
    fs::write(&meta_path, corrupt_data.clone())?;

    // Create VectorStore with HuggingFace embeddings
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store = VectorStore::new(embeddings);
    store.set_index_path(hnsw_path.to_string_lossy().to_string());

    // This should NOT panic, even with corrupt snapshot files
    // Verify store is in a usable state
    assert_eq!(store.len(), 0);

    // Verify we can insert and search (store should be functional)
    store.insert_text(1, None, "test", "code_entity")?;

    let results = store.search("test", 5, SearchScope::Global)?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);

    Ok(())
}

/// Test that HNSW index load_or_empty works correctly
#[tokio::test]
async fn test_hnsw_load_or_empty_returns_valid_index() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Test with non-existent path
    let nonexistent_path = root.join("nonexistent");

    // Create VectorStore with HuggingFace embeddings
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store = VectorStore::new(embeddings);
    store.set_index_path(nonexistent_path.to_string_lossy().to_string());

    // Should be empty but functional
    assert_eq!(store.len(), 0);

    // Should be able to add vectors and search
    store.insert_text(1, None, "test content", "code_entity")?;
    store.insert_text(2, None, "xyz qwerty asdfgh", "code_entity")?;
    let results = store.search("test", 5, SearchScope::Global)?;
    // Should find at least 1 result (the exact match)
    assert!(results.len() >= 1);
    // Should find the exact match first (highest similarity)
    assert_eq!(results[0].id, 1);

    Ok(())
}
