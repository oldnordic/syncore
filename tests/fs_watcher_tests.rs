//! APEX 2.2-FW: Filewatcher Tests (TDD-First)
//!
//! Tests for fs_watcher module before implementation exists.
//! Expected to fail until src/fs_watcher/mod.rs is implemented.

use anyhow;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

// These imports will fail until implementation exists
use syncore::fs_watcher::{start_fs_watcher, FsEvent};

// Simple timeout helper for crossbeam channels
fn recv_with_timeout<T>(rx: &crossbeam::channel::Receiver<T>, timeout_dur: Duration) -> Option<T> {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout_dur {
        match rx.try_recv() {
            Ok(val) => return Some(val),
            Err(crossbeam::channel::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(crossbeam::channel::TryRecvError::Disconnected) => return None,
        }
    }
    None
}

// ============================================================================
// TEST 1: Create and Modify Events
// ============================================================================

#[tokio::test]
async fn test_fs_watcher_emits_create_and_modify() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    // Start watcher
    let mut handle = start_fs_watcher(root.clone()).expect("Failed to start watcher");

    // Give watcher time to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create a file
    let test_file = root.join("test.txt");
    fs::write(&test_file, "initial content").expect("Failed to write file");

    // Wait for Create or Modified event
    let event = tokio::task::spawn_blocking({
        let rx = handle.rx.clone();
        move || {
            recv_with_timeout(&rx, Duration::from_secs(2)).ok_or_else(|| anyhow::anyhow!("Timeout"))
        }
    })
    .await
    .unwrap()
    .unwrap();

    assert_eq!(event.path(), &test_file);
    assert!(
        matches!(event, FsEvent::Created(_) | FsEvent::Modified(_)),
        "Expected Created or Modified event, got {:?}",
        event
    );

    // Modify the file
    fs::write(&test_file, "modified content").expect("Failed to modify file");

    // Wait for Modified event
    let event = tokio::task::spawn_blocking({
        let rx = handle.rx.clone();
        move || {
            recv_with_timeout(&rx, Duration::from_secs(2)).ok_or_else(|| anyhow::anyhow!("Timeout"))
        }
    })
    .await
    .unwrap()
    .unwrap();

    assert_eq!(event.path(), &test_file);
    assert!(matches!(event, FsEvent::Modified(_)), "Expected Modified event, got {:?}", event);
}

// ============================================================================
// TEST 2: Debounce Rapid Writes
// ============================================================================

#[tokio::test]
async fn test_fs_watcher_debounces_rapid_writes() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let mut handle = start_fs_watcher(root.clone()).expect("Failed to start watcher");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let test_file = root.join("rapid.txt");

    // Write file multiple times rapidly
    for i in 0..10 {
        fs::write(&test_file, format!("write {}", i)).expect("Failed to write");
        // Small delay to trigger burst
        std::thread::sleep(Duration::from_millis(5));
    }

    // Wait for debounce window
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Collect all events
    let mut event_count = 0;
    loop {
        let rx = handle.rx.clone();
        let result =
            tokio::task::spawn_blocking(move || recv_with_timeout(&rx, Duration::from_millis(100)))
                .await
                .unwrap();

        match result {
            Some(_) => event_count += 1,
            None => break,
        }
    }

    // Should have debounced to at most 2 events (create + 1 batched modify)
    assert!(event_count <= 2, "Expected ≤2 events due to debouncing, got {}", event_count);
}

// ============================================================================
// TEST 3: Ignore Files Outside Root
// ============================================================================

#[tokio::test]
async fn test_fs_watcher_ignores_outside_root() {
    let temp_dir_a = TempDir::new().expect("Failed to create temp dir A");
    let temp_dir_b = TempDir::new().expect("Failed to create temp dir B");

    let root_a = temp_dir_a.path().to_path_buf();
    let root_b = temp_dir_b.path().to_path_buf();

    // Watch dir A only
    let mut handle = start_fs_watcher(root_a.clone()).expect("Failed to start watcher");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Modify file in dir B (not watched)
    let file_b = root_b.join("outside.txt");
    fs::write(&file_b, "content").expect("Failed to write to B");

    // Should not receive any events
    let result = tokio::task::spawn_blocking({
        let rx = handle.rx.clone();
        move || recv_with_timeout(&rx, Duration::from_millis(500))
    })
    .await
    .unwrap();

    assert!(result.is_none(), "Should not receive events for files outside watched root");
}

// ============================================================================
// TEST 4: Handle Delete
// ============================================================================

#[tokio::test]
async fn test_fs_watcher_handles_delete() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let mut handle = start_fs_watcher(root.clone()).expect("Failed to start watcher");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create file
    let test_file = root.join("deleteme.txt");
    fs::write(&test_file, "content").expect("Failed to write file");

    // Wait for create event
    tokio::task::spawn_blocking({
        let rx = handle.rx.clone();
        move || {
            recv_with_timeout(&rx, Duration::from_secs(2)).ok_or_else(|| anyhow::anyhow!("Timeout"))
        }
    })
    .await
    .unwrap()
    .unwrap();

    // Delete file
    fs::remove_file(&test_file).expect("Failed to delete file");

    // Wait for Removed event
    let event = tokio::task::spawn_blocking({
        let rx = handle.rx.clone();
        move || {
            recv_with_timeout(&rx, Duration::from_secs(2)).ok_or_else(|| anyhow::anyhow!("Timeout"))
        }
    })
    .await
    .unwrap()
    .unwrap();

    assert_eq!(event.path(), &test_file);
    assert!(matches!(event, FsEvent::Removed(_)), "Expected Removed event, got {:?}", event);
}

// ============================================================================
// TEST 5: Clean Shutdown
// ============================================================================

#[tokio::test]
async fn test_fs_watcher_shutdown_cleanly() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let handle = start_fs_watcher(root.clone()).expect("Failed to start watcher");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Drop handle explicitly
    drop(handle);

    // Small delay to ensure cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // If we reach here without panic/deadlock, test passes
    assert!(true, "Watcher shut down cleanly");
}
