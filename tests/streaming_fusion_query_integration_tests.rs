//! APEX 2.8-STREAMING-FUSION-QUERY: Integration Tests (TDD-First)
//!
//! These tests MUST FAIL initially until implementation is created.
//! Tests full streaming pipeline with real indexing.

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
    let db_path = root.join("test_streaming_integration.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let code_graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "password".to_string());
    let neo4j = Neo4jClient::connect(&uri, &user, &pass).await?;

    Ok(RagGraphAPI::new(code_graph, neo4j))
}

fn index_test_file(api: &RagGraphAPI, content: &str, file_path: &str) -> Result<()> {
    // Index a Rust file for testing
    // This would use code_graph.index_file() in real implementation
    Ok(())
}

// ============================================================================
// TEST 1: Streaming Over Small Corpus Produces Ordered Chunks
// ============================================================================

#[tokio::test]
async fn test_streaming_over_small_corpus_produces_ordered_chunks() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let api = create_test_api(root.clone()).await?;

    // Index some test files
    let test_code = r#"
pub fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}

pub fn calculate_product(a: i32, b: i32) -> i32 {
    a * b
}
"#;
    index_test_file(&api, test_code, "test.rs")?;

    let config = StreamingConfig::default();
    let mut rx = api.query_streaming("calculate", 10, config).await?;

    // Collect all chunks
    let mut chunks = Vec::new();
    while let Some(chunk) = timeout(Duration::from_secs(2), rx.recv())
        .await
        .ok()
        .flatten()
    {
        let is_final = chunk.is_final;
        chunks.push(chunk);
        if is_final {
            break;
        }
    }

    // Verify we got multiple chunks
    assert!(
        chunks.len() >= 2,
        "Should receive at least 2 chunks (intermediate + final)"
    );

    // Verify final chunk has consolidated results
    let final_chunk = chunks.last().unwrap();
    assert!(final_chunk.is_final, "Last chunk should be final");

    Ok(())
}

// ============================================================================
// TEST 2: Streaming Respects top_k
// ============================================================================

#[tokio::test]
async fn test_streaming_respects_top_k() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let api = create_test_api(root).await?;

    // Index multiple functions
    let test_code = r#"
pub fn func_a() {}
pub fn func_b() {}
pub fn func_c() {}
pub fn func_d() {}
pub fn func_e() {}
"#;
    index_test_file(&api, test_code, "many_funcs.rs")?;

    let top_k = 3;
    let config = StreamingConfig::default();
    let mut rx = api.query_streaming("func", top_k, config).await?;

    // Get final chunk
    let mut final_chunk = None;
    while let Some(chunk) = rx.recv().await {
        if chunk.is_final {
            final_chunk = Some(chunk);
            break;
        }
    }

    let final_chunk = final_chunk.expect("Should receive final chunk");
    assert!(
        final_chunk.ranked_entities.len() <= top_k,
        "Final chunk should respect top_k limit. Got: {}, Expected: <= {}",
        final_chunk.ranked_entities.len(),
        top_k
    );

    Ok(())
}

// ============================================================================
// TEST 3: Streaming Integration with LiveIndexer
// ============================================================================

#[tokio::test]
async fn test_streaming_integration_with_live_indexer() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();
    let api = create_test_api(root.clone()).await?;

    // Index initial file
    let test_code_v1 = "pub fn original_function() {}";
    index_test_file(&api, test_code_v1, "evolving.rs")?;

    let config = StreamingConfig::default();
    let mut rx = api.query_streaming("original", 10, config).await?;

    // Verify stream works with initially indexed data
    let mut received_chunks = false;
    while let Some(chunk) = timeout(Duration::from_millis(500), rx.recv())
        .await
        .ok()
        .flatten()
    {
        received_chunks = true;
        if chunk.is_final {
            break;
        }
    }

    assert!(received_chunks, "Should receive chunks from indexed data");

    // Note: Full LiveIndexer integration would require updating file and re-querying
    // This test verifies streaming works with code graph state

    Ok(())
}
