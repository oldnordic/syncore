//! HNSW Corruption Test - Direct test for hnsw_rs panic on UnexpectedEof
//! This test creates the exact HNSW file structure that triggers the panic

use anyhow::Result;
use std::fs;
use syncore::vector::hnsw::{HnswVectorIndex, HnswConfig};
use tempfile::TempDir;

/// Test that HNSW index load handles corrupted snapshot files without panicking
/// This directly tests the hnsw_rs panic path at line 1257 in hnswio.rs
#[tokio::test]
async fn test_hnsw_direct_corrupted_snapshot_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // HNSW creates multiple files with basename prefix:
    // - {basename}.hnsw_graph (graph structure)
    // - {basename}.hnsw_data (vector data)
    // - {basename}.hnsw_graph_layer (layer info)
    let hnsw_path = root.join("test.hnsw");
    let dir = hnsw_path.parent().unwrap();
    let basename = hnsw_path.file_stem().and_then(|s| s.to_str()).unwrap();

    // Create truncated HNSW files that will trigger read_exact().unwrap() panic
    // The key is creating .hnsw.graph file with incomplete data
    let graph_file = dir.join(format!("{}.hnsw.graph", basename));
    let data_file = dir.join(format!("{}.hnsw.data", basename));
    let layer_file = dir.join(format!("{}.hnsw.layer", basename));

    // Write a truncated graph file that will cause UnexpectedEof when read_exact tries to read usize
    // This specifically triggers the panic at line 1257 in hnswio.rs
    let truncated_graph_data = vec![0u8; 10]; // Too small to contain valid usize
    fs::write(&graph_file, truncated_graph_data)?;

    // Write minimal data files
    fs::write(&data_file, vec![0u8; 10])?;
    fs::write(&layer_file, vec![0u8; 10])?;

    // Now try to load this corrupted HNSW index
    let config = HnswConfig::default();
    let mut index = HnswVectorIndex::new(config, 42)?;

    // This should NOT panic, even though hnsw_rs will hit UnexpectedEof
    // The current implementation will panic because hnsw_rs uses .unwrap()
    let result = index.load_from_disk(&hnsw_path);

    // Should return an error, not panic
    assert!(result.is_err());

    // Check the error message contains information about the failure
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("deserialization failed") ||
            error_msg.contains("UnexpectedEof") ||
            error_msg.contains("failed to fill whole buffer") ||
            error_msg.contains("corrupted") ||
            error_msg.contains("panic caught"));

    Ok(())
}

/// Test that zero-length HNSW files don't panic
#[tokio::test]
async fn test_hnsw_zero_length_snapshot_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    let hnsw_path = root.join("test.hnsw");
    let dir = hnsw_path.parent().unwrap();
    let basename = hnsw_path.file_stem().and_then(|s| s.to_str()).unwrap();

    // Create zero-length HNSW files
    let graph_file = dir.join(format!("{}.hnsw.graph", basename));
    let data_file = dir.join(format!("{}.hnsw.data", basename));

    fs::write(&graph_file, vec![])?; // Empty file
    fs::write(&data_file, vec![])?;  // Empty file

    let config = HnswConfig::default();
    let mut index = HnswVectorIndex::new(config, 42)?;

    // Should not panic
    let result = index.load_from_disk(&hnsw_path);
    assert!(result.is_err());

    Ok(())
}