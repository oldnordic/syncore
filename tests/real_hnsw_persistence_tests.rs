//! REAL HNSW Persistence Tests
//!
//! TDD tests for HNSW index persistence functionality.
//! These tests MUST fail initially and drive the implementation.
//!
//! Requirements:
//! - save_to_disk() / load_from_disk() round-trip
//! - rebuild_from_vectors() when index file missing
//! - delete() operation removes vectors correctly
//! - startup loads index automatically
//! - search accuracy matches brute-force cosine

use anyhow::Result;
use syncore::vector::hnsw::{HnswConfig, HnswVectorIndex};
use syncore::vector::traits::VectorIndex;
use std::path::PathBuf;

/// Test index file path
fn test_index_path() -> PathBuf {
    PathBuf::from("/tmp/syncore_test_hnsw.index")
}

/// Clean up test files
/// Note: hnsw_rs creates multiple files with basename prefix
fn cleanup() {
    use std::fs;
    let path = test_index_path();
    let dir = path.parent().unwrap();
    let basename = path.file_stem().unwrap().to_str().unwrap();

    // Remove all files matching the pattern: basename.hnsw.*
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let filename = entry.file_name();
            if let Some(name) = filename.to_str() {
                if name.starts_with(basename) && name.contains(".hnsw.") {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }
}

#[test]
fn test_hnsw_insert_and_search() -> Result<()> {
    cleanup();

    let config = HnswConfig::default();
    let mut index = HnswVectorIndex::new(config, 42)?;

    // Insert 100 vectors with dense random-like patterns
    // Using deterministic pseudo-random values for reproducibility
    for i in 0..100 {
        let vec: Vec<f32> = (0..384)
            .map(|j| ((i * 7 + j * 13) % 100) as f32 / 100.0)
            .collect();
        index.add(i as i64, vec)?;
    }

    assert_eq!(index.len(), 100);
    assert_eq!(index.dimension(), Some(384));

    // Search for similar vector (matching vector 50's pattern)
    let query: Vec<f32> = (0..384)
        .map(|j| ((50 * 7 + j * 13) % 100) as f32 / 100.0)
        .collect();

    let results = index.search(&query, 5)?;
    assert_eq!(results.len(), 5);

    // ID 50 should be most similar (cosine = 1.0)
    assert_eq!(results[0].0, 50);
    assert!((results[0].1 - 1.0).abs() < 0.01);

    cleanup();
    Ok(())
}

#[test]
fn test_hnsw_persistence_roundtrip() -> Result<()> {
    cleanup();

    let config = HnswConfig::default();
    let mut index = HnswVectorIndex::new(config.clone(), 42)?;

    // Insert test vectors with dense random-like patterns
    for i in 0..50 {
        let vec: Vec<f32> = (0..384)
            .map(|j| ((i * 7 + j * 13) % 100) as f32 / 100.0)
            .collect();
        index.add(i as i64, vec)?;
    }

    // PHASE 2: This will fail - save_to_disk not implemented yet
    index.save_to_disk(&test_index_path())?;

    // Create new index and load
    let mut index2 = HnswVectorIndex::new(config, 42)?;
    index2.load_from_disk(&test_index_path())?;

    // Verify loaded index matches original
    assert_eq!(index2.len(), 50);
    // Note: dimension is inferred lazily on first search, check it after

    // Verify search results match
    let query: Vec<f32> = (0..384).map(|j| ((0 * 7 + j * 13) % 100) as f32 / 100.0).collect();
    let results1 = index.search(&query, 5)?;
    let results2 = index2.search(&query, 5)?;

    // After first search, dimension should be inferred
    assert_eq!(index2.dimension(), Some(384));

    assert_eq!(results1.len(), results2.len());
    for (r1, r2) in results1.iter().zip(results2.iter()) {
        assert_eq!(r1.0, r2.0); // Same IDs
        assert!((r1.1 - r2.1).abs() < 0.001); // Same similarities
    }

    cleanup();
    Ok(())
}

#[test]
fn test_hnsw_rebuild_from_vectors_if_file_missing() -> Result<()> {
    cleanup();

    // This test simulates startup when index file is missing
    // Index should rebuild from provided vectors

    let config = HnswConfig::default();
    let mut index = HnswVectorIndex::new(config, 42)?;

    // Simulate vectors loaded from SQLite with dense random-like patterns
    let vectors: Vec<(i64, Vec<f32>)> = (0..30)
        .map(|i| {
            let vec: Vec<f32> = (0..384)
                .map(|j| ((i * 7 + j * 13) % 100) as f32 / 100.0)
                .collect();
            (i as i64, vec)
        })
        .collect();

    // PHASE 2: This will fail - rebuild_from_vectors not implemented
    index.rebuild_from_vectors(&vectors)?;

    assert_eq!(index.len(), 30);
    assert_eq!(index.dimension(), Some(384));

    // Search should work immediately after rebuild
    let query: Vec<f32> = (0..384).map(|j| ((0 * 7 + j * 13) % 100) as f32 / 100.0).collect();
    let results = index.search(&query, 5)?;
    assert_eq!(results.len(), 5);

    cleanup();
    Ok(())
}

#[test]
fn test_hnsw_delete_requires_rebuild() -> Result<()> {
    cleanup();

    let config = HnswConfig::default();
    let mut index = HnswVectorIndex::new(config.clone(), 42)?;

    // Insert 20 vectors with dense random-like patterns
    for i in 0..20 {
        let vec: Vec<f32> = (0..384)
            .map(|j| ((i * 7 + j * 13) % 100) as f32 / 100.0)
            .collect();
        index.add(i as i64, vec)?;
    }

    assert_eq!(index.len(), 20);

    // PHASE 3: delete() returns error indicating rebuild required
    let result = index.delete(10);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("rebuild"));

    // Workaround: rebuild without deleted ID with dense random-like patterns
    let vectors: Vec<(i64, Vec<f32>)> = (0..20)
        .filter(|i| *i != 10) // Exclude ID 10
        .map(|i| {
            let vec: Vec<f32> = (0..384)
                .map(|j| ((i * 7 + j * 13) % 100) as f32 / 100.0)
                .collect();
            (i as i64, vec)
        })
        .collect();

    index.rebuild_from_vectors(&vectors)?;

    assert_eq!(index.len(), 19);

    // Verify deleted vector not in search results
    let query: Vec<f32> = (0..384).map(|j| ((0 * 7 + j * 13) % 100) as f32 / 100.0).collect();
    let results = index.search(&query, 20)?;

    for (id, _) in results {
        assert_ne!(id, 10, "Deleted vector should not appear in results");
    }

    cleanup();
    Ok(())
}

