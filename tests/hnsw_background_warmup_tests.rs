//! TDD Tests for HNSW Background Warmup
//!
//! Tests the Pattern A: Background HNSW Warmup implementation:
//! - HNSW ready flag starts false
//! - HNSW ready flag becomes true after warmup
//! - Brute-force fallback when HNSW not ready
//! - Pending vector queue during warmup
//! - Queue flush when HNSW becomes ready

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use syncore::vector::{RealEmbeddings, SearchScope, VectorStore};

/// Test that HNSW ready flag is initially false
#[test]
fn test_hnsw_flag_initially_false() {
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let store = VectorStore::new(embeddings);

    // HNSW should NOT be ready initially (requires warmup)
    assert!(
        !store.is_hnsw_ready(),
        "HNSW ready flag should be false initially"
    );
}

/// Test that HNSW ready flag can be set to true
#[test]
fn test_hnsw_flag_becomes_true_after_warmup() {
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let store = VectorStore::new(embeddings);

    // Initially false
    assert!(!store.is_hnsw_ready());

    // Simulate warmup completion
    store.set_hnsw_ready(true);

    // Now should be true
    assert!(
        store.is_hnsw_ready(),
        "HNSW ready flag should be true after set_hnsw_ready(true)"
    );
}

/// Test that search works with brute-force fallback when HNSW not ready
#[test]
fn test_fallback_bruteforce_when_not_ready() {
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let mut store = VectorStore::new(embeddings);

    // HNSW not ready (default state)
    assert!(!store.is_hnsw_ready());

    // Insert some vectors
    store
        .insert_text(1, None, "cat sitting on mat", "test")
        .unwrap();
    store
        .insert_text(2, None, "dog running in park", "test")
        .unwrap();
    store
        .insert_text(3, None, "car driving on road", "test")
        .unwrap();

    // Search should still work via brute-force fallback
    let results = store.search("cat on mat", 2, SearchScope::Global).unwrap();

    // Should get results even with HNSW not ready
    assert!(
        !results.is_empty(),
        "Search should work with brute-force fallback"
    );
    assert!(results.len() <= 2, "Should respect k limit");
}

/// Test that vectors are queued when HNSW not ready
#[test]
fn test_insert_queue_when_not_ready() {
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let mut store = VectorStore::new(embeddings);

    // HNSW not ready
    assert!(!store.is_hnsw_ready());

    // Insert vectors - they should be queued
    store
        .insert_text(1, None, "test vector one", "test")
        .unwrap();
    store
        .insert_text(2, None, "test vector two", "test")
        .unwrap();

    // Vectors should be in the main list
    assert_eq!(store.len(), 2, "Vectors should be stored in main list");

    // Search should still work (via brute-force)
    let results = store.search("test vector", 5, SearchScope::Global).unwrap();
    assert!(
        !results.is_empty(),
        "Search should work with queued vectors"
    );
}

/// Test that pending vectors are flushed when HNSW becomes ready
#[test]
fn test_queue_flush_when_ready() {
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let mut store = VectorStore::new(embeddings);

    // HNSW not ready initially
    assert!(!store.is_hnsw_ready());

    // Insert vectors while HNSW not ready (queued)
    store
        .insert_text(1, None, "queued vector one", "test")
        .unwrap();
    store
        .insert_text(2, None, "queued vector two", "test")
        .unwrap();

    // Mark HNSW as ready and flush
    store.set_hnsw_ready(true);
    let flushed = store.flush_pending_vectors().unwrap();

    // Should have flushed 2 vectors
    assert_eq!(flushed, 2, "Should flush 2 pending vectors");

    // Subsequent flush should return 0
    let flushed_again = store.flush_pending_vectors().unwrap();
    assert_eq!(
        flushed_again, 0,
        "Second flush should return 0 (queue empty)"
    );
}

/// Test that HNSW ready flag is shared correctly via Arc
#[test]
fn test_hnsw_ready_flag_shared() {
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let store = VectorStore::new(embeddings);

    // Get the shared flag
    let flag = store.hnsw_ready_flag();

    // Initially false
    assert!(!flag.load(Ordering::SeqCst));

    // Set via store method
    store.set_hnsw_ready(true);

    // Shared flag should reflect the change
    assert!(
        flag.load(Ordering::SeqCst),
        "Shared flag should reflect store's hnsw_ready state"
    );
}

/// Test search works correctly after HNSW becomes ready
#[test]
fn test_search_after_warmup() {
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let mut store = VectorStore::new(embeddings);

    // Insert vectors while HNSW not ready
    store
        .insert_text(1, None, "machine learning algorithm", "test")
        .unwrap();
    store
        .insert_text(2, None, "deep neural network", "test")
        .unwrap();
    store
        .insert_text(3, None, "natural language processing", "test")
        .unwrap();

    // Mark HNSW ready and flush
    store.set_hnsw_ready(true);
    store.flush_pending_vectors().unwrap();

    // Search should now use HNSW (though behavior is same, internal path differs)
    let results = store
        .search("neural network", 2, SearchScope::Global)
        .unwrap();

    assert!(!results.is_empty(), "Search should work after HNSW warmup");
    assert!(results.len() <= 2, "Should respect k limit");
}

/// Integration test: VectorStore wrapped in Arc<Mutex> (as used in production)
#[test]
fn test_vectorstore_in_arc_mutex() {
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Get lock and check initial state
    {
        let vs = store.lock().unwrap();
        assert!(!vs.is_hnsw_ready());
    }

    // Insert vectors
    {
        let mut vs = store.lock().unwrap();
        vs.insert_text(1, None, "test data", "test").unwrap();
    }

    // Set ready and flush
    {
        let mut vs = store.lock().unwrap();
        vs.set_hnsw_ready(true);
        let flushed = vs.flush_pending_vectors().unwrap();
        assert_eq!(flushed, 1);
    }

    // Search
    {
        let vs = store.lock().unwrap();
        let results = vs.search("test", 5, SearchScope::Global).unwrap();
        assert!(!results.is_empty());
    }
}
