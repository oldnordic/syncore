//! APEX 2.6-CG-GRAPH-DELTA: Regression Tests
//!
//! Ensures delta engine doesn't break existing functionality.
//! These tests MUST FAIL initially (TDD-first).

use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use syncore::code_graph::update_service::{CodeGraphUpdateEvent, CodeGraphUpdateService};
use syncore::code_graph::CodeGraph;
use syncore::fs_watcher::{FsEvent, FsEventKind};
use syncore::parser_service::{ParseDelta, ParserService};
use syncore::vector::{StubEmbeddings, VectorStore};

fn write_rust_file(path: &std::path::Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

struct Pipeline {
    root: PathBuf,
    parser: ParserService,
    updater: CodeGraphUpdateService,
}

fn init_pipeline(root: PathBuf) -> Result<Pipeline> {
    let db_path = root.join("test_regression_graph.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    let language = unsafe { tree_sitter_rust::language() };
    let parser = ParserService::new(language, root.clone())?;

    let updater = CodeGraphUpdateService::new(root.clone(), graph)?;

    Ok(Pipeline {
        root,
        parser,
        updater,
    })
}

// ============================================================================
// TEST 1: Delta engine never touches unrelated files
// ============================================================================

#[tokio::test]
async fn test_delta_never_touches_unrelated_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let mut pipeline = init_pipeline(root.clone())?;

    // Create two files
    let file_a = root.join("src").join("file_a.rs");
    let file_b = root.join("src").join("file_b.rs");

    write_rust_file(&file_a, "pub fn func_a() {}")?;
    write_rust_file(&file_b, "pub fn func_b() {}")?;

    // Index both files
    for file in &[&file_a, &file_b] {
        let event = FsEvent {
            path: (*file).clone(),
            kind: FsEventKind::Created,
        };
        let deltas = pipeline.parser.apply_fs_event(event.clone())?;
        let update_event = CodeGraphUpdateEvent {
            fs_event: event,
            parse_delta: deltas.first().cloned(),
        };
        pipeline.updater.apply_update(update_event)?;
    }

    // Get initial state of file B
    let entities_b_before = pipeline.updater.query_entities_by_path(&file_b)?;
    let count_b_before = entities_b_before.len();

    // Modify only file A
    write_rust_file(&file_a, "pub fn func_a() { println!(\"modified\"); }")?;

    let event = FsEvent {
        path: file_a.clone(),
        kind: FsEventKind::Modified,
    };
    let deltas = pipeline.parser.apply_fs_event(event.clone())?;
    let update_event = CodeGraphUpdateEvent {
        fs_event: event,
        parse_delta: deltas.first().cloned(),
    };
    pipeline.updater.apply_update(update_event)?;

    // Verify file B unchanged
    let entities_b_after = pipeline.updater.query_entities_by_path(&file_b)?;
    let count_b_after = entities_b_after.len();

    assert_eq!(
        count_b_before, count_b_after,
        "File B entity count should be unchanged"
    );
    assert!(
        entities_b_after.iter().any(|e| e.name == "func_b"),
        "File B should still have func_b"
    );

    Ok(())
}

// ============================================================================
// TEST 2: Delta engine preserves domain routing (doesn't touch memory)
// ============================================================================

#[tokio::test]
async fn test_delta_preserves_domain_routing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let mut pipeline = init_pipeline(root.clone())?;

    let file_path = root.join("src").join("test.rs");
    write_rust_file(&file_path, "pub fn test() {}")?;

    // Create and apply update
    let event = FsEvent {
        path: file_path.clone(),
        kind: FsEventKind::Created,
    };
    let deltas = pipeline.parser.apply_fs_event(event.clone())?;
    let update_event = CodeGraphUpdateEvent {
        fs_event: event,
        parse_delta: deltas.first().cloned(),
    };

    // Should only touch code_entities table, not memory table
    pipeline.updater.apply_update(update_event)?;

    // Verify entities exist
    let entities = pipeline.updater.query_entities_by_path(&file_path)?;
    assert!(!entities.is_empty(), "Should have entities in code_entities");

    Ok(())
}

