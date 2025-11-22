//! TDD Tests for MCP Code Relationship Tools Integration
//! Tests MCP tool handlers for indexing and querying code relationships.

use serde_json::json;
use std::sync::Arc;
use syncore::mcp::McpServer;
use tempfile::{tempdir, TempDir};
use tokio::sync::Mutex;

async fn create_test_server() -> (McpServer, TempDir) {
    let temp = tempdir().unwrap();
    let db_path = temp.path().join("test.db");
    let server = McpServer::new_with_path(db_path).await.unwrap();
    (server, temp)
}

#[tokio::test]
async fn test_mcp_index_file() {
    let (server, _db_temp) = create_test_server().await;

    // Create a temporary Rust file to index
    let temp = tempdir().unwrap();
    let file_path = temp.path().join("sample.rs");
    std::fs::write(
        &file_path,
        r#"
use std::io::Result;
use anyhow::Context;

struct Handler {
    value: i32,
}

impl Default for Handler {
    fn default() -> Self {
        Handler { value: 0 }
    }
}

fn process() -> i32 {
    validate();
    compute()
}

fn validate() {}
fn compute() -> i32 { 42 }
"#,
    )
    .unwrap();

    // Call MCP tool to index the file
    let params = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = server.handle_code_relationship_index(params).await.unwrap();

    // Should report successful indexing
    assert!(result["success"].as_bool().unwrap());
    assert!(result["imports_found"].as_u64().unwrap() >= 2);
    assert!(result["functions_found"].as_u64().unwrap() >= 3);
    assert!(result["structs_found"].as_u64().unwrap() >= 1);
    assert!(result["trait_impls_found"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn test_mcp_query_import_graph() {
    let (server, _db_temp) = create_test_server().await;

    // Pre-populate with some relationships
    let temp = tempdir().unwrap();
    let file1 = temp.path().join("main.rs");
    let file2 = temp.path().join("lib.rs");

    std::fs::write(
        &file1,
        r#"
use std::collections::HashMap;
use crate::lib::process;
fn main() {}
"#,
    )
    .unwrap();

    std::fs::write(
        &file2,
        r#"
use serde::Serialize;
pub fn process() {}
"#,
    )
    .unwrap();

    // Index both files
    server
        .handle_code_relationship_index(json!({
            "file_path": file1.to_str().unwrap()
        }))
        .await
        .unwrap();

    server
        .handle_code_relationship_index(json!({
            "file_path": file2.to_str().unwrap()
        }))
        .await
        .unwrap();

    // Query import graph
    let params = json!({
        "query_type": "imports",
        "file": file1.to_str().unwrap()
    });

    let result = server.handle_code_relationship_query(params).await.unwrap();

    let imports = result["imports"].as_array().unwrap();
    assert!(imports
        .iter()
        .any(|v| v.as_str().unwrap().contains("HashMap")));
    assert!(imports
        .iter()
        .any(|v| v.as_str().unwrap().contains("process")));
}

#[tokio::test]
async fn test_mcp_query_call_graph() {
    let (server, _db_temp) = create_test_server().await;

    let temp = tempdir().unwrap();
    let file_path = temp.path().join("calls.rs");

    std::fs::write(
        &file_path,
        r#"
fn main() {
    init();
    process_request();
}

fn init() {
    setup_config();
}

fn process_request() {
    validate_input();
    execute_action();
}

fn setup_config() {}
fn validate_input() {}
fn execute_action() {}
"#,
    )
    .unwrap();

    // Index the file
    server
        .handle_code_relationship_index(json!({
            "file_path": file_path.to_str().unwrap()
        }))
        .await
        .unwrap();

    // Query call graph for main
    let params = json!({
        "query_type": "calls",
        "function": "main"
    });

    let result = server.handle_code_relationship_query(params).await.unwrap();

    let calls = result["calls"].as_array().unwrap();
    assert!(calls.iter().any(|v| v.as_str().unwrap() == "init"));
    assert!(calls
        .iter()
        .any(|v| v.as_str().unwrap() == "process_request"));
}

#[tokio::test]
async fn test_mcp_similarity_search() {
    let (server, _db_temp) = create_test_server().await;

    let temp = tempdir().unwrap();

    // Create files with similar functions
    let file1 = temp.path().join("auth.rs");
    std::fs::write(
        &file1,
        r#"
fn validate_user_credentials(user: &User, password: &str) -> bool {
    user.check_password(password) && user.is_active()
}
"#,
    )
    .unwrap();

    let file2 = temp.path().join("auth2.rs");
    std::fs::write(
        &file2,
        r#"
fn verify_user_login(usr: &User, pwd: &str) -> bool {
    usr.password_matches(pwd) && usr.active()
}
"#,
    )
    .unwrap();

    let file3 = temp.path().join("math.rs");
    std::fs::write(
        &file3,
        r#"
fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )
    .unwrap();

    // Index all files
    for f in [&file1, &file2, &file3] {
        server
            .handle_code_relationship_index(json!({
                "file_path": f.to_str().unwrap()
            }))
            .await
            .unwrap();
    }

    // Search for similar functions
    let params = json!({
        "query": "validate user authentication credentials",
        "limit": 2
    });

    let result = server.handle_code_similarity_search(params).await.unwrap();

    let matches = result["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 2);

    // Should find auth functions, not math
    let function_names: Vec<&str> = matches
        .iter()
        .map(|m| m["function"].as_str().unwrap())
        .collect();

    assert!(
        function_names.contains(&"validate_user_credentials")
            || function_names.contains(&"verify_user_login")
    );
    assert!(!function_names.contains(&"calculate_sum"));
}

#[tokio::test]
async fn test_mcp_broadcasts_events() {
    let (server, _db_temp) = create_test_server().await;

    // Set up event listener
    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();

    server
        .subscribe_to_events(move |event_name, _data| {
            let events = events_clone.clone();
            let event_name_owned = event_name.to_string();
            Box::pin(async move {
                events.lock().await.push(event_name_owned);
            })
        })
        .await;

    // Index a file
    let temp = tempdir().unwrap();
    let file_path = temp.path().join("event_test.rs");
    std::fs::write(&file_path, "fn test() {}").unwrap();

    server
        .handle_code_relationship_index(json!({
            "file_path": file_path.to_str().unwrap()
        }))
        .await
        .unwrap();

    // Check events were broadcast
    let captured_events = events.lock().await;
    assert!(captured_events.contains(&"relationship_indexed".to_string()));
}

#[tokio::test]
async fn test_mcp_query_implementors() {
    let (server, _db_temp) = create_test_server().await;

    let temp = tempdir().unwrap();
    let file_path = temp.path().join("impls.rs");

    std::fs::write(
        &file_path,
        r#"
struct TypeA;
struct TypeB;
struct TypeC;

impl Clone for TypeA {
    fn clone(&self) -> Self { TypeA }
}

impl Clone for TypeB {
    fn clone(&self) -> Self { TypeB }
}

impl Default for TypeC {
    fn default() -> Self { TypeC }
}
"#,
    )
    .unwrap();

    // Index the file
    server
        .handle_code_relationship_index(json!({
            "file_path": file_path.to_str().unwrap()
        }))
        .await
        .unwrap();

    // Query implementors of Clone trait
    let params = json!({
        "query_type": "implementors",
        "trait_name": "Clone"
    });

    let result = server.handle_code_relationship_query(params).await.unwrap();

    let implementors = result["implementors"].as_array().unwrap();
    assert_eq!(implementors.len(), 2);
    assert!(implementors.iter().any(|v| v.as_str().unwrap() == "TypeA"));
    assert!(implementors.iter().any(|v| v.as_str().unwrap() == "TypeB"));
}
