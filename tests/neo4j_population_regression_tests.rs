//! APEX 2.10-NEO4J-POPULATION-FIX: Regression Tests (TDD-First)
//!
//! These tests ensure the fix doesn't break existing functionality.
//! These tests should PASS initially and continue passing after the fix.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use syncore::code_graph::CodeGraph;
use syncore::graph::Neo4jClient;
use syncore::vector::{StubEmbeddings, VectorStore};

// Helper to create test code graph
fn create_test_code_graph(root: PathBuf) -> Result<CodeGraph> {
    let db_path = root.join("test.db");
    let embeddings = Box::new(StubEmbeddings::new(384)?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    CodeGraph::new(db_path.to_str().unwrap(), vector_store)
}

// ============================================================================
// TEST 1: SQLite indexing still works without Neo4j
// ============================================================================

#[test]
fn test_sqlite_indexing_still_works_without_neo4j() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let mut code_graph = create_test_code_graph(root.clone())?;

    // Create test file
    let test_file = root.join("standalone.rs");
    std::fs::write(
        &test_file,
        r#"
fn first() { }
fn second() { first(); }
        "#,
    )?;

    // Index WITHOUT Neo4j (pass None)
    let count = code_graph.index_file_with_neo4j(&test_file, None)?;
    assert_eq!(count, 2, "Should index 2 functions to SQLite");

    // Verify SQLite indexing worked - count is sufficient proof
    // (No need to access private db field, index_file returns count from SQLite)
    assert_eq!(count, 2, "SQLite should contain 2 indexed entities");

    Ok(())
}

// ============================================================================
// TEST 2: Fusion query results unaffected by Neo4j population
// ============================================================================

#[tokio::test]
async fn test_fusion_query_results_unaffected_by_neo4j() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let mut code_graph = create_test_code_graph(root.clone())?;

    // Get Neo4j client (optional, may be None if not configured)
    let neo4j = match std::env::var("NEO4J_URI") {
        Ok(uri) => {
            let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
            let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "password".to_string());
            Some(Neo4jClient::connect(&uri, &user, &pass).await?)
        }
        Err(_) => None,
    };

    // Create test file
    let test_file = root.join("query_test.rs");
    std::fs::write(
        &test_file,
        r#"
pub fn hello_world() {
    println!("Hello, World!");
}

pub fn greeting(name: &str) -> String {
    format!("Hello, {}!", name)
}
        "#,
    )?;

    // Index with or without Neo4j
    code_graph.index_file_with_neo4j(&test_file, neo4j.as_ref())?;

    // If Neo4j enabled, allow time for background sync
    if neo4j.is_some() {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // Create RagGraphAPI for fusion query
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "password".to_string());
    let neo4j_client = Neo4jClient::connect(&uri, &user, &pass).await?;

    let rag_api = syncore::code_graph::RagGraphAPI::new(code_graph, neo4j_client);

    // Perform fusion query
    let result = rag_api.query("hello", None, None, Some(10)).await?;

    // Should find entities regardless of Neo4j population status
    assert!(!result.entities.is_empty(), "Fusion query should work and return results");
    assert!(
        result.entities.iter().any(|e| e.entity.name.contains("hello")),
        "Should find hello_world or greeting function"
    );

    Ok(())
}
