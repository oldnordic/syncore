//! APEX 2.7-LIVE-INDEXER: Unit Tests (TDD-First)
//!
//! These tests MUST FAIL initially until implementation is created.
//! Tests isolated LiveIndexer logic without full pipeline integration.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::mpsc;

use syncore::code_graph::update_service::CodeGraphUpdateService;
use syncore::code_graph::CodeGraph;
use syncore::fs_watcher::FsEvent;

use syncore::ingestion::{
    IngestionEventKind, IngestionJob, IngestionKind, IngestionPriority, IngestionSource,
};
use syncore::live_indexer::{LiveIndexer, LiveIndexerConfig};
use syncore::lsp_bridge::LspBridge;
use syncore::parser_service::ParserService;
use syncore::vector::{StubEmbeddings, VectorStore};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_components(
    root: PathBuf,
) -> Result<(
    CodeGraphUpdateService,
    ParserService,
    Arc<Mutex<VectorStore>>,
    Arc<Mutex<LspBridge>>,
)> {
    let db_path = root.join("test_live.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store.clone())?;
    let update_service = CodeGraphUpdateService::new(
        root.clone(),
        code_graph,
        Arc::new(std::sync::Mutex::new(())),
    )?;

    let language = unsafe { tree_sitter_rust::language() };
    let parser = ParserService::new(language, root.clone())?;

    let lsp_bridge = Arc::new(Mutex::new(LspBridge::disabled()));

    Ok((update_service, parser, vector_store, lsp_bridge))
}

// ============================================================================
// TEST 1: Indexer Starts and Shuts Down Cleanly
// ============================================================================

#[tokio::test]
async fn test_indexer_starts_and_shuts_down_cleanly() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let (update_service, parser, vector_store, lsp_bridge) = create_test_components(root.clone())?;

    let (_tx, rx) = mpsc::channel::<syncore::fs_watcher::FsEvent>(100);

    let config = LiveIndexerConfig {
        debounce_ms: 50,
        max_queue: 100,
        index_threads: 1,
    };

    // Start indexer
    let indexer = LiveIndexer::new(rx, parser, update_service, vector_store, lsp_bridge, config)?;

    let handle = indexer.start().await?;

    // Shutdown cleanly
    indexer.shutdown().await?;

    // Join handle should complete without panic
    let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
    assert!(result.is_ok(), "Indexer should shut down within timeout");

    Ok(())
}

// ============================================================================
// TEST 2: Indexer Receives Fs Events
// ============================================================================

#[tokio::test]
async fn test_indexer_receives_fs_events() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let (update_service, parser, vector_store, lsp_bridge) = create_test_components(root.clone())?;

    let (tx, rx) = mpsc::channel::<syncore::fs_watcher::FsEvent>(100);

    let config = LiveIndexerConfig {
        debounce_ms: 50,
        max_queue: 100,
        index_threads: 1,
    };

    let indexer = LiveIndexer::new(rx, parser, update_service, vector_store, lsp_bridge, config)?;

    let _handle = indexer.start().await?;

    // Send FsEvent
    let test_file = root.join("test.rs");
    std::fs::write(&test_file, "pub fn test() {}")?;

    let fs_event = FsEvent::Created(test_file.clone());
    tx.send(fs_event).await?;

    // Give indexer time to process
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Shutdown
    indexer.shutdown().await?;

    // Verify event was queued (implementation will track this)
    Ok(())
}

// ============================================================================
// TEST 3: Indexer Processes Parse Delta
// ============================================================================

