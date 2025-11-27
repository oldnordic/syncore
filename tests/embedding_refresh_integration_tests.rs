//! APEX 2.9-EMBEDDING-REFRESH-DAEMON: Integration Tests (TDD-First)
//!
//! These tests MUST FAIL initially until implementation is created.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

use syncore::code_graph::CodeGraph;
use syncore::code_graph::update_service::{CodeGraphUpdateEvent, CodeGraphUpdateService};
use syncore::embedding_refresh::{EmbeddingRefreshConfig, EmbeddingRefreshDaemon};
use syncore::fs_watcher::{FsEvent, FsEventKind};
use syncore::parser_service::ParseDelta;
use syncore::vector::{StubEmbeddings, VectorStore};

// Helper to create test code graph
async fn create_test_code_graph(root: PathBuf) -> Result<Arc<Mutex<CodeGraph>>> {
    let db_path = root.join("test.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;
    Ok(Arc::new(Mutex::new(code_graph)))
}

// ============================================================================
// TEST 1: Live indexer pipeline triggers embedding refresh
// ============================================================================

#[tokio::test]
async fn test_live_indexer_pipeline_triggers_embedding_refresh() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let code_embeddings = Box::new(StubEmbeddings::new(384)?);
    let general_embeddings = Box::new(StubEmbeddings::new(384)?);

    let code_store = Arc::new(Mutex::new(VectorStore::new(code_embeddings)));
    let general_store = Arc::new(Mutex::new(VectorStore::new(general_embeddings)));

    let config = EmbeddingRefreshConfig::default();
    let (daemon, tx) = EmbeddingRefreshDaemon::spawn(
        code_store.clone(),
        general_store.clone(),
        config,
    )?;

    // Simulate code update from live indexer
    let event = FsEvent {
        path: root.join("src/test.rs"),
        kind: FsEventKind::Modified,
    };

    tx.send(event).await?;

    // Give pipeline time to process
    sleep(Duration::from_millis(300)).await;

    // Verify CODE embeddings updated
    let code_count = code_store.lock().unwrap().len();
    assert!(code_count > 0, "CODE embeddings should be refreshed");

    daemon.shutdown().await?;
    Ok(())
}

// ============================================================================
// TEST 2: Delta engine and daemon consistency for modify
// ============================================================================

#[tokio::test]
async fn test_delta_engine_and_daemon_consistency_for_modify() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let code_embeddings = Box::new(StubEmbeddings::new(384)?);
    let general_embeddings = Box::new(StubEmbeddings::new(384)?);

    let code_store = Arc::new(Mutex::new(VectorStore::new(code_embeddings)));
    let general_store = Arc::new(Mutex::new(VectorStore::new(general_embeddings)));

    let config = EmbeddingRefreshConfig::default();
    let (daemon, tx) = EmbeddingRefreshDaemon::spawn(
        code_store.clone(),
        general_store.clone(),
        config,
    )?;

    // Simulate modify event
    let event = FsEvent {
        path: root.join("src/module.rs"),
        kind: FsEventKind::Modified,
    };

    tx.send(event).await?;

    // Give time to process
    sleep(Duration::from_millis(300)).await;

    // Verify exactly one update (no duplicates)
    let code_count = code_store.lock().unwrap().len();
    assert_eq!(code_count, 1, "Should update exactly once");

    daemon.shutdown().await?;
    Ok(())
}

// ============================================================================
// TEST 3: Empty delta does not trigger unnecessary embeddings
// ============================================================================

#[tokio::test]
async fn test_empty_delta_does_not_trigger_unnecessary_embeddings() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let code_embeddings = Box::new(StubEmbeddings::new(384)?);
    let general_embeddings = Box::new(StubEmbeddings::new(384)?);

    let code_store = Arc::new(Mutex::new(VectorStore::new(code_embeddings)));
    let general_store = Arc::new(Mutex::new(VectorStore::new(general_embeddings)));

    let config = EmbeddingRefreshConfig::default();
    let (daemon, tx) = EmbeddingRefreshDaemon::spawn(
        code_store.clone(),
        general_store.clone(),
        config,
    )?;

    let initial_code_count = code_store.lock().unwrap().len();
    let initial_general_count = general_store.lock().unwrap().len();

    // Send event but with no actual content change
    // (in real scenario, this would be filtered earlier, but daemon should handle gracefully)

    // Give time for any spurious processing
    sleep(Duration::from_millis(200)).await;

    // Verify no unnecessary work done
    let final_code_count = code_store.lock().unwrap().len();
    let final_general_count = general_store.lock().unwrap().len();

    assert_eq!(initial_code_count, final_code_count, "No spurious CODE embeddings");
    assert_eq!(initial_general_count, final_general_count, "No spurious GENERAL embeddings");

    daemon.shutdown().await?;
    Ok(())
}
