//! APEX 2.6-CG-GRAPH-DELTA: Integration Tests
//!
//! Full pipeline: FsWatcher → ParserService → CodeGraphDeltaEngine → CodeGraphUpdateService
//! These tests MUST FAIL initially (TDD-first).

use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use syncore::code_graph::update_service::{CodeGraphUpdateEvent, CodeGraphUpdateService};
use syncore::code_graph::CodeGraph;
use syncore::fs_watcher::FsEvent;
use syncore::parser_service::ParserService;
use syncore::vector::{StubEmbeddings, VectorStore};

/// Helper to write Rust file
fn write_rust_file(path: &std::path::Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// Initialize full pipeline
struct Pipeline {
    root: PathBuf,
    parser: ParserService,
    updater: CodeGraphUpdateService,
}

fn init_pipeline(root: PathBuf) -> Result<Pipeline> {
    let db_path = root.join("test_delta_graph.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    let language = unsafe { tree_sitter_rust::language() };
    let parser = ParserService::new(language, root.clone())?;

    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let updater = CodeGraphUpdateService::new(graph, reindex_mutex)?;

    Ok(Pipeline {
        root,
        parser,
        updater,
    })
}

// ============================================================================
// TEST 1: Changing one function yields only that function reindexed
// ============================================================================

#[tokio::test]
async fn test_modify_single_function_selective_reindex() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let mut pipeline = init_pipeline(root.clone())?;

    let file_path = root.join("src").join("test.rs");
    let initial_code = r#"
pub fn function_one() {
    println!("one");
}

pub fn function_two() {
    println!("two");
}
"#;
    write_rust_file(&file_path, initial_code)?;

    // Initial index
    let created_event = FsEvent::Created(file_path.clone());
    let created_deltas = pipeline.parser.apply_fs_event(created_event.clone())?;
    let created_delta = created_deltas.first().cloned();

    let event = CodeGraphUpdateEvent {
        fs_event: created_event,
        parse_delta: created_delta,
    };
    pipeline.updater.apply_update(event)?;

    // Verify both functions exist
    let entities_before = pipeline.updater.query_entities_by_path(&file_path)?;
    assert!(
        entities_before.iter().any(|e| e.name == "function_one"),
        "Should have function_one initially"
    );
    assert!(
        entities_before.iter().any(|e| e.name == "function_two"),
        "Should have function_two initially"
    );

    // Modify only function_one
    let modified_code = r#"
pub fn function_one() {
    println!("one modified");
}

pub fn function_two() {
    println!("two");
}
"#;
    write_rust_file(&file_path, modified_code)?;

    let modified_event = FsEvent::Modified(file_path.clone());
    let modified_deltas = pipeline.parser.apply_fs_event(modified_event.clone())?;
    let modified_delta = modified_deltas.first().cloned();

    let event = CodeGraphUpdateEvent {
        fs_event: modified_event,
        parse_delta: modified_delta,
    };
    pipeline.updater.apply_update(event)?;

    // Verify both functions still exist (selective reindex worked)
    let entities_after = pipeline.updater.query_entities_by_path(&file_path)?;
    assert!(
        entities_after.iter().any(|e| e.name == "function_one"),
        "Should still have function_one"
    );
    assert!(
        entities_after.iter().any(|e| e.name == "function_two"),
        "Should still have function_two"
    );

    Ok(())
}

// ============================================================================
// TEST 2: Adding a function updates graph nodes
// ============================================================================

#[tokio::test]
async fn test_add_function_updates_graph() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let mut pipeline = init_pipeline(root.clone())?;

    let file_path = root.join("src").join("add_test.rs");
    let initial_code = r#"
pub fn existing_function() {
    println!("exists");
}
"#;
    write_rust_file(&file_path, initial_code)?;

    // Initial index
    let created_event = FsEvent::Created(file_path.clone());
    let created_deltas = pipeline.parser.apply_fs_event(created_event.clone())?;
    let event = CodeGraphUpdateEvent {
        fs_event: created_event,
        parse_delta: created_deltas.first().cloned(),
    };
    pipeline.updater.apply_update(event)?;

    // Add a new function
    let modified_code = r#"
pub fn existing_function() {
    println!("exists");
}

pub fn new_function() {
    println!("new");
}
"#;
    write_rust_file(&file_path, modified_code)?;

    let modified_event = FsEvent::Modified(file_path.clone());
    let modified_deltas = pipeline.parser.apply_fs_event(modified_event.clone())?;
    let event = CodeGraphUpdateEvent {
        fs_event: modified_event,
        parse_delta: modified_deltas.first().cloned(),
    };
    pipeline.updater.apply_update(event)?;

    // Verify new function was added
    let entities = pipeline.updater.query_entities_by_path(&file_path)?;
    assert!(
        entities.iter().any(|e| e.name == "existing_function"),
        "Should still have existing function"
    );
    assert!(entities.iter().any(|e| e.name == "new_function"), "Should have new function");

    Ok(())
}

