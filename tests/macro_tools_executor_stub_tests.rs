//! Executor Stub Tests
//!
//! Tests for RealExecutorStub - a production-like executor that performs
//! deterministic multi-step orchestration WITHOUT calling real MCP tools.
//!
//! This validates:
//! 1. Step ordering matches planner expectations
//! 2. Arguments are propagated correctly
//! 3. Synthetic responses are type-stable and deterministic
//! 4. Error detection happens early (missing fields, invalid types)
//! 5. NO real I/O occurs (no SQLite, Neo4j, vectors, Ollama, file writes)

use anyhow::Result;
use serde_json::{json, Value};
use syncore::macro_tools::code::execute_code_macro;
use syncore::macro_tools::executor_stub::RealExecutorStub;
use syncore::macro_tools::task::execute_task_macro;

// ============================================================================
// TEST 1: Code Semantic Search - Step Ordering
// ============================================================================

#[test]
fn test_stub_executes_code_semantic_search_steps_in_order() {
    let stub = RealExecutorStub::new();

    let request = json!({
        "action": "semantic_search",
        "query": "find async message bus implementation",
        "limit": 5
    });

    execute_code_macro(&request, &stub).unwrap();

    // Verify execution order
    let steps = stub.get_executed_steps();
    assert_eq!(steps.len(), 3, "Should execute 3 steps");

    assert_eq!(steps[0].tool_name, "mapping_search");
    assert_eq!(
        steps[0].params["query"],
        "find async message bus implementation"
    );

    assert_eq!(steps[1].tool_name, "code_search");
    assert_eq!(
        steps[1].params["query"],
        "find async message bus implementation"
    );

    assert_eq!(steps[2].tool_name, "vector_search");
    assert_eq!(
        steps[2].params["query"],
        "find async message bus implementation"
    );
    assert_eq!(steps[2].params["limit"], 5);
}

// ============================================================================
// TEST 2: Code Analyze Module - Step Ordering
// ============================================================================

#[test]
fn test_stub_executes_code_analyze_module_steps() {
    let stub = RealExecutorStub::new();

    let request = json!({
        "action": "analyze_module",
        "file_path": "/src/message_bus.rs",
        "focus": "agent communication"
    });

    execute_code_macro(&request, &stub).unwrap();

    let steps = stub.get_executed_steps();
    assert_eq!(steps.len(), 3, "Should execute 3 steps");

    assert_eq!(steps[0].tool_name, "parser_analyze");
    assert_eq!(steps[0].params["file_path"], "/src/message_bus.rs");

    assert_eq!(steps[1].tool_name, "mapping_deps");
    assert_eq!(steps[1].params["path"], "/src/message_bus.rs");

    assert_eq!(steps[2].tool_name, "code_search");
    assert_eq!(steps[2].params["query"], "agent communication");
}

// ============================================================================
// TEST 3: Task Next - Step Ordering
// ============================================================================

#[test]
fn test_stub_executes_task_next_steps_in_order() {
    let stub = RealExecutorStub::new();

    let request = json!({
        "action": "next",
        "prd_title": "Macro Tools Implementation",
        "strategy": "priority"
    });

    execute_task_macro(&request, &stub).unwrap();

    let steps = stub.get_executed_steps();
    assert_eq!(steps.len(), 3, "Should execute 3 steps");

    assert_eq!(steps[0].tool_name, "intellitask_task_statistics");
    assert_eq!(steps[1].tool_name, "intellitask_next_ready");
    assert_eq!(steps[2].tool_name, "intellitask_prioritize");
    assert_eq!(steps[2].params["strategy"], "priority");
}

// ============================================================================
// TEST 4: Structured Fake Results
// ============================================================================

