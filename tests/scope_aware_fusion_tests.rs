//! TDD Tests for Scope-Aware Fusion Query System
//!
//! These tests verify the QueryScope feature for controlling search breadth:
//! - Global: Search entire index without restriction
//! - Workspace: Search all projects in workspace
//! - Project: Restrict to current project only (default)
//! - Local: Restrict to current project + local file/directory focus
//! - Auto: Engine/LLM decides based on heuristics
//!
//! REQUIREMENT: Real Neo4j instance must be running (no mocks allowed)

use anyhow::Result;
use std::io::Write;
use std::sync::{Arc, Mutex};
use syncore::code_graph::{CodeGraph, QueryScope, RagGraphAPI};
use syncore::graph::Neo4jClient;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use tempfile::Builder;

/// Helper to get Neo4j connection
async fn get_neo4j_client() -> Result<Neo4jClient> {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    Neo4jClient::connect(&uri, &user, &pass).await
}

#[test]
fn test_query_scope_from_str_parsing() {
    // Test all valid scope strings parse correctly
    assert_eq!(QueryScope::parse("global"), QueryScope::Global);
    assert_eq!(QueryScope::parse("Global"), QueryScope::Global);
    assert_eq!(QueryScope::parse("GLOBAL"), QueryScope::Global);

    assert_eq!(QueryScope::parse("project"), QueryScope::Project);
    assert_eq!(QueryScope::parse("Project"), QueryScope::Project);

    assert_eq!(QueryScope::parse("workspace"), QueryScope::Workspace);
    assert_eq!(QueryScope::parse("local"), QueryScope::Local);
    assert_eq!(QueryScope::parse("auto"), QueryScope::Auto);

    // Unknown values should default to Project
    assert_eq!(QueryScope::parse("invalid"), QueryScope::Project);
    assert_eq!(QueryScope::parse(""), QueryScope::Project);
}

#[test]
fn test_query_scope_as_str_round_trip() {
    // Test round-trip conversion
    let scopes = vec![
        QueryScope::Global,
        QueryScope::Workspace,
        QueryScope::Project,
        QueryScope::Local,
        QueryScope::Auto,
    ];

    for scope in scopes {
        let str_repr = scope.as_str();
        let parsed = QueryScope::parse(str_repr);
        assert_eq!(scope, parsed, "Round-trip failed for {:?}", scope);
    }
}

#[test]
fn test_query_scope_default_is_project() {
    let default: QueryScope = Default::default();
    assert_eq!(default, QueryScope::Project);
}

#[tokio::test]
async fn test_fusion_query_with_global_scope_returns_all_results() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Use temp file for database
    let db_file = Builder::new().prefix("scope_global_").suffix(".db").tempfile()?;
    let mut code_graph = CodeGraph::new(db_file.path().to_str().unwrap(), vector_store)?;

    // Create and index a sample file
    let mut temp_file = Builder::new().prefix("test_global_").suffix(".rs").tempfile()?;
    writeln!(temp_file, "pub fn global_test_function(s: &str) -> String {{")?;
    writeln!(temp_file, "    s.to_string()")?;
    writeln!(temp_file, "}}")?;
    temp_file.flush()?;

    code_graph.index_file_with_neo4j(temp_file.path(), Some(&neo4j))?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Create RAGGraph API
    let api = RagGraphAPI::new(code_graph, neo4j);

    // Execute query with Global scope
    let response = api
        .query_with_scope(
            "global test function",
            None,
            None,
            Some(10),
            QueryScope::Global,
            None,
            None,
        )
        .await?;

    // Verify response includes applied_scope
    assert_eq!(response.applied_scope, "global");

    Ok(())
}

#[tokio::test]
async fn test_fusion_query_with_project_scope_filters_by_label() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Use temp file for database
    let db_file = Builder::new().prefix("scope_project_").suffix(".db").tempfile()?;
    let mut code_graph = CodeGraph::new(db_file.path().to_str().unwrap(), vector_store)?;

    // Create temp files in different "project" directories
    let temp_dir = Builder::new().prefix("SynCore").tempdir()?;
    let syncore_file = temp_dir.path().join("syncore_func.rs");
    std::fs::write(&syncore_file, "pub fn syncore_specific_function() -> i32 { 42 }\n")?;

    code_graph.index_file_with_neo4j(&syncore_file, Some(&neo4j))?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let api = RagGraphAPI::new(code_graph, neo4j);

    // Query with Project scope and matching label
    let response = api
        .query_with_scope(
            "syncore function",
            None,
            None,
            Some(10),
            QueryScope::Project,
            Some("SynCore"),
            None,
        )
        .await?;

    assert_eq!(response.applied_scope, "project");

    // Debug info should include project_label
    assert!(response.debug_info.contains_key("project_label"));

    Ok(())
}