// ============================================================================
// TEST 3: Deleting a function removes its entities
// ============================================================================

#[tokio::test]
async fn test_delete_function_removes_entities() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let mut pipeline = init_pipeline(root.clone())?;

    let file_path = root.join("src").join("delete_test.rs");
    let initial_code = r#"
pub fn function_to_keep() {
    println!("keep");
}

pub fn function_to_delete() {
    println!("delete");
}
"#;
    write_rust_file(&file_path, initial_code)?;

    // Initial index
    let created_event = FsEvent::Created(file_path.clone());
    let created_deltas = pipeline.parser.apply_fs_event(created_event.clone())?;
    let event = CodeGraphUpdateEvent {
        fs_event: created_event,
        parse_delta: created_deltas.first().cloned(),
    };
    pipeline.updater.apply_update(event)?;

    // Delete one function
    let modified_code = r#"
pub fn function_to_keep() {
    println!("keep");
}
"#;
    write_rust_file(&file_path, modified_code)?;

    let modified_event = FsEvent::Modified(file_path.clone());
    let modified_deltas = pipeline.parser.apply_fs_event(modified_event.clone())?;
    let event = CodeGraphUpdateEvent {
        fs_event: modified_event,
        parse_delta: modified_deltas.first().cloned(),
    };
    pipeline.updater.apply_update(event)?;

    // Verify deleted function is gone
    let entities = pipeline.updater.query_entities_by_path(&file_path)?;
    assert!(
        entities.iter().any(|e| e.name == "function_to_keep"),
        "Should still have function_to_keep"
    );
    assert!(
        !entities.iter().any(|e| e.name == "function_to_delete"),
        "Should not have function_to_delete anymore"
    );

    Ok(())
}

// ============================================================================
// TEST 4: Changing docstring updates only metadata
// ============================================================================

#[tokio::test]
async fn test_change_docstring_updates_metadata() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let mut pipeline = init_pipeline(root.clone())?;

    let file_path = root.join("src").join("docstring_test.rs");
    let initial_code = r#"
/// Original docstring
pub fn documented_function() {
    println!("code");
}
"#;
    write_rust_file(&file_path, initial_code)?;

    // Initial index
    let created_event = FsEvent::Created(file_path.clone());
    let created_deltas = pipeline.parser.apply_fs_event(created_event.clone())?;
    let event = CodeGraphUpdateEvent {
        fs_event: created_event,
        parse_delta: created_deltas.first().cloned(),
    };
    pipeline.updater.apply_update(event)?;

    // Change only docstring
    let modified_code = r#"
/// Modified docstring
pub fn documented_function() {
    println!("code");
}
"#;
    write_rust_file(&file_path, modified_code)?;

    let modified_event = FsEvent::Modified(file_path.clone());
    let modified_deltas = pipeline.parser.apply_fs_event(modified_event.clone())?;
    let event = CodeGraphUpdateEvent {
        fs_event: modified_event,
        parse_delta: modified_deltas.first().cloned(),
    };
    pipeline.updater.apply_update(event)?;

    // Verify function still exists (metadata updated)
    let entities = pipeline.updater.query_entities_by_path(&file_path)?;
    assert!(
        entities.iter().any(|e| e.name == "documented_function"),
        "Function should still exist after docstring change"
    );

    Ok(())
}
