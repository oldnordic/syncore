//! TDD Tests for USearch HNSW Integration
//! RED phase: These tests define the expected API for USearchStore

use anyhow::Result;
use syncore::vector::{Hit, SearchScope};

// Import will fail until we implement USearchStore
// use syncore::vector::USearchStore;

#[test]
fn test_usearch_basic_insert_and_search() -> Result<()> {
    // RED: This test will fail until USearchStore is implemented

    // Create USearch-backed store with cosine similarity
    let mut store = syncore::vector::USearchStore::new(384)?;

    // Insert vectors
    store.insert(1, None, &[0.1_f32; 384], "rust programming")?;
    store.insert(2, None, &[0.2_f32; 384], "python scripting")?;
    store.insert(3, None, &[0.15_f32; 384], "rust web development")?;

    // Search should return results sorted by similarity
    let results = store.search(&[0.1_f32; 384], 2)?;

    assert_eq!(results.len(), 2, "Should return top 2 results");
    assert_eq!(results[0].id, 1, "First result should be most similar");
    assert!(results[0].score > 0.9, "Score should be high for exact match");

    Ok(())
}

#[test]
fn test_usearch_persistence() -> Result<()> {
    let temp_path = "/tmp/usearch_test_index";

    // Create and populate index
    {
        let mut store = syncore::vector::USearchStore::new(384)?;
        store.insert(1, None, &[0.1_f32; 384], "test doc 1")?;
        store.insert(2, None, &[0.2_f32; 384], "test doc 2")?;
        store.save(temp_path)?;
    }

    // Load and verify
    {
        let store = syncore::vector::USearchStore::load(temp_path, 384)?;
        assert_eq!(store.len(), 2, "Should have 2 vectors after load");

        let results = store.search(&[0.1_f32; 384], 1)?;
        assert_eq!(results[0].id, 1, "Should find correct vector after load");
    }

    // Cleanup
    let _ = std::fs::remove_file(format!("{}.usearch", temp_path));
    let _ = std::fs::remove_file(format!("{}.meta", temp_path));

    Ok(())
}

#[test]
fn test_usearch_vs_linear_performance() -> Result<()> {
    use std::time::Instant;

    // Use smaller dataset for faster test (index build is O(n log n))
    let num_vectors = 100;
    let dim = 64; // Smaller dimension for faster test

    // Generate random-ish vectors
    let vectors: Vec<Vec<f32>> = (0..num_vectors)
        .map(|i| {
            (0..dim)
                .map(|j| ((i * j) % 100) as f32 / 100.0)
                .collect()
        })
        .collect();

    let query = vec![0.5_f32; dim];

    // USearch with lower ef_construction for faster build
    let mut usearch_store = syncore::vector::USearchStore::with_options(
        dim,
        syncore::vector::USearchOptions {
            metric: syncore::vector::USearchMetric::Cosine,
            connectivity: 16,
            expansion_add: 32, // Lower ef_construction for faster build
            expansion_search: 16,
        },
    )?;

    let build_start = Instant::now();
    for (i, vec) in vectors.iter().enumerate() {
        usearch_store.insert(i as i64, None, vec, &format!("doc {}", i))?;
    }
    let build_time = build_start.elapsed();

    // First search triggers index build
    let search_start = Instant::now();
    let _results = usearch_store.search(&query, 10)?;
    let first_search_time = search_start.elapsed();

    // Subsequent searches should be fast (no rebuild)
    let search_start2 = Instant::now();
    let _results2 = usearch_store.search(&query, 10)?;
    let cached_search_time = search_start2.elapsed();

    println!("Build + Insert time for {} vectors: {:?}", num_vectors, build_time);
    println!("First search (includes index build): {:?}", first_search_time);
    println!("Cached search (no rebuild): {:?}", cached_search_time);

    // Cached search should be very fast (HNSW advantage)
    // Linear search would be O(n), HNSW is O(log n)
    assert!(
        cached_search_time.as_millis() < 50,
        "Cached HNSW search should be fast"
    );

    // Verify we get results
    let results = usearch_store.search(&query, 5)?;
    assert!(!results.is_empty(), "Should return results");
    println!("Top result score: {:.4}", results[0].score);

    Ok(())
}

#[test]
fn test_usearch_with_task_metadata() -> Result<()> {
    let mut store = syncore::vector::USearchStore::new(384)?;

    // Insert with task associations
    store.insert(1, Some(100), &[0.1_f32; 384], "task 100 doc")?;
    store.insert(2, Some(100), &[0.2_f32; 384], "task 100 another")?;
    store.insert(3, Some(200), &[0.15_f32; 384], "task 200 doc")?;

    // Search globally
    let global_results = store.search(&[0.1_f32; 384], 10)?;
    assert_eq!(global_results.len(), 3, "Global search returns all");

    // Search within task scope
    let task_results = store.search_task(&[0.1_f32; 384], 10, 100)?;
    assert_eq!(task_results.len(), 2, "Task search returns only task 100 docs");

    for hit in &task_results {
        assert_eq!(hit.task_id, Some(100), "All results should be from task 100");
    }

    Ok(())
}

#[test]
fn test_usearch_hybrid_store_integration() -> Result<()> {
    // Test that USearchStore can be used as a drop-in replacement
    // via a trait or enum wrapper

    use syncore::vector::{Embeddings, HuggingFaceEmbeddings};

    let embeddings = HuggingFaceEmbeddings::new()?;

    // Create hybrid store that uses USearch internally but provides
    // the same API as VectorStore
    let mut hybrid = syncore::vector::HybridVectorStore::new(
        Box::new(embeddings),
        syncore::vector::VectorBackend::USearch,
    )?;

    // Same API as VectorStore
    hybrid.insert_text(1, None, "rust programming language", "general")?;
    hybrid.insert_text(2, None, "python programming language", "general")?;

    let results = hybrid.search("rust", 2, SearchScope::Global)?;
    assert!(!results.is_empty(), "Should find results");
    assert!(results[0].text.contains("rust"), "Should find rust doc first");

    Ok(())
}

#[test]
fn test_usearch_error_handling() -> Result<()> {
    let mut store = syncore::vector::USearchStore::new(384)?;

    // Wrong dimension should error
    let wrong_dim_vec = vec![0.1_f32; 100]; // Not 384
    let result = store.insert(1, None, &wrong_dim_vec, "wrong dim");
    assert!(result.is_err(), "Should error on wrong dimension");

    // Search on empty index should return empty results, not error
    let results = store.search(&[0.1_f32; 384], 10)?;
    assert_eq!(results.len(), 0, "Empty index returns empty results");

    Ok(())
}

#[test]
fn test_usearch_index_options() -> Result<()> {
    // Test customizable HNSW parameters
    let store = syncore::vector::USearchStore::with_options(
        384,
        syncore::vector::USearchOptions {
            metric: syncore::vector::USearchMetric::Cosine,
            connectivity: 16,      // M parameter
            expansion_add: 128,    // ef_construction
            expansion_search: 64,  // ef
        },
    )?;

    assert_eq!(store.dimensions(), 384);

    Ok(())
}
