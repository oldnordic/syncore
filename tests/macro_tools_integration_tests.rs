//! Macro Tools Integration Tests
//!
//! End-to-end validation of the logical path:
//! request → planner → macro tool → execution interface → RealExecutorStub → synthetic result
//!
//! CRITICAL: NO REAL I/O
//! - Use in-memory SQLite (`:memory:`)
//! - Use fake Neo4j client mocks
//! - Use fake vector index context
//! - Use stubbed tree-sitter results
//! - Use RealExecutorStub for deterministic results
//!
//! These tests validate MCP tool signatures WITHOUT calling real databases.

use serde_json::json;
use syncore::macro_tools::code::execute_code_macro;
use syncore::macro_tools::executor_stub::RealExecutorStub;
use syncore::macro_tools::planner::{CodeMacroPlan, TaskMacroPlan};
use syncore::macro_tools::task::execute_task_macro;

// ============================================================================
// TEST 1: Code Semantic Search End-to-End
// ============================================================================

#[test]
fn test_code_semantic_search_e2e() {
    // Setup: Create executor stub
    let stub = RealExecutorStub::new();

    // Create macro request
    let request = json!({
        "action": "semantic_search",
        "query": "find async message bus implementation",
        "limit": 5
    });

    // Execute via macro tool handler
    execute_code_macro(&request, &stub).expect("semantic_search should succeed");

    // Validate: Plan was created correctly
    let plan = CodeMacroPlan::from_request(&request).expect("should create plan");
    match plan {
        CodeMacroPlan::SemanticSearch {
            query,
            limit,
        } => {
            assert_eq!(query, "find async message bus implementation");
            assert_eq!(limit, 5);
        }
        _ => panic!("Expected SemanticSearch plan"),
    }

    // Validate: Execution ordering
    let steps = stub.get_executed_steps();
    assert_eq!(steps.len(), 3, "Should execute 3 steps");
    assert_eq!(steps[0].tool_name, "mapping_search");
    assert_eq!(steps[1].tool_name, "code_search");
    assert_eq!(steps[2].tool_name, "vector_search");

    // Validate: Arguments flow through actual MCP tool API types
    assert_eq!(steps[0].params["query"], "find async message bus implementation");
    assert_eq!(steps[1].params["query"], "find async message bus implementation");
    assert_eq!(steps[2].params["query"], "find async message bus implementation");
    assert_eq!(steps[2].params["limit"], 5);

    // Validate: Synthetic results match expected structure
    assert!(steps[0].synthetic_result["results"].is_array());
    assert!(steps[1].synthetic_result["matches"].is_array());
    assert!(steps[2].synthetic_result["results"].is_array());
    assert_eq!(steps[2].synthetic_result["results"].as_array().unwrap().len(), 5);
}

// ============================================================================
// TEST 2: Code Analyze Module End-to-End
// ============================================================================

#[test]
fn test_code_analyze_module_e2e() {
    let stub = RealExecutorStub::new();

    // Create macro request with proper MCP tool types
    let request = json!({
        "action": "analyze_module",
        "file_path": "/src/message_bus.rs",
        "focus": "agent communication patterns"
    });

    // Execute
    execute_code_macro(&request, &stub).expect("analyze_module should succeed");

    // Validate plan creation
    let plan = CodeMacroPlan::from_request(&request).expect("should create plan");
    match plan {
        CodeMacroPlan::AnalyzeModule {
            file_path,
            focus,
        } => {
            assert_eq!(file_path, "/src/message_bus.rs");
            assert_eq!(focus, "agent communication patterns");
        }
        _ => panic!("Expected AnalyzeModule plan"),
    }

    // Validate execution
    let steps = stub.get_executed_steps();
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].tool_name, "parser_analyze");
    assert_eq!(steps[1].tool_name, "mapping_deps");
    assert_eq!(steps[2].tool_name, "code_search");

    // Validate arguments use proper MCP tool request types
    assert_eq!(steps[0].params["file_path"], "/src/message_bus.rs");
    assert_eq!(steps[1].params["path"], "/src/message_bus.rs");
    assert_eq!(steps[2].params["query"], "agent communication patterns");

    // Validate synthetic results
    assert!(steps[0].synthetic_result["functions"].is_array());
    assert!(steps[0].synthetic_result["structs"].is_array());
    assert!(steps[1].synthetic_result["dependencies"].is_array());
    assert!(steps[2].synthetic_result["matches"].is_array());
}

// ============================================================================
// TEST 3: Task Next End-to-End
// ============================================================================

#[test]
fn test_task_next_e2e() {
    let stub = RealExecutorStub::new();

    // Create macro request
    let request = json!({
        "action": "next",
        "prd_title": "Macro Tools Implementation",
        "strategy": "priority"
    });

    // Execute
    execute_task_macro(&request, &stub).expect("task next should succeed");

    // Validate plan
    let plan = TaskMacroPlan::from_request(&request).expect("should create plan");
    match plan {
        TaskMacroPlan::Next {
            prd_title,
            strategy,
        } => {
            assert_eq!(prd_title, Some("Macro Tools Implementation".to_string()));
            assert_eq!(strategy, "priority");
        }
        _ => panic!("Expected Next plan"),
    }

    // Validate execution sequence
    let steps = stub.get_executed_steps();
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].tool_name, "intellitask_task_statistics");
    assert_eq!(steps[1].tool_name, "intellitask_next_ready");
    assert_eq!(steps[2].tool_name, "intellitask_prioritize");

    // Validate arguments
    assert_eq!(steps[2].params["strategy"], "priority");

    // Validate synthetic results structure
    assert!(steps[0].synthetic_result["total_tasks"].is_number());
    assert!(steps[0].synthetic_result["completed"].is_number());
    assert!(steps[1].synthetic_result["ready_tasks"].is_array());
    assert!(steps[2].synthetic_result["task_id"].is_number());
    assert!(steps[2].synthetic_result["priority_score"].is_number());
}