#[test]
fn test_stub_injects_structured_fake_results_for_each_step() {
    let stub = RealExecutorStub::new();

    let request = json!({
        "action": "semantic_search",
        "query": "test query",
        "limit": 3
    });

    execute_code_macro(&request, &stub).unwrap();

    let steps = stub.get_executed_steps();

    // Step 1: mapping_search should have fake file results
    let mapping_result = &steps[0].synthetic_result;
    assert!(mapping_result.is_object());
    assert!(mapping_result["results"].is_array());
    let files = mapping_result["results"].as_array().unwrap();
    assert!(!files.is_empty(), "Should have synthetic file results");

    // Step 2: code_search should have fake code matches
    let code_result = &steps[1].synthetic_result;
    assert!(code_result.is_object());
    assert!(code_result["matches"].is_array());
    let matches = code_result["matches"].as_array().unwrap();
    assert!(!matches.is_empty(), "Should have synthetic code matches");

    // Step 3: vector_search should have fake vector hits
    let vector_result = &steps[2].synthetic_result;
    assert!(vector_result.is_object());
    assert!(vector_result["results"].is_array());
    let results = vector_result["results"].as_array().unwrap();
    assert!(!results.is_empty(), "Should have synthetic vector results");
    assert_eq!(results.len(), 3, "Should respect limit parameter");
}

// ============================================================================
// TEST 5: Task Synthetic Results
// ============================================================================

#[test]
fn test_stub_task_next_produces_structured_results() {
    let stub = RealExecutorStub::new();

    let request = json!({
        "action": "next",
        "strategy": "priority"
    });

    execute_task_macro(&request, &stub).unwrap();

    let steps = stub.get_executed_steps();

    // Step 1: task_statistics should have fake stats
    let stats_result = &steps[0].synthetic_result;
    assert!(stats_result.is_object());
    assert!(stats_result["total_tasks"].is_number());
    assert!(stats_result["completed"].is_number());
    assert!(stats_result["pending"].is_number());

    // Step 2: next_ready should have fake ready tasks
    let ready_result = &steps[1].synthetic_result;
    assert!(ready_result.is_object());
    assert!(ready_result["ready_tasks"].is_array());

    // Step 3: prioritize should have prioritized task
    let prioritize_result = &steps[2].synthetic_result;
    assert!(prioritize_result.is_object());
    assert!(prioritize_result["task_id"].is_number());
    assert!(prioritize_result["priority_score"].is_number());
}

// ============================================================================
// TEST 6: Error Detection - Missing Fields
// ============================================================================

#[test]
fn test_stub_detects_missing_fields() {
    let stub = RealExecutorStub::new();

    // Missing 'query' field
    let request = json!({
        "action": "semantic_search",
        "limit": 5
    });

    let result = execute_code_macro(&request, &stub);
    assert!(result.is_err(), "Should detect missing query field");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("query"),
        "Error should mention missing field"
    );
}

// ============================================================================
// TEST 7: Argument Preservation
// ============================================================================

#[test]
fn test_stub_preserves_arguments_exactly() {
    let stub = RealExecutorStub::new();

    let complex_query = "find all async fn implementations with Result<T, E> return types";
    let file_path = "/very/deep/nested/path/to/module.rs";

    let request = json!({
        "action": "analyze_module",
        "file_path": file_path,
        "focus": complex_query
    });

    execute_code_macro(&request, &stub).unwrap();

    let steps = stub.get_executed_steps();

    // Verify exact argument preservation
    assert_eq!(steps[0].params["file_path"], file_path);
    assert_eq!(steps[1].params["path"], file_path);
    assert_eq!(steps[2].params["query"], complex_query);
}

// ============================================================================
// TEST 8: Deterministic Behavior
// ============================================================================

#[test]
fn test_stub_produces_deterministic_results() {
    let request = json!({
        "action": "semantic_search",
        "query": "test determinism",
        "limit": 5
    });

    // Execute twice
    let stub1 = RealExecutorStub::new();
    execute_code_macro(&request, &stub1).unwrap();
    let steps1 = stub1.get_executed_steps();

    let stub2 = RealExecutorStub::new();
    execute_code_macro(&request, &stub2).unwrap();
    let steps2 = stub2.get_executed_steps();

    // Results should be identical
    assert_eq!(steps1.len(), steps2.len());

    for i in 0..steps1.len() {
        assert_eq!(steps1[i].tool_name, steps2[i].tool_name);
        assert_eq!(steps1[i].params, steps2[i].params);
        assert_eq!(steps1[i].synthetic_result, steps2[i].synthetic_result);
    }
}

