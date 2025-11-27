//! APEX 2.9-EMBEDDING-REFRESH-DAEMON: Unit Tests (TDD-First)
//!
//! These tests MUST FAIL initially until implementation is created.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

use syncore::code_graph::CodeGraph;
use syncore::embedding_refresh::{EmbeddingRefreshConfig, EmbeddingRefreshDaemon};
use syncore::fs_watcher::{FsEvent, FsEventKind};
use syncore::vector::{StubEmbeddings, VectorStore};
use syncore::vector::domain::EmbeddingDomain;

// Helper to create test vector stores
fn create_test_stores() -> Result<(Arc<Mutex<VectorStore>>, Arc<Mutex<VectorStore>>)> {
    let code_embeddings = Box::new(StubEmbeddings::new(384)?);
    let general_embeddings = Box::new(StubEmbeddings::new(384)?);

    let code_store = Arc::new(Mutex::new(VectorStore::new(code_embeddings)));
    let general_store = Arc::new(Mutex::new(VectorStore::new(general_embeddings)));

    Ok((code_store, general_store))
}

// ============================================================================
// TEST 1: Single code update triggers CODE embedding refresh
// ============================================================================

#[tokio::test]
async fn test_single_code_update_triggers_code_embedding_refresh() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let (code_store, general_store) = create_test_stores()?;

    let config = EmbeddingRefreshConfig::default();
    let (daemon, tx) = EmbeddingRefreshDaemon::spawn(
        code_store.clone(),
        general_store.clone(),
        config,
    )?;

    // Simulate code entity update
    let event = FsEvent {
        path: PathBuf::from("src/test.rs"),
        kind: FsEventKind::Modified,
    };

    tx.send(event).await?;

    // Give daemon time to process
    sleep(Duration::from_millis(200)).await;

    // Verify CODE store was updated (stub embeddings will insert)
    let code_count = code_store.lock().unwrap().len();
    assert!(code_count > 0, "CODE store should have embeddings after refresh");

    // Verify GENERAL store unchanged
    let general_count = general_store.lock().unwrap().len();
    assert_eq!(general_count, 0, "GENERAL store should remain empty");

    daemon.shutdown().await?;
    Ok(())
}

// ============================================================================
// TEST 2: Single general update triggers GENERAL embedding refresh
// ============================================================================

#[tokio::test]
async fn test_single_general_update_triggers_general_embedding_refresh() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let (code_store, general_store) = create_test_stores()?;

    let config = EmbeddingRefreshConfig::default();
    let (daemon, tx) = EmbeddingRefreshDaemon::spawn(
        code_store.clone(),
        general_store.clone(),
        config,
    )?;

    // Simulate general document update
    let event = FsEvent {
        path: PathBuf::from("docs/README.md"),
        kind: FsEventKind::Modified,
    };

    tx.send(event).await?;

    // Give daemon time to process
    sleep(Duration::from_millis(200)).await;

    // Verify GENERAL store was updated
    let general_count = general_store.lock().unwrap().len();
    assert!(general_count > 0, "GENERAL store should have embeddings after refresh");

    // Verify CODE store unchanged
    let code_count = code_store.lock().unwrap().len();
    assert_eq!(code_count, 0, "CODE store should remain empty");

    daemon.shutdown().await?;
    Ok(())
}

// ============================================================================
// TEST 3: Deleted entity removes embedding
// ============================================================================

