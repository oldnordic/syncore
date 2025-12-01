//! APEX 2.3-CG: Code Graph Incremental Integration Tests (TDD-First)
//!
//! Full pipeline integration tests:
//!   FsWatcher → ParserService → CodeGraphUpdateService
//!
//! Expected to fail until implementation exists.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use syncore::code_graph::update_service::{CodeGraphUpdateEvent, CodeGraphUpdateService};
use syncore::code_graph::CodeGraph;
use syncore::fs_watcher::FsEvent;
use syncore::parser_service::{ParseDelta, ParserService};
use syncore::vector::{StubEmbeddings, VectorStore};

/// Helper: create a small Rust file with given contents
fn write_rust_file<P: AsRef<Path>>(path: P, contents: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

/// Pipeline struct containing all components
struct Pipeline {
    pub root: PathBuf,
    pub parser: ParserService,
    pub updater: CodeGraphUpdateService,
}

/// Initialize the full pipeline with real components
fn init_pipeline(root: PathBuf) -> anyhow::Result<Pipeline> {
    // Initialize CodeGraph with persistent DB in temp dir
    let db_path = root.join("test_code_graph.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    // Initialize ParserService for Rust language
    let language = unsafe { tree_sitter_rust::language() };
    let parser = ParserService::new(language, root.clone())?;

    // Initialize CodeGraphUpdateService
    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let updater = CodeGraphUpdateService::new(root.clone(), graph, reindex_mutex)?;

    Ok(Pipeline {
        root,
        parser,
        updater,
    })
}

/// Helper to apply an FsEvent + ParseDelta through the update service
fn apply_pipeline_update(
    updater: &mut CodeGraphUpdateService,
    fs_event: FsEvent,
    delta: Option<ParseDelta>,
) -> anyhow::Result<u64> {
    let event = CodeGraphUpdateEvent {
        fs_event,
        parse_delta: delta,
    };
    let affected = updater.apply_update(event)?;
    Ok(affected)
}

// ============================================================================
// TEST 6: Full Pipeline - New File
// ============================================================================

#[tokio::test]
async fn test_full_fw_ip_cg_pipeline_new_file() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let mut pipeline = init_pipeline(root.clone())?;

    let file_path = root.join("src").join("new_file.rs");
    write_rust_file(&file_path, "fn new_function() {}\n")?;

    // Create FsEvent for Created
    let fs_event = FsEvent::Created(file_path.clone());

    // Use ParserService to produce ParseDelta
    let deltas = pipeline.parser.apply_fs_event(fs_event.clone())?;
    let delta = if !deltas.is_empty() {
        Some(deltas[0].clone())
    } else {
        None
    };

    // Apply update through pipeline
    let affected = apply_pipeline_update(&mut pipeline.updater, fs_event, delta)?;

    assert!(affected > 0, "Expected at least one graph entity to be affected for new file");

    // Query CodeGraph to verify entities exist
    let entities = pipeline.updater.query_entities_by_path(&file_path)?;

    assert!(!entities.is_empty(), "Should have at least one entity for new_file.rs");

    assert!(
        entities.iter().any(|e| e.name == "new_function"),
        "Should have entity for new_function"
    );

    Ok(())
}

// ============================================================================
// TEST 7: Full Pipeline - Modify File
// ============================================================================

#[tokio::test]
async fn test_full_fw_ip_cg_pipeline_modify_file() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let mut pipeline = init_pipeline(root.clone())?;

    let file_path = root.join("src").join("modify_file.rs");
    write_rust_file(&file_path, "fn original_name() {}\n")?;

    // FIRST: simulate initial index (Created)
    let created_event = FsEvent::Created(file_path.clone());

    let created_deltas = pipeline.parser.apply_fs_event(created_event.clone())?;
    let created_delta = if !created_deltas.is_empty() {
        Some(created_deltas[0].clone())
    } else {
        None
    };

    apply_pipeline_update(&mut pipeline.updater, created_event, created_delta)?;

    // Verify initial state
    let entities_before = pipeline.updater.query_entities_by_path(&file_path)?;
    assert!(
        entities_before.iter().any(|e| e.name == "original_name"),
        "Should have entity for original_name initially"
    );

    // THEN: modify the file
    write_rust_file(&file_path, "fn updated_name() {}\n")?;

    let modified_event = FsEvent::Modified(file_path.clone());

    let modified_deltas = pipeline.parser.apply_fs_event(modified_event.clone())?;
    let modified_delta = if !modified_deltas.is_empty() {
        Some(modified_deltas[0].clone())
    } else {
        None
    };

    let affected = apply_pipeline_update(&mut pipeline.updater, modified_event, modified_delta)?;

    assert!(affected > 0, "Expected at least one graph entity to be affected for modified file");

    // Query CodeGraph to verify changes
    let entities_after = pipeline.updater.query_entities_by_path(&file_path)?;

    assert!(
        !entities_after.iter().any(|e| e.name == "original_name"),
        "Old entity for original_name should be removed"
    );

    assert!(
        entities_after.iter().any(|e| e.name == "updated_name"),
        "New entity for updated_name should exist"
    );

    Ok(())
}

// ============================================================================
// TEST 8: Full Pipeline - Delete File
// ============================================================================

#[tokio::test]
async fn test_full_fw_ip_cg_pipeline_delete_file() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let mut pipeline = init_pipeline(root.clone())?;

    let file_path = root.join("src").join("delete_me.rs");
    write_rust_file(&file_path, "fn doomed() {}\n")?;

    // FIRST: index as created
    let created_event = FsEvent::Created(file_path.clone());

    let created_deltas = pipeline.parser.apply_fs_event(created_event.clone())?;
    let created_delta = if !created_deltas.is_empty() {
        Some(created_deltas[0].clone())
    } else {
        None
    };

    apply_pipeline_update(&mut pipeline.updater, created_event, created_delta)?;

    // Verify entity exists
    let entities_before = pipeline.updater.query_entities_by_path(&file_path)?;
    assert!(!entities_before.is_empty(), "Should have entities before delete");

    // THEN: delete the file
    fs::remove_file(&file_path)?;

    let removed_event = FsEvent::Removed(file_path.clone());

    // For delete, ParseDelta is None (file no longer exists to parse)
    let affected = apply_pipeline_update(&mut pipeline.updater, removed_event, None)?;

    assert!(affected >= 0, "Expected apply_update not to panic for deleted file event");

    // Query CodeGraph to assert no entities remain
    let entities_after = pipeline.updater.query_entities_by_path(&file_path)?;

    assert!(entities_after.is_empty(), "Should have no entities after delete");

    Ok(())
}
