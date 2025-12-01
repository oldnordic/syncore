//! APEX 2.2-FW: Filewatcher + Incremental Parser Bridge Tests (TDD-First)
//!
//! Tests for integration between fs_watcher and parser_service.
//! Expected to fail until both modules are implemented.

use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

use syncore::fs_watcher::start_fs_watcher;
use syncore::parser_service::ParserService;

// ============================================================================
// TEST 11: FS Event Triggers Incremental Parse
// ============================================================================

#[tokio::test]
async fn test_fs_event_triggers_incremental_parse() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    // Start filewatcher
    let mut watcher_handle = start_fs_watcher(root.clone()).expect("Failed to start watcher");

    // Create parser service
    let language = unsafe { tree_sitter_rust::language() };
    let mut parser_service =
        ParserService::new(language, root.clone()).expect("Failed to create parser");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create Rust file
    let test_file = root.join("bridge_test.rs");
    std::fs::write(&test_file, "fn main() {}").expect("Failed to write file");

    // Wait for fs event
    let fs_event = {
        let rx = watcher_handle.rx.clone();
        tokio::task::spawn_blocking(move || rx.recv().expect("Channel closed"))
            .await
            .expect("Task panicked")
    };

    assert_eq!(fs_event.path(), &test_file);

    // Apply fs_event to parser
    let deltas =
        parser_service.apply_fs_event(fs_event).expect("Failed to apply fs event to parser");

    assert_eq!(deltas.len(), 1, "Should produce one parse delta");
    assert_eq!(deltas[0].path, test_file);
    assert!(!deltas[0].had_errors, "Parse should have no errors");

    // Modify file
    std::fs::write(&test_file, "fn main() { println!(\"modified\"); }")
        .expect("Failed to modify file");

    // Wait for modify event
    let fs_event = {
        let rx = watcher_handle.rx.clone();
        tokio::task::spawn_blocking(move || rx.recv().expect("Channel closed"))
            .await
            .expect("Task panicked")
    };

    // Apply modify event
    let deltas = parser_service.apply_fs_event(fs_event).expect("Failed to apply modify event");

    assert_eq!(deltas.len(), 1);
    assert!(
        !deltas[0].changed_ranges.is_empty(),
        "Should have changed ranges from incremental parse"
    );

    // Verify changed_ranges are reasonable
    let total_changed =
        deltas[0].changed_ranges.iter().map(|r| r.end_byte - r.start_byte).sum::<usize>();

    assert!(
        total_changed > 0 && total_changed < 500,
        "Changed ranges should be localized: {} bytes",
        total_changed
    );
}

// ============================================================================
// TEST 12: Ignore Non-Rust Files
// ============================================================================

#[tokio::test]
async fn test_ignore_non_rust_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let mut watcher_handle = start_fs_watcher(root.clone()).expect("Failed to start watcher");

    // Parser only for Rust
    let language = unsafe { tree_sitter_rust::language() };
    let mut parser_service =
        ParserService::new(language, root.clone()).expect("Failed to create parser");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create non-Rust file
    let txt_file = root.join("readme.txt");
    std::fs::write(&txt_file, "This is not Rust code").expect("Failed to write txt file");

    // Wait for fs event
    let fs_event = {
        let rx = watcher_handle.rx.clone();
        tokio::task::spawn_blocking(move || rx.recv().expect("Channel closed"))
            .await
            .expect("Task panicked")
    };

    assert_eq!(fs_event.path(), &txt_file);

    // Apply to parser - should produce no deltas for unsupported extension
    let deltas =
        parser_service.apply_fs_event(fs_event).expect("Should handle non-Rust files gracefully");

    assert!(deltas.is_empty(), "Non-Rust files should produce no parse deltas");

    // Create Rust file - should produce delta
    let rs_file = root.join("test.rs");
    std::fs::write(&rs_file, "fn main() {}").expect("Failed to write Rust file");

    let fs_event =
        tokio::task::spawn_blocking(move || watcher_handle.rx.recv().expect("Channel closed"))
            .await
            .expect("Task panicked");

    let deltas = parser_service.apply_fs_event(fs_event).expect("Should parse Rust file");

    assert_eq!(deltas.len(), 1, "Rust files should produce parse deltas");
    assert_eq!(deltas[0].path, rs_file);
}
