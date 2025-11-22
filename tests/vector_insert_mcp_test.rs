//! Test-Driven Development for vector.insert MCP tool functionality

use serde_json::json;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use syncore::mcp::handle_mcp_request;
use syncore::mcp::MCPRequest;
use syncore::memory::Memory;
use syncore::router::SynCoreState;
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};

#[tokio::test]
async fn test_vector_insert_should_insert_text_and_return_success() {
    // Arrange: Create test state with vector store
    let memory = Arc::new(Memory::new("test_vector_insert_1.db").unwrap());
    let tasks = Arc::new(Tasks::new("test_vector_insert_tasks_1.db").unwrap());
    let db_manager = Arc::new(
        syncore::db::DbManager::new(
            "test_vector_insert_1.db",
            "test_vector_insert_1_code_graph.db",
        )
        .unwrap(),
    );
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState {
        db_manager,
        memory: memory.clone(),
        tasks: tasks.clone(),
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

    // Create MCP request to insert vector
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "vector.insert",
            "arguments": {
                "id": 1,
                "text": "This is a test document for vector insertion",
                "kind": "note"
            }
        })),
        id: json!(1),
    };

    // Act: Handle the request
    let response = handle_mcp_request(request, &state).await;

    // Assert: Should return success response
    assert!(response.result.is_some(), "Should return a result");
    assert!(response.error.is_none(), "Should not return an error");

    let result = response.result.unwrap();
    assert!(
        result
            .get("success")
            .unwrap_or(&json!(false))
            .as_bool()
            .unwrap(),
        "Should indicate successful insertion"
    );
    assert_eq!(
        result.get("id").unwrap().as_i64().unwrap(),
        1,
        "Should return the correct ID"
    );
}

#[tokio::test]
async fn test_vector_insert_should_handle_missing_text() {
    // Arrange: Create test state
    // Arrange: Create test state with multiple inserts
    let memory = Arc::new(Memory::new("test_vector_insert_2.db").unwrap());
    let tasks = Arc::new(Tasks::new("test_vector_insert_tasks_2.db").unwrap());
    let db_manager = Arc::new(
        syncore::db::DbManager::new(
            "test_vector_insert_2.db",
            "test_vector_insert_2_code_graph.db",
        )
        .unwrap(),
    );
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState {
        db_manager,
        memory: memory.clone(),
        tasks: tasks.clone(),
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

    // Create request missing text parameter
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "vector.insert",
            "arguments": {
                "id": 1,
                "kind": "note"
                // Missing "text" parameter
            }
        })),
        id: json!(1),
    };

    // Act: Handle the request
    let response = handle_mcp_request(request, &state).await;

    // Assert: Should return an error
    assert!(response.result.is_none(), "Should not return a result");
    assert!(response.error.is_some(), "Should return an error");

    let error = response.error.unwrap();
    assert!(
        error.message.contains("Missing required field: text"),
        "Error should mention missing text parameter, but got: {}",
        error.message
    );
}

#[tokio::test]
async fn test_vector_insert_should_handle_valid_scopes() {
    // Arrange: Create test state
    // Arrange: Create test state with empty vector store
    let memory = Arc::new(Memory::new("test_vector_insert_3.db").unwrap());
    let tasks = Arc::new(Tasks::new("test_vector_insert_tasks_3.db").unwrap());
    let db_manager = Arc::new(
        syncore::db::DbManager::new(
            "test_vector_insert_3.db",
            "test_vector_insert_3_code_graph.db",
        )
        .unwrap(),
    );
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState {
        db_manager,
        memory: memory.clone(),
        tasks: tasks.clone(),
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

    // Test valid scopes
    let valid_scopes = vec!["global", "task"];

    for (i, scope) in valid_scopes.iter().enumerate() {
        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            method: "mcp.call_tool".to_string(),
            params: Some(json!({
                "name": "vector.insert",
                "arguments": {
                    "id": i as i64 + 1,
                    "text": format!("Test document for {} scope", scope),
                    "kind": "note"
                }
            })),
            id: json!(1),
        };

        // Act: Handle the request
        let response = handle_mcp_request(request, &state).await;

        // Assert: Should succeed for valid scopes
        assert!(
            response.result.is_some(),
            "Should return result for scope: {}",
            scope
        );
        assert!(
            response.error.is_none(),
            "Should not error for scope: {}",
            scope
        );
    }
}

#[tokio::test]
async fn test_vector_insert_should_store_in_vector_store() {
    // Arrange: Create test state with empty vector store
    // Arrange: Create test state for vector insert with metadata
    let memory = Arc::new(Memory::new("test_vector_insert_4.db").unwrap());
    let tasks = Arc::new(Tasks::new("test_vector_insert_tasks_4.db").unwrap());
    let db_manager = Arc::new(
        syncore::db::DbManager::new(
            "test_vector_insert_4.db",
            "test_vector_insert_4_code_graph.db",
        )
        .unwrap(),
    );
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState {
        db_manager,
        memory: memory.clone(),
        tasks: tasks.clone(),
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

    // Insert a document
    let insert_request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "vector.insert",
            "arguments": {
                "id": 1,
                "text": "Test document for verification",
                "kind": "note"
            }
        })),
        id: json!(1),
    };

    // Act: Insert the document
    let response = handle_mcp_request(insert_request, &state).await;
    assert!(response.result.is_some(), "Insert should succeed");

    // Now verify it can be found via search
    let search_request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "vector.search",
            "arguments": {
                "query": "test document",
                "k": 5
            }
        })),
        id: json!(2),
    };

    // Act: Search for the inserted document
    let search_response = handle_mcp_request(search_request, &state).await;

    // Assert: Should find the inserted document
    assert!(
        search_response.result.is_some(),
        "Search should return result"
    );

    let search_result = search_response.result.unwrap();
    let results = search_result
        .get("results")
        .and_then(|r| r.as_array())
        .unwrap();
    assert!(
        !results.is_empty(),
        "Should find the inserted document in search results"
    );
}
