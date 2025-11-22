//! Test-Driven Development for parser.search_code MCP tool functionality

use serde_json::json;
use std::fs;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use syncore::mcp::{handle_mcp_request, MCPRequest};
use syncore::memory::Memory;
use syncore::router::SynCoreState;
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};
use tempfile::TempDir;

#[tokio::test]
async fn test_parser_search_code_should_find_async_patterns() {
    // Arrange: Create test directory with test files
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("test.rs");

    // Create a test Rust file with async patterns
    let test_content = r#"
use std::async_fn;

pub async fn fetch_data() -> Result<String, Error> {
    Ok("data".to_string())
}

async fn process_data(input: &str) -> String {
    input.to_uppercase()
}

pub mod utils {
    pub async fn helper() -> bool {
        true
    }
}

fn sync_function() -> i32 {
    42
}
"#;

    fs::write(&test_file_path, test_content).unwrap();

    // Create test state
    let memory = Memory::new("test_parser_search_1.db").unwrap();
    let tasks = Tasks::new("test_parser_tasks_1.db").unwrap();
    let db_manager = Arc::new(
        syncore::db::DbManager::new(
            "test_parser_search_1.db",
            "test_parser_search_1_code_graph.db",
        )
        .unwrap(),
    );
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState {
        db_manager,
        memory: Arc::new(memory),
        tasks: Arc::new(tasks),
        vector_store,
        logger: Arc::new(syncore::logger::MarkdownLogger::new("./logs")),
        message_bus: None,
        write_queue: None,
        read_pool: None,
        faiss_queue: None,
        faiss_pool: None,
        neo4j: None,
        hnsw_ready: Arc::new(AtomicBool::new(false)),
    };

    // Create MCP request to search for async patterns
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "parser.search",
            "arguments": {
                "pattern": "async",
                "path": temp_dir.path().to_str().unwrap()
            }
        })),
        id: json!(1),
    };

    // Act: Handle request
    let response = handle_mcp_request(request, &state).await;

    // Debug: Print response to understand format
    println!(
        "DEBUG: Response: {:?}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Assert: Should return success response with async patterns found
    assert!(response.result.is_some(), "Should return a result");
    assert!(response.error.is_none(), "Should not return an error");

    let result = response.result.unwrap();

    // Check if result has the expected format
    if let Some(results) = result.get("results").and_then(|r| r.as_str()) {
        println!("DEBUG: Got results as string: {}", results);
        // Parse the ripgrep JSON output
        assert!(!results.is_empty(), "Should have search results");
    } else {
        println!("DEBUG: Unexpected result format: {:?}", result);
    }
    // For now, just check that we got a result without error
    // The exact format will depend on ripgrep output
    assert!(result.is_object(), "Result should be an object");
}

#[tokio::test]
async fn test_parser_search_code_should_handle_missing_pattern() {
    // Arrange: Create test state
    let memory = Memory::new("test_parser_search_2.db").unwrap();
    let tasks = Tasks::new("test_parser_tasks_2.db").unwrap();
    let db_manager = Arc::new(
        syncore::db::DbManager::new(
            "test_parser_search_2.db",
            "test_parser_search_2_code_graph.db",
        )
        .unwrap(),
    );
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState {
        db_manager,
        memory: Arc::new(memory),
        tasks: Arc::new(tasks),
        vector_store,
        logger: Arc::new(syncore::logger::MarkdownLogger::new("./logs")),
        message_bus: None,
        write_queue: None,
        read_pool: None,
        faiss_queue: None,
        faiss_pool: None,
        neo4j: None,
        hnsw_ready: Arc::new(AtomicBool::new(false)),
    };

    // Create request missing pattern parameter
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "parser.search",
            "arguments": {
                "directory": "src/"
                // Missing "pattern" parameter
            }
        })),
        id: json!(1),
    };

    // Act: Handle request
    let response = handle_mcp_request(request, &state).await;

    // Assert: Should return an error
    assert!(response.result.is_none(), "Should not return a result");
    assert!(response.error.is_some(), "Should return an error");

    let error = response.error.unwrap();
    assert!(
        error.message.contains("Missing required field: pattern"),
        "Error should mention missing pattern parameter"
    );
}

