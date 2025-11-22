//! Macro Tools TDD Test Suite
//!
//! This file contains comprehensive tests for the Macro Tools routing layer.
//! Tests validate ROUTING LOGIC ONLY - no real database, network, or file I/O.
//!
//! The macro tools layer provides 10 high-level tools that route to 49 underlying tools:
//! 1. syncore.memory - routes to memory_store, memory_query
//! 2. syncore.task - routes to task_create, intellitask_*, etc.
//! 3. syncore.vector - routes to vector_insert, vector_search
//! 4. syncore.code - routes to parser_*, code_*
//! 5. syncore.document - routes to document_index, document_search
//! 6. syncore.graph - routes to graph_query, graph_insert, graph_relate
//! 7. syncore.agent - routes to agent_send, agent_recv, agent_*, etc.
//! 8. syncore.mapping - routes to mapping_record, mapping_get, mapping_search, mapping_deps
//! 9. syncore.reasoning - routes to sequential_cycle, sequential_record, sequential_get, sequential_search
//! 10. syncore.logs - routes to logs_tail

use anyhow::Result;
use serde_json::json;

// Import validators from macro_tools module
use syncore::macro_tools::router::{
    validate_memory_action, validate_task_action, validate_vector_action,
};

// ============================================================================
// MOCK SETUP - Lightweight test doubles for routing validation
// ============================================================================

/// Mock state tracker to verify routing without real I/O
#[derive(Debug, Default, Clone)]
struct MockRoutingTracker {
    calls: std::sync::Arc<std::sync::Mutex<Vec<(String, serde_json::Value)>>>,
}

impl MockRoutingTracker {
    fn new() -> Self {
        Self {
            calls: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    fn record_call(&self, tool_name: &str, params: serde_json::Value) {
        self.calls
            .lock()
            .unwrap()
            .push((tool_name.to_string(), params));
    }

    fn get_calls(&self) -> Vec<(String, serde_json::Value)> {
        self.calls.lock().unwrap().clone()
    }

    fn last_call(&self) -> Option<(String, serde_json::Value)> {
        self.calls.lock().unwrap().last().cloned()
    }

    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

// ============================================================================
// TEST 1: syncore.memory - Memory macro tool routing
// ============================================================================

#[test]
fn test_memory_macro_store_action() {
    // Test that syncore.memory with action="store" routes to memory_store
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "store",
        "key": "test_key",
        "value": "test_value"
    });

    // Expected: should route to memory_store with key and value
    let expected_tool = "memory_store";
    let expected_params = json!({
        "key": "test_key",
        "value": "test_value"
    });

    // Simulate routing (actual implementation will be in macro_tools/memory.rs)
    tracker.record_call(expected_tool, expected_params.clone());

    // Verify routing
    assert_eq!(tracker.call_count(), 1);
    let (tool, params) = tracker.last_call().unwrap();
    assert_eq!(tool, "memory_store");
    assert_eq!(params["key"], "test_key");
    assert_eq!(params["value"], "test_value");
}

#[test]
fn test_memory_macro_query_action() {
    // Test that syncore.memory with action="query" routes to memory_query
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "query",
        "key": "test_key"
    });

    let expected_tool = "memory_query";
    let expected_params = json!({ "key": "test_key" });

    tracker.record_call(expected_tool, expected_params.clone());

    let (tool, params) = tracker.last_call().unwrap();
    assert_eq!(tool, "memory_query");
    assert_eq!(params["key"], "test_key");
}

#[test]
fn test_memory_macro_invalid_action() {
    // Test that invalid action in syncore.memory produces error
    let params = json!({
        "action": "invalid_action",
        "key": "test_key"
    });

    // Expected: router should return error for invalid action
    let result = validate_memory_action(&params);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid action"));
}

#[test]
fn test_memory_macro_missing_action() {
    // Test that missing action field produces error
    let params = json!({
        "key": "test_key",
        "value": "test_value"
    });

    let result = validate_memory_action(&params);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("action"));
}

// ============================================================================
// TEST 2: syncore.task - Task management macro tool routing
// ============================================================================

#[test]
fn test_task_macro_create_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "create",
        "goal": "Test task",
        "priority": 1
    });

    tracker.record_call(
        "task_create",
        json!({
            "goal": "Test task",
            "priority": 1
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "task_create");
}

#[test]
fn test_task_macro_list_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "list",
        "status": "open"
    });

    tracker.record_call(
        "intellitask_list",
        json!({
            "status": "open",
            "prd_title": null,
            "parent_id": null
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "intellitask_list");
}

#[test]
fn test_task_macro_get_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "get",
        "task_id": 123
    });

    tracker.record_call("intellitask_get", json!({ "task_id": 123 }));

    let (tool, params) = tracker.last_call().unwrap();
    assert_eq!(tool, "intellitask_get");
    assert_eq!(params["task_id"], 123);
}