// ============================================================================
// TEST 9: Code Index Directory Results
// ============================================================================

#[test]
fn test_stub_code_index_directory_produces_results() {
    let stub = RealExecutorStub::new();

    let request = json!({
        "action": "index_directory",
        "directory": "/src/macro_tools",
        "pattern": "**/*.rs"
    });

    execute_code_macro(&request, &stub).unwrap();

    let steps = stub.get_executed_steps();
    assert_eq!(steps.len(), 2, "Should execute 2 steps");

    // Step 1: code_index_directory
    assert_eq!(steps[0].tool_name, "code_index_directory");
    let index_result = &steps[0].synthetic_result;
    assert!(index_result["indexed_files"].is_number());
    assert!(index_result["indexed_files"].as_i64().unwrap() > 0);

    // Step 2: mapping_record
    assert_eq!(steps[1].tool_name, "mapping_record");
    let record_result = &steps[1].synthetic_result;
    assert!(record_result["recorded"].is_boolean());
    assert_eq!(record_result["recorded"], true);
}

// ============================================================================
// TEST 10: Task Bootstrap From PRD Results
// ============================================================================

#[test]
fn test_stub_task_bootstrap_produces_results() {
    let stub = RealExecutorStub::new();

    let request = json!({
        "action": "bootstrap_from_prd",
        "prd_text": "Build a new feature for user authentication",
        "auto_expand": true
    });

    execute_task_macro(&request, &stub).unwrap();

    let steps = stub.get_executed_steps();
    assert_eq!(steps.len(), 3);

    // Step 1: intellitask_generate
    let generate_result = &steps[0].synthetic_result;
    assert!(generate_result["tasks"].is_array());
    assert!(!generate_result["tasks"].as_array().unwrap().is_empty());

    // Step 2: intellitask_save
    let save_result = &steps[1].synthetic_result;
    assert!(save_result["saved"].is_boolean());
    assert_eq!(save_result["saved"], true);

    // Step 3: intellitask_subtasks
    let subtasks_result = &steps[2].synthetic_result;
    assert!(subtasks_result["subtasks"].is_array());
}

// ============================================================================
// TEST 11: Task Complete Results
// ============================================================================

#[test]
fn test_stub_task_complete_produces_results() {
    let stub = RealExecutorStub::new();

    let request = json!({
        "action": "complete",
        "task_id": 42,
        "suggest_next": true
    });

    execute_task_macro(&request, &stub).unwrap();

    let steps = stub.get_executed_steps();
    assert_eq!(steps.len(), 3);

    // Step 1: intellitask_update_status
    let update_result = &steps[0].synthetic_result;
    assert!(update_result["updated"].is_boolean());
    assert_eq!(update_result["updated"], true);
    assert_eq!(update_result["task_id"], 42);

    // Step 2: intellitask_subtask_stats
    let stats_result = &steps[1].synthetic_result;
    assert!(stats_result["total_subtasks"].is_number());
    assert!(stats_result["completed_subtasks"].is_number());

    // Step 3: intellitask_next_ready
    let next_result = &steps[2].synthetic_result;
    assert!(next_result["next_task_id"].is_number());
}

// ============================================================================
// TEST 12: No Real I/O Validation
// ============================================================================

#[test]
fn test_stub_performs_no_real_io() {
    let stub = RealExecutorStub::new();

    // Execute multiple operations
    let requests = vec![
        json!({"action": "semantic_search", "query": "test", "limit": 5}),
        json!({"action": "analyze_module", "file_path": "/test.rs", "focus": "test"}),
        json!({"action": "index_directory", "directory": "/test", "pattern": "*.rs"}),
    ];

    for request in requests {
        execute_code_macro(&request, &stub).unwrap();
    }

    // Verify no I/O flags are set
    assert!(!stub.performed_sqlite_io(), "Should not perform SQLite I/O");
    assert!(!stub.performed_neo4j_io(), "Should not perform Neo4j I/O");
    assert!(!stub.performed_vector_io(), "Should not perform vector I/O");
    assert!(!stub.performed_file_io(), "Should not perform file I/O");
    assert!(
        !stub.performed_network_io(),
        "Should not perform network I/O"
    );
}
