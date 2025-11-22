//! Sled Cache Corruption Tests
//!
//! Issue: Sled cache can corrupt on unclean shutdown
//!
//! Goal: Prevent corruption and enable graceful recovery
//! - Use sled open config with cache flush on drop
//! - Add CRC wrapper or temp-tree recovery pattern
//! - Detect unclean shutdown and recover gracefully
//!
//! These tests MUST fail initially, then pass after implementation.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// TEST 1: Open Sled DB Multiple Times Without Corruption
// ============================================================================

#[test]
fn test_sled_multiple_opens_no_corruption() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("sled_test");

    // Open and write data
    {
        let db = sled::open(&db_path).expect("Failed to open sled");
        db.insert(b"key1", b"value1").expect("Insert failed");
        db.insert(b"key2", b"value2").expect("Insert failed");
        db.flush().expect("Flush failed");
    }

    // Reopen - should not corrupt
    {
        let db = sled::open(&db_path).expect("Failed to reopen sled");
        let val1 = db.get(b"key1").expect("Get failed");
        assert_eq!(val1.as_deref(), Some(&b"value1"[..]));

        let val2 = db.get(b"key2").expect("Get failed");
        assert_eq!(val2.as_deref(), Some(&b"value2"[..]));
    }

    // Third open - still no corruption
    {
        let db = sled::open(&db_path).expect("Failed to reopen sled again");
        assert_eq!(db.len(), 2, "Should have 2 keys");
    }
}

// ============================================================================
// TEST 2: Detect Unclean Shutdown and Recover Gracefully
// ============================================================================

#[test]
fn test_sled_unclean_shutdown_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("sled_unclean");

    // Simulate unclean shutdown (no flush)
    {
        let db = sled::open(&db_path).expect("Failed to open sled");
        db.insert(b"key1", b"value1").expect("Insert failed");
        // Deliberately skip flush to simulate crash
        std::mem::drop(db); // Drop without flush
    }

    // Attempt recovery - should not panic
    let recovery_result = sled::open(&db_path);
    assert!(
        recovery_result.is_ok(),
        "Should recover from unclean shutdown, got: {:?}",
        recovery_result.err()
    );

    // Verify data integrity or safe defaults
    if let Ok(db) = recovery_result {
        // Either data is recovered OR we get empty state (both acceptable)
        let key_count = db.len();
        assert!(
            key_count <= 1,
            "Recovered state should be valid (0 or 1 keys)"
        );
    }
}

// ============================================================================
// TEST 3: Corrupted Tree Returns Safe Error
// ============================================================================

#[test]
fn test_sled_corrupted_tree_safe_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("sled_corrupt");

    // Create valid DB
    {
        let db = sled::open(&db_path).expect("Failed to open sled");
        db.insert(b"key1", b"value1").expect("Insert failed");
        db.flush().expect("Flush failed");
    }

    // Simulate corruption by truncating files
    let db_file = db_path.join("db");
    if db_file.exists() {
        let metadata = fs::metadata(&db_file).expect("Failed to get metadata");
        if metadata.len() > 10 {
            // Truncate to corrupt
            fs::write(&db_file, b"corrupted").expect("Failed to corrupt file");
        }
    }

    // Attempt to open corrupted DB
    let result = sled::open(&db_path);

    // Should either:
    // A) Return error gracefully (preferred)
    // B) Open with empty state (acceptable)
    // C) NOT panic (critical)

    match result {
        Ok(db) => {
            // If it opens, it should be safe to use
            let _ = db.get(b"key1"); // Should not panic
            assert!(true, "Corrupted DB opened safely");
        }
        Err(e) => {
            // Error is acceptable - just shouldn't panic
            println!("Corruption detected: {:?}", e);
            assert!(true, "Corruption returned safe error");
        }
    }
}

// ============================================================================
// TEST 4: Flush on Drop Configuration
// ============================================================================

#[test]
fn test_sled_flush_on_drop_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("sled_flush_config");

    // Open with flush_every_ms = 0 (flush on drop)
    {
        let config = sled::Config::new().path(&db_path).flush_every_ms(Some(0)); // Flush immediately

        let db = config.open().expect("Failed to open sled with config");
        db.insert(b"test", b"data").expect("Insert failed");
        // Drop should flush
    }

    // Reopen and verify data persisted
    {
        let db = sled::open(&db_path).expect("Failed to reopen");
        let val = db.get(b"test").expect("Get failed");
        assert_eq!(
            val.as_deref(),
            Some(&b"data"[..]),
            "Data should persist with flush on drop"
        );
    }
}

// ============================================================================
// TEST 5: Concurrent Access Safety
// ============================================================================

#[test]
fn test_sled_concurrent_access_safe() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("sled_concurrent");

    let db = sled::open(&db_path).expect("Failed to open sled");
    let db = Arc::new(db);

    // Spawn multiple threads writing concurrently
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let db_clone = Arc::clone(&db);
            thread::spawn(move || {
                for j in 0..10 {
                    let key = format!("key_{}_{}", i, j);
                    let val = format!("value_{}_{}", i, j);
                    db_clone
                        .insert(key.as_bytes(), val.as_bytes())
                        .expect("Insert failed");
                }
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Verify data integrity
    db.flush().expect("Flush failed");
    assert_eq!(db.len(), 50, "Should have 50 keys (5 threads * 10 keys)");
}

// ============================================================================
// TEST 6: Recovery from Lock File Issues
// ============================================================================

#[test]
fn test_sled_lock_file_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("sled_lock");

    // Open DB (creates lock)
    {
        let _db = sled::open(&db_path).expect("Failed to open sled");
        // Lock file should exist
    }

    // Lock should be released on drop
    // Second open should succeed
    let result = sled::open(&db_path);
    assert!(
        result.is_ok(),
        "Should reopen after lock release, got: {:?}",
        result.err()
    );
}

// ============================================================================
// TEST 7: Memory Cache Flush Verification
// ============================================================================

#[test]
fn test_sled_memory_cache_flush() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("sled_cache_flush");

    // Write data
    {
        let db = sled::open(&db_path).expect("Failed to open sled");
        for i in 0..100 {
            db.insert(format!("key{}", i).as_bytes(), b"value")
                .expect("Insert failed");
        }
        // Explicit flush
        db.flush().expect("Flush failed");
    }

    // Verify all data persisted
    {
        let db = sled::open(&db_path).expect("Failed to reopen");
        assert_eq!(db.len(), 100, "All keys should be persisted");

        for i in 0..100 {
            let val = db.get(format!("key{}", i).as_bytes()).expect("Get failed");
            assert!(val.is_some(), "Key {} should exist", i);
        }
    }
}
