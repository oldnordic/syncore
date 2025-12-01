//! APEX 2.9-EMBEDDING-REFRESH-DAEMON: Regression Tests (TDD-First)
//!
//! These tests MUST FAIL initially until implementation is created.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

use syncore::code_graph::{CodeGraph, RagGraphAPI};
use syncore::embedding_refresh::{EmbeddingRefreshConfig, EmbeddingRefreshDaemon};
use syncore::fs_watcher::FsEvent;
use syncore::graph::Neo4jClient;
use syncore::vector::{StubEmbeddings, VectorStore};

// Helper to create test RAG API
async fn create_test_rag_api(root: PathBuf) -> Result<RagGraphAPI> {
    let db_path = root.join("test_rag.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "password".to_string());
    let neo4j = Neo4jClient::connect(&uri, &user, &pass).await?;

    Ok(RagGraphAPI::new(code_graph, neo4j))
}

// ============================================================================
// TEST 1: Embedding refresh does not break sync fusion query
// ============================================================================

#[tokio::test]
async fn test_embedding_refresh_does_not_break_sync_fusion_query() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let code_embeddings = Box::new(StubEmbeddings::new(384)?);
    let general_embeddings = Box::new(StubEmbeddings::new(384)?);

    let code_store = Arc::new(Mutex::new(VectorStore::new(code_embeddings)));
    let general_store = Arc::new(Mutex::new(VectorStore::new(general_embeddings)));

    let api = create_test_rag_api(root.clone()).await?;

    // Run sync query before refresh
    let result_before = api.query("test", None, None, Some(10)).await?;
    assert!(
        result_before.entities.len() >= 0,
        "Sync query should work before refresh"
    );

    // Start embedding refresh daemon
    let config = EmbeddingRefreshConfig::default();
    let (daemon, tx) =
        EmbeddingRefreshDaemon::spawn(code_store.clone(), general_store.clone(), config)?;

    // Trigger refresh
    let event = FsEvent::Modified(root.join("src/test.rs"));
    tx.send(event)?;

    sleep(Duration::from_millis(300)).await;

    // Run sync query after refresh
    let result_after = api.query("test", None, None, Some(10)).await?;
    assert!(
        result_after.entities.len() >= 0,
        "Sync query should work after refresh"
    );

    daemon.shutdown().await?;
    Ok(())
}

// ============================================================================
// TEST 2: Embedding refresh does not break streaming fusion query
// ============================================================================

#[tokio::test]
async fn test_embedding_refresh_does_not_break_streaming_fusion_query() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let code_embeddings = Box::new(StubEmbeddings::new(384)?);
    let general_embeddings = Box::new(StubEmbeddings::new(384)?);

    let code_store = Arc::new(Mutex::new(VectorStore::new(code_embeddings)));
    let general_store = Arc::new(Mutex::new(VectorStore::new(general_embeddings)));

    let api = create_test_rag_api(root.clone()).await?;

    // Start embedding refresh daemon
    let config = EmbeddingRefreshConfig::default();
    let (daemon, tx) =
        EmbeddingRefreshDaemon::spawn(code_store.clone(), general_store.clone(), config)?;

    // Trigger refresh in background
    let event = FsEvent::Modified(root.join("src/test.rs"));
    tx.send(event)?;

    // Run streaming query concurrently
    use syncore::code_graph::streaming::StreamingConfig;
    let stream_config = StreamingConfig::default();
    let mut rx = api.query_streaming("test", 10, stream_config).await?;

    // Collect chunks
    let mut received = false;
    while let Some(chunk) = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .ok()
        .flatten()
    {
        received = true;
        if chunk.is_final {
            break;
        }
    }

    assert!(received, "Streaming query should work during refresh");

    daemon.shutdown().await?;
    Ok(())
}

// ============================================================================
// TEST 3: Embedding refresh does not corrupt dual-domain separation
// ============================================================================

#[tokio::test]
async fn test_embedding_refresh_does_not_corrupt_dual_domain_separation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let code_embeddings = Box::new(StubEmbeddings::new(384)?);
    let general_embeddings = Box::new(StubEmbeddings::new(384)?);

    let code_store = Arc::new(Mutex::new(VectorStore::new(code_embeddings)));
    let general_store = Arc::new(Mutex::new(VectorStore::new(general_embeddings)));

    let config = EmbeddingRefreshConfig::default();
    let (daemon, tx) =
        EmbeddingRefreshDaemon::spawn(code_store.clone(), general_store.clone(), config)?;

    // Send mixed CODE and GENERAL updates
    let code_event = FsEvent::Modified(root.join("src/code.rs"));
    let general_event = FsEvent::Modified(root.join("docs/README.md"));

    tx.send(code_event)?;
    tx.send(general_event)?;

    sleep(Duration::from_millis(400)).await;

    // Verify domain separation maintained
    let code_count = code_store.lock().unwrap().len();
    let general_count = general_store.lock().unwrap().len();

    assert!(code_count > 0, "CODE store should have CODE embeddings");
    assert!(
        general_count > 0,
        "GENERAL store should have GENERAL embeddings"
    );

    // Verify no cross-contamination (both stores updated independently)
    // Each should have exactly 1 embedding from their respective domain
    assert_eq!(
        code_count, 1,
        "CODE store should have exactly 1 CODE embedding"
    );
    assert_eq!(
        general_count, 1,
        "GENERAL store should have exactly 1 GENERAL embedding"
    );

    daemon.shutdown().await?;
    Ok(())
}
