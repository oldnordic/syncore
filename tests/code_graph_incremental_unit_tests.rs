//! APEX 2.3-CG: Code Graph Incremental Update Unit Tests (TDD-First)
//!
//! Tests for CodeGraphUpdateService that consumes ParseDelta + FsEvent
//! and performs incremental code graph updates.
//!
//! Expected to fail until implementation exists.

use std::path::Path;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use syncore::code_graph::update_service::{CodeGraphUpdateEvent, CodeGraphUpdateService};
use syncore::code_graph::CodeGraph;
use syncore::fs_watcher::FsEvent;
use syncore::parser_service::ParseDelta;
use syncore::vector::{StubEmbeddings, VectorStore};

// ============================================================================
// TEST 1: Insert Entities for New File
// ============================================================================

#[tokio::test]
async fn test_update_service_inserts_entities_for_new_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    // Create a small Rust file with a function
    let test_file = root.join("new.rs");
    let code = "pub fn hello() { println!(\"Hello\"); }";
    std::fs::write(&test_file, code).expect("Failed to write test file");

    // Initialize graph backend
    let db_path = root.join("test_graph.db");
    let embeddings = Box::new(StubEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)
        .expect("Failed to create CodeGraph");

    // Create update service
    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let mut update_service = CodeGraphUpdateService::new(code_graph, reindex_mutex)
        .expect("Failed to create CodeGraphUpdateService");

    // Simulate FsEventKind::Created + ParseDelta with had_errors = false
    let fs_event = FsEvent::Created(test_file.clone());

    let parse_delta = Some(ParseDelta {
        path: test_file.clone(),
        changed_ranges: vec![],
        had_errors: false,
    });

    let event = CodeGraphUpdateEvent {
        fs_event,
        parse_delta,
    };

    // Call apply_update
    let affected = update_service.apply_update(event).expect("Failed to apply update");

    // Assert: at least one entity was created
    assert!(affected > 0, "Should have created at least one entity, got: {}", affected);

    // Verify entity exists in graph using existing query APIs
    let entities =
        update_service.query_entities_by_path(&test_file).expect("Failed to query entities");

    assert!(!entities.is_empty(), "Should have at least one entity for the file");

    assert!(entities.iter().any(|e| e.name == "hello"), "Should have entity for 'hello' function");
}

// ============================================================================
// TEST 2: Update Entities on Modify
// ============================================================================

#[tokio::test]
async fn test_update_service_updates_entities_on_modify() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let test_file = root.join("modify.rs");
    let initial_code = "pub fn original() {}";
    std::fs::write(&test_file, initial_code).expect("Failed to write test file");

    // Setup graph
    let db_path = root.join("test_graph.db");
    let embeddings = Box::new(StubEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)
        .expect("Failed to create CodeGraph");

    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let mut update_service = CodeGraphUpdateService::new(code_graph, reindex_mutex)
        .expect("Failed to create CodeGraphUpdateService");

    // Index initial content
    let create_event = CodeGraphUpdateEvent {
        fs_event: FsEvent::Created(test_file.clone()),
        parse_delta: Some(ParseDelta {
            path: test_file.clone(),
            changed_ranges: vec![],
            had_errors: false,
        }),
    };

    update_service.apply_update(create_event).expect("Failed to apply initial update");

    // Modify file: change function name and add another function
    let modified_code = "pub fn modified() {}\npub fn another() {}";
    std::fs::write(&test_file, modified_code).expect("Failed to modify file");

    // Apply modify event
    let modify_event = CodeGraphUpdateEvent {
        fs_event: FsEvent::Modified(test_file.clone()),
        parse_delta: Some(ParseDelta {
            path: test_file.clone(),
            changed_ranges: vec![],
            had_errors: false,
        }),
    };

    let affected =
        update_service.apply_update(modify_event).expect("Failed to apply modify update");

    assert!(affected > 0, "Should have affected entities");

    // Verify: old entity removed, new entities present
    let entities =
        update_service.query_entities_by_path(&test_file).expect("Failed to query entities");

    assert!(
        !entities.iter().any(|e| e.name == "original"),
        "Old 'original' function should be removed"
    );

    assert!(entities.iter().any(|e| e.name == "modified"), "New 'modified' function should exist");

    assert!(entities.iter().any(|e| e.name == "another"), "New 'another' function should exist");
}

// ============================================================================
// TEST 3: Remove Entities on Delete
// ============================================================================

