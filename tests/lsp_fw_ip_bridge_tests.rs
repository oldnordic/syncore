//! APEX 2.5-LSP: LSP + FsWatcher/ParserService Bridge Tests (TDD-First)
//!
//! Tests for integration between FsWatcher/ParserService and LspBridge.
//! Expected to fail until implementation exists.

use std::time::Duration;
use tempfile::TempDir;

use syncore::fs_watcher::{FsEvent, FsEventKind};
use syncore::lsp_bridge::{on_fs_event_update_lsp, LspBridge, LspStatus};
use syncore::parser_service::ParserService;

// ============================================================================
// TEST 7: FS Event didOpen Sent for New File
// ============================================================================

#[tokio::test]
#[ignore = "Requires rust-analyzer installed"]
async fn test_fs_event_did_open_sent_for_new_file() {
    // Skip if rust-analyzer not available
    if which::which("rust-analyzer").is_err() {
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    // Create LSP bridge
    let bridge = LspBridge::try_start(&root, "rust")
        .await
        .expect("Failed to start bridge");

    assert_eq!(bridge.status(), LspStatus::Ready);

    // Create parser service
    let language = unsafe { tree_sitter_rust::language() };
    let parser = ParserService::new(language).expect("Failed to create parser");

    // Create Rust file
    let test_file = root.join("new.rs");
    let code = "fn main() {}";
    std::fs::write(&test_file, code).expect("Failed to write file");

    // Simulate FsEvent for Created
    let event = FsEvent {
        path: test_file.clone(),
        kind: FsEventKind::Created,
    };

    // Call helper to update LSP
    let result = on_fs_event_update_lsp(&bridge, &parser, &event).await;

    assert!(
        result.is_ok(),
        "Helper should send didOpen without error: {:?}",
        result.err()
    );

    // Give LSP time to process
    tokio::time::sleep(Duration::from_millis(100)).await;

    // We can't directly verify didOpen was sent without inspecting internal state,
    // but at least ensure no panic/error occurred
    assert_eq!(bridge.status(), LspStatus::Ready);
}

// ============================================================================
// TEST 8: FS Event didChange Sent for Modify
// ============================================================================

#[tokio::test]
#[ignore = "Requires rust-analyzer installed"]
async fn test_fs_event_did_change_sent_for_modify() {
    if which::which("rust-analyzer").is_err() {
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let bridge = LspBridge::try_start(&root, "rust")
        .await
        .expect("Failed to start bridge");

    let language = unsafe { tree_sitter_rust::language() };
    let mut parser = ParserService::new(language).expect("Failed to create parser");

    // Create and open file
    let test_file = root.join("modify.rs");
    let initial_code = "fn main() {}";
    std::fs::write(&test_file, initial_code).expect("Failed to write file");

    let create_event = FsEvent {
        path: test_file.clone(),
        kind: FsEventKind::Created,
    };

    on_fs_event_update_lsp(&bridge, &parser, &create_event)
        .await
        .expect("Failed to send didOpen");

    // Apply event to parser to track state
    parser
        .apply_fs_event(create_event)
        .expect("Failed to apply create event");

    // Modify file
    let modified_code = "fn main() { println!(\"modified\"); }";
    std::fs::write(&test_file, modified_code).expect("Failed to modify file");

    let modify_event = FsEvent {
        path: test_file.clone(),
        kind: FsEventKind::Modified,
    };

    // Call helper for didChange
    let result = on_fs_event_update_lsp(&bridge, &parser, &modify_event).await;

    assert!(
        result.is_ok(),
        "Helper should send didChange without error: {:?}",
        result.err()
    );

    assert_eq!(bridge.status(), LspStatus::Ready);
}

// ============================================================================
// TEST 9: FS Event Delete Does Not Panic
// ============================================================================

#[tokio::test]
async fn test_fs_event_delete_does_not_panic() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    // Create disabled bridge (no rust-analyzer required for this test)
    let bridge = LspBridge::try_start(&root, "nonexistent-language")
        .await
        .expect("Failed to create disabled bridge");

    assert_eq!(bridge.status(), LspStatus::Disabled);

    let language = unsafe { tree_sitter_rust::language() };
    let parser = ParserService::new(language).expect("Failed to create parser");

    let deleted_file = root.join("deleted.rs");

    // Simulate delete event
    let event = FsEvent {
        path: deleted_file.clone(),
        kind: FsEventKind::Removed,
    };

    // Should not panic, should gracefully handle
    let result = on_fs_event_update_lsp(&bridge, &parser, &event).await;

    assert!(
        result.is_ok(),
        "Delete event should be handled gracefully"
    );
}
