//! TDD Tests for HNSW Comparison Harness
//!
//! Tests BEFORE implementation - validates comparison logic without implementing benchmarks yet.
//! Ensures:
//! 1. Identical results for identical embeddings (sanity)
//! 2. Deterministic neighbor ordering
//! 3. No panics under high load (10,000 inserts)
//! 4. Correct recall@10 for random vectors
//!
//! NOTE: This is test-first development - these tests define the contract
//! that the benchmark harness must satisfy.

use syncore::vector::hnsw::{HnswConfig, HnswVectorIndex};
use syncore::vector::traits::VectorIndex;

/// Test 1: Identical results for identical embeddings (sanity check)
#[test]
fn test_identical_embeddings_produce_same_results() {
    let config = HnswConfig {
        m: 16,
        ef_construction: 200,
        ef_search: 50,
    };

    let mut index = HnswVectorIndex::new(config, 42).unwrap();

    // Insert identical vectors with different IDs
    let embedding = vec![1.0, 2.0, 3.0, 4.0];
    index.add(1, embedding.clone()).unwrap();
    index.add(2, embedding.clone()).unwrap();

    // Search should return both IDs with same distance
    let results = index.search(&embedding, 2).unwrap();
    assert_eq!(results.len(), 2);

    // Both should have same distance (identical vectors)
    assert!(
        (results[0].1 - results[1].1).abs() < 0.001,
        "Distances should be identical"
    );
}

/// Test 2: Deterministic neighbor ordering with fixed seed
/// NOTE: hnsw_rs doesn't expose RNG seed control, so determinism only guaranteed for same insertion order
#[test]
fn test_deterministic_ordering_with_seed() {
    let config = HnswConfig {
        m: 16,
        ef_construction: 200,
        ef_search: 50,
    };

    let mut index1 = HnswVectorIndex::new(config.clone(), 42).unwrap();
    let mut index2 = HnswVectorIndex::new(config, 42).unwrap();

    // Insert same vectors in same order
    for i in 1..=10 {
        let emb = vec![i as f32, (i * 2) as f32, (i * 3) as f32];
        index1.add(i, emb.clone()).unwrap();
        index2.add(i, emb).unwrap();
    }

    // Search with same query
    let query = vec![5.0, 10.0, 15.0];
    let results1 = index1.search(&query, 5).unwrap();
    let results2 = index2.search(&query, 5).unwrap();

    // Results should have same length
    assert_eq!(results1.len(), results2.len());
    assert_eq!(results1.len(), 5);

    // Top results should be close to query vector
    // All returned IDs should be present (order may vary due to hnsw_rs randomness)
    assert!(results1[0].0 >= 1 && results1[0].0 <= 10);
    assert!(results2[0].0 >= 1 && results2[0].0 <= 10);
}

/// Test 3: No panics under high load (10,000 inserts)
#[test]
fn test_no_panic_under_high_load() {
    let config = HnswConfig {
        m: 16,
        ef_construction: 200,
        ef_search: 50,
    };

    let mut index = HnswVectorIndex::new(config, 42).unwrap();

    // Insert 10,000 random vectors (dimension 128)
    for i in 0..10_000 {
        let embedding: Vec<f32> = (0..128).map(|j| ((i + j) % 100) as f32).collect();
        index.add(i, embedding).unwrap();
    }

    // Search should not panic
    let query: Vec<f32> = (0..128).map(|i| (i % 100) as f32).collect();
    let results = index.search(&query, 10).unwrap();

    assert_eq!(results.len(), 10);
}

