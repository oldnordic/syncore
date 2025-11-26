//! APEX 2.2-FW: Filewatcher Tests (TDD-First)
//!
//! Tests for fs_watcher module before implementation exists.
//! Expected to fail until src/fs_watcher/mod.rs is implemented.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

// These imports will fail until implementation exists
use syncore::fs_watcher::{start_fs_watcher, FsEvent, FsEventKind};

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
    let event = timeout(Duration::from_secs(2), handle.rx.recv())
        .await
        .expect("Timeout waiting for create event")
        .expect("Channel closed");

    assert_eq!(event.path, test_file);
    assert!(
        matches!(event.kind, FsEventKind::Created | FsEventKind::Modified),
        "Expected Created or Modified event, got {:?}",
        event.kind
    );

    // Modify the file
    fs::write(&test_file, "modified content").expect("Failed to modify file");

    // Wait for Modified event
    let event = timeout(Duration::from_secs(2), handle.rx.recv())
        .await
        .expect("Timeout waiting for modify event")
        .expect("Channel closed");

    assert_eq!(event.path, test_file);
    assert!(
        matches!(event.kind, FsEventKind::Modified),
        "Expected Modified event, got {:?}",
        event.kind
    );
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
    while let Ok(Some(_)) = timeout(Duration::from_millis(100), handle.rx.recv()).await {
        event_count += 1;
    }

    // Should have debounced to at most 2 events (create + 1 batched modify)
    assert!(
        event_count <= 2,
        "Expected ≤2 events due to debouncing, got {}",
        event_count
    );
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
    let result = timeout(Duration::from_millis(500), handle.rx.recv()).await;

    assert!(
        result.is_err(),
        "Should not receive events for files outside watched root"
    );
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
    timeout(Duration::from_secs(2), handle.rx.recv())
        .await
        .expect("Timeout waiting for create")
        .expect("Channel closed");

    // Delete file
    fs::remove_file(&test_file).expect("Failed to delete file");

    // Wait for Removed event
    let event = timeout(Duration::from_secs(2), handle.rx.recv())
        .await
        .expect("Timeout waiting for remove event")
        .expect("Channel closed");

    assert_eq!(event.path, test_file);
    assert!(
        matches!(event.kind, FsEventKind::Removed),
        "Expected Removed event, got {:?}",
        event.kind
    );
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
