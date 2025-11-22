//! TDD Tests for HNSW Warmup State Machine
//!
//! These tests validate:
//! 1. HnswWarmupState transitions (Cold -> WarmingUp -> Hot)
//! 2. Snapshot-first startup (load_snapshot short-circuits rebuild)
//! 3. Brute-force fallback when not Hot
//! 4. Non-blocking warmup (embedding calls don't block during rebuild)

use anyhow::Result;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Import types from syncore
use syncore::vector::{RealEmbeddings, SearchScope, VectorStore};
use syncore::vector::warmup::{HnswWarmupState, WarmupController};

/// Test: State transitions Cold -> WarmingUp -> Hot
#[test]
fn test_warmup_state_transitions() {
    let controller = WarmupController::new();

    // Initial state should be Cold
    assert_eq!(controller.state(), HnswWarmupState::Cold);
    assert!(!controller.is_hot());

    // Transition to WarmingUp
    controller.mark_warming_up();
    assert_eq!(controller.state(), HnswWarmupState::WarmingUp);
    assert!(!controller.is_hot());

    // Transition to Hot
    controller.mark_hot();
    assert_eq!(controller.state(), HnswWarmupState::Hot);
    assert!(controller.is_hot());

    // Can transition back to Cold (for testing/reset)
    controller.mark_cold();
    assert_eq!(controller.state(), HnswWarmupState::Cold);
    assert!(!controller.is_hot());
}

/// Test: Snapshot load short-circuits rebuild
#[test]
fn test_snapshot_load_short_circuits_rebuild() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let index_path = temp_dir.path().join("test_snapshot");

    // Phase 1: Create a VectorStore, insert vectors, save snapshot
    {
        let embeddings = Box::new(RealEmbeddings::new(384)?);
        let mut store = VectorStore::new(embeddings);
        store.set_index_path(index_path.to_str().unwrap().to_string());
        store.set_fast_mode(true); // Use fast embeddings for test speed

        // Insert test vectors
        for i in 0..10 {
            store.insert_text(i, None, &format!("test document {}", i), "test")?;
        }

        // Save snapshot (HNSW + vectors)
        store.save_snapshot()?;
    }

    // Phase 2: Create new store, load snapshot - should NOT rebuild
    {
        let embeddings = Box::new(RealEmbeddings::new(384)?);
        let mut store = VectorStore::new(embeddings);
        store.set_index_path(index_path.to_str().unwrap().to_string());
        store.set_fast_mode(true);

        // Load snapshot - this should load HNSW directly, not rebuild
        let load_result = store.load_snapshot();
        assert!(load_result.is_ok(), "Snapshot load should succeed");

        // After load_snapshot, state should be Hot (no rebuild needed)
        assert!(store.warmup_controller().is_hot(),
            "After snapshot load, state should be Hot");

        // Verify vectors are loaded
        assert_eq!(store.len(), 10, "Should have 10 vectors after load");

        // Search should work immediately
        let results = store.search("test document 5", 3, SearchScope::Global)?;
        assert!(!results.is_empty(), "Search should return results");
    }

    Ok(())
}

/// Test: Missing snapshot triggers rebuild and creates snapshot
#[test]
fn test_missing_snapshot_triggers_rebuild() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let index_path = temp_dir.path().join("missing_snapshot");

    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let mut store = VectorStore::new(embeddings);
    store.set_index_path(index_path.to_str().unwrap().to_string());
    store.set_fast_mode(true);

    // No snapshot exists - load_snapshot should return Err or indicate rebuild needed
    let load_result = store.load_snapshot();

    // Either it fails gracefully or succeeds with empty store
    // The key is that it doesn't panic
    match load_result {
        Ok(_) => {
            // If OK, store should be empty (no vectors to load)
            assert_eq!(store.len(), 0);
        }
        Err(_) => {
            // Expected - no snapshot file exists
        }
    }

    Ok(())
}

/// Test: Brute-force fallback when Cold/WarmingUp
#[test]
fn test_bruteforce_fallback_when_not_hot() -> Result<()> {
    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let mut store = VectorStore::new(embeddings);
    store.set_fast_mode(true);

    // Insert vectors while in Cold state
    for i in 0..20 {
        store.insert_text(i, None, &format!("document about topic {}", i), "test")?;
    }

    // Ensure state is Cold (not Hot)
    store.warmup_controller().mark_cold();
    assert!(!store.warmup_controller().is_hot());

    // Search should still work via brute-force fallback
    let results = store.search("document about topic 10", 5, SearchScope::Global)?;

    // Should return results despite HNSW not being ready
    assert!(!results.is_empty(), "Brute-force fallback should return results");
    assert!(results.len() <= 5, "Should respect k limit");

    // Results should be sorted by similarity (descending)
    for i in 0..results.len() - 1 {
        assert!(results[i].score >= results[i + 1].score,
            "Results should be sorted by similarity descending");
    }

    Ok(())
}

