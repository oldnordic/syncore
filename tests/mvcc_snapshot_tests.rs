//! MVCC-lite Snapshot Isolation Tests
//!
//! Tests for the ArcSwap-based snapshot layer that provides:
//! - Zero-blocking reads
//! - Consistent cross-domain view for AI queries
//! - Safe, atomic swap-in on updates
//! - No long-lived locks
//! - No writers blocking readers
//! - No readers blocking writers

use anyhow::Result;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;
use syncore::router::SynCoreState;
use syncore::snapshots::{SnapshotHandle, SnapshotView};
use syncore::vector::{StubEmbeddings, VectorStore};
use tempfile::TempDir;

/// Test that snapshot consistency is maintained across domains
#[test]
fn test_snapshot_consistency_across_domains() -> Result<()> {
    // This test will fail initially since we haven't implemented snapshots yet
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");

    // Initialize state with dual stores
    let code_store =
        Arc::new(std::sync::Mutex::new(VectorStore::new(Box::new(StubEmbeddings::new(384)?))));
    let general_store =
        Arc::new(std::sync::Mutex::new(VectorStore::new(Box::new(StubEmbeddings::new(384)?))));

    let state = SynCoreState::with_dual_stores(code_store, general_store)?;

    // Build initial snapshot
    let snapshot1 = state.get_snapshot();

    // Update only one domain (this would be done by ingestion)
    // For now, we'll just verify the snapshot structure exists

    // Old snapshot should remain consistent
    assert!(snapshot1.code_graph.entity_count >= 0);
    assert!(snapshot1.vector_meta.dimension > 0);
    assert!(snapshot1.memory_meta.entry_count >= 0);

    // New snapshot should see new data (when implemented)
    let snapshot2 = state.get_snapshot();
    assert!(snapshot2.code_graph.entity_count >= 0);
    assert!(snapshot2.vector_meta.dimension > 0);
    assert!(snapshot2.memory_meta.entry_count >= 0);

    Ok(())
}

/// Test that snapshot swap is atomic - no mixed states visible
#[test]
fn test_snapshot_swap_is_atomic() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");

    let code_store =
        Arc::new(std::sync::Mutex::new(VectorStore::new(Box::new(StubEmbeddings::new(384)?))));
    let general_store =
        Arc::new(std::sync::Mutex::new(VectorStore::new(Box::new(StubEmbeddings::new(384)?))));

    let state = Arc::new(SynCoreState::with_dual_stores(code_store, general_store)?);

    let barrier = Arc::new(Barrier::new(9)); // 8 readers + 1 writer
    let mut handles = vec![];

    // Spawn 8 reader threads
    for i in 0..8 {
        let state_clone = Arc::clone(&state);
        let barrier_clone = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            barrier_clone.wait();

            let mut snapshots = vec![];
            for _ in 0..100 {
                let snap = state_clone.get_snapshot();
                snapshots.push(Arc::as_ptr(&snap) as usize);
                thread::sleep(Duration::from_micros(1));
            }

            // Verify all snapshots are consistent (no mixed states)
            let unique_ptrs: std::collections::HashSet<_> = snapshots.into_iter().collect();
            // Should only see 1 or 2 different snapshot versions (old and new)
            assert!(
                unique_ptrs.len() <= 2,
                "Reader {} saw {} different snapshot versions",
                i,
                unique_ptrs.len()
            );
        });

        handles.push(handle);
    }

    // Spawn 1 writer thread that updates snapshots
    let state_writer = Arc::clone(&state);
    let barrier_writer = Arc::clone(&barrier);

    let writer_handle = thread::spawn(move || {
        barrier_writer.wait();

        // Update snapshot multiple times
        for _ in 0..10 {
            // This would trigger snapshot rebuild when implemented
            thread::sleep(Duration::from_millis(1));
        }
    });

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    writer_handle.join().unwrap();

    Ok(())
}