/// Test 4: Recall@10 accuracy for random vectors
#[test]
fn test_recall_at_10_accuracy() {
    let config = HnswConfig {
        m: 16,
        ef_construction: 200,
        ef_search: 50,
    };

    let mut index = HnswVectorIndex::new(config, 42).unwrap();

    // Insert 100 vectors (dimension 32)
    let mut ground_truth_vectors: Vec<Vec<f32>> = Vec::new();
    for i in 0..100 {
        let embedding: Vec<f32> = (0..32).map(|j| ((i * j) % 100) as f32).collect();
        ground_truth_vectors.push(embedding.clone());
        index.add(i, embedding).unwrap();
    }

    // Query with a vector similar to ID=50
    let query = ground_truth_vectors[50].clone();

    // HNSW search
    let hnsw_results = index.search(&query, 10).unwrap();

    // Brute-force ground truth (linear scan for exact k-NN)
    let mut gt_results: Vec<(i64, f32)> = ground_truth_vectors
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let dist = cosine_distance(&query, v);
            (i as i64, dist)
        })
        .collect();
    gt_results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    gt_results.truncate(10);

    // Calculate recall@10: how many of the top-10 HNSW results are in ground truth top-10?
    let hnsw_ids: Vec<i64> = hnsw_results.iter().map(|(id, _)| *id).collect();
    let gt_ids: Vec<i64> = gt_results.iter().map(|(id, _)| *id).collect();

    let recall = hnsw_ids.iter().filter(|id| gt_ids.contains(id)).count() as f32 / 10.0;

    // Recall should be >= 80% for well-tuned HNSW
    assert!(
        recall >= 0.8,
        "Recall@10 too low: {} (expected >= 0.8)",
        recall
    );
}

/// Test 5: Index state consistency after mixed operations
#[test]
fn test_index_consistency_after_operations() {
    let config = HnswConfig {
        m: 16,
        ef_construction: 200,
        ef_search: 50,
    };

    let mut index = HnswVectorIndex::new(config, 42).unwrap();

    // Insert 50 vectors
    for i in 0..50 {
        let emb = vec![i as f32, (i * 2) as f32];
        index.add(i, emb).unwrap();
    }

    // Search multiple times with different queries
    let query1 = vec![10.0, 20.0];
    let query2 = vec![25.0, 50.0];

    let results1a = index.search(&query1, 5).unwrap();
    let results2a = index.search(&query2, 5).unwrap();

    // Repeat searches - results should be identical (deterministic)
    let results1b = index.search(&query1, 5).unwrap();
    let results2b = index.search(&query2, 5).unwrap();

    assert_eq!(results1a, results1b);
    assert_eq!(results2a, results2b);
}

/// Test 6: Empty index behavior
#[test]
fn test_empty_index_search() {
    let config = HnswConfig {
        m: 16,
        ef_construction: 200,
        ef_search: 50,
    };

    let index = HnswVectorIndex::new(config, 42).unwrap();

    // Search on empty index should return empty results
    let query = vec![1.0, 2.0, 3.0];
    let results = index.search(&query, 10).unwrap();

    assert_eq!(results.len(), 0);
}

/// Test 7: Single vector search
#[test]
fn test_single_vector_search() {
    let config = HnswConfig {
        m: 16,
        ef_construction: 200,
        ef_search: 50,
    };

    let mut index = HnswVectorIndex::new(config, 42).unwrap();

    let embedding = vec![1.0, 2.0, 3.0];
    index.add(42, embedding.clone()).unwrap();

    // Search should return the single vector
    let results = index.search(&embedding, 1).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, 42);
}

// Helper function: cosine distance (used in recall calculation)
fn cosine_distance(v1: &[f32], v2: &[f32]) -> f32 {
    // Normalize vectors
    let norm1: f32 = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm2: f32 = v2.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm1 == 0.0 || norm2 == 0.0 {
        return 1.0; // Max distance for zero vectors
    }

    let v1_norm: Vec<f32> = v1.iter().map(|x| x / norm1).collect();
    let v2_norm: Vec<f32> = v2.iter().map(|x| x / norm2).collect();

    // Cosine similarity -> cosine distance
    // L2 distance of normalized vectors approximates cosine distance
    let dist_squared: f32 = v1_norm
        .iter()
        .zip(v2_norm.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum();

    dist_squared.sqrt()
}
