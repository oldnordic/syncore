//! APEX 2.8-STREAMING-FUSION-QUERY: Unit Tests (TDD-First)
//!
//! These tests MUST FAIL initially until implementation is created.
//! Tests isolated streaming logic without full RAGGraph pipeline.

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

async fn create_test_components(root: std::path::PathBuf) -> Result<(CodeGraph, Neo4jClient)> {
    let db_path = root.join("test_streaming.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    // Try to connect to Neo4j, use default connection for tests
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "password".to_string());

    let neo4j = Neo4jClient::connect(&uri, &user, &pass).await?;

    Ok((code_graph, neo4j))
}

// ============================================================================
// TEST 1: Stream First Vector Chunk Arrives
// ============================================================================

#[tokio::test]
async fn test_stream_first_vector_chunk_arrives() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let (code_graph, neo4j) = create_test_components(root).await?;
    let api = RagGraphAPI::new(code_graph, neo4j);

    let config = StreamingConfig {
        chunk_vector: true,
        chunk_graph: true,
        chunk_fusion: true,
    };

    // Start streaming query
    let mut rx = api.query_streaming("test query", 10, config).await?;

    // Wait for first chunk (vector search results)
    let first_chunk = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("First chunk should arrive within 2 seconds")
        .expect("Channel should not be closed");

    // Verify first chunk is from vector search
    assert!(!first_chunk.is_final, "First chunk should not be final");

    Ok(())
}

// ============================================================================
// TEST 2: Stream Second Graph Chunk Arrives
// ============================================================================

#[tokio::test]
async fn test_stream_second_graph_chunk_arrives() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let (code_graph, neo4j) = create_test_components(root).await?;
    let api = RagGraphAPI::new(code_graph, neo4j);

    let config = StreamingConfig::default();
    let mut rx = api.query_streaming("test query", 10, config).await?;

    // Skip first chunk
    let _ = rx.recv().await;

    // Wait for second chunk (graph expansion results)
    let second_chunk = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("Second chunk should arrive")
        .expect("Channel should not be closed");

    assert!(!second_chunk.is_final, "Second chunk should not be final");

    Ok(())
}

// ============================================================================
// TEST 3: Stream Third Fusion Chunk Arrives
// ============================================================================

#[tokio::test]
async fn test_stream_third_fusion_chunk_arrives() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let (code_graph, neo4j) = create_test_components(root).await?;
    let api = RagGraphAPI::new(code_graph, neo4j);

    let config = StreamingConfig::default();
    let mut rx = api.query_streaming("test query", 10, config).await?;

    // Skip first two chunks
    let _ = rx.recv().await;
    let _ = rx.recv().await;

    // Wait for third chunk (fusion results)
    let third_chunk = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("Third chunk should arrive")
        .expect("Channel should not be closed");

    assert!(!third_chunk.is_final, "Third chunk should not be final yet");

    Ok(())
}

// ============================================================================
// TEST 4: Stream Final Chunk Has is_final=true
// ============================================================================

#[tokio::test]
async fn test_stream_final_chunk_has_is_final_true() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let (code_graph, neo4j) = create_test_components(root).await?;
    let api = RagGraphAPI::new(code_graph, neo4j);

    let config = StreamingConfig::default();
    let mut rx = api.query_streaming("test query", 10, config).await?;

    // Collect all chunks
    let mut chunks = Vec::new();
    while let Some(chunk) = rx.recv().await {
        let is_final = chunk.is_final;
        chunks.push(chunk);
        if is_final {
            break;
        }
    }

    // Verify final chunk
    assert!(!chunks.is_empty(), "Should receive at least one chunk");
    let final_chunk = chunks.last().unwrap();
    assert!(final_chunk.is_final, "Last chunk should have is_final=true");

    Ok(())
}

// ============================================================================
// TEST 5: Stream Empty Query Returns Final Empty Chunk
// ============================================================================

#[tokio::test]
async fn test_stream_empty_query_returns_final_empty_chunk() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let (code_graph, neo4j) = create_test_components(root).await?;
    let api = RagGraphAPI::new(code_graph, neo4j);

    let config = StreamingConfig::default();
    let mut rx = api.query_streaming("", 10, config).await?;

    // Should receive at least a final chunk
    let chunk = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("Should receive final chunk quickly")
        .expect("Channel should not be closed");

    assert!(
        chunk.is_final,
        "Empty query should return final chunk immediately"
    );
    assert!(
        chunk.ranked_entities.is_empty(),
        "Empty query should have no entities"
    );

    Ok(())
}

// ============================================================================
// TEST 6: Stream Does Not Block When No Results
// ============================================================================

#[tokio::test]
async fn test_stream_does_not_block_when_no_results() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let (code_graph, neo4j) = create_test_components(root).await?;
    let api = RagGraphAPI::new(code_graph, neo4j);

    let config = StreamingConfig::default();

    // Query should return immediately even with no indexed data
    let mut rx = api
        .query_streaming("nonexistent_function", 10, config)
        .await?;

    // Collect all chunks until we get the final one
    let mut final_chunk = None;
    while let Some(chunk) = timeout(Duration::from_millis(500), rx.recv())
        .await
        .ok()
        .flatten()
    {
        if chunk.is_final {
            final_chunk = Some(chunk);
            break;
        }
    }

    let chunk = final_chunk.expect("Should receive final chunk for no results");
    assert!(chunk.is_final, "Should receive final chunk for no results");

    Ok(())
}