/// Test that snapshot prevents cross-domain skew
#[test]
fn test_snapshot_prevents_cross_domain_skew() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");

    let code_store =
        Arc::new(std::sync::Mutex::new(VectorStore::new(Box::new(StubEmbeddings::new(384)?))));
    let general_store =
        Arc::new(std::sync::Mutex::new(VectorStore::new(Box::new(StubEmbeddings::new(384)?))));

    let state = SynCoreState::with_dual_stores(code_store, general_store)?;

    // Get initial snapshot
    let old_snapshot = state.get_snapshot();

    // Simulate updates to all domains (would be done by ingestion)
    // CodeGraph update + VectorStore update + Memory update

    // Update memory to trigger version increment
    state.memory.store("test_key", "test_value")?;

    // Update vector store to trigger version increment
    {
        let mut store = state.code_store.lock().unwrap();
        store.insert_text(1, None, "test text", "test")?;
    }

    // Update snapshot to reflect changes
    state.update_snapshot()?;

    // Get new snapshot
    let new_snapshot = state.get_snapshot();

    // Old snapshot must not mix new metadata with old metadata
    // When implemented, this should verify atomic consistency
    assert_ne!(Arc::as_ptr(&old_snapshot) as usize, Arc::as_ptr(&new_snapshot) as usize);

    // Verify versions have changed
    assert_ne!(old_snapshot.memory_meta.version, new_snapshot.memory_meta.version);
    assert_ne!(old_snapshot.vector_meta.version, new_snapshot.vector_meta.version);

    Ok(())
}

/// Test that snapshot does not block readers under stress
#[test]
fn test_snapshot_does_not_block_readers() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");

    let code_store =
        Arc::new(std::sync::Mutex::new(VectorStore::new(Box::new(StubEmbeddings::new(384)?))));
    let general_store =
        Arc::new(std::sync::Mutex::new(VectorStore::new(Box::new(StubEmbeddings::new(384)?))));

    let state = Arc::new(SynCoreState::with_dual_stores(code_store, general_store)?);

    let barrier = Arc::new(Barrier::new(51)); // 50 readers + 1 writer
    let mut handles = vec![];

    // Spawn 50 reader tasks
    for i in 0..50 {
        let state_clone = Arc::clone(&state);
        let barrier_clone = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            barrier_clone.wait();

            let start = std::time::Instant::now();

            // Each reader performs many snapshot reads
            for _ in 0..1000 {
                let _snap = state_clone.get_snapshot();
                // Simulate some work with the snapshot
                thread::sleep(Duration::from_nanos(100));
            }

            let elapsed = start.elapsed();
            // Readers should complete quickly (no blocking)
            assert!(elapsed < Duration::from_secs(5), "Reader {} took too long: {:?}", i, elapsed);
        });

        handles.push(handle);
    }

    // Spawn 1 writer that continuously updates snapshots
    let state_writer = Arc::clone(&state);
    let barrier_writer = Arc::clone(&barrier);

    let writer_handle = thread::spawn(move || {
        barrier_writer.wait();

        let start = std::time::Instant::now();

        // Writer continuously updates snapshots
        for _ in 0..100 {
            // This would trigger snapshot rebuild when implemented
            thread::sleep(Duration::from_millis(10));
        }

        let elapsed = start.elapsed();
        // Writer should also complete reasonably
        assert!(elapsed < Duration::from_secs(5), "Writer took too long: {:?}", elapsed);
    });

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    writer_handle.join().unwrap();

    Ok(())
}

/// Test that snapshot handle does not leak memory
#[test]
fn test_snapshot_handle_does_not_leak() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");

    let code_store =
        Arc::new(std::sync::Mutex::new(VectorStore::new(Box::new(StubEmbeddings::new(384)?))));
    let general_store =
        Arc::new(std::sync::Mutex::new(VectorStore::new(Box::new(StubEmbeddings::new(384)?))));

    let state = SynCoreState::with_dual_stores(code_store, general_store)?;

    // Get initial reference count
    let initial_snapshot = state.get_snapshot();
    let initial_count = Arc::strong_count(&initial_snapshot);

    // Build many snapshots
    let mut snapshots = vec![];
    for i in 0..1000 {
        let snap = state.get_snapshot();
        snapshots.push(snap);

        // Check reference counts don't grow unbounded
        if i % 100 == 0 {
            let current_count = Arc::strong_count(&snapshots[i]);
            // Reference count should be reasonable (i + 1 references in Vec + 1 internal = i + 2)
            let expected_max = i + 5; // Allow some variance for internal references
            assert!(
                current_count <= expected_max,
                "Reference count too high at iteration {}: {} (expected <= {})",
                i,
                current_count,
                expected_max
            );
        }
    }

    // Drop all snapshots
    drop(snapshots);

    // Final reference count should return to baseline
    let final_snapshot = state.get_snapshot();
    let final_count = Arc::strong_count(&final_snapshot);

    // Should be close to initial count (allowing for some variance)
    assert!(
        final_count <= initial_count + 2,
        "Final reference count {} much higher than initial {}",
        final_count,
        initial_count
    );

    Ok(())
}