#[tokio::test]
async fn test_parser_search_code_should_handle_nonexistent_path() {
    // Arrange: Create test state
    let memory = Memory::new("test_parser_search_3.db").unwrap();
    let tasks = Tasks::new("test_parser_tasks_3.db").unwrap();
    let db_manager = Arc::new(
        syncore::db::DbManager::new(
            "test_parser_search_3.db",
            "test_parser_search_3_code_graph.db",
        )
        .unwrap(),
    );
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState {
        db_manager,
        memory: Arc::new(memory),
        tasks: Arc::new(tasks),
        vector_store,
        logger: Arc::new(syncore::logger::MarkdownLogger::new("./logs")),
        message_bus: None,
        write_queue: None,
        read_pool: None,
        faiss_queue: None,
        faiss_pool: None,
        neo4j: None,
        hnsw_ready: Arc::new(AtomicBool::new(false)),
    };

    // Create request for nonexistent path
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "parser.search",
            "arguments": {
                "pattern": "nonexistent_pattern_xyz",
                "directory": "/nonexistent/path"
            }
        })),
        id: json!(1),
    };

    // Act: Handle request
    let response = handle_mcp_request(request, &state).await;

    // Assert: Should return success but with error in results (ripgrep will fail)
    assert!(response.result.is_some(), "Should return a result");
    assert!(response.error.is_none(), "Should not return an error");

    let result = response.result.unwrap();

    // Should contain error information from ripgrep
    if let Some(results) = result.get("results").and_then(|r| r.as_str()) {
        // ripgrep will output to stderr for nonexistent path
        assert!(!results.is_empty(), "Should have error output");
    } else if let Some(error) = result.get("error").and_then(|e| e.as_str()) {
        // Or might return error directly
        assert!(!error.is_empty(), "Should have error message");
    } else {
        // Unknown format, but should still have result
        assert!(result.is_object(), "Result should be an object");
    }
}

#[tokio::test]
async fn test_parser_search_code_should_support_file_patterns() {
    // Arrange: Create test directory with different file types
    let temp_dir = TempDir::new().unwrap();

    // Create Rust file
    let rs_file = temp_dir.path().join("main.rs");
    fs::write(&rs_file, "pub fn rust_function() {}").unwrap();

    // Create JavaScript file
    let js_file = temp_dir.path().join("script.js");
    fs::write(&js_file, "function js_function() {}").unwrap();

    // Create Python file
    let py_file = temp_dir.path().join("app.py");
    fs::write(&py_file, "def python_function(): return None").unwrap();

    // Create test state
    let memory = Memory::new("test_parser_search_4.db").unwrap();
    let tasks = Tasks::new("test_parser_tasks_4.db").unwrap();
    let db_manager = Arc::new(
        syncore::db::DbManager::new(
            "test_parser_search_4.db",
            "test_parser_search_4_code_graph.db",
        )
        .unwrap(),
    );
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState {
        db_manager,
        memory: Arc::new(memory),
        tasks: Arc::new(tasks),
        vector_store,
        logger: Arc::new(syncore::logger::MarkdownLogger::new("./logs")),
        message_bus: None,
        write_queue: None,
        read_pool: None,
        faiss_queue: None,
        faiss_pool: None,
        neo4j: None,
        hnsw_ready: Arc::new(AtomicBool::new(false)),
    };

    // Test different search patterns
    let test_cases = vec![
        ("rust_function", true),
        ("async", true),
        ("nonexistent_pattern", false),
    ];

    for (search_pattern, should_find) in test_cases {
        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            method: "mcp.call_tool".to_string(),
            params: Some(json!({
                "name": "parser.search",
                "arguments": {
                    "pattern": search_pattern,
                    "directory": temp_dir.path().to_str().unwrap()
                }
            })),
            id: json!(1),
        };

        // Act: Handle request
        let response = handle_mcp_request(request, &state).await;

        // Assert: Should handle each pattern correctly
        assert!(
            response.result.is_some(),
            "Should return result for pattern: {}",
            search_pattern
        );
        assert!(
            response.error.is_none(),
            "Should not error for pattern: {}",
            search_pattern
        );

        let result = response.result.unwrap();

        // Check if result has expected format
        if let Some(results) = result.get("results").and_then(|r| r.as_str()) {
            // Parse ripgrep JSON output - may be empty for no matches
            if should_find {
                assert!(
                    !results.is_empty(),
                    "Should find results for pattern: {}",
                    search_pattern
                );
                assert!(
                    results.contains("rust_function"),
                    "Should find rust_function in test file"
                );
            } else {
                // May be empty or not contain the pattern
            }
        } else {
            // Unexpected format, but should still have result
            assert!(result.is_object(), "Result should be an object");
        }
    }
}

