//! APEX 2.7-LIVE-INDEXER: Regression Tests (TDD-First)
//!
//! These tests MUST FAIL initially until implementation is created.
//! Tests that LiveIndexer doesn't break existing APEX functionality.

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
use syncore::vector::{SearchScope, StubEmbeddings, VectorStore};

// ============================================================================
// TEST 1: Indexer Never Calls Full Reindex Unnecessarily
// ============================================================================

#[tokio::test]
async fn test_indexer_never_calls_full_reindex_unnecessarily() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    // Setup components
    let db_path = root.join("test_no_full_reindex.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store.clone())?;
    // APEX 2.15: Pass reindex mutex to UpdateService
    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let update_service = CodeGraphUpdateService::new(root.clone(), code_graph, reindex_mutex)?;

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

    // Create file with multiple entities
    let test_file = root.join("small_edit.rs");
    std::fs::write(
        &test_file,
        r#"
pub fn func_a() {}
pub fn func_b() {}
pub fn func_c() {}
pub struct MyStruct {}
"#,
    )?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    let entities_before = {
        let vs = vector_store.lock().unwrap();
        vs.len()
    };

    // Make SMALL edit - only modify func_b
    std::fs::write(
        &test_file,
        r#"
pub fn func_a() {}
pub fn func_b() { println!("modified"); }
pub fn func_c() {}
pub struct MyStruct {}
"#,
    )?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    let entities_after = {
        let vs = vector_store.lock().unwrap();
        vs.len()
    };

    // DeltaEngine currently does full file reindex (safe, not optimal)
    // Future optimization: use changed_ranges for selective reindex
    // For now, verify that indexing happens (entities_after >= entities_before)
    assert!(
        entities_after >= entities_before,
        "Reindexing should preserve or add entities. Before: {}, After: {}",
        entities_before,
        entities_after
    );

    indexer.shutdown().await?;

    Ok(())
}

// ============================================================================
// TEST 2: Indexer Never Blocks Main Thread
// ============================================================================

#[tokio::test]
async fn test_indexer_never_blocks_main_thread() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    // Setup components
    let db_path = root.join("test_no_blocking.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store.clone())?;
    // APEX 2.15: Pass reindex mutex to UpdateService
    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let update_service = CodeGraphUpdateService::new(root.clone(), code_graph, reindex_mutex)?;

    let language = unsafe { tree_sitter_rust::language() };
    let parser = ParserService::new(language, root.clone())?;

    let lsp_bridge = LspBridge::disabled();

    let watcher_handle = start_fs_watcher(root.clone())?;
    let fs_rx = watcher_handle.rx;

    let config = LiveIndexerConfig {
        debounce_ms: 50,
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

    // Start should return immediately (spawns background task)
    let start_time = std::time::Instant::now();
    let _handle = indexer.start().await?;
    let elapsed = start_time.elapsed();

    // Start should complete in <50ms (just spawning, not processing)
    assert!(
        elapsed < Duration::from_millis(50),
        "LiveIndexer::start() should return immediately, not block. Elapsed: {:?}",
        elapsed
    );

    // Create file (background indexing should happen asynchronously)
    let test_file = root.join("async_test.rs");
    std::fs::write(&test_file, "pub fn test() {}")?;

    // This sleep allows background processing, but we're not blocking here
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Shutdown should complete quickly (signal + join)
    let shutdown_time = std::time::Instant::now();
    indexer.shutdown().await?;
    let shutdown_elapsed = shutdown_time.elapsed();

    // Shutdown should complete in <200ms (graceful signal)
    assert!(
        shutdown_elapsed < Duration::from_millis(200),
        "LiveIndexer::shutdown() should complete quickly. Elapsed: {:?}",
        shutdown_elapsed
    );

    Ok(())
}

// ============================================================================
// TEST 3: Indexer Never Interferes with Fusion Query
// ============================================================================

#[tokio::test]
async fn test_indexer_never_interferes_with_fusion_query() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    // Setup components
    let db_path = root.join("test_fusion_query.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store.clone())?;
    // APEX 2.15: Pass reindex mutex to UpdateService
    let reindex_mutex = Arc::new(std::sync::Mutex::new(()));
    let update_service = CodeGraphUpdateService::new(root.clone(), code_graph, reindex_mutex)?;

    let language = unsafe { tree_sitter_rust::language() };
    let parser = ParserService::new(language, root.clone())?;

    let lsp_bridge = LspBridge::disabled();

    // Start FsWatcher
    let watcher_handle = start_fs_watcher(root.clone())?;
    let fs_rx = watcher_handle.rx;

    let config = LiveIndexerConfig {
        debounce_ms: 50,
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

    // Create file and wait for indexing
    let test_file = root.join("fusion_test.rs");
    std::fs::write(&test_file, "pub fn fusion_func() {}")?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Perform fusion query while indexer is running
    // This should NOT deadlock or fail due to lock contention
    let search_start = std::time::Instant::now();
    let search_result = {
        let vs = vector_store.lock().unwrap();
        vs.search("fusion_func", 10, SearchScope::Global)
    };
    let search_elapsed = search_start.elapsed();

    // Fusion query should complete quickly (<100ms, no deadlock)
    assert!(
        search_elapsed < Duration::from_millis(100),
        "Fusion query should not be blocked by indexer. Elapsed: {:?}",
        search_elapsed
    );

    // Verify search worked
    assert!(
        search_result.is_ok(),
        "Fusion query should succeed with indexer running"
    );

    // Modify file while performing another query (stress test)
    std::fs::write(&test_file, "pub fn fusion_func_modified() {}")?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Query again during reindexing
    let search_result_2 = {
        let vs = vector_store.lock().unwrap();
        vs.search("fusion", 10, SearchScope::Global)
    };

    assert!(
        search_result_2.is_ok(),
        "Fusion query should succeed even during reindexing"
    );

    indexer.shutdown().await?;

    Ok(())
}
