//! TDD Test Suite for HNSW Vector Index
//!
//! These tests are written BEFORE implementation to ensure correct behavior.
//! All tests must pass with the real HNSW implementation (no mocks).

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use syncore::vector::hnsw::{HnswConfig, HnswVectorIndex};
use syncore::vector::traits::VectorIndex;

/// Helper: Generate random normalized vector for testing
fn random_vector(dim: usize, rng: &mut StdRng) -> Vec<f32> {
    let mut vec: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
    // Normalize for cosine similarity
    let magnitude: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        vec.iter_mut().for_each(|x| *x /= magnitude);
    }
    vec
}

/// Helper: Brute-force linear search for ground truth comparison
fn brute_force_search(vectors: &[(i64, Vec<f32>)], query: &[f32], k: usize) -> Vec<(i64, f32)> {
    let mut distances: Vec<(i64, f32)> = vectors
        .iter()
        .map(|(id, vec)| {
            // Cosine similarity
            let dot: f32 = query.iter().zip(vec.iter()).map(|(a, b)| a * b).sum();
            (*id, dot)
        })
        .collect();

    // Sort by distance (descending for cosine similarity - higher is better)
    distances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    distances.into_iter().take(k).collect()
}

/// Test 1: Insert single vector and retrieve it
#[test]
fn test_insert_single_vector() {
    let config = HnswConfig::default();
    let mut index = HnswVectorIndex::new(config, 42).expect("Failed to create HNSW index");

    let vec = vec![1.0, 0.0, 0.0, 0.0]; // 4D unit vector
    index.add(1, vec.clone()).expect("Failed to add vector");

    assert_eq!(index.len(), 1, "Index should contain 1 vector");
    assert_eq!(index.dimension(), Some(4), "Dimension should be 4");

    // Search for the same vector
    let results = index.search(&vec, 1).expect("Search failed");
    assert_eq!(results.len(), 1, "Should return 1 result");
    assert_eq!(results[0].0, 1, "Should return ID 1");
    assert!(
        results[0].1 > 0.99,
        "Cosine similarity should be ~1.0 for identical vector"
    );
}

/// Test 2: Insert multiple vectors and verify nearest neighbors
#[test]
fn test_insert_multiple_vectors() {
    let config = HnswConfig::default();
    let mut index = HnswVectorIndex::new(config, 42).expect("Failed to create HNSW index");
    let mut rng = StdRng::seed_from_u64(42);

    let n_vectors = 100;
    let dim = 8;
    let mut vectors = Vec::new();

    // Insert random vectors
    for i in 0..n_vectors {
        let vec = random_vector(dim, &mut rng);
        index.add(i, vec.clone()).expect("Failed to add vector");
        vectors.push((i, vec));
    }

    assert_eq!(
        index.len(),
        n_vectors as usize,
        "Should contain all vectors"
    );

    // Search for a known vector
    let query = &vectors[0].1;
    let results = index.search(query, 5).expect("Search failed");

    assert_eq!(results.len(), 5, "Should return 5 results");
    assert_eq!(
        results[0].0, 0,
        "First result should be ID 0 (query vector itself)"
    );

    // Verify results are sorted by similarity (descending)
    for i in 0..results.len() - 1 {
        assert!(
            results[i].1 >= results[i + 1].1,
            "Results should be sorted by similarity (descending)"
        );
    }
}

/// Test 3: HNSW determinism with fixed seed
#[test]
fn test_hnsw_determinism_with_fixed_seed() {
    let config = HnswConfig::default();
    let seed = 12345u64;
    let dim = 16;
    let n_vectors = 50;

    // Build first index
    let mut index1 = HnswVectorIndex::new(config.clone(), seed).expect("Failed to create index1");
    let mut rng = StdRng::seed_from_u64(seed);
    let vectors: Vec<(i64, Vec<f32>)> = (0..n_vectors)
        .map(|i| (i, random_vector(dim, &mut rng)))
        .collect();

    for (id, vec) in &vectors {
        index1
            .add(*id, vec.clone())
            .expect("Failed to add to index1");
    }

    // Build second index with same seed and vectors
    let mut index2 = HnswVectorIndex::new(config.clone(), seed).expect("Failed to create index2");
    for (id, vec) in &vectors {
        index2
            .add(*id, vec.clone())
            .expect("Failed to add to index2");
    }

    // Query both indices
    let query = &vectors[0].1;
    let results1 = index1.search(query, 10).expect("Search failed on index1");
    let results2 = index2.search(query, 10).expect("Search failed on index2");

    // Results must be identical (deterministic graph construction)
    assert_eq!(results1.len(), results2.len(), "Result counts should match");

    for (r1, r2) in results1.iter().zip(results2.iter()) {
        assert_eq!(r1.0, r2.0, "IDs should match");
        assert!((r1.1 - r2.1).abs() < 1e-6, "Distances should match");
    }
}

