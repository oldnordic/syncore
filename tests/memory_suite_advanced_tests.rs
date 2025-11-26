//! Tests for advanced memory suite commands exposed via MCP
//!
//! APEX 1.9-M: Memory Suite MCP Expansion Tests

use serde_json::json;
use std::sync::Arc;
use syncore::memory::Memory;
use syncore::mcp_tools::memory_suite::{MemorySuite, MemorySuiteArgs};
use syncore::router::SynCoreState;
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};
use std::sync::Mutex;

/// Helper to create test suite
fn create_test_suite(db_suffix: &str) -> MemorySuite {
    let db_path = format!(":memory:_{}", db_suffix);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(":memory:").expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);

    MemorySuite::new(state)
}

#[test]
fn test_mcp_delete() {
    let suite = create_test_suite("delete");

    // Store a key first
    let store_args = MemorySuiteArgs {
        command: "store".to_string(),
        key: Some("test_key".to_string()),
        value: Some("test_value".to_string()),
        ..Default::default()
    };
    let store_result = suite.execute(store_args);
    assert!(store_result.success, "Store should succeed");

    // Delete the key
    let delete_args = MemorySuiteArgs {
        command: "delete".to_string(),
        key: Some("test_key".to_string()),
        ..Default::default()
    };
    let result = suite.execute(delete_args);

    assert!(result.success, "Delete should succeed");
    assert!(result.data.get("success").and_then(|v| v.as_bool()).unwrap_or(false));
}

#[test]
fn test_mcp_list_keys() {
    let suite = create_test_suite("list_keys");

    // Store multiple keys
    for i in 1..=3 {
        let args = MemorySuiteArgs {
            command: "store".to_string(),
            key: Some(format!("key_{}", i)),
            value: Some(format!("value_{}", i)),
            ..Default::default()
        };
        suite.execute(args);
    }

    // List keys
    let args = MemorySuiteArgs {
        command: "list_keys".to_string(),
        limit: Some(10),
        ..Default::default()
    };
    let result = suite.execute(args);

    assert!(result.success, "list_keys should succeed");
    let keys = result.data.get("keys").and_then(|v| v.as_array());
    assert!(keys.is_some() && keys.unwrap().len() >= 3);
}

#[test]
fn test_mcp_memory_stats() {
    let suite = create_test_suite("stats");

    // Store some memories
    for i in 1..=5 {
        let args = MemorySuiteArgs {
            command: "store".to_string(),
            key: Some(format!("key_{}", i)),
            value: Some(format!("value_{}", i)),
            ..Default::default()
        };
        suite.execute(args);
    }

    // Get stats
    let args = MemorySuiteArgs {
        command: "memory_stats".to_string(),
        ..Default::default()
    };
    let result = suite.execute(args);

    assert!(result.success, "memory_stats should succeed");
    let count = result.data.get("count").and_then(|v| v.as_i64());
    assert!(count.is_some() && count.unwrap() >= 5);
}

#[test]
fn test_help_includes_all_commands() {
    let suite = create_test_suite("help");

    let args = MemorySuiteArgs {
        command: "help".to_string(),
        ..Default::default()
    };
    let result = suite.execute(args);

    assert!(result.success, "help should succeed");
    let commands = result.data.get("commands").and_then(|v| v.as_array());
    assert!(commands.is_some());

    let command_list = commands.unwrap();
    // Verify new commands are in the list
    let command_strings: Vec<String> = command_list.iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect();

    assert!(command_strings.contains(&"delete".to_string()));
    assert!(command_strings.contains(&"list_keys".to_string()));
    assert!(command_strings.contains(&"memory_stats".to_string()));
    assert!(command_strings.contains(&"search_semantic".to_string()));
    assert!(command_strings.contains(&"query_recent".to_string()));
}