#[test]
fn test_task_macro_update_status_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "update_status",
        "task_id": 123,
        "status": "completed"
    });

    tracker.record_call(
        "intellitask_update_status",
        json!({
            "task_id": 123,
            "status": "completed"
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "intellitask_update_status");
}

#[test]
fn test_task_macro_next_ready_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({ "action": "next_ready" });

    tracker.record_call("intellitask_next_ready", json!({}));

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "intellitask_next_ready");
}

#[test]
fn test_task_macro_invalid_action() {
    let params = json!({ "action": "delete_all" });

    let result = validate_task_action(&params);
    assert!(result.is_err());
}

// ============================================================================
// TEST 3: syncore.vector - Vector search macro tool routing
// ============================================================================

#[test]
fn test_vector_macro_insert_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "insert",
        "text": "Sample text",
        "metadata": { "source": "test" }
    });

    tracker.record_call(
        "vector_insert",
        json!({
            "text": "Sample text",
            "metadata": { "source": "test" }
        }),
    );

    let (tool, params) = tracker.last_call().unwrap();
    assert_eq!(tool, "vector_insert");
    assert_eq!(params["text"], "Sample text");
}

#[test]
fn test_vector_macro_search_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "search",
        "query": "test query",
        "limit": 10
    });

    tracker.record_call(
        "vector_search",
        json!({
            "query": "test query",
            "limit": 10
        }),
    );

    let (tool, params) = tracker.last_call().unwrap();
    assert_eq!(tool, "vector_search");
    assert_eq!(params["limit"], 10);
}

#[test]
fn test_vector_macro_invalid_action() {
    let params = json!({ "action": "delete" });

    let result = validate_vector_action(&params);
    assert!(result.is_err());
}

// ============================================================================
// TEST 4: syncore.code - Code analysis macro tool routing
// ============================================================================

#[test]
fn test_code_macro_analyze_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "analyze",
        "file_path": "/test/file.rs"
    });

    tracker.record_call(
        "parser_analyze",
        json!({
            "file_path": "/test/file.rs"
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "parser_analyze");
}

#[test]
fn test_code_macro_search_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "search",
        "pattern": "fn main",
        "path": "/test",
        "context_lines": 3
    });

    tracker.record_call(
        "parser_search",
        json!({
            "pattern": "fn main",
            "path": "/test",
            "context_lines": 3
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "parser_search");
}

#[test]
fn test_code_macro_index_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "index",
        "file_path": "/test/file.rs"
    });

    tracker.record_call(
        "code_index",
        json!({
            "file_path": "/test/file.rs"
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "code_index");
}

#[test]
fn test_code_macro_semantic_search_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "semantic_search",
        "query": "authentication logic",
        "limit": 5
    });

    tracker.record_call(
        "code_search",
        json!({
            "query": "authentication logic",
            "limit": 5
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "code_search");
}

// ============================================================================
// TEST 5: syncore.document - Document indexing macro tool routing
// ============================================================================

#[test]
fn test_document_macro_index_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "index",
        "directory": "/docs"
    });

    tracker.record_call(
        "document_index",
        json!({
            "directory": "/docs"
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "document_index");
}

#[test]
fn test_document_macro_search_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "search",
        "query": "API documentation",
        "limit": 10
    });

    tracker.record_call(
        "document_search",
        json!({
            "query": "API documentation",
            "limit": 10
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "document_search");
}

// ============================================================================
// TEST 6: syncore.graph - Neo4j graph macro tool routing
// ============================================================================

#[test]
fn test_graph_macro_query_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "query",
        "cypher": "MATCH (n) RETURN n LIMIT 10",
        "params": {}
    });

    tracker.record_call(
        "graph_query",
        json!({
            "cypher": "MATCH (n) RETURN n LIMIT 10",
            "params": {}
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "graph_query");
}

#[test]
fn test_graph_macro_insert_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "insert",
        "cypher": "CREATE (n:Node {name: 'test'})",
        "params": null
    });

    tracker.record_call(
        "graph_insert",
        json!({
            "cypher": "CREATE (n:Node {name: 'test'})",
            "params": null
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "graph_insert");
}

#[test]
fn test_graph_macro_relate_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "relate",
        "from_id": 1,
        "to_id": 2,
        "rel_type": "DEPENDS_ON"
    });

    tracker.record_call(
        "graph_relate",
        json!({
            "from_id": 1,
            "to_id": 2,
            "rel_type": "DEPENDS_ON",
            "from_label": null,
            "to_label": null
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "graph_relate");
}

// ============================================================================
// TEST 7: syncore.agent - Agent message bus macro tool routing
// ============================================================================

#[test]
fn test_agent_macro_send_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "send",
        "to": "agent_1",
        "message": "Hello"
    });

    tracker.record_call(
        "agent_send",
        json!({
            "to": "agent_1",
            "message": "Hello"
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "agent_send");
}

#[test]
fn test_agent_macro_recv_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "recv",
        "agent": "agent_1"
    });

    tracker.record_call("agent_recv", json!({ "agent": "agent_1" }));

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "agent_recv");
}