#[test]
fn test_hnsw_knn_accuracy_against_bruteforce() -> Result<()> {
    cleanup();

    // Generate random-ish vectors for accuracy comparison
    let config = HnswConfig {
        m: 16,
        ef_construction: 200,
        ef_search: 100,
    };
    let mut index = HnswVectorIndex::new(config, 42)?;

    let vectors: Vec<(i64, Vec<f32>)> = (0..100)
        .map(|i| {
            let vec: Vec<f32> = (0..384)
                .map(|j| ((i * 7 + j * 13) % 100) as f32 / 100.0)
                .collect();
            (i as i64, vec)
        })
        .collect();

    // Insert into HNSW
    for (id, vec) in &vectors {
        index.add(*id, vec.clone())?;
    }

    // Query vector
    let query: Vec<f32> = (0..384).map(|i| (i % 50) as f32 / 50.0).collect();

    // HNSW search
    let hnsw_results = index.search(&query, 10)?;

    // Brute-force search (ground truth)
    let mut brute_results: Vec<(i64, f32)> = vectors
        .iter()
        .map(|(id, vec)| {
            let cosine = cosine_similarity(&query, vec);
            (*id, cosine)
        })
        .collect();
    brute_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    brute_results.truncate(10);

    // HNSW should get at least 8/10 top results correct (80% recall)
    let hnsw_ids: Vec<i64> = hnsw_results.iter().map(|(id, _)| *id).collect();
    let brute_ids: Vec<i64> = brute_results.iter().map(|(id, _)| *id).collect();

    let matches = hnsw_ids.iter().filter(|id| brute_ids.contains(id)).count();
    assert!(
        matches >= 8,
        "HNSW recall too low: {}/10 (expected >= 8/10)",
        matches
    );

    cleanup();
    Ok(())
}

#[test]
#[ignore] // Run manually during integration testing
fn test_hnsw_startup_loads_index_before_use() -> Result<()> {
    cleanup();

    // This test verifies startup behavior:
    // 1. Create and save index
    // 2. Simulate restart
    // 3. Index should auto-load on first use

    let config = HnswConfig::default();
    let mut index = HnswVectorIndex::new(config.clone(), 42)?;

    // Insert and save with dense random-like patterns
    for i in 0..50 {
        let vec: Vec<f32> = (0..384)
            .map(|j| ((i * 7 + j * 13) % 100) as f32 / 100.0)
            .collect();
        index.add(i as i64, vec)?;
    }
    index.save_to_disk(&test_index_path())?;

    // Simulate restart - create new index
    let index2 = HnswVectorIndex::new(config, 42)?;

    // PHASE 5: This should auto-load on first search
    let query: Vec<f32> = (0..384).map(|j| ((0 * 7 + j * 13) % 100) as f32 / 100.0).collect();
    let results = index2.search(&query, 5)?;

    assert_eq!(results.len(), 5);
    assert_eq!(index2.len(), 50);

    cleanup();
    Ok(())
}

// Helper function for brute-force cosine similarity
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