// ============================================================================
// TEST 4: Task Bootstrap From PRD End-to-End
// ============================================================================

#[test]
fn test_task_bootstrap_from_prd_e2e() {
    let stub = RealExecutorStub::new();

    // Create macro request with PRD text
    let prd_text = "Build a comprehensive user authentication system with OAuth2 support";
    let request = json!({
        "action": "bootstrap_from_prd",
        "prd_text": prd_text,
        "auto_expand": true
    });

    // Execute
    execute_task_macro(&request, &stub).expect("bootstrap_from_prd should succeed");

    // Validate plan
    let plan = TaskMacroPlan::from_request(&request).expect("should create plan");
    match plan {
        TaskMacroPlan::BootstrapFromPRD {
            prd_text: text,
            auto_expand,
        } => {
            assert_eq!(text, prd_text);
            assert_eq!(auto_expand, true);
        }
        _ => panic!("Expected BootstrapFromPRD plan"),
    }

    // Validate execution sequence
    let steps = stub.get_executed_steps();
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].tool_name, "intellitask_generate");
    assert_eq!(steps[1].tool_name, "intellitask_save");
    assert_eq!(steps[2].tool_name, "intellitask_subtasks");

    // Validate argument normalization
    assert!(steps[0].params["prd_content"].as_str().unwrap().contains("authentication"));
    assert!(steps[0].params["prd_content"].as_str().unwrap().contains("OAuth2"));

    // Validate stable synthetic outputs
    assert!(steps[0].synthetic_result["tasks"].is_array());
    let tasks = steps[0].synthetic_result["tasks"].as_array().unwrap();
    assert!(!tasks.is_empty(), "Should generate tasks from PRD");

    assert_eq!(steps[1].synthetic_result["saved"], true);
    assert!(steps[2].synthetic_result["subtasks"].is_array());
}

// ============================================================================
// TEST 5: Error Handling End-to-End
// ============================================================================

#[test]
fn test_error_handling_e2e() {
    let stub = RealExecutorStub::new();

    // Test 1: Missing required field (query)
    let request1 = json!({
        "action": "semantic_search",
        "limit": 5
        // Missing "query"
    });

    let result1 = execute_code_macro(&request1, &stub);
    assert!(result1.is_err(), "Should produce macro-level error, not panic");
    let error1 = result1.unwrap_err().to_string();
    assert!(error1.contains("query"), "Error should mention missing field");

    // Test 2: Missing required field (file_path)
    let request2 = json!({
        "action": "analyze_module",
        "focus": "test"
        // Missing "file_path"
    });

    let result2 = execute_code_macro(&request2, &stub);
    assert!(result2.is_err(), "Should produce macro-level error");
    let error2 = result2.unwrap_err().to_string();
    assert!(error2.contains("file_path"), "Error should mention missing field");

    // Test 3: Invalid action
    let request3 = json!({
        "action": "invalid_action"
    });

    let result3 = execute_code_macro(&request3, &stub);
    assert!(result3.is_err(), "Should reject invalid action");

    // Test 4: Missing action field
    let request4 = json!({
        "query": "test"
    });

    let result4 = execute_code_macro(&request4, &stub);
    assert!(result4.is_err(), "Should require action field");
    let error4 = result4.unwrap_err().to_string();
    assert!(error4.contains("action"), "Error should mention missing action");
}

// ============================================================================
// TEST 6: Deterministic Behavior End-to-End
// ============================================================================

#[test]
fn test_deterministic_behavior_e2e() {
    // Create identical requests
    let request = json!({
        "action": "semantic_search",
        "query": "test determinism",
        "limit": 3
    });

    // Execute twice with different stub instances
    let stub1 = RealExecutorStub::new();
    execute_code_macro(&request, &stub1).expect("first execution should succeed");
    let steps1 = stub1.get_executed_steps();

    let stub2 = RealExecutorStub::new();
    execute_code_macro(&request, &stub2).expect("second execution should succeed");
    let steps2 = stub2.get_executed_steps();

    // Validate identical results
    assert_eq!(steps1.len(), steps2.len());

    for i in 0..steps1.len() {
        assert_eq!(steps1[i].tool_name, steps2[i].tool_name);
        assert_eq!(steps1[i].params, steps2[i].params);
        assert_eq!(steps1[i].synthetic_result, steps2[i].synthetic_result);
    }

    // Test with task macro as well
    let task_request = json!({
        "action": "next",
        "strategy": "priority"
    });

    let task_stub1 = RealExecutorStub::new();
    execute_task_macro(&task_request, &task_stub1).expect("first task execution should succeed");
    let task_steps1 = task_stub1.get_executed_steps();

    let task_stub2 = RealExecutorStub::new();
    execute_task_macro(&task_request, &task_stub2).expect("second task execution should succeed");
    let task_steps2 = task_stub2.get_executed_steps();

    // Validate identical task results
    assert_eq!(task_steps1.len(), task_steps2.len());

    for i in 0..task_steps1.len() {
        assert_eq!(task_steps1[i].tool_name, task_steps2[i].tool_name);
        assert_eq!(task_steps1[i].params, task_steps2[i].params);
        assert_eq!(task_steps1[i].synthetic_result, task_steps2[i].synthetic_result);
    }
}
