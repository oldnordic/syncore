use serde_json::json;
use std::fs;
use std::sync::{Arc, Mutex};
use syncore::mcp::{handle_mcp_request, MCPRequest};
use syncore::memory::Memory;
use syncore::router::SynCoreState;
use syncore::tasks::Tasks;
use syncore::vector::VectorStore;

#[tokio::test]
async fn test_mcp_list_tools() {
    // Clean up any existing test files
    let _ = fs::remove_file("test_mcp.db");
    let _ = fs::remove_dir_all("test_mcp_cache");

    let memory = Memory::new("test_mcp.db").unwrap();
    let tasks = Tasks::new("test_mcp_tasks.db").unwrap();
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let state = SynCoreState::with_dual_stores(vector_store.clone(), vector_store).unwrap();

    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.list_tools".to_string(),
        params: None,
        id: json!("test-1"),
    };

    let response = handle_mcp_request(request, &state).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let tools = response.result.unwrap();
    assert!(tools.is_array());
    let tools_array = tools.as_array().unwrap();
    assert_eq!(tools_array.len(), 8); // memory.store, memory.query, task.create, vector.insert, vector.search, logs.tail, parser.analyze, parser.search

    // Check that memory.store is in the list
    let memory_store = tools_array
        .iter()
        .find(|tool| tool.get("name").and_then(|n| n.as_str()) == Some("memory.store"));
    assert!(memory_store.is_some());
}

#[tokio::test]
async fn test_mcp_call_tool_memory_store() {
    let _ = fs::remove_file("test_mcp_memory.db");
    let _ = fs::remove_dir_all("test_mcp_memory_cache");

    let memory = Memory::new("test_mcp_memory.db").unwrap();
    let tasks = Tasks::new("test_mcp_memory_tasks.db").unwrap();
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let state = SynCoreState::with_dual_stores(vector_store.clone(), vector_store).unwrap();

    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "memory.store",
            "arguments": {
                "key": "test_key",
                "value": "test_value"
            }
        })),
        id: json!("test-2"),
    };

    let response = handle_mcp_request(request, &state).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let result = response.result.unwrap();
    // Should get {"success": true} response from memory store
    assert_eq!(result, serde_json::json!({"success": true}));
}

#[tokio::test]
async fn test_mcp_call_tool_memory_query() {
    let _ = fs::remove_file("test_mcp_query.db");
    let _ = fs::remove_dir_all("test_mcp_query_cache");

    let memory = Memory::new("test_mcp_query.db").unwrap();
    let tasks = Tasks::new("test_mcp_query_tasks.db").unwrap();
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let state = SynCoreState::with_dual_stores(vector_store.clone(), vector_store).unwrap();

    // First store a value
    let _ = state.memory.store("query_test", "query_value");

    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "memory.query",
            "arguments": {
                "key": "query_test"
            }
        })),
        id: json!("test-3"),
    };

    let response = handle_mcp_request(request, &state).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let result = response.result.unwrap();
    // Memory query returns {"value": "..."} format
    assert_eq!(result, serde_json::json!({"value": "query_value"}));
}

#[tokio::test]
async fn test_mcp_call_tool_task_create() {
    let _ = fs::remove_file("test_mcp_task.db");
    let _ = fs::remove_dir_all("test_mcp_task_cache");

    let memory = Memory::new("test_mcp_task.db").unwrap();
    let tasks = Tasks::new("test_mcp_task_tasks.db").unwrap();
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let state = SynCoreState::with_dual_stores(vector_store.clone(), vector_store).unwrap();

    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "task.create",
            "arguments": {
                "goal": "Test task goal"
            }
        })),
        id: json!("test-4"),
    };

    let response = handle_mcp_request(request, &state).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let result = response.result.unwrap();
    println!("DEBUG: Task create response: {:?}", result);
    assert!(result.get("success").and_then(|s| s.as_bool()) == Some(true));
    assert!(result.get("task_id").and_then(|id| id.as_i64()).is_some());
}

