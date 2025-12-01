//! Integration tests for VectorIndex trait implementation on VectorStore
//!
//! These tests verify that VectorStore correctly implements the VectorIndex trait
//! with deterministic, production-ready behavior.

use anyhow::Result;
use syncore::vector::{RealEmbeddings, VectorIndex, VectorMeta, VectorStore};

#[test]
fn test_vectorindex_add_inserts_vector() -> Result<()> {
    let dim = 384;
    let embeddings = Box::new(RealEmbeddings::new(dim)?);
    let meta = VectorMeta {
        dim,
        m: 32,
        ef_construction: 200,
        ef_search: 100,
    };
    let mut store = VectorStore::with_meta(embeddings, meta);
    store.set_index_path("/tmp/test_vectorindex_add.idx".to_string());
    store.set_fast_mode(false);

    // Add a vector with known embedding
    let embedding = vec![0.5; dim];
    store.add(100, embedding.clone())?;

    // Verify it was added
    assert_eq!(store.len(), 1);

    // Verify we can search and find it (using VectorIndex trait method)
    let query = vec![0.5; dim];
    let results = VectorIndex::search(&store, &query, 1)?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, 100); // ID should match

    Ok(())
}

#[test]
fn test_vectorindex_search_cosine_similarity() -> Result<()> {
    let dim = 4; // Use small dimension for test vectors
    let embeddings = Box::new(RealEmbeddings::new(dim)?);
    let meta = VectorMeta {
        dim,
        m: 32,
        ef_construction: 200,
        ef_search: 100,
    };
    let mut store = VectorStore::with_meta(embeddings, meta);
    store.set_index_path("/tmp/test_vectorindex_cosine.idx".to_string());
    store.set_fast_mode(false);

    // Add three vectors with known relationships
    // Vector 1: [1, 0, 0, 0] - orthogonal to others
    store.add(1, vec![1.0, 0.0, 0.0, 0.0])?;

    // Vector 2: [0, 1, 0, 0] - orthogonal to 1, similar to 3
    store.add(2, vec![0.0, 1.0, 0.0, 0.0])?;

    // Vector 3: [0, 0.7071, 0.7071, 0] - similar to 2
    store.add(3, vec![0.0, f64::consts::FRAC_1_SQRT_2, f64::consts::FRAC_1_SQRT_2, 0.0])?;

    // Query with vector similar to 2 and 3
    let query = vec![0.0, 1.0, 0.0, 0.0];
    let results = VectorIndex::search(&store, &query, 3)?;

    // Should return all 3, but 2 should be first (exact match)
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].0, 2); // Exact match
    assert!(results[0].1 > 0.9); // High similarity

    // Vector 3 should be second (similar to 2)
    assert_eq!(results[1].0, 3);
    assert!(results[1].1 > 0.5); // Moderate similarity

    // Vector 1 should be last (orthogonal, ~0 similarity)
    assert_eq!(results[2].0, 1);
    assert!(results[2].1.abs() < 0.1); // Near zero

    Ok(())
}

#[test]
fn test_vectorindex_dimension_matches_vectors() -> Result<()> {
    let dim = 128;
    let embeddings = Box::new(RealEmbeddings::new(dim)?);
    let meta = VectorMeta {
        dim,
        m: 32,
        ef_construction: 200,
        ef_search: 100,
    };
    let mut store = VectorStore::with_meta(embeddings, meta);
    store.set_index_path("/tmp/test_vectorindex_dim.idx".to_string());

    // Before adding any vectors, dimension should match embeddings
    assert_eq!(store.dimension(), Some(dim));

    // Add a vector
    let embedding = vec![0.5; dim];
    store.add(1, embedding)?;

    // After adding, dimension should still match
    assert_eq!(store.dimension(), Some(dim));

    Ok(())
}

#[test]
fn test_vectorindex_len_reflects_count() -> Result<()> {
    let dim = 256;
    let embeddings = Box::new(RealEmbeddings::new(dim)?);
    let meta = VectorMeta {
        dim,
        m: 32,
        ef_construction: 200,
        ef_search: 100,
    };
    let mut store = VectorStore::with_meta(embeddings, meta);
    store.set_index_path("/tmp/test_vectorindex_len.idx".to_string());

    // Initially empty
    assert_eq!(store.len(), 0);

    // Add vectors one by one
    for i in 1..=5 {
        store.add(i as i64, vec![0.5; dim])?;
        assert_eq!(store.len(), i);
    }

    Ok(())
}

