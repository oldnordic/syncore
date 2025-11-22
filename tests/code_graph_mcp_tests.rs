//! TDD Tests for Code Graph MCP Tools
//! Verifies code_graph_index, code_graph_query, code_graph_explain, and code_graph_impact handlers.

use serde_json::{json, Value};
use std::fs;
use syncore::mcp::code_graph_tools::{
    handle_code_graph_explain, handle_code_graph_impact, handle_code_graph_index,
    handle_code_graph_query,
};
use tempfile::TempDir;

/// Helper to create a test Rust project
fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create lib.rs
    fs::write(
        src_dir.join("lib.rs"),
        r#"
use std::collections::HashMap;

pub mod utils;

pub fn main_function() {
    let result = process_data(42);
    helper_function();
}

fn process_data(x: i32) -> i32 {
    x * 2
}

fn helper_function() {
    utils::utility();
}

pub struct MainStruct {
    data: HashMap<String, i32>,
}

pub trait Processor {
    fn process(&self);
}

impl Processor for MainStruct {
    fn process(&self) {
        // implementation
    }
}
"#,
    )
    .unwrap();

    // Create utils.rs
    fs::write(
        src_dir.join("utils.rs"),
        r#"
pub fn utility() {
    println!("utility called");
}

pub fn another_helper() {
    utility();
}
"#,
    )
    .unwrap();

    temp_dir
}

/// Helper to create test environment with isolated paths
fn setup_test_env(temp_dir: &TempDir) -> Value {
    let db_path = temp_dir.path().join("test.db");
    let vectors_dir = temp_dir.path().join("vectors");
    fs::create_dir_all(&vectors_dir).unwrap();

    json!({
        "db_path": db_path.to_str().unwrap(),
        "vectors_dir": vectors_dir.to_str().unwrap(),
        "namespace": "test_mcp"
    })
}

#[tokio::test]
async fn test_code_graph_index_indexes_directory() {
    let project_dir = create_test_project();
    let env = setup_test_env(&project_dir);

    let params = json!({
        "directory": project_dir.path().join("src").to_str().unwrap(),
        "recursive": true,
        "db_path": env["db_path"],
        "vectors_dir": env["vectors_dir"],
        "namespace": env["namespace"]
    });

    let result = handle_code_graph_index(params)
        .await
        .expect("Should index directory");

    assert!(
        result["success"].as_bool().unwrap(),
        "Indexing should succeed"
    );
    assert!(
        result["files_indexed"].as_u64().unwrap() >= 2,
        "Should index at least 2 files"
    );
    assert!(
        result["functions_found"].as_u64().unwrap() >= 5,
        "Should find at least 5 functions"
    );
    assert!(
        result["calls_found"].as_u64().unwrap() >= 3,
        "Should find at least 3 call edges"
    );
}

#[tokio::test]
async fn test_code_graph_query_returns_imports() {
    let project_dir = create_test_project();
    let env = setup_test_env(&project_dir);

    // First, index the project
    let index_params = json!({
        "directory": project_dir.path().join("src").to_str().unwrap(),
        "recursive": true,
        "db_path": env["db_path"],
        "vectors_dir": env["vectors_dir"],
        "namespace": env["namespace"]
    });
    handle_code_graph_index(index_params).await.unwrap();

    // Query for a specific file
    let query_params = json!({
        "file": "lib.rs",
        "include_imports": true,
        "include_calls": false,
        "include_implementations": false,
        "db_path": env["db_path"],
        "namespace": env["namespace"]
    });

    let result = handle_code_graph_query(query_params)
        .await
        .expect("Should query graph");

    let imports = result["imports"].as_array().unwrap();
    assert!(!imports.is_empty(), "Should find imports");
    assert!(imports
        .iter()
        .any(|i| i.as_str().unwrap().contains("HashMap")));
}

#[tokio::test]
async fn test_code_graph_query_returns_calls() {
    let project_dir = create_test_project();
    let env = setup_test_env(&project_dir);

    // Index first
    let index_params = json!({
        "directory": project_dir.path().join("src").to_str().unwrap(),
        "recursive": true,
        "db_path": env["db_path"],
        "vectors_dir": env["vectors_dir"],
        "namespace": env["namespace"]
    });
    handle_code_graph_index(index_params).await.unwrap();

    // Query calls for main_function
    let query_params = json!({
        "function": "main_function",
        "include_imports": false,
        "include_calls": true,
        "include_implementations": false,
        "db_path": env["db_path"],
        "namespace": env["namespace"]
    });

    let result = handle_code_graph_query(query_params)
        .await
        .expect("Should query calls");

    let calls = result["calls"].as_array().unwrap();
    assert!(calls
        .iter()
        .any(|c| c.as_str().unwrap().contains("process_data")));
    assert!(calls
        .iter()
        .any(|c| c.as_str().unwrap().contains("helper_function")));
}

#[tokio::test]
async fn test_code_graph_query_returns_implementations() {
    let project_dir = create_test_project();
    let env = setup_test_env(&project_dir);

    // Index first
    let index_params = json!({
        "directory": project_dir.path().join("src").to_str().unwrap(),
        "recursive": true,
        "db_path": env["db_path"],
        "vectors_dir": env["vectors_dir"],
        "namespace": env["namespace"]
    });
    handle_code_graph_index(index_params).await.unwrap();

    // Query implementations
    let query_params = json!({
        "struct": "MainStruct",
        "include_imports": false,
        "include_calls": false,
        "include_implementations": true,
        "db_path": env["db_path"],
        "namespace": env["namespace"]
    });

    let result = handle_code_graph_query(query_params)
        .await
        .expect("Should query implementations");

    let impls = result["implementations"].as_array().unwrap();
    assert!(impls
        .iter()
        .any(|i| i.as_str().unwrap().contains("Processor")));
}

