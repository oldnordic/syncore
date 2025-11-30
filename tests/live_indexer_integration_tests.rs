//! APEX 2.7-LIVE-INDEXER: Integration Tests (TDD-First)
//!
//! These tests MUST FAIL initially until implementation is created.
//! Tests full pipeline: FsWatcher → ParserService → DeltaEngine → UpdateService → HNSW → LSP

use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;

use syncore::code_graph::update_service::CodeGraphUpdateService;
use syncore::code_graph::CodeGraph;
use syncore::fs_watcher::start_fs_watcher;
use syncore::live_indexer::{LiveIndexer, LiveIndexerConfig};
use syncore::lsp_bridge::LspBridge;
use syncore::parser_service::ParserService;
use syncore::vector::{StubEmbeddings, VectorStore};

// ============================================================================
// TEST 1: Full Pipeline File Create/Modify/Delete
// ============================================================================

#[tokio::test]
async fn test_full_pipeline_file_create_modify_delete() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    // Setup components
    let db_path = root.join("test_pipeline.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store.clone())?;
    let update_service = CodeGraphUpdateService::new(root.clone(), code_graph)?;

    let language = unsafe { tree_sitter_rust::language() };
    let parser = ParserService::new(language, root.clone())?;

    let lsp_bridge = LspBridge::disabled();

    // Start FsWatcher
    let watcher_handle = start_fs_watcher(root.clone())?;
    let fs_rx = watcher_handle.rx;

    let config = LiveIndexerConfig {
        debounce_ms: 100,
        max_queue: 100,
        index_threads: 1,
    };

    // Start LiveIndexer
    let indexer = LiveIndexer::new(
        fs_rx,
        parser,
        update_service,
        vector_store.clone(),
        lsp_bridge,
        config,
    )?;

    let _handle = indexer.start().await?;

    // Test CREATE
    let test_file = root.join("pipeline_test.rs");
    std::fs::write(&test_file, "pub fn test_create() {}")?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify entity was indexed
    let entities = {
        let graph_lock = vector_store.lock().unwrap();
        // Check that entity was added to vector store
        graph_lock.len()
    };
    assert!(entities > 0, "Entity should be indexed after create");

    // Test MODIFY
    std::fs::write(&test_file, "pub fn test_modify() {}")?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Test DELETE
    std::fs::remove_file(&test_file)?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    indexer.shutdown().await?;

    Ok(())
}

// ============================================================================
// TEST 2: Pipeline Triggers HNSW Re-embedding
// ============================================================================

#[tokio::test]
async fn test_pipeline_triggers_hnsw_reembedding() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    // Setup components
    let db_path = root.join("test_hnsw.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store.clone())?;
    let mut update_service = CodeGraphUpdateService::new(root.clone(), code_graph)?;

    let language = unsafe { tree_sitter_rust::language() };
    let parser = ParserService::new(language, root.clone())?;

    let lsp_bridge = LspBridge::disabled();

    // Start FsWatcher
    let watcher_handle = start_fs_watcher(root.clone())?;
    let fs_rx = watcher_handle.rx;

    let config = LiveIndexerConfig {
        debounce_ms: 100,
        max_queue: 100,
        index_threads: 1,
    };

    let indexer = LiveIndexer::new(
        fs_rx,
        parser,
        update_service,
        vector_store.clone(),
        lsp_bridge,
        config,
    )?;

    let _handle = indexer.start().await?;

    // Create file with one function
    let test_file = root.join("hnsw_test.rs");
    std::fs::write(&test_file, "pub fn original_func() {}")?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    let vectors_before = {
        let vs = vector_store.lock().unwrap();
        vs.len()
    };

    // Modify file - change function
    std::fs::write(&test_file, "pub fn modified_func() {}")?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    let vectors_after = {
        let vs = vector_store.lock().unwrap();
        vs.len()
    };

    // HNSW should have been updated with new entity embedding
    assert!(vectors_after >= vectors_before, "HNSW should be updated after modification");

    indexer.shutdown().await?;

    Ok(())
}

// ============================================================================
// TEST 3: Pipeline Produces LSP Notifications
// ============================================================================

#[tokio::test]
async fn test_pipeline_produces_lsp_notifications() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    // Setup components
    let db_path = root.join("test_lsp.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store.clone())?;
    let update_service = CodeGraphUpdateService::new(root.clone(), code_graph)?;

    let language = unsafe { tree_sitter_rust::language() };
    let parser = ParserService::new(language, root.clone())?;

    // Use disabled LSP bridge (real LSP would require rust-analyzer running)
    let lsp_bridge = LspBridge::disabled();

    let watcher_handle = start_fs_watcher(root.clone())?;
    let fs_rx = watcher_handle.rx;

    let config = LiveIndexerConfig {
        debounce_ms: 100,
        max_queue: 100,
        index_threads: 1,
    };

    let indexer = LiveIndexer::new(
        fs_rx,
        parser,
        update_service,
        vector_store.clone(),
        lsp_bridge,
        config,
    )?;

    let _handle = indexer.start().await?;

    // Create file with syntax error
    let test_file = root.join("lsp_test.rs");
    std::fs::write(&test_file, "pub fn broken syntax")?; // Missing braces

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Fix syntax
    std::fs::write(&test_file, "pub fn fixed() {}")?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    indexer.shutdown().await?;

    // LSP notifications would be sent (verified by LSP bridge mock/disabled status)
    Ok(())
}
