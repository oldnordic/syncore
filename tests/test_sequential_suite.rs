//! Sequential Suite Tests - TDD for Missing Sequential Tools
//!
//! Phase 6 - TDD tests for all 9 sequential tools
//! These tests validate that RealExecutor returns proper envelope-wrapped responses.
//!
//! Tools covered (9 sequential tools):
//! 1. sequential_next
//! 2. sequential_run
//! 3. sequential_reason
//! 4. sequential_status
//! 5. sequential_reset
//! 6. sequential_record
//! 7. sequential_get
//! 8. sequential_search
//! 9. sequential_cycle
//!
//! Test methodology:
//! - Use real SQLite in-memory databases
//! - Call execute_real_tool_async directly
//! - Validate envelope structure and JSON schemas
//! - Test state persistence across sequential operations
//! - Verify integration with memory_suite and reasoning engine

mod real_executor_test_helpers;
use real_executor_test_helpers::{
    assert_error_envelope, assert_error_fields, assert_success_envelope, unwrap_data, unwrap_error,
};

use serde_json::json;
use std::sync::{Arc, Mutex};
use syncore::macro_tools::executor_real::RealExecutor;
use syncore::memory::Memory;
use syncore::router::SynCoreState;
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};

/// Helper to create a RealExecutor with fresh state
fn create_test_executor(suffix: &str) -> RealExecutor {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let db_path = format!(":memory:_sequential_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);

    RealExecutor::new(Arc::new(state))
}

// ============================================================================
// Test 1: sequential_tools_exist_in_help
// ============================================================================

#[test]
fn test_sequential_tools_exist_in_help() {
    let executor = create_test_executor("help");

    let params = json!({
        "command": "help",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("memory_suite", &params).await });

    // Should succeed and contain help text
    assert!(result.is_ok(), "memory_suite help should succeed: {:?}", result.err());
    let envelope = result.unwrap();
    assert_success_envelope(&envelope);

    let data = unwrap_data(&envelope);
    let help_text = data["text"].as_str().expect("help should return text");

    // Verify all sequential tools are listed in help
    assert!(help_text.contains("sequential_next"), "help should list sequential_next");
    assert!(help_text.contains("sequential_run"), "help should list sequential_run");
    assert!(help_text.contains("sequential_reason"), "help should list sequential_reason");
    assert!(help_text.contains("sequential_status"), "help should list sequential_status");
    assert!(help_text.contains("sequential_reset"), "help should list sequential_reset");
    assert!(help_text.contains("sequential_record"), "help should list sequential_record");
    assert!(help_text.contains("sequential_get"), "help should list sequential_get");
    assert!(help_text.contains("sequential_search"), "help should list sequential_search");
    assert!(help_text.contains("sequential_cycle"), "help should list sequential_cycle");
}

// ============================================================================
// Test 2: sequential_next_invocable
// ============================================================================

#[test]
fn test_sequential_next_invocable() {
    let executor = create_test_executor("next");

    let params = json!({
        "task_id": 1,
        "step_number": 1,
        "thought": "Next step analysis",
        "action": "analyze code",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("sequential_next", &params).await });

    // Should succeed
    assert!(result.is_ok(), "sequential_next should succeed: {:?}", result.err());
    let envelope = result.unwrap();
    assert_success_envelope(&envelope);

    let data = unwrap_data(&envelope);
    assert!(data.get("step_id").is_some(), "Should return step_id");
    assert!(data.get("sequence_id").is_some(), "Should return sequence_id");
}

// ============================================================================
// Test 3: sequential_run_invocable
// ============================================================================

#[test]
fn test_sequential_run_invocable() {
    let executor = create_test_executor("run");

    let params = json!({
        "sequence_id": "seq_123",
        "max_steps": 5,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("sequential_run", &params).await });

    // Should succeed
    assert!(result.is_ok(), "sequential_run should succeed: {:?}", result.err());
    let envelope = result.unwrap();
    assert_success_envelope(&envelope);

    let data = unwrap_data(&envelope);
    assert!(data.get("executed_steps").is_some(), "Should return executed_steps");
    assert!(data.get("final_status").is_some(), "Should return final_status");
}

// ============================================================================
// Test 4: sequential_reason_invocable
// ============================================================================

#[test]
fn test_sequential_reason_invocable() {
    let executor = create_test_executor("reason");

    let params = json!({
        "context": "Task requires analysis of code dependencies",
        "depth": 3,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("sequential_reason", &params).await });

    // Should succeed
    assert!(result.is_ok(), "sequential_reason should succeed: {:?}", result.err());
    let envelope = result.unwrap();
    assert_success_envelope(&envelope);

    let data = unwrap_data(&envelope);
    assert!(data.get("reasoning_steps").is_some(), "Should return reasoning_steps");
    assert!(data.get("conclusion").is_some(), "Should return conclusion");
}

// ============================================================================
// Test 5: sequential_status_invocable
// ============================================================================

#[test]
fn test_sequential_status_invocable() {
    let executor = create_test_executor("status");

    let params = json!({
        "sequence_id": "seq_123",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("sequential_status", &params).await });

    // Should succeed
    assert!(result.is_ok(), "sequential_status should succeed: {:?}", result.err());
    let envelope = result.unwrap();
    assert_success_envelope(&envelope);

    let data = unwrap_data(&envelope);
    assert!(data.get("sequence_id").is_some(), "Should return sequence_id");
    assert!(data.get("current_step").is_some(), "Should return current_step");
    assert!(data.get("total_steps").is_some(), "Should return total_steps");
    assert!(data.get("status").is_some(), "Should return status");
}

// ============================================================================
// Test 6: sequential_reset_invocable
// ============================================================================

#[test]
fn test_sequential_reset_invocable() {
    let executor = create_test_executor("reset");

    let params = json!({
        "sequence_id": "seq_123",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("sequential_reset", &params).await });

    // Should succeed
    assert!(result.is_ok(), "sequential_reset should succeed: {:?}", result.err());
    let envelope = result.unwrap();
    assert_success_envelope(&envelope);

    let data = unwrap_data(&envelope);
    assert!(data.get("reset").is_some(), "Should return reset confirmation");
    assert_eq!(data["reset"], true, "Should confirm reset was performed");
}

// ============================================================================
// Test 7: sequential_record_invocable
// ============================================================================

#[test]
fn test_sequential_record_invocable() {
    let executor = create_test_executor("record");

    let params = json!({
        "task_id": 1,
        "step_number": 1,
        "thought": "Analysis of code structure",
        "reasoning": "Need to understand dependencies",
        "action": "examine imports",
        "observation": "Found 5 dependencies",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("sequential_record", &params).await });

    // Should succeed
    assert!(result.is_ok(), "sequential_record should succeed: {:?}", result.err());
    let envelope = result.unwrap();
    assert_success_envelope(&envelope);

    let data = unwrap_data(&envelope);
    assert!(data.get("recorded").is_some(), "Should return recorded confirmation");
    assert!(data.get("step_id").is_some(), "Should return step_id");
}

// ============================================================================
// Test 8: sequential_get_invocable
// ============================================================================

#[test]
fn test_sequential_get_invocable() {
    let executor = create_test_executor("get");

    let params = json!({
        "task_id": 1,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("sequential_get", &params).await });

    // Should succeed
    assert!(result.is_ok(), "sequential_get should succeed: {:?}", result.err());
    let envelope = result.unwrap();
    assert_success_envelope(&envelope);

    let data = unwrap_data(&envelope);
    assert!(data.get("steps").is_some(), "Should return steps array");
    assert!(data["steps"].is_array(), "Steps should be an array");
}

// ============================================================================
// Test 9: sequential_search_invocable
// ============================================================================

#[test]
fn test_sequential_search_invocable() {
    let executor = create_test_executor("search");

    let params = json!({
        "query": "dependency analysis",
        "limit": 10,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("sequential_search", &params).await });

    // Should succeed
    assert!(result.is_ok(), "sequential_search should succeed: {:?}", result.err());
    let envelope = result.unwrap();
    assert_success_envelope(&envelope);

    let data = unwrap_data(&envelope);
    assert!(data.get("results").is_some(), "Should return results array");
    assert!(data["results"].is_array(), "Results should be an array");
}

// ============================================================================
// Test 10: sequential_cycle_invocable
// ============================================================================

#[test]
fn test_sequential_cycle_invocable() {
    let executor = create_test_executor("cycle");

    let params = json!({
        "max_cycles": 3,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { executor.execute_real_tool_async("sequential_cycle", &params).await });

    // Should succeed
    assert!(result.is_ok(), "sequential_cycle should succeed: {:?}", result.err());
    let envelope = result.unwrap();
    assert_success_envelope(&envelope);

    let data = unwrap_data(&envelope);
    assert!(data.get("cycles_detected").is_some(), "Should return cycles_detected");
    assert!(data.get("recommendations").is_some(), "Should return recommendations");
}

// ============================================================================
// Test 11: sequential_routing_chain
// ============================================================================

#[test]
fn test_sequential_routing_chain() {
    // Verify that all sequential tools route through the same chain
    let executor = create_test_executor("routing");

    let sequential_tools = vec![
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

    let rt = tokio::runtime::Runtime::new().unwrap();

    for tool_name in sequential_tools {
        let params = json!({
            "dry_run": true  // Use dry_run to avoid side effects
        });

        let result =
            rt.block_on(async { executor.execute_real_tool_async(tool_name, &params).await });

        // All should route successfully (even if they fail, they should be found)
        assert!(result.is_ok(), "Tool {} should be routable: {:?}", tool_name, result.err());
    }
}

// ============================================================================
// Test 12: sequential_persistence
// ============================================================================

#[test]
fn test_sequential_persistence() {
    let executor = create_test_executor("persistence");

    let rt = tokio::runtime::Runtime::new().unwrap();

    // Step 1: Record a sequential step
    let record_params = json!({
        "task_id": 42,
        "step_number": 1,
        "thought": "Initial analysis",
        "reasoning": "Need to examine the codebase",
        "action": "scan files",
        "observation": "Found main.rs",
        "dry_run": false
    });

    let record_result = rt.block_on(async {
        executor.execute_real_tool_async("sequential_record", &record_params).await
    });

    assert!(record_result.is_ok(), "sequential_record should succeed");
    let record_envelope = record_result.unwrap();
    assert_success_envelope(&record_envelope);

    // Step 2: Retrieve the recorded step
    let get_params = json!({
        "task_id": 42,
        "dry_run": false
    });

    let get_result = rt
        .block_on(async { executor.execute_real_tool_async("sequential_get", &get_params).await });

    assert!(get_result.is_ok(), "sequential_get should succeed");
    let get_envelope = get_result.unwrap();
    assert_success_envelope(&get_envelope);

    let data = unwrap_data(&get_envelope);
    let steps = data["steps"].as_array().expect("Should return steps array");

    // Should find the recorded step
    assert!(!steps.is_empty(), "Should have recorded steps");

    // Verify the step content matches what we recorded
    let found_step = steps.iter().find(|step| {
        step["step_number"].as_i64() == Some(1)
            && step["thought"].as_str() == Some("Initial analysis")
    });

    assert!(found_step.is_some(), "Should find the recorded step with correct content");
}

// ============================================================================
// Test 13: sequential_reasoning_pipeline
// ============================================================================

#[test]
fn test_sequential_reasoning_pipeline() {
    let executor = create_test_executor("reasoning");

    let rt = tokio::runtime::Runtime::new().unwrap();

    // Step 1: Use reasoning engine to analyze a problem
    let reason_params = json!({
        "context": "Code has performance issues in database queries",
        "depth": 2,
        "dry_run": false
    });

    let reason_result = rt.block_on(async {
        executor.execute_real_tool_async("sequential_reason", &reason_params).await
    });

    assert!(reason_result.is_ok(), "sequential_reason should succeed");
    let reason_envelope = reason_result.unwrap();
    assert_success_envelope(&reason_envelope);

    let data = unwrap_data(&reason_envelope);
    assert!(data.get("reasoning_steps").is_some(), "Should return reasoning_steps");
    assert!(data.get("conclusion").is_some(), "Should return conclusion");

    // Verify reasoning structure
    let reasoning_steps = data["reasoning_steps"].as_array().expect("Should be array");
    assert!(!reasoning_steps.is_empty(), "Should have reasoning steps");
}

// ============================================================================
// Test 14: sequential_records_chain_correct
// ============================================================================

#[test]
fn test_sequential_records_chain_correct() {
    let executor = create_test_executor("chain");

    let rt = tokio::runtime::Runtime::new().unwrap();

    let task_id = 123i64;

    // Record multiple steps in sequence
    for step_num in 1..=3 {
        let params = json!({
            "task_id": task_id,
            "step_number": step_num,
            "thought": format!("Step {} analysis", step_num),
            "reasoning": format!("Reasoning for step {}", step_num),
            "action": format!("Action for step {}", step_num),
            "observation": format!("Observation for step {}", step_num),
            "dry_run": false
        });

        let result = rt.block_on(async {
            executor.execute_real_tool_async("sequential_record", &params).await
        });

        assert!(result.is_ok(), "Step {} should record successfully", step_num);
    }

    // Retrieve all steps and verify order
    let get_params = json!({
        "task_id": task_id,
        "dry_run": false
    });

    let get_result = rt
        .block_on(async { executor.execute_real_tool_async("sequential_get", &get_params).await });

    assert!(get_result.is_ok(), "Should retrieve all steps");
    let get_envelope = get_result.unwrap();
    assert_success_envelope(&get_envelope);

    let data = unwrap_data(&get_envelope);
    let steps = data["steps"].as_array().expect("Should return steps array");
    assert_eq!(steps.len(), 3, "Should have all 3 steps");

    // Verify steps are in correct order
    for (i, step) in steps.iter().enumerate() {
        let expected_step = (i + 1) as i64;
        assert_eq!(
            step["step_number"].as_i64(),
            Some(expected_step),
            "Step {} should have correct step_number",
            i + 1
        );
        assert_eq!(
            step["thought"].as_str(),
            Some(&format!("Step {} analysis", expected_step)).map(|x| x.as_str()),
            "Step {} should have correct thought",
            i + 1
        );
    }
}

// ============================================================================
// Test 15: sequential_reset_clears_chain
// ============================================================================

#[test]
fn test_sequential_reset_clears_chain() {
    let executor = create_test_executor("reset_chain");

    let rt = tokio::runtime::Runtime::new().unwrap();

    let sequence_id = "test_sequence_456";
    let task_id = 456i64;

    // Step 1: Record some steps
    for step_num in 1..=3 {
        let params = json!({
            "task_id": task_id,
            "step_number": step_num,
            "thought": format!("Step {} thought", step_num),
            "reasoning": format!("Step {} reasoning", step_num),
            "dry_run": false
        });

        rt.block_on(async { executor.execute_real_tool_async("sequential_record", &params).await })
            .expect("Step should record");
    }

    // Step 2: Verify steps exist
    let get_params = json!({
        "task_id": task_id,
        "dry_run": false
    });

    let get_result = rt
        .block_on(async { executor.execute_real_tool_async("sequential_get", &get_params).await })
        .expect("Should get steps");

    let get_data = unwrap_data(&get_result);
    let steps_before = get_data["steps"].as_array().expect("Should return steps");
    assert_eq!(steps_before.len(), 3, "Should have 3 steps before reset");

    // Step 3: Reset the sequence
    let reset_params = json!({
        "sequence_id": sequence_id,
        "task_id": task_id,
        "dry_run": false
    });

    let reset_result = rt.block_on(async {
        executor.execute_real_tool_async("sequential_reset", &reset_params).await
    });

    assert!(reset_result.is_ok(), "sequential_reset should succeed");
    let reset_envelope = reset_result.unwrap();
    assert_success_envelope(&reset_envelope);

    let reset_data = unwrap_data(&reset_envelope);
    assert_eq!(reset_data["reset"], true, "Should confirm reset");

    // Step 4: Verify steps are cleared
    let get_after_result = rt
        .block_on(async { executor.execute_real_tool_async("sequential_get", &get_params).await });

    assert!(get_after_result.is_ok(), "Should still be able to get after reset");
    let binding = get_after_result.unwrap();
    let get_after_data = unwrap_data(&binding);
    let steps_after = get_after_data["steps"].as_array().expect("Should return steps");
    assert_eq!(steps_after.len(), 0, "Should have no steps after reset");
}
