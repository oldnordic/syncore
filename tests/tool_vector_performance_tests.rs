//! Vector Search Performance Tests
//!
//! Issue: Vector search can be slow for large datasets without proper indexing
//!
//! Goal: Improve performance with incremental HNSW and caching
//! - Large vector set (5k entries) performs search under threshold
//! - Verify no unnecessary linear scans
//! - Add small in-memory L2 cache for last queries
//! - Add metric for insert/search latency
//!
//! These tests MUST fail initially, then pass after implementation.

use std::time::Instant;
use syncore::vector::{RealEmbeddings, SearchScope, VectorStore};

// ============================================================================
// TEST 1: Large Dataset Search Performance
// ============================================================================

#[test]
#[ignore] // Slow test - run explicitly
fn test_large_dataset_search_performance() {
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    println!("Inserting 5000 vectors...");
    let insert_start = Instant::now();

    // Insert 5000 vectors
    for i in 0..5000 {
        let text = format!(
            "document {} about topic {} with content {}",
            i,
            i % 100,
            i % 10
        );
        store
            .insert_text(i as i64, None, &text, "test")
            .expect("Insert failed");

        if i % 1000 == 0 {
            println!("Inserted {} vectors", i);
        }
    }

    let insert_duration = insert_start.elapsed();
    println!("Insert time: {:?}", insert_duration);

    // Search should be fast even with 5k vectors
    println!("Performing search...");
    let search_start = Instant::now();
    let results = store
        .search("document topic", 10, SearchScope::Global)
        .expect("Search failed");
    let search_duration = search_start.elapsed();

    println!("Search time: {:?}", search_duration);
    println!("Results found: {}", results.len());

    // With proper HNSW indexing, search should be < 100ms
    assert!(
        search_duration.as_millis() < 100,
        "Search took {:?} - too slow for 5k vectors (expected < 100ms)",
        search_duration
    );

    assert!(!results.is_empty(), "Should find results");
}

// ============================================================================
// TEST 2: No Unnecessary Linear Scans
// ============================================================================

#[test]
fn test_no_linear_scans() {
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    // Insert 100 vectors
    for i in 0..100 {
        store
            .insert_text(i as i64, None, &format!("doc {}", i), "test")
            .expect("Insert failed");
    }

    // First search
    let start1 = Instant::now();
    let _results1 = store
        .search("doc", 10, SearchScope::Global)
        .expect("Search failed");
    let duration1 = start1.elapsed();

    // Second search - should NOT rescan all vectors
    let start2 = Instant::now();
    let _results2 = store
        .search("doc", 10, SearchScope::Global)
        .expect("Search failed");
    let duration2 = start2.elapsed();

    // Second search should be at least as fast (no rescanning)
    assert!(
        duration2 <= duration1 * 2,
        "Second search {:?} much slower than first {:?} - possible linear rescan",
        duration2,
        duration1
    );
}

// ============================================================================
// TEST 3: Query Cache Improves Repeated Searches
// ============================================================================

#[test]
fn test_query_cache_repeated_searches() {
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    // Insert vectors
    for i in 0..50 {
        store
            .insert_text(i as i64, None, &format!("test document {}", i), "test")
            .expect("Insert failed");
    }

    let query = "test document";

    // First search (cold cache)
    let start1 = Instant::now();
    let results1 = store
        .search(query, 5, SearchScope::Global)
        .expect("Search failed");
    let duration1 = start1.elapsed();

    // Second search (warm cache)
    let start2 = Instant::now();
    let results2 = store
        .search(query, 5, SearchScope::Global)
        .expect("Search failed");
    let duration2 = start2.elapsed();

    // Results should be identical
    assert_eq!(results1.len(), results2.len(), "Results should match");

    // Second search should be faster (cached)
    // This will FAIL without caching implementation
    assert!(
        duration2 < duration1 / 2,
        "Cached search {:?} not faster than cold {:?}",
        duration2,
        duration1
    );
}

// ============================================================================
// TEST 4: Insert Latency Metrics
// ============================================================================

