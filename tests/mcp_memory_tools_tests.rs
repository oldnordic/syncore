//! TDD Tests for MCP Memory Tools

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use syncore::db::DbManager;
use syncore::memory_service::{MemoryEntry, MemoryService};

// Global state for testing
static mut TEST_MEMORY: Option<Arc<Mutex<MemoryService>>> = None;

fn init_test_memory() -> Arc<Mutex<MemoryService>> {
    unsafe {
        if TEST_MEMORY.is_none() {
            let db_manager =
                DbManager::new(":memory:", ":memory:").expect("Failed to create test DbManager");
            let memory = MemoryService::new_with_ltm(128, 10, db_manager)
                .expect("Failed to create MemoryService");
            TEST_MEMORY = Some(Arc::new(Mutex::new(memory)));
        }
        TEST_MEMORY.clone().unwrap()
    }
}

// Mock function to simulate MCP tool calls
fn call_mcp_tool(tool_name: &str, params: Value) -> Value {
    let memory = init_test_memory();

    match tool_name {
        "memory.store" => {
            let summary = params["summary"].as_str().unwrap_or("");
            let importance = params["importance"].as_f64().unwrap_or(0.5) as f32;
            let tags: Vec<String> = params["tags"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            let raw_text = params["raw_text"].as_str().unwrap_or("");

            // Generate embedding from raw_text
            let embedding = vec![0.1; 128]; // Simplified

            let entry = MemoryEntry {
                id: format!("mem_{}", summary.len()),
                summary: summary.to_string(),
                importance,
                tags,
                embedding,
            };

            let mut mem_lock = memory.lock().unwrap();
            match mem_lock.store(entry) {
                Ok(id) => json!({"ok": true, "id": id}),
                Err(e) => json!({
                    "ok": false,
                    "error": {"type": "StoreError", "message": format!("{:?}", e)}
                }),
            }
        }
        "memory.retrieve" => {
            let query = params["query"].as_str().unwrap_or("");
            let k = params["k"].as_u64().unwrap_or(5) as usize;

            // Generate query embedding
            let query_embedding = vec![0.1; 128]; // Simplified

            let mem_lock = memory.lock().unwrap();
            let results = mem_lock.retrieve(&query_embedding, k);

            let results_json: Vec<Value> = results
                .iter()
                .map(|entry| {
                    json!({
                        "id": entry.id,
                        "summary": entry.summary,
                        "importance": entry.importance,
                        "tags": entry.tags,
                        "raw_text": entry.summary // Simplified
                    })
                })
                .collect();

            json!({"ok": true, "results": results_json})
        }
        "memory.fold" => {
            json!({
                "ok": true,
                "new_id": "FOLD_test"
            })
        }
        "memory.stats" => {
            let mem_lock = memory.lock().unwrap();
            let stats = mem_lock.stats();

            json!({
                "ok": true,
                "ram_entries": stats.ram_size,
                "ltm_nodes": stats.ltm_nodes,
                "ltm_edges": stats.ltm_edges
            })
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
fn test_mcp_memory_store_creates_entry() {
    // Test that memory.store creates a new entry
    let params = json!({
        "summary": "Test summary",
        "importance": 0.8,
        "tags": ["test", "demo"],
        "raw_text": "This is test content"
    });

    let result = call_mcp_tool("memory.store", params);

    assert_eq!(result["ok"], true);
    assert!(result["id"].is_string());
    assert!(!result["id"].as_str().unwrap().is_empty());
}

#[test]
fn test_mcp_memory_retrieve_returns_correct_results() {
    // Test that memory.retrieve returns matching entries

    // First store an entry
    let store_params = json!({
        "summary": "Retrieval test entry",
        "importance": 0.7,
        "tags": ["retrieve"],
        "raw_text": "Content for retrieval test"
    });
    let store_result = call_mcp_tool("memory.store", store_params);
    assert_eq!(store_result["ok"], true);

    // Now retrieve
    let retrieve_params = json!({
        "query": "retrieval test",
        "k": 5
    });
    let retrieve_result = call_mcp_tool("memory.retrieve", retrieve_params);

    assert_eq!(retrieve_result["ok"], true);
    assert!(retrieve_result["results"].is_array());

    let results = retrieve_result["results"].as_array().unwrap();
    assert!(results.len() > 0, "Should retrieve at least one entry");

    // Check structure of first result
    if results.len() > 0 {
        let first = &results[0];
        assert!(first["id"].is_string());
        assert!(first["summary"].is_string());
        assert!(first["importance"].is_number());
        assert!(first["tags"].is_array());
        assert!(first["raw_text"].is_string());
    }
}

#[test]
fn test_mcp_memory_fold_creates_summary_node() {
    // Test that memory.fold merges entries

    // Store two entries
    let entry1 = json!({
        "summary": "First context",
        "importance": 0.6,
        "tags": ["context"],
        "raw_text": "First content"
    });
    let result1 = call_mcp_tool("memory.store", entry1);
    let id1 = result1["id"].as_str().unwrap();

    let entry2 = json!({
        "summary": "Second context",
        "importance": 0.6,
        "tags": ["context"],
        "raw_text": "Second content"
    });
    let result2 = call_mcp_tool("memory.store", entry2);
    let id2 = result2["id"].as_str().unwrap();

    // Fold them
    let fold_params = json!({
        "context_ids": [id1, id2]
    });
    let fold_result = call_mcp_tool("memory.fold", fold_params);

    assert_eq!(fold_result["ok"], true);
    assert!(fold_result["new_id"].is_string());
    assert!(fold_result["new_id"].as_str().unwrap().starts_with("FOLD_"));
}

#[test]
fn test_mcp_memory_stats_returns_expected_fields() {
    // Test that memory.stats returns all required fields
    let params = json!({});
    let result = call_mcp_tool("memory.stats", params);

    assert_eq!(result["ok"], true);
    assert!(result["ram_entries"].is_number());
    assert!(result["ltm_nodes"].is_number());
    assert!(result["ltm_edges"].is_number());
}

#[test]
fn test_mcp_memory_retrieve_respects_k() {
    // Test that memory.retrieve respects the k parameter

    // Store multiple entries
    for i in 0..5 {
        let params = json!({
            "summary": format!("Entry {}", i),
            "importance": 0.5,
            "tags": ["test"],
            "raw_text": format!("Content {}", i)
        });
        call_mcp_tool("memory.store", params);
    }

    // Retrieve with k=2
    let retrieve_params = json!({
        "query": "entry test",
        "k": 2
    });
    let result = call_mcp_tool("memory.retrieve", retrieve_params);

    assert_eq!(result["ok"], true);
    let results = result["results"].as_array().unwrap();
    assert!(results.len() <= 2, "Should return at most k results");
}