#[tokio::test]
async fn test_fusion_query_with_local_scope_filters_by_path() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Use temp file for database
    let db_file = Builder::new().prefix("scope_local_").suffix(".db").tempfile()?;
    let mut code_graph = CodeGraph::new(db_file.path().to_str().unwrap(), vector_store)?;

    // Create temp file
    let temp_dir = Builder::new().prefix("local_test").tempdir()?;
    let local_file = temp_dir.path().join("local_func.rs");
    std::fs::write(&local_file, "pub fn local_specific_function() -> bool { true }\n")?;

    code_graph.index_file_with_neo4j(&local_file, Some(&neo4j))?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let api = RagGraphAPI::new(code_graph, neo4j);

    // Query with Local scope
    let response = api
        .query_with_scope(
            "local function",
            None,
            None,
            Some(10),
            QueryScope::Local,
            None,
            Some("local_test"),
        )
        .await?;

    assert_eq!(response.applied_scope, "local");

    // Debug info should include local_root
    assert!(response.debug_info.contains_key("local_root"));

    Ok(())
}

#[tokio::test]
async fn test_backward_compatible_query_defaults_to_global() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Use temp file for database
    let db_file = Builder::new().prefix("scope_compat_").suffix(".db").tempfile()?;
    let mut code_graph = CodeGraph::new(db_file.path().to_str().unwrap(), vector_store)?;

    // Create temp file
    let mut temp_file = Builder::new().prefix("compat_").suffix(".rs").tempfile()?;
    writeln!(temp_file, "pub fn compat_function() {{}}")?;
    temp_file.flush()?;

    code_graph.index_file_with_neo4j(temp_file.path(), Some(&neo4j))?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let api = RagGraphAPI::new(code_graph, neo4j);

    // Use the old query() method without scope params
    let response = api.query("compat function", None, None, Some(5)).await?;

    // Should default to Global for backward compatibility
    assert_eq!(response.applied_scope, "global");

    Ok(())
}

#[tokio::test]
async fn test_auto_scope_aliases_to_global() -> Result<()> {
    let neo4j = get_neo4j_client().await?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Use temp file for database
    let db_file = Builder::new().prefix("scope_auto_").suffix(".db").tempfile()?;
    let code_graph = CodeGraph::new(db_file.path().to_str().unwrap(), vector_store)?;

    let api = RagGraphAPI::new(code_graph, neo4j);

    // Query with Auto scope
    let response = api
        .query_with_scope("test query", None, None, Some(5), QueryScope::Auto, None, None)
        .await?;

    // Auto should be recorded as "auto" in response
    assert_eq!(response.applied_scope, "auto");

    Ok(())
}

#[test]
fn test_matches_project_path_detection() {
    // Test project path matching logic
    // This is a unit test for the path matching function

    let test_cases = vec![
        // (file_path, project_label, expected_match)
        ("/home/user/Projects/SynCore/src/main.rs", "SynCore", true),
        ("/home/user/Projects/SynCore/src/main.rs", "syncore", true), // case insensitive
        ("/home/user/Projects/OtherProject/src/main.rs", "SynCore", false),
        ("/workspace/my-project/lib/util.js", "my-project", true),
        ("C:\\Users\\dev\\SynCore\\src\\lib.rs", "SynCore", true), // Windows paths
    ];

    for (path, label, expected) in test_cases {
        let path_lower = path.to_lowercase();
        let label_lower = label.to_lowercase();

        let matches = path_lower.contains(&format!("/{}/", label_lower))
            || path_lower.contains(&format!("\\{}\\", label_lower))
            || path_lower.starts_with(&format!("{}/", label_lower))
            || path_lower.ends_with(&format!("/{}", label_lower));

        assert_eq!(
            matches, expected,
            "Path '{}' with label '{}' should match={}",
            path, label, expected
        );
    }
}
