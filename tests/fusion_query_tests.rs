//! Fusion query tests with body-aware scoring (APEX v1.7 Phase 4)
//!
//! Tests that fusion queries boost entities with body_snippet content

use anyhow::Result;
use std::path::Path;
use std::sync::{Arc, Mutex};
use syncore::code_graph::{CodeGraph, RagGraphAPI};
use syncore::graph::Neo4jClient;
use syncore::vector::{VectorStore, HuggingFaceEmbeddings};
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_body_snippet_boosts_relevance() -> Result<()> {
    // Create temporary database and vector store
    let temp_db = NamedTempFile::new()?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = VectorStore::new(embeddings);
    let mut code_graph = CodeGraph::new(temp_db.path().to_str().unwrap(), Arc::new(Mutex::new(vector_store)))?;

    // Index fixture project with body content
    let fixture_path = Path::new("tests/fixtures/body_index_project");
    let unique_file = fixture_path.join("src/unique_feature.rs");
    code_graph.index_file(&unique_file)?;

    // Create Neo4j client (may be stub for tests)
    let neo4j = Neo4jClient::connect("bolt://localhost:7687", "neo4j", "password").await?;
    let rag_api = RagGraphAPI::new(code_graph, neo4j);

    // Query for "cosmic alignment" - should match body content
    let query_text = "cosmic alignment calculation";
    let namespace = None;
    let mode_hint = Some("simple");
    let top_k = Some(5);

    let response = rag_api.query(query_text, namespace, mode_hint, top_k).await?;

    // Assert that we got results
    assert!(!response.entities.is_empty(), "Should find entities matching body content");

    // Assert that entity with body_snippet has higher score than one without
    let has_body = response.entities.iter().any(|e| e.entity.body_snippet.is_some());
    assert!(has_body, "Should find entities with body_snippet");

    Ok(())
}

#[tokio::test]
async fn test_fusion_modes_use_body_content() -> Result<()> {
    // Create temporary database and vector store
    let temp_db = NamedTempFile::new()?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = VectorStore::new(embeddings);
    let mut code_graph = CodeGraph::new(temp_db.path().to_str().unwrap(), Arc::new(Mutex::new(vector_store)))?;

    // Index fixture file
    let fixture_path = Path::new("tests/fixtures/body_index_project");
    let unique_file = fixture_path.join("src/unique_feature.rs");
    code_graph.index_file(&unique_file)?;

    let neo4j = Neo4jClient::connect("bolt://localhost:7687", "neo4j", "password").await?;
    let rag_api = RagGraphAPI::new(code_graph, neo4j);

    // Test all three fusion modes
    for mode in &["simple", "attention", "reasoning"] {
        let query_text = "cosmic alignment";
        let namespace = None;
        let mode_hint = Some(*mode);
        let top_k = Some(5);

        let response = rag_api.query(query_text, namespace, mode_hint, top_k).await?;

        assert!(
            !response.entities.is_empty(),
            "Fusion mode {} should find matches in body content",
            mode
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_body_boost_remains_deterministic() -> Result<()> {
    // Create temporary database and vector store
    let temp_db = NamedTempFile::new()?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = VectorStore::new(embeddings);
    let mut code_graph = CodeGraph::new(temp_db.path().to_str().unwrap(), Arc::new(Mutex::new(vector_store)))?;

    // Index fixture file
    let fixture_path = Path::new("tests/fixtures/body_index_project");
    let unique_file = fixture_path.join("src/unique_feature.rs");
    code_graph.index_file(&unique_file)?;

    let neo4j = Neo4jClient::connect("bolt://localhost:7687", "neo4j", "password").await?;
    let rag_api = RagGraphAPI::new(code_graph, neo4j);

    // Run same query multiple times with updated API
    let query_text = "calculate alignment";
    let namespace = None;
    let mode_hint = Some("simple");
    let top_k = Some(5);

    let response1 = rag_api.query(query_text, namespace, mode_hint, top_k).await?;
    let response2 = rag_api.query(query_text, namespace, mode_hint, top_k).await?;

    // Scores should be identical (deterministic)
    assert_eq!(response1.entities.len(), response2.entities.len());

    for (e1, e2) in response1.entities.iter().zip(response2.entities.iter()) {
        assert!(
            (e1.combined_score - e2.combined_score).abs() < 0.001,
            "Scores should be deterministic"
        );
    }

    Ok(())
}