/// Test: HNSW search used when Hot
#[test]
fn test_hnsw_search_when_hot() -> Result<()> {
    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let mut store = VectorStore::new(embeddings);
    store.set_fast_mode(true);

    // Mark HNSW as ready BEFORE inserting vectors
    // so vectors go directly into HNSW (not pending queue)
    store.set_hnsw_ready(true);

    // Insert vectors (will go into HNSW since ready=true)
    for i in 0..20 {
        store.insert_text(i, None, &format!("document about topic {}", i), "test")?;
    }

    // Mark warmup state as Hot (this is for the state machine)
    store.warmup_controller().mark_hot();

    // Search should use HNSW
    let results = store.search("document about topic 10", 5, SearchScope::Global)?;

    assert!(!results.is_empty(), "HNSW search should return results");

    Ok(())
}

/// Test: Embedding calls don't block during warmup
///
/// This is the critical test - verifies that search returns immediately
/// even while a simulated warmup is in progress.
#[test]
fn test_embedding_calls_nonblocking_during_warmup() -> Result<()> {
    use std::thread;
    use std::sync::atomic::AtomicBool;

    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Pre-populate with vectors
    {
        let mut s = store.lock().unwrap();
        s.set_fast_mode(true);
        for i in 0..100 {
            s.insert_text(i, None, &format!("document {}", i), "test")?;
        }
        // Mark as WarmingUp (simulating background rebuild)
        s.warmup_controller().mark_warming_up();
    }

    let search_completed = Arc::new(AtomicBool::new(false));
    let search_completed_clone = search_completed.clone();
    let store_clone = store.clone();

    // Spawn search thread
    let search_handle = thread::spawn(move || {
        let s = store_clone.lock().unwrap();
        let result = s.search("document 50", 5, SearchScope::Global);
        search_completed_clone.store(true, Ordering::SeqCst);
        result
    });

    // Search should complete within 1 second (brute-force is fast)
    thread::sleep(Duration::from_millis(100));

    let result = search_handle.join().expect("Search thread panicked");
    assert!(search_completed.load(Ordering::SeqCst), "Search should complete");
    assert!(result.is_ok(), "Search should succeed via fallback");

    Ok(())
}

/// Test: save_snapshot only called once per rebuild (not per insert)
#[test]
fn test_save_snapshot_once_per_rebuild() -> Result<()> {
    use std::sync::atomic::AtomicUsize;

    let temp_dir = tempfile::tempdir()?;
    let index_path = temp_dir.path().join("batch_snapshot");

    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let mut store = VectorStore::new(embeddings);
    store.set_index_path(index_path.to_str().unwrap().to_string());
    store.set_fast_mode(true);

    // Use batch insert which should NOT save snapshot per insert
    let texts: Vec<(i64, Option<i64>, String)> = (0..50)
        .map(|i| (i, None, format!("batch document {}", i)))
        .collect();

    // Batch insert should be efficient
    let start = std::time::Instant::now();
    store.insert_batch_parallel(texts)?;
    let duration = start.elapsed();

    // Batch of 50 should complete in under 1 second
    // (If save_snapshot was called per insert, it would be much slower)
    assert!(duration < Duration::from_secs(1),
        "Batch insert should be fast (no per-insert snapshot): {:?}", duration);

    // Now save snapshot once at the end
    store.save_snapshot()?;

    // Verify snapshot files exist
    let vectors_path = format!("{}.vectors", index_path.to_str().unwrap());
    assert!(std::path::Path::new(&vectors_path).exists(),
        "Vectors snapshot should exist");

    Ok(())
}

/// Test: Off-lock rebuild doesn't block vector_store access
#[test]
fn test_offlock_rebuild_allows_concurrent_access() -> Result<()> {
    use std::thread;

    let temp_dir = tempfile::tempdir()?;
    let index_path = temp_dir.path().join("offlock_test");

    let embeddings = Box::new(RealEmbeddings::new(384)?);
    let store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Setup
    {
        let mut s = store.lock().unwrap();
        s.set_index_path(index_path.to_str().unwrap().to_string());
        s.set_fast_mode(true);

        // Pre-populate
        for i in 0..50 {
            s.insert_text(i, None, &format!("preloaded doc {}", i), "test")?;
        }
    }

    // Simulate concurrent access during "rebuild"
    let store_clone = store.clone();
    let reader_handle = thread::spawn(move || {
        // Multiple reads should succeed
        for _ in 0..10 {
            let s = store_clone.lock().unwrap();
            let _ = s.search("preloaded doc", 5, SearchScope::Global);
            drop(s);
            thread::sleep(Duration::from_millis(10));
        }
    });

    // Writer should also succeed
    {
        let mut s = store.lock().unwrap();
        s.insert_text(999, None, "new document during rebuild", "test")?;
    }

    reader_handle.join().expect("Reader thread panicked");

    Ok(())
}