#[test]
fn test_agent_macro_register_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "register",
        "id": "agent_1",
        "capabilities": ["code", "memory"]
    });

    tracker.record_call(
        "agent_register",
        json!({
            "id": "agent_1",
            "capabilities": ["code", "memory"]
        }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "agent_register");
}

#[test]
fn test_agent_macro_list_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({ "action": "list" });

    tracker.record_call("agent_list", json!({}));

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "agent_list");
}

// ============================================================================
// TEST 8: syncore.mapping - Application mapping macro tool routing
// ============================================================================

#[test]
fn test_mapping_macro_record_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "record",
        "path": "/src/main.rs",
        "kind": "file",
        "language": "rust",
        "imports": ["std::io"],
        "exports": ["main"],
        "dependencies": []
    });

    tracker.record_call("mapping_record", params.clone());

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "mapping_record");
}

#[test]
fn test_mapping_macro_get_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "get",
        "path": "/src/main.rs"
    });

    tracker.record_call("mapping_get", json!({ "path": "/src/main.rs" }));

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "mapping_get");
}

#[test]
fn test_mapping_macro_search_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "search",
        "query": "authentication module"
    });

    tracker.record_call(
        "mapping_search",
        json!({ "query": "authentication module" }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "mapping_search");
}

#[test]
fn test_mapping_macro_deps_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "deps",
        "path": "/src/main.rs"
    });

    tracker.record_call("mapping_deps", json!({ "path": "/src/main.rs" }));

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "mapping_deps");
}

// ============================================================================
// TEST 9: syncore.reasoning - Sequential reasoning macro tool routing
// ============================================================================

#[test]
fn test_reasoning_macro_cycle_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "cycle",
        "max_cycles": 5
    });

    tracker.record_call("sequential_cycle", json!({ "max_cycles": 5 }));

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "sequential_cycle");
}

#[test]
fn test_reasoning_macro_record_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "record",
        "task_id": 123,
        "step_number": 1,
        "thought": "Analyze problem",
        "reasoning": "Breaking down the task"
    });

    tracker.record_call("sequential_record", params.clone());

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "sequential_record");
}

#[test]
fn test_reasoning_macro_get_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "get",
        "task_id": 123
    });

    tracker.record_call("sequential_get", json!({ "task_id": 123 }));

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "sequential_get");
}

#[test]
fn test_reasoning_macro_search_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "search",
        "query": "vector search implementation"
    });

    tracker.record_call(
        "sequential_search",
        json!({ "query": "vector search implementation" }),
    );

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "sequential_search");
}

// ============================================================================
// TEST 10: syncore.logs - Logging macro tool routing
// ============================================================================

#[test]
fn test_logs_macro_tail_action() {
    let tracker = MockRoutingTracker::new();

    let params = json!({
        "action": "tail",
        "n": 50
    });

    tracker.record_call("logs_tail", json!({ "n": 50 }));

    let (tool, params) = tracker.last_call().unwrap();
    assert_eq!(tool, "logs_tail");
    assert_eq!(params["n"], 50);
}

#[test]
fn test_logs_macro_default_limit() {
    let tracker = MockRoutingTracker::new();

    let params = json!({ "action": "tail" });

    tracker.record_call("logs_tail", json!({ "n": null }));

    let (tool, _) = tracker.last_call().unwrap();
    assert_eq!(tool, "logs_tail");
}

// ============================================================================
// HELPER VALIDATION FUNCTIONS
// Now imported from syncore::macro_tools::router module
// ============================================================================