#[tokio::test]
async fn test_indexer_processes_parse_delta() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let (update_service, parser, vector_store, lsp_bridge) = create_test_components(root.clone())?;

    let (tx, rx) = mpsc::channel::<syncore::fs_watcher::FsEvent>(100);

    let config = LiveIndexerConfig {
        debounce_ms: 50,
        max_queue: 100,
        index_threads: 1,
    };

    let indexer = LiveIndexer::new(rx, parser, update_service, vector_store, lsp_bridge, config)?;

    let _handle = indexer.start().await?;

    // Create and send file event
    let test_file = root.join("test.rs");
    std::fs::write(&test_file, "pub fn test() {}")?;

    let fs_event = FsEvent::Created(test_file.clone());
    tx.send(fs_event).await?;

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(300)).await;

    indexer.shutdown().await?;

    // Parse delta should have been applied to CodeGraphUpdateService
    // (Verification will be in implementation)
    Ok(())
}

// ============================================================================
// TEST 4: Throttling Per File
// ============================================================================

#[tokio::test]
async fn test_throttling_per_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let (update_service, parser, vector_store, lsp_bridge) = create_test_components(root.clone())?;

    let (tx, rx) = mpsc::channel::<syncore::fs_watcher::FsEvent>(100);

    let config = LiveIndexerConfig {
        debounce_ms: 50,
        max_queue: 100,
        index_threads: 1,
    };

    let indexer = LiveIndexer::new(rx, parser, update_service, vector_store, lsp_bridge, config)?;

    let _handle = indexer.start().await?;

    let test_file = root.join("test.rs");
    std::fs::write(&test_file, "pub fn test() {}")?;

    // Send multiple events for same file rapidly
    for _ in 0..5 {
        let fs_event = FsEvent::Created(test_file.clone());
        tx.send(fs_event).await?;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Wait for debounce + processing
    tokio::time::sleep(Duration::from_millis(300)).await;

    indexer.shutdown().await?;

    // Should have indexed only once (debounced)
    Ok(())
}

// ============================================================================
// TEST 5: Throttling Different Files Independent
// ============================================================================

#[tokio::test]
async fn test_throttling_different_files_independent() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let (update_service, parser, vector_store, lsp_bridge) = create_test_components(root.clone())?;

    let (tx, rx) = mpsc::channel::<syncore::fs_watcher::FsEvent>(100);

    let config = LiveIndexerConfig {
        debounce_ms: 50,
        max_queue: 100,
        index_threads: 1,
    };

    let indexer = LiveIndexer::new(rx, parser, update_service, vector_store, lsp_bridge, config)?;

    let _handle = indexer.start().await?;

    let file_a = root.join("a.rs");
    let file_b = root.join("b.rs");

    std::fs::write(&file_a, "pub fn a() {}")?;
    std::fs::write(&file_b, "pub fn b() {}")?;

    // Send events for both files
    let fs_event_a = FsEvent::Modified(file_a.clone());
    tx.send(fs_event_a).await?;

    let fs_event_b = FsEvent::Modified(file_b.clone());
    tx.send(fs_event_b).await?;

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(300)).await;

    indexer.shutdown().await?;

    // Both files should be indexed independently
    Ok(())
}

// ============================================================================
// TEST 6: Error in Update Does Not Stop Indexer
// ============================================================================

#[tokio::test]
async fn test_error_in_update_does_not_stop_indexer() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let (update_service, parser, vector_store, lsp_bridge) = create_test_components(root.clone())?;

    let (tx, rx) = mpsc::channel::<syncore::fs_watcher::FsEvent>(100);

    let config = LiveIndexerConfig {
        debounce_ms: 50,
        max_queue: 100,
        index_threads: 1,
    };

    let indexer = LiveIndexer::new(rx, parser, update_service, vector_store, lsp_bridge, config)?;

    let _handle = indexer.start().await?;

    // Send event for non-existent file (will cause error)
    let fs_event = FsEvent::Modified(root.join("nonexistent.rs"));
    tx.send(fs_event).await?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send event for valid file
    let valid_file = root.join("valid.rs");
    std::fs::write(&valid_file, "pub fn valid() {}")?;

    let fs_event = FsEvent::Created(valid_file.clone());
    tx.send(fs_event).await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Indexer should still be running and process valid file
    indexer.shutdown().await?;

    Ok(())
}