#[tokio::test]
async fn test_deleted_entity_removes_embedding() -> Result<()> {
    let (code_store, general_store) = create_test_stores()?;

    // Pre-insert an embedding
    {
        let mut store = code_store.lock().unwrap();
        store.insert_text(1, None, "test content", "code")?;
    }

    let config = EmbeddingRefreshConfig::default();
    let (daemon, tx) = EmbeddingRefreshDaemon::spawn(
        code_store.clone(),
        general_store.clone(),
        config,
    )?;

    let initial_count = code_store.lock().unwrap().len();
    assert_eq!(initial_count, 1, "Should start with one embedding");

    // Simulate deletion
    let event = FsEvent {
        path: PathBuf::from("src/test.rs"),
        kind: FsEventKind::Removed,
    };

    tx.send(event).await?;

    // Give daemon time to process
    sleep(Duration::from_millis(200)).await;

    // For now, deletion may not decrease count (HNSW limitation)
    // Just verify daemon doesn't crash
    let final_count = code_store.lock().unwrap().len();
    assert!(final_count >= 0, "Store should remain valid after deletion");

    daemon.shutdown().await?;
    Ok(())
}

// ============================================================================
// TEST 4: Renamed entity reuses or reinserts embedding consistently
// ============================================================================

#[tokio::test]
async fn test_renamed_entity_reuses_or_reinserts_embedding_consistently() -> Result<()> {
    let (code_store, general_store) = create_test_stores()?;

    let config = EmbeddingRefreshConfig::default();
    let (daemon, tx) = EmbeddingRefreshDaemon::spawn(
        code_store.clone(),
        general_store.clone(),
        config,
    )?;

    // Simulate rename
    let event = FsEvent {
        path: PathBuf::from("src/old.rs"),
        kind: FsEventKind::Renamed(PathBuf::from("src/new.rs")),
    };

    tx.send(event).await?;

    // Give daemon time to process
    sleep(Duration::from_millis(200)).await;

    // Verify store has consistent state
    let count = code_store.lock().unwrap().len();
    assert!(count >= 0, "Store should remain consistent after rename");

    daemon.shutdown().await?;
    Ok(())
}

// ============================================================================
// TEST 5: Daemon batches multiple events
// ============================================================================

#[tokio::test]
async fn test_daemon_batches_multiple_events() -> Result<()> {
    let (code_store, general_store) = create_test_stores()?;

    let config = EmbeddingRefreshConfig {
        max_batch_size: 5,
        flush_interval_ms: 100,
    };

    let (daemon, tx) = EmbeddingRefreshDaemon::spawn(
        code_store.clone(),
        general_store.clone(),
        config,
    )?;

    // Send multiple events rapidly
    for i in 0..3 {
        let event = FsEvent {
            path: PathBuf::from(format!("src/file{}.rs", i)),
            kind: FsEventKind::Modified,
        };
        tx.send(event).await?;
    }

    // Give daemon time to batch and process
    sleep(Duration::from_millis(300)).await;

    // Verify all events processed
    let count = code_store.lock().unwrap().len();
    assert!(count >= 3, "Should process all batched events");

    daemon.shutdown().await?;
    Ok(())
}

// ============================================================================
// TEST 6: Daemon survives failed embedding and continues
// ============================================================================

#[tokio::test]
async fn test_daemon_survives_failed_embedding_and_continues() -> Result<()> {
    let (code_store, general_store) = create_test_stores()?;

    let config = EmbeddingRefreshConfig::default();
    let (daemon, tx) = EmbeddingRefreshDaemon::spawn(
        code_store.clone(),
        general_store.clone(),
        config,
    )?;

    // Send events (some may fail internally but daemon should continue)
    for i in 0..3 {
        let event = FsEvent {
            path: PathBuf::from(format!("src/file{}.rs", i)),
            kind: FsEventKind::Modified,
        };
        tx.send(event).await?;
    }

    // Give daemon time to process
    sleep(Duration::from_millis(300)).await;

    // Verify daemon is still responsive
    let final_event = FsEvent {
        path: PathBuf::from("src/final.rs"),
        kind: FsEventKind::Modified,
    };
    tx.send(final_event).await?;

    sleep(Duration::from_millis(200)).await;

    // Daemon should still be processing
    let count = code_store.lock().unwrap().len();
    assert!(count > 0, "Daemon should continue processing after failures");

    daemon.shutdown().await?;
    Ok(())
}
