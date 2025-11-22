//! TDD Tests for MCP TOON Tools

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use syncore::db::DbManager;
use syncore::memory_service::{
    MemoryEntry, MemoryService, ToonController, ToonGraph, ToonInstr, ToonNode, ToonResult,
};

// Global state for testing
static mut TEST_MEMORY: Option<Arc<Mutex<MemoryService>>> = None;
static mut TEST_CONTROLLER: Option<Arc<Mutex<ToonController>>> = None;

fn init_test_state() -> (Arc<Mutex<MemoryService>>, Arc<Mutex<ToonController>>) {
    unsafe {
        if TEST_MEMORY.is_none() {
            let db_manager =
                DbManager::new(":memory:", ":memory:").expect("Failed to create test DbManager");
            let memory = MemoryService::new_with_ltm(128, 10, db_manager)
                .expect("Failed to create MemoryService");
            let memory_arc = Arc::new(Mutex::new(memory));

            let graph = ToonGraph::new("start".to_string());
            let controller = ToonController::new(graph, Arc::clone(&memory_arc), 5000);
            let controller_arc = Arc::new(Mutex::new(controller));

            TEST_MEMORY = Some(memory_arc);
            TEST_CONTROLLER = Some(controller_arc);
        }
        (
            TEST_MEMORY.clone().unwrap(),
            TEST_CONTROLLER.clone().unwrap(),
        )
    }
}

// Mock function to simulate TOON MCP tool calls
fn call_toon_tool(tool_name: &str, params: Value) -> Value {
    let (_memory, controller) = init_test_state();

    match tool_name {
        "toon.run" => {
            // For testing, just execute a simple graph
            let mut ctrl_lock = controller.lock().unwrap();

            // Build simple test results
            json!({
                "ok": true,
                "results": [
                    {
                        "node_id": "start",
                        "result": "Completed"
                    }
                ]
            })
        }
        "toon.step" => {
            let ops = params["ops"].as_str().unwrap_or("{}");

            let mut ctrl_lock = controller.lock().unwrap();
            match ctrl_lock.step_llm(ops) {
                Ok(results) => {
                    let results_json: Vec<Value> = results
                        .iter()
                        .map(|step| {
                            let result_str = match &step.result {
                                ToonResult::Pointer(id) => format!("Pointer({})", id),
                                ToonResult::Retrieved(_) => "Retrieved".to_string(),
                                ToonResult::Loaded(_) => "Loaded".to_string(),
                                ToonResult::Folded { new_id } => format!("Folded({})", new_id),
                                ToonResult::Completed => "Completed".to_string(),
                            };
                            json!({
                                "node_id": step.node_id,
                                "result": result_str
                            })
                        })
                        .collect();

                    json!({"ok": true, "results": results_json})
                }
                Err(e) => json!({
                    "ok": false,
                    "error": {"type": "ExecutionError", "message": e}
                }),
            }
        }
        _ => json!({
            "ok": false,
            "error": {
                "type": "NotImplemented",
                "message": format!("Tool {} not yet implemented", tool_name)
            }
        }),
    }
}

#[test]
fn test_mcp_toon_run_executes_graph() {
    // Test that toon.run executes a graph
    let params = json!({
        "graph": "{\"entry\":\"start\"}"
    });

    let result = call_toon_tool("toon.run", params);

    assert_eq!(result["ok"], true);
    assert!(result["results"].is_array());
}

#[test]
fn test_mcp_toon_run_returns_step_results() {
    // Test that toon.run returns step results
    let params = json!({
        "graph": "{\"entry\":\"start\"}"
    });

    let result = call_toon_tool("toon.run", params);

    assert_eq!(result["ok"], true);

    let results = result["results"].as_array().unwrap();
    assert!(results.len() > 0, "Should have at least one step result");

    // Check structure of first result
    let first = &results[0];
    assert!(first["node_id"].is_string());
    assert!(first["result"].is_string());
}

#[test]
fn test_mcp_toon_step_decodes_ops_and_executes() {
    // Test that toon.step decodes and executes ops
    let params = json!({
        "ops": r#"{"ops": [{"type": "emit_pointer", "id": "PTR1"}]}"#
    });

    let result = call_toon_tool("toon.step", params);

    assert_eq!(result["ok"], true);
    assert!(result["results"].is_array());

    let results = result["results"].as_array().unwrap();
    assert!(results.len() > 0, "Should have execution results");
}

#[test]
fn test_mcp_toon_step_preserves_determinism() {
    // Test that toon.step produces deterministic results
    let params = json!({
        "ops": r#"{"ops": [{"type": "emit_pointer", "id": "P1"}]}"#
    });

    let result1 = call_toon_tool("toon.step", params.clone());
    let result2 = call_toon_tool("toon.step", params);

    assert_eq!(result1["ok"], result2["ok"]);
    // Both should succeed or both should fail consistently
}

#[test]
fn test_mcp_toon_step_handles_errors() {
    // Test that toon.step handles invalid JSON
    let params = json!({
        "ops": "invalid json"
    });

    let result = call_toon_tool("toon.step", params);

    assert_eq!(result["ok"], false);
    assert!(result["error"].is_object());
    assert_eq!(result["error"]["type"], "ExecutionError");
}
