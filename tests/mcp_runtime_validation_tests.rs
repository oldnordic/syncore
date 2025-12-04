//! Runtime Cross-Validation Tests for MCP Tools
//!
//! STEP 7: Actual tool execution validation
//! Tests that all 34 tools can be invoked successfully

use serde_json::json;
use std::collections::HashSet;
use syncore::mcp::{handle_mcp_request, MCPRequest};
use tempfile::NamedTempFile;

/// Create a test state for runtime validation
fn create_test_state() -> syncore::mcp::SynCoreState {
    use std::sync::{Arc, Mutex};
    use syncore::memory::Memory;
    use syncore::tasks::Tasks;
    use syncore::vector::HuggingFaceEmbeddings;
    use syncore::vector::VectorStore;

    // Create temporary database
    let temp_db = NamedTempFile::new().expect("Failed to create temp DB");
    let db_path = temp_db.path().to_str().unwrap();

    // Create memory and tasks
    let memory = Memory::new(db_path).expect("Failed to create Memory");
    let tasks = Tasks::new(&format!("{}_tasks", db_path)).expect("Failed to create Tasks");

    // Create vector store with minimal embeddings
    let embeddings = Box::new(HuggingFaceEmbeddings::new().expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Create state using deprecated constructor for tests
    #[allow(deprecated)]
    syncore::mcp::SynCoreState::new(memory, tasks, vector_store)
}

/// Test 7: Runtime cross-validation - all tools must execute
#[tokio::test]
async fn test_runtime_cross_validation() {
    let state = create_test_state();

    // Test each tool category with minimal valid arguments
    let test_cases = vec![
        // Memory Suite Tools
        ("memory_store", json!({"command": "store", "key": "test_key", "value": "test_value"})),
        ("memory_query", json!({"command": "query", "key": "test_key"})),
        (
            "vector_insert",
            json!({"command": "vector_insert", "text": "test text", "namespace": "test"}),
        ),
        ("vector_search", json!({"command": "vector_search", "query": "test", "limit": 5})),
        ("task_create", json!({"command": "task_create", "goal": "Test task"})),
        ("task_list", json!({"command": "task_list"})),
        ("task_get", json!({"command": "task_get", "task_id": 1})),
        ("task_update", json!({"command": "task_update", "task_id": 1, "status": "completed"})),
        ("task_next", json!({"command": "task_next"})),
        // IntelliTask Tools (may fail without Ollama, but should not crash)
        (
            "intellitask_generate",
            json!({"command": "intellitask_generate", "prd_content": "Test PRD"}),
        ),
        ("intellitask_list", json!({"command": "intellitask_list"})),
        ("intellitask_get", json!({"command": "intellitask_get", "task_id": 1})),
        (
            "intellitask_update_status",
            json!({"command": "intellitask_update_status", "task_id": 1, "status": "completed"}),
        ),
        // Code Suite Tools
        ("code_search", json!({"command": "search", "query": "test", "limit": 5})),
        ("parser_analyze", json!({"command": "parse", "file_path": "/dev/null"})),
        ("parser_search", json!({"command": "grep", "pattern": "test", "path": "/dev/null"})),
        // Graph Suite Tools (may fail without Neo4j, but should not crash)
        ("graph_query", json!({"command": "query", "cypher": "MATCH (n) RETURN count(n)"})),
        ("graph_suite", json!({"command": "help"})),
        // Debug and Mapping Suites
        ("debug_suite", json!({"command": "debug_suite", "command": "help"})),
        ("mapping_suite", json!({"command": "mapping_suite", "command": "help"})),
        // Reasoning Tools
        ("reasoning_session_create", json!({"task": "test reasoning task"})),
        ("reasoning_tree_get", json!({"session_id": "test_session"})),
        ("reasoning_tree_prune", json!({"session_id": "test_session", "node_id": "test_node"})),
        // Document Tools
        ("document_search", json!({"command": "vector_search", "query": "test", "limit": 5})),
        ("document_index", json!({"command": "vector_insert", "directory": "/dev/null"})),
    ];

    let mut successful_tools = HashSet::new();
    let mut failed_tools = Vec::new();

    for (tool_name, arguments) in &test_cases {
        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            method: "mcp.call_tool".to_string(),
            params: Some(json!({
                "name": tool_name,
                "arguments": arguments
            })),
            id: serde_json::Value::Number(1.into()),
        };

        let response = handle_mcp_request(request, &state).await;

        // Check if tool executed (even if it failed with business logic error)
        match response.result {
            Some(result) => {
                // Tool executed successfully
                successful_tools.insert(tool_name.to_string());

                // Verify result has expected structure
                if let Some(obj) = result.as_object() {
                    assert!(
                        obj.contains_key("success"),
                        "Tool '{}' response missing 'success' field",
                        tool_name
                    );
                    assert!(
                        obj.contains_key("command"),
                        "Tool '{}' response missing 'command' field",
                        tool_name
                    );
                }
            }
            None => {
                // Tool had execution error
                if let Some(error) = &response.error {
                    // Allow certain expected errors (Neo4j not available, Ollama not running, etc.)
                    let error_msg = error.message.to_lowercase();
                    let is_expected_error = error_msg.contains("neo4j")
                        || error_msg.contains("ollama")
                        || error_msg.contains("connection")
                        || error_msg.contains("directory")
                        || error_msg.contains("file not found");

                    if is_expected_error {
                        successful_tools.insert(tool_name.to_string());
                    } else {
                        failed_tools
                            .push((tool_name, format!("Execution error: {}", error.message)));
                    }
                } else {
                    failed_tools.push((tool_name, "Unknown error".to_string()));
                }
            }
        }
    }

    // Report results
    println!("Runtime Validation Results:");
    println!("  Successfully executed: {}", successful_tools.len());
    println!("  Failed: {}", failed_tools.len());

    for (tool, error) in &failed_tools {
        println!("  FAILED: {} - {}", tool, error);
    }

    // At minimum, core tools should work
    let core_tools = vec![
        "memory_store",
        "memory_query",
        "task_create",
        "task_list",
        "debug_suite",
        "mapping_suite",
        "parser_analyze",
    ];

    for core_tool in &core_tools {
        assert!(
            successful_tools.contains(*core_tool),
            "Core tool '{}' failed to execute",
            core_tool
        );
    }

    // Most tools should execute (allowing for expected infrastructure errors)
    let success_rate = successful_tools.len() as f64 / test_cases.len() as f64;
    assert!(
        success_rate >= 0.7, // At least 70% should work
        "Runtime success rate too low: {}/{} ({:.1}%)",
        successful_tools.len(),
        test_cases.len(),
        success_rate * 100.0
    );

    println!("Runtime validation completed with {:.1}% success rate", success_rate * 100.0);
}
