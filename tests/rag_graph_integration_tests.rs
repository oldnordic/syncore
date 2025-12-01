//! Phase R2.5 - Full RAGGraph Integration Tests
//!
//! These tests verify the integration of tri-mode fusion (R2.4) with RAGGraph
//! as a first-class SynCore tool callable by mini/worker models.
//!
//! Test Coverage:
//! 1. Simple mode end-to-end (short query)
//! 2. Attention mode end-to-end (semantic query)
//! 3. Reasoning mode end-to-end (multi-file causal query)
//! 4. Router auto-selection (mode_hint=None)
//! 5. MCP tool integration
//! 6. Backward compatibility (R2.2, R2.3, R2.4)
//!
//! REQUIREMENT: Real Neo4j instance must be running for full integration tests

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

/// Helper to create a test code graph with sample data
async fn setup_test_code_graph() -> Result<(CodeGraph, Neo4jClient)> {
    use std::io::Write;
    use tempfile::Builder;

    let neo4j = get_neo4j_client().await?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(":memory:", vector_store)?;

    // Create sample file: format_string function
    let mut temp_file = Builder::new().prefix("test_").suffix(".rs").tempfile()?;
    writeln!(temp_file, "/// Formats a string by converting to uppercase")?;
    writeln!(temp_file, "pub fn format_string(s: &str) -> String {{")?;
    writeln!(temp_file, "    s.to_uppercase()")?;
    writeln!(temp_file, "}}")?;
    temp_file.flush()?;

    code_graph.index_file_with_neo4j(temp_file.path(), Some(&neo4j))?;

    // Wait for async Neo4j sync
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    Ok((code_graph, neo4j))
}

#[tokio::test]
async fn test_rag_graph_simple_mode_end_to_end() -> Result<()> {
    use syncore::code_graph::rag_graph_api::RagGraphAPI;

    let (code_graph, neo4j) = setup_test_code_graph().await?;

    let api = RagGraphAPI::new(code_graph, neo4j);

    // Short query should trigger Simple mode
    let result = api
        .query(
            "fmt",          // query
            None,           // namespace
            Some("simple"), // mode_hint
            Some(5),        // top_k
        )
        .await?;

    // Verify result structure
    assert!(result.entities.len() <= 5, "Should respect top_k limit");
    assert_eq!(result.selected_mode, "simple");
    assert!(result.debug_info.contains_key("vector_score"));
    assert!(result.debug_info.contains_key("graph_score"));

    Ok(())
}

#[tokio::test]
async fn test_rag_graph_attention_mode_end_to_end() -> Result<()> {
    use syncore::code_graph::rag_graph_api::RagGraphAPI;

    let (code_graph, neo4j) = setup_test_code_graph().await?;

    let api = RagGraphAPI::new(code_graph, neo4j);

    // Semantic query should trigger Attention mode
    let result = api
        .query(
            "explain why format function converts to uppercase",
            None,
            Some("attention"),
            Some(10),
        )
        .await?;

    assert!(result.entities.len() <= 10);
    assert_eq!(result.selected_mode, "attention");
    assert!(result.debug_info.contains_key("attention_alpha"));
    assert!(result.debug_info.contains_key("context_complexity"));

    Ok(())
}

#[tokio::test]
async fn test_rag_graph_reasoning_mode_end_to_end() -> Result<()> {
    use syncore::code_graph::rag_graph_api::RagGraphAPI;

    let (code_graph, neo4j) = setup_test_code_graph().await?;

    let api = RagGraphAPI::new(code_graph, neo4j);

    // Causal query should trigger Reasoning mode
    let result = api
        .query(
            "trace dependency from format_string to uppercase",
            None,
            Some("reasoning"),
            Some(10),
        )
        .await?;

    assert!(result.entities.len() <= 10);
    assert_eq!(result.selected_mode, "reasoning");
    assert!(result.debug_info.contains_key("gamma_term"));
    assert!(result.debug_info.contains_key("higher_order_score"));

    Ok(())
}