#[tokio::test]
async fn test_mcp_invalid_tool() {
    let _ = fs::remove_file("test_mcp_invalid.db");
    let _ = fs::remove_dir_all("test_mcp_invalid_cache");

    let memory = Memory::new("test_mcp_invalid.db").unwrap();
    let tasks = Tasks::new("test_mcp_invalid_tasks.db").unwrap();
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let state = SynCoreState::with_dual_stores(vector_store.clone(), vector_store).unwrap();

    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(json!({
            "name": "nonexistent.tool",
            "arguments": {}
        })),
        id: json!("test-5"),
    };

    let response = handle_mcp_request(request, &state).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.result.is_none());
    assert!(response.error.is_some());

    let error = response.error.unwrap();
    assert_eq!(error.code, -32602); // Invalid params (no schema found)
    assert!(error.message.contains("No schema found"));
}

#[tokio::test]
async fn test_mcp_invalid_method() {
    let _ = fs::remove_file("test_mcp_method.db");
    let _ = fs::remove_dir_all("test_mcp_method_cache");

    let memory = Memory::new("test_mcp_method.db").unwrap();
    let tasks = Tasks::new("test_mcp_method_tasks.db").unwrap();
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let state = SynCoreState::with_dual_stores(vector_store.clone(), vector_store).unwrap();

    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.nonexistent_method".to_string(),
        params: None,
        id: json!("test-6"),
    };

    let response = handle_mcp_request(request, &state).await;

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.result.is_none());
    assert!(response.error.is_some());

    let error = response.error.unwrap();
    assert_eq!(error.code, -32601); // Method not found
    assert!(error.message.contains("Method not found"));
}

#[tokio::test]
async fn vector_search_defaults_ok() {
    let _ = fs::remove_file("test_vector_defaults.db");
    let _ = fs::remove_dir_all("test_vector_defaults_cache");

    let memory = Memory::new("test_vector_defaults.db").unwrap();
    let tasks = Tasks::new("test_vector_defaults_tasks.db").unwrap();
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let state = SynCoreState::with_dual_stores(vector_store.clone(), vector_store).unwrap();

    // Insert some test data
    {
        let mut store = state.general_store.lock().unwrap();
        store.insert_text(1, Some(1), "branch misprediction in CPU pipeline", "note").unwrap();
        store.insert_text(2, Some(1), "SIMD fused kernel optimization", "note").unwrap();
    }

    // Test with only "query" provided (defaults k=8, scope=global)
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(serde_json::json!({
            "name": "vector.search",
            "arguments": {
                "query": "branch misprediction"
            }
        })),
        id: json!("vector-test-1"),
    };

    let response = handle_mcp_request(request, &state).await;
    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let result = response.result.unwrap();
    let hits = result["results"].as_array().unwrap();
    assert!(hits.len() <= 8);
}

#[tokio::test]
async fn vector_search_scope_task_ok() {
    let _ = fs::remove_file("test_vector_task.db");
    let _ = fs::remove_dir_all("test_vector_task_cache");

    let memory = Memory::new("test_vector_task.db").unwrap();
    let tasks = Tasks::new("test_vector_task_tasks.db").unwrap();
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384).unwrap(),
    ))));
    let state = SynCoreState::with_dual_stores(vector_store.clone(), vector_store).unwrap();

    // Insert test data for different tasks
    {
        let mut store = state.general_store.lock().unwrap();
        store.insert_text(1, Some(7), "task 7: simd fused operations", "note").unwrap();
        store.insert_text(2, Some(8), "task 8: different content", "note").unwrap();
        store.insert_text(3, Some(7), "task 7: more simd work", "note").unwrap();
    }

    // Test with task scope using the new JSON format
    let request = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "mcp.call_tool".to_string(),
        params: Some(serde_json::json!({
            "name": "vector.search",
            "arguments": {
                "query": "simd fused",
                "k": 5,
                "scope": {"task": {"task_id": 7}}
            }
        })),
        id: json!("vector-test-2"),
    };

    let response = handle_mcp_request(request, &state).await;
    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let result = response.result.unwrap();
    let hits = result["results"].as_array().unwrap();
    assert!(hits.len() <= 5);
}
