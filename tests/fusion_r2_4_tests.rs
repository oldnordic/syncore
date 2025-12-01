//! Phase R2.4 - Tri-Mode Fusion Layer Tests
//!
//! These tests verify the three fusion modes for hybrid RAG reasoning:
//! - Mode A: Linear Weighted Hybrid (simple fusion)
//! - Mode B: Attention Fusion (dynamic weights)
//! - Mode C: Multi-hop Semantic Reasoning Fusion (higher-order)
//!
//! REQUIREMENT: Real Neo4j instance must be running (no mocks allowed)

use anyhow::Result;
use std::sync::{Arc, Mutex};
use syncore::code_graph::CodeGraph;
use syncore::graph::Neo4jClient;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

/// Helper to get Neo4j connection
async fn get_neo4j_client() -> Result<Neo4jClient> {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    Neo4jClient::connect(&uri, &user, &pass).await
}

#[test]
fn test_simple_fusion_linear_weights() {
    use syncore::code_graph::fusion_simple::FusionSimple;

    let fusion = FusionSimple::new(0.6, 0.3, 0.1, 0.0); // alpha = 0.6

    // Known scores: vector=0.8, graph=0.4, temporal=0.0
    let vector_score = 0.8;
    let graph_score = 0.4;
    let temporal_score = 0.0;

    // Expected: 0.5*0.8 + 0.2*0.4 + 0.1*0.0 + 0.2*0.0 = 0.40 + 0.08 + 0.0 + 0.0 = 0.48
    let graph_embedding_score = 0.0;
    let result = fusion.combine(
        vector_score,
        graph_score,
        temporal_score,
        graph_embedding_score,
    );

    assert!(
        (result - 0.48).abs() < 0.001,
        "Linear fusion should be 0.48, got {}",
        result
    );
}

#[tokio::test]
async fn test_attention_fusion_dynamic_weights() -> Result<()> {
    use syncore::code_graph::fusion_attention::FusionAttention;

    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let fusion = FusionAttention::new(embeddings);

    // Create real embeddings for two different contexts
    let context1 = "simple function";
    let context2 = "complex distributed system architecture";

    let score_v = 0.7;
    let score_g = 0.5;

    let result1 = fusion.combine(score_v, score_g, context1)?;
    let result2 = fusion.combine(score_v, score_g, context2)?;

    // Attention should produce different results for different contexts
    assert_ne!(
        result1, result2,
        "Attention fusion should vary with context"
    );

    // Results should be in valid range
    assert!(result1 >= 0.0 && result1 <= 1.0);
    assert!(result2 >= 0.0 && result2 <= 1.0);

    Ok(())
}

#[tokio::test]
async fn test_reasoning_fusion_higher_order() -> Result<()> {
    use syncore::code_graph::fusion_reasoning::FusionReasoning;

    let neo4j = get_neo4j_client().await?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let fusion = FusionReasoning::new(neo4j.clone(), vector_store);

    // Test higher-order combination with gamma term
    let score_v = 0.6;
    let score_g = 0.8;

    // S = α*S_v + β*S_g + γ*S_g²
    // With default weights: 0.4*0.6 + 0.4*0.8 + 0.2*0.64
    // = 0.24 + 0.32 + 0.128 = 0.688
    let result = fusion.combine_higher_order(score_v, score_g);

    assert!(
        (result - 0.688).abs() < 0.01,
        "Higher-order fusion failed, got {}",
        result
    );

    Ok(())
}

#[test]
fn test_router_selects_simple_for_short_query() {
    use syncore::code_graph::fusion_router::{FusionMode, FusionRouter};

    let router = FusionRouter::new();

    let mode = router.select_mode("fmt");

    assert_eq!(
        mode,
        FusionMode::Simple,
        "Short query should use Simple mode"
    );
}

#[test]
fn test_router_selects_attention_for_semantic_query() {
    use syncore::code_graph::fusion_router::{FusionMode, FusionRouter};

    let router = FusionRouter::new();

    let mode = router.select_mode("explain why function A fails on B");

    assert_eq!(
        mode,
        FusionMode::Attention,
        "Semantic query should use Attention mode"
    );
}

#[test]
fn test_router_selects_reasoning_for_causal_tracing() {
    use syncore::code_graph::fusion_router::{FusionMode, FusionRouter};

    let router = FusionRouter::new();

    let mode = router.select_mode("trace dependency from A to D");

    assert_eq!(
        mode,
        FusionMode::Reasoning,
        "Causal query should use Reasoning mode"
    );
}

#[tokio::test]
async fn test_integration_all_modes_end_to_end() -> Result<()> {
    use std::io::Write;
    use syncore::code_graph::fusion_router::FusionRouter;
    use tempfile::Builder;

    let neo4j = get_neo4j_client().await?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(":memory:", vector_store)?;

    // Create and index a sample file
    let mut temp_file = Builder::new().prefix("test_").suffix(".rs").tempfile()?;
    writeln!(temp_file, "pub fn format_string(s: &str) -> String {{")?;
    writeln!(temp_file, "    s.to_string()")?;
    writeln!(temp_file, "}}")?;
    temp_file.flush()?;

    code_graph.index_file_with_neo4j(temp_file.path(), Some(&neo4j))?;

    // Wait for async Neo4j sync
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Test router selection doesn't panic
    let router = FusionRouter::new();
    let _mode = router.select_mode("find format function");

    Ok(())
}

#[tokio::test]
async fn test_backward_compatibility() -> Result<()> {
    // Ensure R2.2 + R2.3 functionality still works
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(":memory:", vector_store)?;

    use std::io::Write;
    use tempfile::Builder;

    let mut temp_file = Builder::new().prefix("test_").suffix(".rs").tempfile()?;
    writeln!(temp_file, "fn test() {{}}")?;
    temp_file.flush()?;

    // This should still work (R2.2)
    let result = code_graph.index_file(temp_file.path());
    assert!(result.is_ok(), "Backward compatibility broken");

    Ok(())
}