#[tokio::test]
async fn test_rag_graph_router_mode_auto_selection() -> Result<()> {
    use syncore::code_graph::rag_graph_api::RagGraphAPI;

    let (code_graph, neo4j) = setup_test_code_graph().await?;

    let api = RagGraphAPI::new(code_graph, neo4j);

    // Test 1: Short query -> Simple
    let result1 = api.query("fmt", None, None, Some(5)).await?;
    assert_eq!(result1.selected_mode, "simple");

    // Test 2: Semantic query -> Attention (must be 4+ tokens with semantic keyword)
    let result2 = api.query("explain why format function works", None, None, Some(5)).await?;
    assert_eq!(result2.selected_mode, "attention");

    // Test 3: Causal query -> Reasoning
    let result3 = api.query("trace dependency from A to B", None, None, Some(5)).await?;
    assert_eq!(result3.selected_mode, "reasoning");

    Ok(())
}

#[tokio::test]
async fn test_rag_graph_tool_mcp_integration() -> Result<()> {
    // This test verifies the tool can be called via MCP protocol
    // We'll test the request/response JSON schema

    use syncore::code_graph::rag_graph_api::RagGraphQueryRequest;

    let request = RagGraphQueryRequest {
        query: "find format function".to_string(),
        namespace: None,
        mode_hint: Some("simple".to_string()),
        top_k: Some(10),
        scope: None,
        project_label: None,
        local_root: None,
    };

    // Verify JSON serialization works
    let json = serde_json::to_string(&request)?;
    assert!(json.contains("find format function"));
    assert!(json.contains("simple"));

    // Verify deserialization works
    let parsed: RagGraphQueryRequest = serde_json::from_str(&json)?;
    assert_eq!(parsed.query, "find format function");
    assert_eq!(parsed.mode_hint, Some("simple".to_string()));
    assert_eq!(parsed.top_k, Some(10));

    Ok(())
}

#[tokio::test]
async fn test_rag_graph_backwards_compatibility() -> Result<()> {
    // Verify R2.2, R2.3, R2.4 functionality still works after R2.5 integration

    use std::io::Write;
    use syncore::code_graph::fusion_router::FusionRouter;
    use syncore::code_graph::fusion_simple::FusionSimple;
    use tempfile::Builder;

    // Test R2.2: Basic indexing still works
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(":memory:", vector_store)?;

    let mut temp_file = Builder::new().prefix("test_").suffix(".rs").tempfile()?;
    writeln!(temp_file, "fn test() {{}}")?;
    temp_file.flush()?;

    let result = code_graph.index_file(temp_file.path());
    assert!(result.is_ok(), "R2.2 backward compatibility broken");

    // Test R2.3: Neo4j integration still works
    let neo4j = get_neo4j_client().await?;
    let embeddings2 = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store2 = Arc::new(Mutex::new(VectorStore::new(embeddings2)));
    let mut code_graph2 = CodeGraph::new(":memory:", vector_store2)?;

    let mut temp_file2 = Builder::new().prefix("test_").suffix(".rs").tempfile()?;
    writeln!(temp_file2, "fn test2() {{}}")?;
    temp_file2.flush()?;

    let result2 = code_graph2.index_file_with_neo4j(temp_file2.path(), Some(&neo4j));
    assert!(result2.is_ok(), "R2.3 backward compatibility broken");

    // Test R2.4: Fusion modes still work
    let fusion = FusionSimple::new(0.6, 0.3, 0.1, 0.0);
    let score = fusion.combine(0.8, 0.4, 0.0, 0.0);
    assert!((score - 0.64).abs() < 0.001, "R2.4 backward compatibility broken");

    let router = FusionRouter::new();
    let mode = router.select_mode("fmt");
    assert_eq!(
        mode,
        syncore::code_graph::fusion_router::FusionMode::Simple,
        "R2.4 router backward compatibility broken"
    );

    Ok(())
}
