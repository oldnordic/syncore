//! Test MCP Tool Contract Compliance
//!
//! Tests to ensure all tools referenced in documentation actually exist
//! and function correctly.

use serde_json::json;
use std::sync::Arc;
use syncore::mcp_tools::memory_suite::{MemorySuite, MemorySuiteArgs};
use syncore::mcp_tools::SuiteDispatcher;
use syncore::memory;
use syncore::router::SynCoreState;
use syncore::tasks;
use syncore::vector;

#[test]
fn test_task_create_dependency_tool_exists() {
    let state = create_test_state();
    let suite = MemorySuite::new(state);

    let args = MemorySuiteArgs {
        command: "task_create_dependency".to_string(),
        task_id: Some(1),
        depends_on_task_id: Some(2),
        ..Default::default()
    };

    // Create tasks first to establish valid IDs
    let create_args1 = MemorySuiteArgs {
        command: "task_create".to_string(),
        goal: Some("Test task 1".to_string()),
        priority: Some(1),
        ..Default::default()
    };

    let create_args2 = MemorySuiteArgs {
        command: "task_create".to_string(),
        goal: Some("Test task 2".to_string()),
        priority: Some(2),
        ..Default::default()
    };

    // Create the tasks
    let _ = suite.dispatch("task_create", serde_json::to_value(create_args1).unwrap());
    let _ = suite.dispatch("task_create", serde_json::to_value(create_args2).unwrap());

    // Now test dependency creation
    let result = suite.dispatch("task_create_dependency", serde_json::to_value(args).unwrap());

    assert!(result.success, "task_create_dependency should succeed");
    let data = result.data.as_object().unwrap();
    assert_eq!(data.get("created").unwrap(), &serde_json::Value::Bool(true));
}

#[test]
fn test_task_get_graph_tool_exists() {
    let state = create_test_state();
    let suite = MemorySuite::new(state);

    let args = MemorySuiteArgs {
        command: "task_get_graph".to_string(),
        ..Default::default()
    };

    let result = suite.dispatch("task_get_graph", serde_json::to_value(args).unwrap());

    assert!(result.success, "task_get_graph should succeed");
    let data = result.data.as_object().unwrap();
    assert!(data.contains_key("tasks"), "Should contain tasks array");
    assert!(data.contains_key("dependencies"), "Should contain dependencies array");
    assert!(data.contains_key("total_tasks"), "Should contain total_tasks count");
    assert!(data.contains_key("total_dependencies"), "Should contain total_dependencies count");
}

#[test]
fn test_all_documented_tools_exist() {
    let state = create_test_state();
    let suite = MemorySuite::new(state);

    let commands = suite.list_commands();

    // Tools that should exist according to updated documentation
    let required_tools = vec![
        "task_create_dependency",
        "task_get_graph",
        "task_create",
        "task_list",
        "task_get",
        "task_update",
        "task_next",
        "store", // Changed from memory_store
        "query", // Changed from memory_query
        "vector_insert",
        "vector_search",
        "intellitask_generate",
        "intellitask_list",
        "help",
    ];

    for tool in required_tools {
        assert!(commands.iter().any(|&cmd| cmd == tool), "Missing tool: {}", tool);
    }
}

#[test]
fn test_task_dependency_validation() {
    let state = create_test_state();
    let suite = MemorySuite::new(state);

    // Test with missing task_id
    let args = MemorySuiteArgs {
        command: "task_create_dependency".to_string(),
        depends_on_task_id: Some(1),
        ..Default::default()
    };

    let result = suite.dispatch("task_create_dependency", serde_json::to_value(args).unwrap());
    assert!(!result.success, "Should fail without task_id");
    assert!(result.error.unwrap().contains("Missing required parameter: task_id"));

    // Test with missing depends_on_task_id
    let args = MemorySuiteArgs {
        command: "task_create_dependency".to_string(),
        task_id: Some(1),
        ..Default::default()
    };

    let result = suite.dispatch("task_create_dependency", serde_json::to_value(args).unwrap());
    assert!(!result.success, "Should fail without depends_on_task_id");
    assert!(result.error.unwrap().contains("Missing required parameter: depends_on_task_id"));
}

#[test]
fn test_help_includes_new_tools() {
    let state = create_test_state();
    let suite = MemorySuite::new(state);

    let help_result = suite.dispatch("help", serde_json::json!({"command": "help"}));
    assert!(help_result.success, "Help command should succeed");

    let help_data = help_result.data.as_object().unwrap();
    let commands = help_data.get("commands").unwrap().as_array().unwrap();

    let commands_str: Vec<String> =
        commands.iter().map(|cmd| cmd.as_str().unwrap().to_string()).collect();

    assert!(
        commands_str.contains(&"task_create_dependency".to_string()),
        "Help should include task_create_dependency"
    );
    assert!(
        commands_str.contains(&"task_get_graph".to_string()),
        "Help should include task_get_graph"
    );
}

