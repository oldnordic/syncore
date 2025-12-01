//! APEX 2.8-STREAMING-FUSION-QUERY: Regression Tests (TDD-First)
//!
//! These tests MUST FAIL initially until implementation is created.
//! Tests that streaming doesn't break existing functionality.

use anyhow::Result;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::time::{timeout, Duration};

use syncore::code_graph::rag_graph_api::RagGraphAPI;
use syncore::code_graph::streaming::{RagStreamingChunk, StreamingConfig};
use syncore::code_graph::CodeGraph;
use syncore::graph::Neo4jClient;
use syncore::vector::{StubEmbeddings, VectorStore};

// ============================================================================
// Helper Functions
// ============================================================================

async fn create_test_api(root: std::path::PathBuf) -> Result<RagGraphAPI> {
    let db_path = root.join("test_streaming_regression.db");
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
// TEST 1: Streaming Does Not Break Sync Query
// ============================================================================

#[tokio::test]
async fn test_streaming_does_not_break_sync_query() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let api = create_test_api(root).await?;

    // First, verify sync query still works
    let sync_result = api.query("test query", None, None, Some(10)).await?;
    assert!(sync_result.entities.len() >= 0, "Sync query should work");

    // Then verify streaming query also works
    let config = StreamingConfig::default();
    let mut rx = api.query_streaming("test query", 10, config).await?;

    let mut received_streaming = false;
    while let Some(chunk) = timeout(Duration::from_millis(500), rx.recv()).await.ok().flatten() {
        received_streaming = true;
        if chunk.is_final {
            break;
        }
    }

    assert!(received_streaming, "Streaming query should also work");

    // Finally, verify sync query still works after streaming
    let sync_result_after = api.query("test query", None, None, Some(10)).await?;
    assert!(sync_result_after.entities.len() >= 0, "Sync query should still work after streaming");

    Ok(())
}

// ============================================================================
// TEST 2: Streaming Works If Graph Empty
// ============================================================================

#[tokio::test]
async fn test_streaming_works_if_graph_empty() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let api = create_test_api(root).await?;

    // Query empty graph
    let config = StreamingConfig::default();
    let mut rx = api.query_streaming("nonexistent", 10, config).await?;

    // Collect all chunks until we get the final one
    let mut final_chunk = None;
    while let Some(chunk) = timeout(Duration::from_millis(500), rx.recv()).await.ok().flatten() {
        if chunk.is_final {
            final_chunk = Some(chunk);
            break;
        }
    }

    let chunk = final_chunk.expect("Should receive final chunk");
    assert!(chunk.is_final, "Should receive final chunk");
    assert!(chunk.ranked_entities.is_empty(), "Empty graph should have no entities");

    Ok(())
}

// ============================================================================
// TEST 3: Streaming Yields Deterministic Results
// ============================================================================

#[tokio::test]
async fn test_streaming_yields_deterministic_results() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let api = create_test_api(root).await?;

    let config = StreamingConfig::default();

    // Run same query twice
    let mut rx1 = api.query_streaming("test", 10, config.clone()).await?;
    let mut rx2 = api.query_streaming("test", 10, config).await?;

    // Collect final chunks from both streams
    let mut final1 = None;
    while let Some(chunk) = timeout(Duration::from_millis(500), rx1.recv()).await.ok().flatten() {
        if chunk.is_final {
            final1 = Some(chunk);
            break;
        }
    }

    let mut final2 = None;
    while let Some(chunk) = timeout(Duration::from_millis(500), rx2.recv()).await.ok().flatten() {
        if chunk.is_final {
            final2 = Some(chunk);
            break;
        }
    }

    let final1 = final1.expect("Should receive final chunk from first stream");
    let final2 = final2.expect("Should receive final chunk from second stream");

    // Verify deterministic results (same number of entities)
    assert_eq!(
        final1.ranked_entities.len(),
        final2.ranked_entities.len(),
        "Streaming should yield deterministic results"
    );

    // If there are entities, verify scores match
    if !final1.ranked_entities.is_empty() {
        for (e1, e2) in final1.ranked_entities.iter().zip(final2.ranked_entities.iter()) {
            assert_eq!(e1.combined_score, e2.combined_score, "Scores should be deterministic");
        }
    }

    Ok(())
}