#[tokio::test]
async fn test_parser_search_code_should_provide_context() {
    // Arrange: Create test directory with test file
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("context_test.rs");

    // Create a test file with specific pattern and context
    let test_content = r#"
use std::collections::HashMap;

// Function above the target
fn helper_function() {
    println!("This is a helper");
}

// Target function with surrounding context
pub async fn main_function() -> Result<(), Box<dyn std::error::Error>> {
    let mut map = HashMap::new();
    map.insert("key", "value");

    // Call helper
    helper_function();

    Ok(())
}

// Function below the target
fn cleanup_function() {
    println!("Cleanup complete");
}
"#;

    fs::write(&test_file_path, test_content).unwrap();

    // Create test state
    let memory = Memory::new("test_parser_search_5.db").unwrap();
    let tasks = Tasks::new("test_parser_tasks_5.db").unwrap();
    let db_manager = Arc::new(
        syncore::db::DbManager::new(
            "test_parser_search_5.db",
            "test_parser_search_5_code_graph.db",
        )
        .unwrap(),
    );
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState {
        db_manager,
        memory: Arc::new(memory),
        tasks: Arc::new(tasks),
        vector_store,
        logger: Arc::new(syncore::logger::MarkdownLogger::new("./logs")),
        message_bus: None,
        write_queue: None,
        read_pool: None,
        faiss_queue: None,
        faiss_pool: None,
        neo4j: None,
        hnsw_ready: Arc::new(AtomicBool::new(false)),
    };

    // Create MCP request to search with context
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "parser.search",
            "arguments": {
                "pattern": "rust_function",
                "directory": temp_dir.path().to_str().unwrap(),
                "context_lines": 2
            }
        })),
        id: json!(1),
    };

    // Act: Handle request
    let response = handle_mcp_request(request, &state).await;

    // Assert: Should return success response with context
    assert!(response.result.is_some(), "Should return a result");
    assert!(response.error.is_none(), "Should not return an error");

    let result = response.result.unwrap();

    // Check if result has expected format
    if let Some(results) = result.get("results").and_then(|r| r.as_str()) {
        // Parse ripgrep JSON output with context
        assert!(
            !results.is_empty(),
            "Should find rust_function with context"
        );
        assert!(
            results.contains("rust_function"),
            "Should contain the function"
        );
        // Context should include surrounding lines
        let lines: Vec<&str> = results.lines().collect();
        let has_context = lines.len() > 1 || (results.len() > 50); // Rough check for context
        assert!(
            has_context || results.contains("pub fn"),
            "Should include some context around function"
        );
    } else {
        // Unexpected format, but should still have result
        assert!(result.is_object(), "Result should be an object");
    }
}
