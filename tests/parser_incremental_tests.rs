//! APEX 2.2-FW: Incremental Parser Tests (TDD-First)
//!
//! Tests for parser_service module before implementation exists.
//! Expected to fail until src/parser_service/mod.rs is implemented.

use std::path::PathBuf;
use tempfile::TempDir;

// These imports will fail until implementation exists
use syncore::parser_service::{ParseDelta, ParserService};

// ============================================================================
// TEST 6: Incremental Parse Detects Small Edit
// ============================================================================

#[test]
fn test_incremental_parse_detects_small_edit() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    // Create parser for Rust
    let language = unsafe { tree_sitter_rust::language() };
    let mut service = ParserService::new(language, root.clone()).expect("Failed to create service");

    // Initial Rust code
    let initial_code = r#"
fn hello() {
    println!("Hello, world!");
}
"#;

    let test_file = root.join("test.rs");
    std::fs::write(&test_file, initial_code).expect("Failed to write file");

    // Initial parse
    let event = syncore::fs_watcher::FsEvent::Created(test_file.clone());

    let deltas = service.apply_fs_event(event).expect("Failed to apply create event");

    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].path, test_file);
    assert!(!deltas[0].had_errors, "Initial parse should have no errors");

    // Modify one line (change println message)
    let modified_code = r#"
fn hello() {
    println!("Hello, Rust!");
}
"#;

    std::fs::write(&test_file, modified_code).expect("Failed to modify file");

    let event = syncore::fs_watcher::FsEvent::Modified(test_file.clone());

    let deltas = service.apply_fs_event(event).expect("Failed to apply modify event");

    assert_eq!(deltas.len(), 1);
    assert!(!deltas[0].changed_ranges.is_empty(), "Should have changed ranges");

    // Verify changed ranges are localized (not entire file)
    let total_changed_bytes: usize =
        deltas[0].changed_ranges.iter().map(|r| r.end_byte - r.start_byte).sum();

    let file_size = modified_code.len();
    assert!(
        total_changed_bytes < file_size / 2,
        "Changed ranges should be localized, not entire file. Changed: {}, Total: {}",
        total_changed_bytes,
        file_size
    );
}

// ============================================================================
// TEST 7: Handle New File
// ============================================================================

#[test]
fn test_incremental_parse_handles_new_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let language = unsafe { tree_sitter_rust::language() };
    let mut service = ParserService::new(language, root.clone()).expect("Failed to create service");

    let test_file = root.join("new.rs");
    std::fs::write(&test_file, "fn main() {}").expect("Failed to write file");

    let event = syncore::fs_watcher::FsEvent::Created(test_file.clone());

    let deltas = service.apply_fs_event(event).expect("Failed to apply create event");

    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].path, test_file);
    assert!(!deltas[0].had_errors, "New file parse should have no errors");
}

// ============================================================================
// TEST 8: Handle Delete
// ============================================================================

#[test]
fn test_incremental_parse_handles_delete() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let language = unsafe { tree_sitter_rust::language() };
    let mut service = ParserService::new(language, root.clone()).expect("Failed to create service");

    // Create and parse file
    let test_file = root.join("deleteme.rs");
    std::fs::write(&test_file, "fn main() {}").expect("Failed to write file");

    let create_event = syncore::fs_watcher::FsEvent::Created(test_file.clone());

    service.apply_fs_event(create_event).expect("Failed to create");

    // Delete file
    std::fs::remove_file(&test_file).expect("Failed to delete file");

    let delete_event = syncore::fs_watcher::FsEvent::Removed(test_file.clone());

    let deltas = service.apply_fs_event(delete_event).expect("Failed to apply delete event");

    // Should return empty deltas or single delta indicating removal
    assert!(
        deltas.is_empty() || deltas[0].changed_ranges.is_empty(),
        "Delete should produce no changed ranges"
    );

    // Verify state is cleaned up (subsequent operations on deleted file should fail gracefully)
    let modify_event = syncore::fs_watcher::FsEvent::Modified(test_file.clone());

    let result = service.apply_fs_event(modify_event);
    assert!(result.is_ok() && result.unwrap().is_empty(), "Modifying deleted file should be no-op");
}

// ============================================================================
// TEST 9: Error Flag on Syntax Error
// ============================================================================

#[test]
fn test_incremental_parse_error_flag() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let language = unsafe { tree_sitter_rust::language() };
    let mut service = ParserService::new(language, root.clone()).expect("Failed to create service");

    // Valid code first
    let test_file = root.join("error.rs");
    std::fs::write(&test_file, "fn main() {}").expect("Failed to write file");

    let create_event = syncore::fs_watcher::FsEvent::Created(test_file.clone());

    let deltas = service.apply_fs_event(create_event).expect("Failed to create");
    assert!(!deltas[0].had_errors, "Initial valid code should have no errors");

    // Introduce syntax error (missing brace)
    let invalid_code = r#"
fn broken() {
    println!("missing closing brace");
"#;

    std::fs::write(&test_file, invalid_code).expect("Failed to write invalid code");

    let modify_event = syncore::fs_watcher::FsEvent::Modified(test_file.clone());

    let deltas = service.apply_fs_event(modify_event).expect("Failed to modify");

    assert_eq!(deltas.len(), 1);
    assert!(deltas[0].had_errors, "Syntax error should set had_errors flag");
}

// ============================================================================
// TEST 10: Rename Handling
// ============================================================================

#[test]
fn test_parse_delta_for_rename() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let language = unsafe { tree_sitter_rust::language() };
    let mut service = ParserService::new(language, root.clone()).expect("Failed to create service");

    // Create file
    let old_path = root.join("old_name.rs");
    std::fs::write(&old_path, "fn main() {}").expect("Failed to write file");

    let create_event = syncore::fs_watcher::FsEvent::Created(old_path.clone());

    service.apply_fs_event(create_event).expect("Failed to create");

    // Rename file
    let new_path = root.join("new_name.rs");
    std::fs::rename(&old_path, &new_path).expect("Failed to rename file");

    // Rename event should trigger removal of old + creation of new
    // Since FsEvent no longer has Renamed, test with separate events
    let remove_event = syncore::fs_watcher::FsEvent::Removed(old_path.clone());
    let create_event = syncore::fs_watcher::FsEvent::Created(new_path.clone());

    let deltas1 = service.apply_fs_event(remove_event).expect("Failed to apply remove");
    let deltas2 = service.apply_fs_event(create_event).expect("Failed to apply create");

    // Should produce deltas for both old (removed) and new (created)
    // Or at least handle the rename gracefully
    let all_deltas = vec![deltas1, deltas2].into_iter().flatten().collect::<Vec<_>>();
    assert!(!all_deltas.is_empty(), "Rename should produce parse deltas");

    // Verify new file is now tracked
    std::fs::write(&new_path, "fn main() { println!(\"renamed\"); }").expect("Failed to modify");

    let modify_event = syncore::fs_watcher::FsEvent::Modified(new_path.clone());

    let deltas = service.apply_fs_event(modify_event).expect("Failed to modify renamed file");

    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].path, new_path);
}