#[test]
fn test_vectorindex_search_deterministic() -> Result<()> {
    let dim = 64;
    let embeddings = Box::new(RealEmbeddings::new(dim)?);
    let meta = VectorMeta {
        dim,
        m: 32,
        ef_construction: 200,
        ef_search: 100,
    };
    let mut store = VectorStore::with_meta(embeddings, meta);
    store.set_index_path("/tmp/test_vectorindex_determ.idx".to_string());
    store.set_fast_mode(false);

    // Add vectors with identical embeddings (ties in similarity)
    let embedding = vec![0.5; dim];
    store.add(10, embedding.clone())?;
    store.add(5, embedding.clone())?;
    store.add(15, embedding.clone())?;
    store.add(1, embedding.clone())?;

    // Query with identical vector
    let query = vec![0.5; dim];

    // Run search multiple times
    let results1 = VectorIndex::search(&store, &query, 4)?;
    let results2 = VectorIndex::search(&store, &query, 4)?;
    let results3 = VectorIndex::search(&store, &query, 4)?;

    // All searches should return identical results (deterministic)
    assert_eq!(results1, results2);
    assert_eq!(results2, results3);

    // All should have perfect similarity
    for (_, score) in &results1 {
        assert!((score - 1.0).abs() < 0.001);
    }

    // Verify ordering is deterministic (should be by ID for ties)
    let ids: Vec<i64> = results1.iter().map(|(id, _)| *id).collect();
    let mut sorted_ids = ids.clone();
    sorted_ids.sort_unstable();
    assert_eq!(ids, sorted_ids, "IDs should be sorted for deterministic ordering");

    Ok(())
}

#[test]
fn test_vectorindex_search_empty_index() -> Result<()> {
    let dim = 384;
    let embeddings = Box::new(RealEmbeddings::new(dim)?);
    let meta = VectorMeta {
        dim,
        m: 32,
        ef_construction: 200,
        ef_search: 100,
    };
    let store = VectorStore::with_meta(embeddings, meta);

    // Search in empty index
    let query = vec![0.5; dim];
    let results = VectorIndex::search(&store, &query, 10)?;

    // Should return empty results
    assert_eq!(results.len(), 0);

    Ok(())
}

#[test]
fn test_vectorindex_add_dimension_mismatch() -> Result<()> {
    let dim = 128;
    let embeddings = Box::new(RealEmbeddings::new(dim)?);
    let meta = VectorMeta {
        dim,
        m: 32,
        ef_construction: 200,
        ef_search: 100,
    };
    let mut store = VectorStore::with_meta(embeddings, meta);
    store.set_index_path("/tmp/test_vectorindex_mismatch.idx".to_string());

    // Try to add vector with wrong dimension
    let wrong_dim_embedding = vec![0.5; dim + 10];
    let result = store.add(1, wrong_dim_embedding);

    // Should return error
    assert!(result.is_err(), "Adding wrong dimension should fail");

    Ok(())
}

#[test]
fn test_vectorindex_search_respects_k() -> Result<()> {
    let dim = 256;
    let embeddings = Box::new(RealEmbeddings::new(dim)?);
    let meta = VectorMeta {
        dim,
        m: 32,
        ef_construction: 200,
        ef_search: 100,
    };
    let mut store = VectorStore::with_meta(embeddings, meta);
    store.set_index_path("/tmp/test_vectorindex_k.idx".to_string());

    // Add 10 vectors
    for i in 1..=10 {
        store.add(i as i64, vec![i as f32; dim])?;
    }

    // Search with k=3
    let query = vec![5.0; dim];
    let results = VectorIndex::search(&store, &query, 3)?;

    // Should return exactly 3 results
    assert_eq!(results.len(), 3);

    Ok(())
}

#[test]
fn test_vectorindex_normalization() -> Result<()> {
    let dim = 4;
    let embeddings = Box::new(RealEmbeddings::new(dim)?);
    let meta = VectorMeta {
        dim,
        m: 32,
        ef_construction: 200,
        ef_search: 100,
    };
    let mut store = VectorStore::with_meta(embeddings, meta);
    store.set_index_path("/tmp/test_vectorindex_norm.idx".to_string());
    store.set_fast_mode(false);

    // Add unnormalized vector
    let unnormalized = vec![10.0, 0.0, 0.0, 0.0]; // Large magnitude
    store.add(1, unnormalized)?;

    // Add normalized version
    let normalized = vec![1.0, 0.0, 0.0, 0.0];
    store.add(2, normalized)?;

    // Query with normalized vector
    let query = vec![1.0, 0.0, 0.0, 0.0];
    let results = VectorIndex::search(&store, &query, 2)?;

    // Both should have similar scores (cosine similarity normalizes)
    assert_eq!(results.len(), 2);
    assert!(
        (results[0].1 - results[1].1).abs() < 0.01,
        "Normalized and unnormalized should have same cosine similarity"
    );

    Ok(())
}
