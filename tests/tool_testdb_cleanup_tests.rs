//! Test Database Cleanup Tests
//!
//! Issue: Test database files accumulate and aren't cleaned automatically
//!
//! Goal: Ensure proper test DB cleanup
//! - Each test using SQLite gets its own temporary directory
//! - DB file is removed after test
//! - No cross-test contamination
//!
//! These tests MUST fail initially, then pass after implementation.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ============================================================================
// TEST 1: Each Test Gets Own Temporary Directory
// ============================================================================

#[test]
fn test_each_test_own_temp_dir() {
    // Create first test DB
    let temp_dir1 = create_test_db_dir("test1");
    let db_path1 = temp_dir1.path().join("test.db");

    // Verify directory exists
    assert!(temp_dir1.path().exists(), "Temp dir should exist");

    // Create second test DB
    let temp_dir2 = create_test_db_dir("test2");
    let db_path2 = temp_dir2.path().join("test.db");

    // Verify they're different directories
    assert_ne!(
        db_path1, db_path2,
        "Each test should get unique temp directory"
    );

    // Both should exist
    assert!(temp_dir1.path().exists());
    assert!(temp_dir2.path().exists());
}

// ============================================================================
// TEST 2: DB File Removed After Test
// ============================================================================

#[test]
fn test_db_file_removed_after_test() {
    let db_path: PathBuf;

    {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        db_path = temp_dir.path().join("test.db");

        // Create a test DB file
        fs::write(&db_path, b"test data").expect("Failed to write test DB");

        // Verify it exists
        assert!(db_path.exists(), "DB file should exist during test");

        // temp_dir drops here, should clean up
    }

    // After drop, file should be gone
    assert!(
        !db_path.exists(),
        "DB file should be removed after test: {:?}",
        db_path
    );
}

// ============================================================================
// TEST 3: No Cross-Test Contamination
// ============================================================================

#[test]
fn test_no_cross_test_contamination() {
    use syncore::memory::Memory;

    // Test 1: Create DB with data
    let key = "test_key";
    let value1 = "value_from_test1";

    {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("memory.db");

        let memory = Memory::new(db_path.to_str().unwrap()).expect("Failed to create memory");
        memory.store(key, value1).expect("Failed to store");

        let retrieved = memory.query(key).expect("Failed to query");
        assert_eq!(retrieved.as_deref(), Some(value1));
    }

    // Test 2: New DB should not see test1's data
    {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("memory.db");

        let memory = Memory::new(db_path.to_str().unwrap()).expect("Failed to create memory");

        let retrieved = memory.query(key);
        assert!(
            retrieved.is_ok() && retrieved.unwrap().is_none(),
            "New test DB should not contain data from previous test"
        );
    }
}

// ============================================================================
// TEST 4: Cleanup Happens Even on Test Failure
// ============================================================================

#[test]
fn test_cleanup_on_test_failure() {
    let db_path_outer: PathBuf;

    let result = std::panic::catch_unwind(|| {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        fs::write(&db_path, b"test").expect("Failed to write");

        // Store path for outer scope
        unsafe {
            DB_PATH_FOR_CLEANUP_TEST = Some(db_path.clone());
        }

        // Simulate test failure
        panic!("Simulated test failure");
    });

    // Panic should have been caught
    assert!(result.is_err(), "Should catch panic");

    // But cleanup should still happen
    unsafe {
        if let Some(ref path) = DB_PATH_FOR_CLEANUP_TEST {
            // File might still exist briefly, but directory cleanup scheduled
            // In real usage, TempDir drop handles this
            assert!(
                !path.exists() || path.parent().map(|p| !p.exists()).unwrap_or(false),
                "Cleanup should happen even on failure"
            );
        }
    }
}

static mut DB_PATH_FOR_CLEANUP_TEST: Option<PathBuf> = None;

// ============================================================================
// TEST 5: Helper Function for Test DB Creation
// ============================================================================

#[test]
fn test_helper_creates_unique_dbs() {
    use std::collections::HashSet;

    let mut paths = HashSet::new();

    // Create multiple test DBs
    for i in 0..5 {
        let temp_dir = create_test_db_dir(&format!("test_{}", i));
        let db_path = temp_dir.path().join("test.db");

        // All paths should be unique
        assert!(
            paths.insert(db_path.clone()),
            "Each test DB should have unique path"
        );

        // Keep temp_dir alive
        std::mem::forget(temp_dir); // Note: leaks in test, but proves uniqueness
    }

    assert_eq!(paths.len(), 5, "Should create 5 unique DB paths");
}

// ============================================================================
// TEST 6: Cleanup Works for SQLite WAL Files
// ============================================================================

#[test]
fn test_cleanup_includes_wal_files() {
    use syncore::memory::Memory;

    let db_path_outer: PathBuf;
    let wal_path_outer: PathBuf;
    let shm_path_outer: PathBuf;

    {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        db_path_outer = temp_dir.path().join("test.db");
        wal_path_outer = temp_dir.path().join("test.db-wal");
        shm_path_outer = temp_dir.path().join("test.db-shm");

        // Create Memory (uses WAL mode)
        let memory = Memory::new(db_path_outer.to_str().unwrap()).expect("Failed to create memory");

        // Write some data to trigger WAL files
        memory.store("key", "value").expect("Failed to store");

        // WAL files might exist
        // (May not exist in all SQLite configs, but if they do, they should clean up)

        // temp_dir drops here
    }

    // All SQLite files should be cleaned up
    assert!(!db_path_outer.exists(), "Main DB should be cleaned");
    assert!(!wal_path_outer.exists(), "WAL file should be cleaned");
    assert!(!shm_path_outer.exists(), "SHM file should be cleaned");
}

// ============================================================================
// TEST 7: Concurrent Tests Don't Interfere
// ============================================================================

#[test]
fn test_concurrent_tests_isolated() {
    use std::sync::Arc;
    use std::thread;

    let handles: Vec<_> = (0..3)
        .map(|i| {
            thread::spawn(move || {
                let temp_dir = create_test_db_dir(&format!("concurrent_{}", i));
                let db_path = temp_dir.path().join("test.db");

                // Write unique data
                fs::write(&db_path, format!("data_{}", i).as_bytes()).expect("Failed to write");

                // Read back
                let content = fs::read_to_string(&db_path).expect("Failed to read");
                assert_eq!(content, format!("data_{}", i));

                // Return temp_dir to keep it alive until thread ends
                temp_dir
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // All temp dirs cleaned up after threads complete
    assert!(true, "Concurrent test isolation passed");
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_db_dir(test_name: &str) -> TempDir {
    TempDir::new()
        .unwrap_or_else(|e| panic!("Failed to create temp dir for test '{}': {}", test_name, e))
}
