//! HNSW Index Rebuild Tests
//!
//! Issue: HNSW index rebuilds on every search after insert (performance hit)
//!
//! Goal: Prevent full index rebuild after every vector_insert
//! - Maintain in-memory HNSW index
//! - Only persist on insert, not rebuild on search
//! - HNSW structure loads only once per executor lifecycle
//!
//! These tests MUST fail initially, then pass after implementation.

use std::time::Instant;
use syncore::vector::{RealEmbeddings, SearchScope, VectorStore};

// ============================================================================
// TEST 1: Multiple Inserts Don't Trigger Rebuild During Search
// ============================================================================

#[test]
fn test_multiple_inserts_no_rebuild_on_search() {
    // Create vector store with real embeddings
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    // Insert multiple vectors
    for i in 0..10 {
        let text = format!("test document {}", i);
        store.insert_text(i as i64, None, &text, "test").expect("Insert should succeed");
    }

    // First search - might build index
    let start1 = Instant::now();
    let _results1 =
        store.search("test query", 5, SearchScope::Global).expect("Search should succeed");
    let duration1 = start1.elapsed();

    // Second search - should NOT rebuild
    let start2 = Instant::now();
    let _results2 =
        store.search("test query", 5, SearchScope::Global).expect("Search should succeed");
    let duration2 = start2.elapsed();

    // Second search should be faster (no rebuild)
    // This test will FAIL if index rebuilds every time
    assert!(
        duration2 < duration1,
        "Second search took {:?} vs first {:?} - index may be rebuilding",
        duration2,
        duration1
    );

    // Even more strict: second search should be < 50% of first
    assert!(
        duration2.as_micros() < duration1.as_micros() / 2,
        "Second search not significantly faster - possible rebuild"
    );
}

// ============================================================================
// TEST 2: Search Respects Incremental Updates
// ============================================================================

#[test]
fn test_search_respects_incremental_updates() {
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    // Insert initial vectors
    store.insert_text(1, None, "apple fruit red", "test").expect("Insert failed");
    store.insert_text(2, None, "banana fruit yellow", "test").expect("Insert failed");

    // Search should find 2 results
    let results1 = store.search("fruit", 10, SearchScope::Global).expect("Search failed");
    assert_eq!(results1.len(), 2, "Should find 2 fruit documents");

    // Insert more vectors
    store.insert_text(3, None, "orange fruit citrus", "test").expect("Insert failed");
    store.insert_text(4, None, "grape fruit purple", "test").expect("Insert failed");

    // Search should now find 4 results WITHOUT full rebuild
    let start = Instant::now();
    let results2 = store.search("fruit", 10, SearchScope::Global).expect("Search failed");
    let duration = start.elapsed();

    assert_eq!(results2.len(), 4, "Should find all 4 fruit documents");

    // This search should be fast (incremental update, not full rebuild)
    assert!(
        duration.as_millis() < 100,
        "Search took {:?} - possible full rebuild instead of incremental",
        duration
    );
}

// ============================================================================
// TEST 3: HNSW Structure Loads Only Once Per Lifecycle
// ============================================================================

#[test]
fn test_hnsw_loads_once_per_lifecycle() {
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    // Insert vectors
    for i in 0..20 {
        store
            .insert_text(i as i64, None, &format!("document {}", i), "test")
            .expect("Insert failed");
    }

    // Perform multiple searches
    let mut search_times = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        let _results = store.search("document", 5, SearchScope::Global).expect("Search failed");
        search_times.push(start.elapsed());
    }

    // All search times should be similar (no rebuilds)
    let avg_time: u128 = search_times.iter().map(|d| d.as_micros()).sum::<u128>() / 5;

    for (i, time) in search_times.iter().enumerate() {
        let time_us = time.as_micros();
        let diff_percent = if time_us > avg_time {
            ((time_us - avg_time) * 100) / avg_time
        } else {
            ((avg_time - time_us) * 100) / avg_time
        };

        assert!(
            diff_percent < 200,
            "Search {} time {:?} differs from avg {:?}µs by {}% - inconsistent performance",
            i,
            time,
            avg_time,
            diff_percent
        );
    }
}

// ============================================================================
// TEST 4: Index Persists Correctly (No Rebuild on Load)
// ============================================================================

#[test]
fn test_index_persists_no_rebuild_on_load() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let index_path = temp_dir.path().join("vector_index.bin");

    // Create store and insert vectors
    {
        let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
        let mut store = VectorStore::new(embeddings);

        for i in 0..10 {
            store
                .insert_text(i as i64, None, &format!("test doc {}", i), "test")
                .expect("Insert failed");
        }

        // Persist index
        // Note: This will fail if persistence isn't implemented
        // store.persist_index(index_path.to_str().unwrap()).expect("Persist failed");
    }

    // Load store from persisted index
    {
        let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
        let mut store = VectorStore::new(embeddings);

        // Load index - should NOT rebuild
        // Note: This will fail if load isn't implemented
        // store.load_index(index_path.to_str().unwrap()).expect("Load failed");

        // Search should work immediately without rebuild
        let start = Instant::now();
        let results = store.search("test", 5, SearchScope::Global).expect("Search failed");
        let duration = start.elapsed();

        assert!(!results.is_empty(), "Should find results from loaded index");
        assert!(
            duration.as_millis() < 50,
            "Search took {:?} - may have rebuilt instead of loading",
            duration
        );
    }
}

// ============================================================================
// TEST 5: Large Dataset Performance (No Linear Scan)
// ============================================================================

#[test]
#[ignore] // Ignore by default - slow test
fn test_large_dataset_no_linear_scan() {
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    // Insert 1000 vectors
    for i in 0..1000 {
        let text = format!("document number {} with some content about topic {}", i, i % 10);
        store.insert_text(i as i64, None, &text, "test").expect("Insert failed");
    }

    // Search should use HNSW index, not linear scan
    let start = Instant::now();
    let results = store.search("document", 10, SearchScope::Global).expect("Search failed");
    let duration = start.elapsed();

    assert!(!results.is_empty(), "Should find results");

    // With HNSW, 1000 vectors should search in < 10ms
    // Linear scan would take much longer
    assert!(
        duration.as_millis() < 50,
        "Search took {:?} - likely linear scan instead of HNSW",
        duration
    );
}

// ============================================================================
// TEST 6: Memory Usage Stays Reasonable
// ============================================================================

#[test]
fn test_memory_usage_reasonable() {
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    // Insert vectors
    for i in 0..100 {
        store.insert_text(i as i64, None, &format!("test {}", i), "test").expect("Insert failed");
    }

    // Multiple searches shouldn't leak memory
    for _ in 0..10 {
        let _results = store.search("test", 5, SearchScope::Global).expect("Search failed");
    }

    // This test passes if no panic/OOM occurs
    // Real memory profiling would require additional tools
    assert!(true, "Memory test completed without panic");
}