#[test]
fn test_task_categories_updated() {
    let state = create_test_state();
    let suite = MemorySuite::new(state);

    let help_result = suite.dispatch("help", serde_json::json!({"command": "help"}));
    assert!(help_result.success, "Help command should succeed");

    let help_data = help_result.data.as_object().unwrap();
    let categories = help_data.get("categories").unwrap().as_object().unwrap();
    let tasks = categories.get("tasks").unwrap().as_array().unwrap();

    let tasks_str: Vec<String> =
        tasks.iter().map(|cmd| cmd.as_str().unwrap().to_string()).collect();

    assert!(
        tasks_str.contains(&"task_create_dependency".to_string()),
        "Task category should include new tools"
    );
    assert!(
        tasks_str.contains(&"task_get_graph".to_string()),
        "Task category should include new tools"
    );
}

fn create_test_state() -> SynCoreState {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_db = format!("test_mcp_contract_{}.db", test_id);
    let _ = std::fs::remove_file(&test_db);

    let memory = memory::Memory::new(&test_db).expect("Failed to create test memory");
    let tasks =
        tasks::Tasks::new(&format!("{}_tasks", test_db)).expect("Failed to create test tasks");
    let embeddings =
        Box::new(vector::RealEmbeddings::new(384).expect("Failed to create test embeddings"));
    let vector_store = Arc::new(std::sync::Mutex::new(vector::VectorStore::new(embeddings)));

    SynCoreState::new(memory, tasks, vector_store)
}

// ============================================================================
// Sequential MCP Tool Registration Tests
// ============================================================================

#[test]
fn test_all_sequential_tools_registered_in_mcp() {
    let state = create_test_state();
    let suite = MemorySuite::new(state);

    // Get help to verify all sequential tools are registered
    let help_result = suite.dispatch("help", serde_json::json!({"command": "help"}));
    assert!(help_result.success, "Help command should succeed");

    let help_data = help_result.data.as_object().unwrap();
    let commands = help_data.get("commands").unwrap().as_array().unwrap();

    // Extract command names
    let command_names: Vec<String> =
        commands.iter().map(|cmd| cmd.as_str().unwrap().to_string()).collect();

    // Verify all 9 sequential tools are registered
    let expected_tools = vec![
        "sequential_next",
        "sequential_run",
        "sequential_reason",
        "sequential_status",
        "sequential_reset",
        "sequential_record",
        "sequential_get",
        "sequential_search",
        "sequential_cycle",
    ];

    for tool in &expected_tools {
        assert!(
            command_names.contains(&tool.to_string()),
            "Sequential tool '{}' should be registered in MCP commands",
            tool
        );
    }

    // Verify sequential category exists
    let categories = help_data.get("categories").unwrap().as_object().unwrap();
    let sequential = categories.get("sequential").unwrap().as_array().unwrap();

    let sequential_tools: Vec<String> =
        sequential.iter().map(|cmd| cmd.as_str().unwrap().to_string()).collect();

    for tool in &expected_tools {
        assert!(
            sequential_tools.contains(&tool.to_string()),
            "Sequential tool '{}' should be in sequential category",
            tool
        );
    }
}

#[test]
fn test_all_sequential_mcp_handlers_callable() {
    use std::sync::Arc;
    use syncore::macro_tools::executor_real::RealExecutor;

    let state = create_test_state();
    let executor = RealExecutor::new(Arc::new(state));
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Test all 9 sequential tools are callable through executor
    let sequential_tools = vec![
        (
            "sequential_next",
            json!({
                "task_id": 1,
                "step_number": 1,
                "thought": "Test thought",
                "reasoning": "Test reasoning",
                "dry_run": false
            }),
        ),
        (
            "sequential_run",
            json!({
                "sequence_id": "test_seq",
                "max_steps": 5,
                "dry_run": false
            }),
        ),
        (
            "sequential_reason",
            json!({
                "context": "Test context",
                "max_cycles": 3,
                "dry_run": false
            }),
        ),
        (
            "sequential_status",
            json!({
                "sequence_id": "test_seq",
                "dry_run": false
            }),
        ),
        (
            "sequential_reset",
            json!({
                "sequence_id": "test_seq",
                "task_id": 1,
                "dry_run": false
            }),
        ),
        (
            "sequential_record",
            json!({
                "task_id": 1,
                "step_number": 1,
                "thought": "Record test thought",
                "reasoning": "Record test reasoning",
                "action": "Record test action",
                "observation": "Record test observation",
                "dry_run": false
            }),
        ),
        (
            "sequential_get",
            json!({
                "task_id": 1,
                "dry_run": false
            }),
        ),
        (
            "sequential_search",
            json!({
                "query": "test search query",
                "dry_run": false
            }),
        ),
        (
            "sequential_cycle",
            json!({
                "max_cycles": 3,
                "dry_run": false
            }),
        ),
    ];

    for (tool_name, params) in sequential_tools {
        let result =
            rt.block_on(async { executor.execute_real_tool_async(tool_name, &params).await });

        // All should be routable (return Ok, even if internal validation fails)
        assert!(
            result.is_ok(),
            "Sequential tool '{}' should be routable through memory_suite: {:?}",
            tool_name,
            result.err()
        );

        let envelope = result.unwrap();

        // Should return a proper JSON structure with 'ok' field and optional 'data'
        assert!(
            envelope.get("ok").is_some(),
            "Response should have 'ok' field for tool '{}', got: {}",
            tool_name,
            envelope
        );

        if envelope.get("ok").unwrap().as_bool().unwrap_or(false) {
            // On success, should have data field
            assert!(
                envelope.get("data").is_some(),
                "Success response should have 'data' field for tool '{}'",
                tool_name
            );
        }
    }
}