#[tokio::test]
async fn test_update_service_removes_entities_on_delete() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let test_file = root.join("delete.rs");
    let code = "pub fn to_delete() {}";
    std::fs::write(&test_file, code).expect("Failed to write test file");

    // Setup graph and index file
    let db_path = root.join("test_graph.db");
    let embeddings = Box::new(StubEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)
        .expect("Failed to create CodeGraph");

    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let mut update_service = CodeGraphUpdateService::new(code_graph, reindex_mutex)
        .expect("Failed to create CodeGraphUpdateService");

    // Index the file first
    let create_event = CodeGraphUpdateEvent {
        fs_event: FsEvent::Created(test_file.clone()),
        parse_delta: Some(ParseDelta {
            path: test_file.clone(),
            changed_ranges: vec![],
            had_errors: false,
        }),
    };

    update_service.apply_update(create_event).expect("Failed to apply initial update");

    // Verify entity exists
    let entities_before =
        update_service.query_entities_by_path(&test_file).expect("Failed to query entities");
    assert!(!entities_before.is_empty(), "Should have entities before delete");

    // Delete the file
    std::fs::remove_file(&test_file).expect("Failed to delete file");

    // Apply delete event
    let delete_event = CodeGraphUpdateEvent {
        fs_event: FsEvent::Removed(test_file.clone()),
        parse_delta: None,
    };

    let affected =
        update_service.apply_update(delete_event).expect("Failed to apply delete update");

    assert!(affected > 0, "Should have affected entities");

    // Verify no entities remain for that path
    let entities_after =
        update_service.query_entities_by_path(&test_file).expect("Failed to query entities");

    assert!(entities_after.is_empty(), "Should have no entities after delete");
}

// ============================================================================
// TEST 4: Handle Rename as Remove + Insert
// ============================================================================

#[tokio::test]
async fn test_update_service_handles_rename_as_remove_and_insert() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let old_path = root.join("old.rs");
    let new_path = root.join("new.rs");
    let code = "pub fn renamed_func() {}";
    std::fs::write(&old_path, code).expect("Failed to write test file");

    // Setup graph and index old path
    let db_path = root.join("test_graph.db");
    let embeddings = Box::new(StubEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)
        .expect("Failed to create CodeGraph");

    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let mut update_service = CodeGraphUpdateService::new(code_graph, reindex_mutex)
        .expect("Failed to create CodeGraphUpdateService");

    // Index at old path
    let create_event = CodeGraphUpdateEvent {
        fs_event: FsEvent::Created(old_path.clone()),
        parse_delta: Some(ParseDelta {
            path: old_path.clone(),
            changed_ranges: vec![],
            had_errors: false,
        }),
    };

    update_service.apply_update(create_event).expect("Failed to apply initial update");

    // Verify entity exists at old path
    let entities_old =
        update_service.query_entities_by_path(&old_path).expect("Failed to query entities");
    assert!(!entities_old.is_empty(), "Should have entities at old path");

    // Simulate rename: rename file on disk
    std::fs::rename(&old_path, &new_path).expect("Failed to rename file");

    // Apply rename event (represented as Renamed with new path)
    let rename_event = CodeGraphUpdateEvent {
        fs_event: FsEvent::Removed(old_path.clone()),
        parse_delta: Some(ParseDelta {
            path: new_path.clone(),
            changed_ranges: vec![],
            had_errors: false,
        }),
    };

    let affected =
        update_service.apply_update(rename_event).expect("Failed to apply rename update");

    assert!(affected > 0, "Should have affected entities");

    // Verify: entities under old path removed
    let entities_old_after =
        update_service.query_entities_by_path(&old_path).expect("Failed to query entities");

    assert!(entities_old_after.is_empty(), "Should have no entities at old path after rename");

    // Verify: entities under new path inserted
    let entities_new =
        update_service.query_entities_by_path(&new_path).expect("Failed to query entities");

    assert!(!entities_new.is_empty(), "Should have entities at new path after rename");

    assert!(
        entities_new.iter().any(|e| e.name == "renamed_func"),
        "Should have renamed_func at new path"
    );
}

// ============================================================================
// TEST 5: Ignore Unsupported Extensions
// ============================================================================

#[tokio::test]
async fn test_update_service_ignores_unsupported_extensions() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    let test_file = root.join("unsupported.txt");
    std::fs::write(&test_file, "This is not source code").expect("Failed to write file");

    // Setup graph
    let db_path = root.join("test_graph.db");
    let embeddings = Box::new(StubEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)
        .expect("Failed to create CodeGraph");

    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let mut update_service = CodeGraphUpdateService::new(code_graph, reindex_mutex)
        .expect("Failed to create CodeGraphUpdateService");

    // Try to index unsupported file
    let event = CodeGraphUpdateEvent {
        fs_event: FsEvent::Created(test_file.clone()),
        parse_delta: None,
    };

    let affected =
        update_service.apply_update(event).expect("Should not fail for unsupported file");

    // Assert: returns 0 affected entities
    assert_eq!(affected, 0, "Should ignore unsupported file extension");

    // Verify: graph unchanged (no entities for .txt file)
    let entities =
        update_service.query_entities_by_path(&test_file).expect("Failed to query entities");

    assert!(entities.is_empty(), "Should have no entities for unsupported file");
}