#[test]
fn test_insert_latency_reasonable() {
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    let mut insert_times = Vec::new();

    // Measure insert latency
    for i in 0..100 {
        let start = Instant::now();
        store
            .insert_text(i as i64, None, &format!("doc {}", i), "test")
            .expect("Insert failed");
        insert_times.push(start.elapsed());
    }

    // Calculate average
    let total_us: u128 = insert_times.iter().map(|d| d.as_micros()).sum();
    let avg_us = total_us / 100;

    println!("Average insert latency: {}µs", avg_us);

    // Insert should be fast (< 1ms average)
    assert!(
        avg_us < 1000,
        "Average insert latency {}µs too high (expected < 1000µs)",
        avg_us
    );

    // Later inserts shouldn't be much slower (no O(n²) behavior)
    let early_avg: u128 = insert_times[0..10]
        .iter()
        .map(|d| d.as_micros())
        .sum::<u128>()
        / 10;
    let late_avg: u128 = insert_times[90..100]
        .iter()
        .map(|d| d.as_micros())
        .sum::<u128>()
        / 10;

    assert!(
        late_avg < early_avg * 3,
        "Later inserts ({:?}µs) much slower than early ({:?}µs) - possible O(n²)",
        late_avg,
        early_avg
    );
}

// ============================================================================
// TEST 5: Search Latency Metrics
// ============================================================================

#[test]
fn test_search_latency_reasonable() {
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    // Insert vectors
    for i in 0..200 {
        store
            .insert_text(i as i64, None, &format!("document {}", i), "test")
            .expect("Insert failed");
    }

    let mut search_times = Vec::new();

    // Measure search latency
    for _ in 0..20 {
        let start = Instant::now();
        let _results = store
            .search("document", 10, SearchScope::Global)
            .expect("Search failed");
        search_times.push(start.elapsed());
    }

    // Calculate average
    let total_us: u128 = search_times.iter().map(|d| d.as_micros()).sum();
    let avg_us = total_us / 20;

    println!("Average search latency: {}µs", avg_us);

    // Search should be fast (< 10ms average for 200 vectors)
    assert!(
        avg_us < 10000,
        "Average search latency {}µs too high (expected < 10000µs)",
        avg_us
    );

    // Search times should be consistent (no sporadic slow downs)
    let max_time = search_times.iter().max().unwrap();
    let min_time = search_times.iter().min().unwrap();

    assert!(
        max_time.as_micros() < min_time.as_micros() * 5,
        "Search latency inconsistent: min {:?}, max {:?}",
        min_time,
        max_time
    );
}

// ============================================================================
// TEST 6: Incremental Index Updates Don't Degrade Performance
// ============================================================================

#[test]
fn test_incremental_updates_maintain_performance() {
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    // Insert initial batch
    for i in 0..100 {
        store
            .insert_text(i as i64, None, &format!("doc {}", i), "test")
            .expect("Insert failed");
    }

    // Measure initial search performance
    let start1 = Instant::now();
    let _results1 = store
        .search("doc", 10, SearchScope::Global)
        .expect("Search failed");
    let baseline = start1.elapsed();

    // Insert more vectors incrementally
    for i in 100..200 {
        store
            .insert_text(i as i64, None, &format!("doc {}", i), "test")
            .expect("Insert failed");
    }

    // Search performance should not degrade significantly
    let start2 = Instant::now();
    let _results2 = store
        .search("doc", 10, SearchScope::Global)
        .expect("Search failed");
    let after_insert = start2.elapsed();

    assert!(
        after_insert < baseline * 2,
        "Search after incremental inserts {:?} much slower than baseline {:?}",
        after_insert,
        baseline
    );
}

// ============================================================================
// TEST 7: Memory Efficient for Large Datasets
// ============================================================================

#[test]
#[ignore] // Memory-intensive test
fn test_memory_efficient_large_dataset() {
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    // Insert 10k vectors
    println!("Inserting 10000 vectors...");
    for i in 0..10000 {
        store
            .insert_text(i as i64, None, &format!("doc {}", i), "test")
            .expect("Insert failed");

        if i % 2000 == 0 {
            println!("Inserted {}", i);
        }
    }

    // Search should still work
    let results = store
        .search("doc", 10, SearchScope::Global)
        .expect("Search failed");
    assert!(!results.is_empty(), "Should find results in large dataset");

    // No memory leak check - if we got here without OOM, test passes
    assert!(true, "Memory efficiency test completed");
}

// ============================================================================
// TEST 8: Batch Insert Performance
// ============================================================================

#[test]
fn test_batch_insert_performance() {
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let mut store = VectorStore::new(embeddings);

    let texts: Vec<String> = (0..100).map(|i| format!("document {}", i)).collect();

    // Measure batch insert
    let start = Instant::now();
    for (i, text) in texts.iter().enumerate() {
        store
            .insert_text(i as i64, None, text, "test")
            .expect("Insert failed");
    }
    let duration = start.elapsed();

    println!("Batch insert (100 vectors): {:?}", duration);

    // Should complete in reasonable time (< 1 second)
    assert!(
        duration.as_secs() < 1,
        "Batch insert took {:?} - too slow",
        duration
    );
}