#[tokio::test]
async fn test_code_graph_query_returns_semantic_neighbors() {
    let project_dir = create_test_project();
    let env = setup_test_env(&project_dir);

    // Index first
    let index_params = json!({
        "directory": project_dir.path().join("src").to_str().unwrap(),
        "recursive": true,
        "db_path": env["db_path"],
        "vectors_dir": env["vectors_dir"],
        "namespace": env["namespace"]
    });
    handle_code_graph_index(index_params).await.unwrap();

    // Query with semantic search
    let query_params = json!({
        "function": "process_data",
        "include_imports": false,
        "include_calls": false,
        "include_implementations": false,
        "include_semantic": true,
        "semantic_limit": 5,
        "db_path": env["db_path"],
        "vectors_dir": env["vectors_dir"],
        "namespace": env["namespace"]
    });

    let result = handle_code_graph_query(query_params)
        .await
        .expect("Should query semantic neighbors");

    let neighbors = result["semantic_neighbors"].as_array().unwrap();
    assert!(!neighbors.is_empty(), "Should find semantic neighbors");
    // Each neighbor should have a similarity score
    assert!(neighbors[0]["score"].as_f64().is_some());
}

#[tokio::test]
async fn test_code_graph_explain_returns_summary() {
    let project_dir = create_test_project();
    let env = setup_test_env(&project_dir);

    // Index first
    let index_params = json!({
        "directory": project_dir.path().join("src").to_str().unwrap(),
        "recursive": true,
        "db_path": env["db_path"],
        "vectors_dir": env["vectors_dir"],
        "namespace": env["namespace"]
    });
    handle_code_graph_index(index_params).await.unwrap();

    // Get explanation for a function
    let explain_params = json!({
        "function": "main_function",
        "db_path": env["db_path"],
        "vectors_dir": env["vectors_dir"],
        "namespace": env["namespace"]
    });

    let result = handle_code_graph_explain(explain_params)
        .await
        .expect("Should explain function");

    // Should return semantic summary and graph neighbors
    assert!(result["summary"].is_string(), "Should have summary");
    assert!(result["callers"].is_array(), "Should have callers");
    assert!(result["callees"].is_array(), "Should have callees");
    assert!(
        result["related_functions"].is_array(),
        "Should have related functions"
    );

    let callees = result["callees"].as_array().unwrap();
    assert!(callees
        .iter()
        .any(|c| c.as_str().unwrap().contains("process_data")));
}

#[tokio::test]
async fn test_code_graph_impact_returns_affected_nodes() {
    let project_dir = create_test_project();
    let env = setup_test_env(&project_dir);

    // Index first
    let index_params = json!({
        "directory": project_dir.path().join("src").to_str().unwrap(),
        "recursive": true,
        "db_path": env["db_path"],
        "vectors_dir": env["vectors_dir"],
        "namespace": env["namespace"]
    });
    handle_code_graph_index(index_params).await.unwrap();

    // Analyze impact of changing helper_function
    let impact_params = json!({
        "function": "helper_function",
        "db_path": env["db_path"],
        "vectors_dir": env["vectors_dir"],
        "namespace": env["namespace"]
    });

    let result = handle_code_graph_impact(impact_params)
        .await
        .expect("Should analyze impact");

    // Should return affected callgraph nodes
    let affected_functions = result["affected_functions"].as_array().unwrap();
    assert!(
        affected_functions
            .iter()
            .any(|f| f.as_str().unwrap().contains("main_function")),
        "main_function calls helper_function, so it's affected"
    );

    // Should return affected files
    let affected_files = result["affected_files"].as_array().unwrap();
    assert!(!affected_files.is_empty(), "Should find affected files");

    // Should return semantic similarity impact
    let semantic_impact = result["semantic_impact"].as_array().unwrap();
    assert!(
        semantic_impact.len() > 0,
        "Should find semantically similar functions"
    );
}

#[tokio::test]
async fn test_code_graph_impact_transitive_dependencies() {
    let project_dir = create_test_project();
    let env = setup_test_env(&project_dir);

    // Index first
    let index_params = json!({
        "directory": project_dir.path().join("src").to_str().unwrap(),
        "recursive": true,
        "db_path": env["db_path"],
        "vectors_dir": env["vectors_dir"],
        "namespace": env["namespace"]
    });
    handle_code_graph_index(index_params).await.unwrap();

    // Analyze impact of changing utility (called by helper_function, which is called by main_function)
    let impact_params = json!({
        "function": "utility",
        "include_transitive": true,
        "db_path": env["db_path"],
        "vectors_dir": env["vectors_dir"],
        "namespace": env["namespace"]
    });

    let result = handle_code_graph_impact(impact_params)
        .await
        .expect("Should analyze transitive impact");

    let affected = result["affected_functions"].as_array().unwrap();
    // utility -> another_helper (direct)
    // utility -> helper_function -> main_function (transitive via utils::utility call)
    assert!(
        affected
            .iter()
            .any(|f| f.as_str().unwrap().contains("another_helper")),
        "another_helper directly calls utility"
    );
}