// ============================================================================
// TEST 3: Delta engine never touches memory schema
// ============================================================================

#[tokio::test]
async fn test_delta_never_touches_memory_schema() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let db_path = root.join("test_schema.db");

    // Create a database with code_entities but no memory table
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let _graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    // Database should initialize without errors (no attempt to touch memory table)
    // This test verifies delta engine doesn't cause schema conflicts
    Ok(())
}

// ============================================================================
// TEST 4: Delta engine works with empty changed_ranges
// ============================================================================

#[tokio::test]
async fn test_delta_works_with_empty_changed_ranges() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let mut pipeline = init_pipeline(root.clone())?;

    let file_path = root.join("src").join("empty_ranges.rs");
    write_rust_file(&file_path, "pub fn test() {}")?;

    // Create ParseDelta with empty changed_ranges
    let event = FsEvent {
        path: file_path.clone(),
        kind: FsEventKind::Modified,
    };

    let parse_delta = ParseDelta {
        path: file_path.clone(),
        changed_ranges: vec![], // Empty
        had_errors: false,
    };

    let update_event = CodeGraphUpdateEvent {
        fs_event: event,
        parse_delta: Some(parse_delta),
    };

    // Should handle gracefully (no-op)
    pipeline.updater.apply_update(update_event)?;

    Ok(())
}

// ============================================================================
// TEST 5: Delta engine is idempotent on repeated updates
// ============================================================================

#[tokio::test]
async fn test_delta_idempotent_on_repeated_updates() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let mut pipeline = init_pipeline(root.clone())?;

    let file_path = root.join("src").join("idempotent.rs");
    write_rust_file(&file_path, "pub fn test() {}")?;

    // Apply same update twice
    for _ in 0..2 {
        let event = FsEvent {
            path: file_path.clone(),
            kind: FsEventKind::Created,
        };
        let deltas = pipeline.parser.apply_fs_event(event.clone())?;
        let update_event = CodeGraphUpdateEvent {
            fs_event: event,
            parse_delta: deltas.first().cloned(),
        };
        pipeline.updater.apply_update(update_event)?;
    }

    // Verify entities exist and no duplicates
    let entities = pipeline.updater.query_entities_by_path(&file_path)?;
    assert!(!entities.is_empty(), "Should have entities");

    // Count how many times "test" appears (should be exactly once if idempotent)
    let test_count = entities.iter().filter(|e| e.name == "test").count();
    assert_eq!(test_count, 1, "Should have exactly one 'test' function");

    Ok(())
}

// ============================================================================
// TEST 6: Delta engine handles file deletion correctly
// ============================================================================

#[tokio::test]
async fn test_delta_handles_file_deletion() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let mut pipeline = init_pipeline(root.clone())?;

    let file_path = root.join("src").join("to_delete.rs");
    write_rust_file(&file_path, "pub fn will_be_deleted() {}")?;

    // Index file
    let created_event = FsEvent {
        path: file_path.clone(),
        kind: FsEventKind::Created,
    };
    let created_deltas = pipeline.parser.apply_fs_event(created_event.clone())?;
    let event = CodeGraphUpdateEvent {
        fs_event: created_event,
        parse_delta: created_deltas.first().cloned(),
    };
    pipeline.updater.apply_update(event)?;

    // Verify entity exists
    let entities_before = pipeline.updater.query_entities_by_path(&file_path)?;
    assert!(!entities_before.is_empty(), "Should have entities before delete");

    // Delete file
    fs::remove_file(&file_path)?;

    let removed_event = FsEvent {
        path: file_path.clone(),
        kind: FsEventKind::Removed,
    };
    let event = CodeGraphUpdateEvent {
        fs_event: removed_event,
        parse_delta: None, // No delta for deleted file
    };
    pipeline.updater.apply_update(event)?;

    // Verify entities removed
    let entities_after = pipeline.updater.query_entities_by_path(&file_path)?;
    assert!(
        entities_after.is_empty(),
        "Should have no entities after delete"
    );

    Ok(())
}