/// Test 4: Empty index search
#[test]
fn test_empty_index_search() {
    let config = HnswConfig::default();
    let index = HnswVectorIndex::new(config, 42).expect("Failed to create HNSW index");

    let query = vec![1.0, 0.0, 0.0, 0.0];
    let results = index
        .search(&query, 5)
        .expect("Search on empty index should not fail");

    assert!(
        results.is_empty(),
        "Empty index should return empty results"
    );
    assert_eq!(
        index.dimension(),
        None,
        "Empty index should have no dimension"
    );
}

/// Test 5: Dimension mismatch handling
#[test]
fn test_dimension_mismatch_handling() {
    let config = HnswConfig::default();
    let mut index = HnswVectorIndex::new(config, 42).expect("Failed to create HNSW index");

    // Add first vector (4D)
    let vec1 = vec![1.0, 0.0, 0.0, 0.0];
    index.add(1, vec1).expect("Failed to add first vector");

    // Try to add vector with different dimension (8D)
    let vec2 = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let result = index.add(2, vec2);

    assert!(
        result.is_err(),
        "Adding vector with different dimension should fail"
    );
}

/// Test 6: Large index performance hint (not a strict test, just verification)
#[test]
fn test_large_index_performance_hint() {
    let config = HnswConfig::default();
    let mut index = HnswVectorIndex::new(config, 42).expect("Failed to create HNSW index");
    let mut rng = StdRng::seed_from_u64(42);

    let n_vectors = 10_000;
    let dim = 128;
    let mut vectors = Vec::new();

    println!(
        "Building index with {} vectors of dimension {}...",
        n_vectors, dim
    );

    // Insert vectors
    let start = std::time::Instant::now();
    for i in 0..n_vectors {
        let vec = random_vector(dim, &mut rng);
        index.add(i, vec.clone()).expect("Failed to add vector");
        vectors.push((i, vec));
    }
    let build_time = start.elapsed();
    println!("Build time: {:?}", build_time);

    // Search queries
    let query = &vectors[0].1;
    let start = std::time::Instant::now();
    let results = index.search(query, 10).expect("Search failed");
    let search_time = start.elapsed();
    println!("Search time: {:?}", search_time);

    assert_eq!(results.len(), 10, "Should return 10 results");

    // HNSW search should be sub-linear (much faster than O(N))
    // Linear search would take ~10ms for 10k vectors with 128D
    // HNSW should be < 1ms typically
    println!(
        "Search completed in {:?} (should be sub-linear)",
        search_time
    );

    // Note: This is a performance hint, not a strict assertion
    // Real performance depends on hardware, but HNSW should be noticeably faster than linear
}

/// Test 7: Verify HNSW accuracy against brute-force
#[test]
fn test_hnsw_accuracy_vs_brute_force() {
    let config = HnswConfig {
        m: 16,
        ef_construction: 200,
        ef_search: 50,
    };
    let mut index = HnswVectorIndex::new(config, 42).expect("Failed to create HNSW index");
    let mut rng = StdRng::seed_from_u64(42);

    let n_vectors = 200;
    let dim = 32;
    let mut vectors = Vec::new();

    // Build index
    for i in 0..n_vectors {
        let vec = random_vector(dim, &mut rng);
        index.add(i, vec.clone()).expect("Failed to add vector");
        vectors.push((i, vec));
    }

    // Query and compare with brute-force
    let query = &vectors[0].1;
    let k = 20;

    let hnsw_results = index.search(query, k).expect("HNSW search failed");
    let exact_results = brute_force_search(&vectors, query, k);

    // HNSW should find the exact top-k neighbors for this small dataset
    // (With ef_search=50 and only 200 vectors, accuracy should be very high)
    let hnsw_ids: std::collections::HashSet<i64> = hnsw_results.iter().map(|(id, _)| *id).collect();
    let exact_ids: std::collections::HashSet<i64> =
        exact_results.iter().map(|(id, _)| *id).collect();

    let recall = hnsw_ids.intersection(&exact_ids).count() as f32 / k as f32;

    println!("HNSW recall@{}: {:.2}%", k, recall * 100.0);
    assert!(
        recall >= 0.95,
        "HNSW recall should be >= 95% for this small dataset (got {:.2}%)",
        recall * 100.0
    );
}

/// Test 8: Multiple searches on same index
#[test]
fn test_multiple_searches() {
    let config = HnswConfig::default();
    let mut index = HnswVectorIndex::new(config, 42).expect("Failed to create HNSW index");
    let mut rng = StdRng::seed_from_u64(42);

    let dim = 16;
    let n_vectors = 100;

    // Build index
    let mut vectors = Vec::new();
    for i in 0..n_vectors {
        let vec = random_vector(dim, &mut rng);
        index.add(i, vec.clone()).expect("Failed to add vector");
        vectors.push((i, vec));
    }

    // Perform multiple searches
    for _ in 0..10 {
        let query_idx = rng.gen_range(0..n_vectors) as usize;
        let query = &vectors[query_idx].1;

        let results = index.search(query, 5).expect("Search failed");
        assert_eq!(results.len(), 5, "Should return 5 results");
        assert_eq!(
            results[0].0, query_idx as i64,
            "First result should be query vector itself"
        );
    }
}
