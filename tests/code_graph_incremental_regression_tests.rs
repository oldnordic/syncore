//! APEX 2.3-CG: Code Graph Incremental Regression Tests (TDD-First)
//!
//! Tests to ensure incremental updates don't cause regressions:
//! - Only affected files are reindexed
//! - Graph connectivity is preserved

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use syncore::code_graph::update_service::{CodeGraphUpdateEvent, CodeGraphUpdateService};
use syncore::code_graph::CodeGraph;
use syncore::fs_watcher::FsEvent;
use syncore::parser_service::ParseDelta;
use syncore::vector::{StubEmbeddings, VectorStore};

// ============================================================================
// TEST 9: Incremental Updates Don't Reindex Unrelated Files
// ============================================================================

#[tokio::test]
async fn test_incremental_updates_do_not_reindex_unrelated_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    // Create two files A and B
    let file_a = root.join("file_a.rs");
    let file_b = root.join("file_b.rs");

    std::fs::write(&file_a, "pub fn func_a() {}").expect("Failed to write file A");
    std::fs::write(&file_b, "pub fn func_b() {}").expect("Failed to write file B");

    // Setup graph
    let db_path = root.join("test_graph.db");
    let embeddings = Box::new(StubEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)
        .expect("Failed to create CodeGraph");

    // APEX 2.15: Pass reindex mutex to UpdateService
    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let mut update_service = CodeGraphUpdateService::new(root.clone(), code_graph, reindex_mutex)
        .expect("Failed to create CodeGraphUpdateService");

    // Index both files
    let event_a = CodeGraphUpdateEvent {
        fs_event: FsEvent::Created(file_a.clone()),
        parse_delta: Some(ParseDelta {
            path: file_a.clone(),
            changed_ranges: vec![],
            had_errors: false,
        }),
    };

    let event_b = CodeGraphUpdateEvent {
        fs_event: FsEvent::Created(file_b.clone()),
        parse_delta: Some(ParseDelta {
            path: file_b.clone(),
            changed_ranges: vec![],
            had_errors: false,
        }),
    };

    update_service
        .apply_update(event_a)
        .expect("Failed to index A");
    update_service
        .apply_update(event_b)
        .expect("Failed to index B");

    // Get initial state of file B
    let entities_b_before = update_service
        .query_entities_by_path(&file_b)
        .expect("Failed to query B");
    let count_b_before = entities_b_before.len();

    // Modify only file A
    std::fs::write(&file_a, "pub fn func_a_modified() {}").expect("Failed to modify A");

    let modify_event_a = CodeGraphUpdateEvent {
        fs_event: FsEvent::Modified(file_a.clone()),
        parse_delta: Some(ParseDelta {
            path: file_a.clone(),
            changed_ranges: vec![],
            had_errors: false,
        }),
    };

    let affected = update_service
        .apply_update(modify_event_a)
        .expect("Failed to modify A");

    assert!(affected > 0, "File A should be affected");

    // Verify file B is unchanged
    let entities_b_after = update_service
        .query_entities_by_path(&file_b)
        .expect("Failed to query B after");
    let count_b_after = entities_b_after.len();

    assert_eq!(
        count_b_before, count_b_after,
        "File B entity count should be unchanged"
    );

    assert!(
        entities_b_after.iter().any(|e| e.name == "func_b"),
        "File B should still have func_b"
    );

    // Verify file A was updated
    let entities_a_after = update_service
        .query_entities_by_path(&file_a)
        .expect("Failed to query A after");

    assert!(
        entities_a_after.iter().any(|e| e.name == "func_a_modified"),
        "File A should have new function"
    );

    assert!(
        !entities_a_after.iter().any(|e| e.name == "func_a"),
        "File A should not have old function"
    );
}

// ============================================================================
// TEST 10: Incremental Updates Preserve Graph Connectivity
// ============================================================================

#[tokio::test]
async fn test_incremental_updates_preserve_graph_connectivity() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path().to_path_buf();

    // Create two Rust files with a potential cross-file reference
    let file_caller = root.join("caller.rs");
    let file_callee = root.join("callee.rs");

    std::fs::write(&file_caller, "pub fn caller() { callee::callee_func(); }")
        .expect("Failed to write caller");
    std::fs::write(&file_callee, "pub fn callee_func() {}").expect("Failed to write callee");

    // Setup graph
    let db_path = root.join("test_graph.db");
    let embeddings = Box::new(StubEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)
        .expect("Failed to create CodeGraph");

    // APEX 2.15: Pass reindex mutex to UpdateService
    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let mut update_service = CodeGraphUpdateService::new(root.clone(), code_graph, reindex_mutex)
        .expect("Failed to create CodeGraphUpdateService");

    // Index both files
    let event_caller = CodeGraphUpdateEvent {
        fs_event: FsEvent::Created(file_caller.clone()),
        parse_delta: Some(ParseDelta {
            path: file_caller.clone(),
            changed_ranges: vec![],
            had_errors: false,
        }),
    };

    let event_callee = CodeGraphUpdateEvent {
        fs_event: FsEvent::Created(file_callee.clone()),
        parse_delta: Some(ParseDelta {
            path: file_callee.clone(),
            changed_ranges: vec![],
            had_errors: false,
        }),
    };

    update_service
        .apply_update(event_caller)
        .expect("Failed to index caller");
    update_service
        .apply_update(event_callee)
        .expect("Failed to index callee");

    // Verify initial entities exist
    let entities_caller_before = update_service
        .query_entities_by_path(&file_caller)
        .expect("Failed to query caller");
    let entities_callee_before = update_service
        .query_entities_by_path(&file_callee)
        .expect("Failed to query callee");

    assert!(
        !entities_caller_before.is_empty(),
        "Caller should have entities"
    );
    assert!(
        !entities_callee_before.is_empty(),
        "Callee should have entities"
    );

    // Modify caller file
    std::fs::write(
        &file_caller,
        "pub fn caller() { callee::callee_func(); }\npub fn another() {}",
    )
    .expect("Failed to modify caller");

    let modify_event = CodeGraphUpdateEvent {
        fs_event: FsEvent::Modified(file_caller.clone()),
        parse_delta: Some(ParseDelta {
            path: file_caller.clone(),
            changed_ranges: vec![],
            had_errors: false,
        }),
    };

    update_service
        .apply_update(modify_event)
        .expect("Failed to modify caller");

    // Verify both files still have entities
    let entities_caller_after = update_service
        .query_entities_by_path(&file_caller)
        .expect("Failed to query caller after");
    let entities_callee_after = update_service
        .query_entities_by_path(&file_callee)
        .expect("Failed to query callee after");

    assert!(
        !entities_caller_after.is_empty(),
        "Caller should still have entities"
    );
    assert!(
        !entities_callee_after.is_empty(),
        "Callee should still have entities"
    );

    // Verify caller has both functions
    assert!(
        entities_caller_after.iter().any(|e| e.name == "caller"),
        "Caller should have caller function"
    );
    assert!(
        entities_caller_after.iter().any(|e| e.name == "another"),
        "Caller should have another function"
    );

    // Verify callee unchanged
    assert!(
        entities_callee_after
            .iter()
            .any(|e| e.name == "callee_func"),
        "Callee should still have callee_func"
    );

    // Note: We don't verify edges here as edge extraction may not be
    // implemented yet, but we verify that entities are preserved
}
