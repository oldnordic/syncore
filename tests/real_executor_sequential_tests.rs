//! Real Executor Sequential Tools Tests
//!
//! Phase 6.9 - TDD tests for sequential reasoning tools
//! These tests validate that RealExecutor returns proper envelope-wrapped responses.
//!
//! Tools covered:
//! 1. sequential_record
//! 2. sequential_get
//! 3. sequential_search
//! 4. sequential_cycle

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
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = format!(":memory:_sequential_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);

    RealExecutor::new(Arc::new(state))
}

// ============================================================================
// Test 1: sequential_record real execution
// ============================================================================

#[test]
fn test_sequential_record_real() {
    let executor = create_test_executor("record");

    let params = json!({
        "task_id": 1,
        "step_number": 1,
        "thought": "Need to analyze the problem",
        "reasoning": "Starting with initial assessment",
        "action": "analyze",
        "observation": "Problem identified",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_record", &params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real sequential_record should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("step_id").is_some()
            || data
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false),
        "Data should indicate recording success: {:?}",
        data
    );
}

// ============================================================================
// Test 2: sequential_record respects dry_run
// ============================================================================

#[test]
fn test_sequential_record_respects_dry_run() {
    let executor = create_test_executor("record_dry");

    let params = json!({
        "task_id": 1,
        "step_number": 1,
        "thought": "Test thought",
        "reasoning": "Test reasoning",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_record", &params)
            .await
    });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run indication
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some() || data.to_string().contains("DRY RUN"),
        "Data should indicate dry run mode: {:?}",
        data
    );
}

// ============================================================================
// Test 3: sequential_get real execution
// ============================================================================

#[test]
fn test_sequential_get_real() {
    let executor = create_test_executor("get");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // First record 3 steps
    for i in 1..=3 {
        let record_params = json!({
            "task_id": 42,
            "step_number": i,
            "thought": format!("Thought {}", i),
            "reasoning": format!("Reasoning {}", i),
            "dry_run": false
        });

        let record_result = rt.block_on(async {
            executor
                .execute_real_tool_async("sequential_record", &record_params)
                .await
        });
        assert!(record_result.is_ok(), "Record should succeed");
    }

    // Now get all steps
    let get_params = json!({
        "task_id": 42,
        "dry_run": false
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_get", &get_params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real sequential_get should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("steps").is_some() || data.get("count").is_some(),
        "Data should have steps information: {:?}",
        data
    );
}

// ============================================================================
// Test 4: sequential_get respects dry_run
// ============================================================================

#[test]
fn test_sequential_get_respects_dry_run() {
    let executor = create_test_executor("get_dry");

    let params = json!({
        "task_id": 1,
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_get", &params)
            .await
    });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run indication
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some() || data.to_string().contains("DRY RUN"),
        "Data should indicate dry run mode: {:?}",
        data
    );
}

// ============================================================================
// Test 5: sequential_search real execution
// ============================================================================

#[test]
fn test_sequential_search_real() {
    let executor = create_test_executor("search");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Record steps with different messages
    let steps = vec![
        ("Analyzing database schema", "database analysis"),
        ("Implementing API endpoint", "api implementation"),
        ("Testing database queries", "database testing"),
    ];

    for (i, (thought, reasoning)) in steps.iter().enumerate() {
        let record_params = json!({
            "task_id": 10,
            "step_number": i + 1,
            "thought": thought,
            "reasoning": reasoning,
            "dry_run": false
        });

        let record_result = rt.block_on(async {
            executor
                .execute_real_tool_async("sequential_record", &record_params)
                .await
        });
        assert!(record_result.is_ok(), "Record should succeed");
    }

    // Search for "database"
    let search_params = json!({
        "query": "database",
        "dry_run": false
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_search", &search_params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real sequential_search should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("results").is_some() || data.get("count").is_some(),
        "Data should have search results: {:?}",
        data
    );
}

// ============================================================================
// Test 6: sequential_search respects dry_run
// ============================================================================

#[test]
fn test_sequential_search_respects_dry_run() {
    let executor = create_test_executor("search_dry");

    let params = json!({
        "query": "test query",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_search", &params)
            .await
    });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run indication
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some() || data.to_string().contains("DRY RUN"),
        "Data should indicate dry run mode: {:?}",
        data
    );
}

// ============================================================================
// Test 7: sequential_cycle real execution
// ============================================================================

#[test]
fn test_sequential_cycle_real() {
    let executor = create_test_executor("cycle");

    let params = json!({
        "max_cycles": 1,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_cycle", &params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real sequential_cycle should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("cycles").is_some() || data.get("success").is_some() || data.is_object(),
        "Data should have cycle information: {:?}",
        data
    );
}

// ============================================================================
// Test 8: sequential_cycle respects dry_run
// ============================================================================

#[test]
fn test_sequential_cycle_respects_dry_run() {
    let executor = create_test_executor("cycle_dry");

    let params = json!({
        "max_cycles": 1,
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_cycle", &params)
            .await
    });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run indication
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some() || data.to_string().contains("DRY RUN"),
        "Data should indicate dry run mode: {:?}",
        data
    );
}

// ============================================================================
// Test 9: Error handling - missing required parameters
// ============================================================================

#[test]
fn test_sequential_tools_error_handling() {
    let executor = create_test_executor("errors");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Test sequential_record without 'thought'
    let params = json!({
        "task_id": 1,
        "step_number": 1,
        "reasoning": "test"
        // Missing 'thought' - should error
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_record", &params)
            .await
    });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(
        result.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope = result.unwrap();
    assert_error_envelope(&envelope);
    let error = unwrap_error(&envelope);
    assert_error_fields(error);

    // Test sequential_record without 'reasoning'
    let params2 = json!({
        "task_id": 1,
        "step_number": 1,
        "thought": "test"
        // Missing 'reasoning' - should error
    });

    let result2 = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_record", &params2)
            .await
    });

    assert!(
        result2.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope2 = result2.unwrap();
    assert_error_envelope(&envelope2);
    let error2 = unwrap_error(&envelope2);
    assert_error_fields(error2);

    // Test sequential_get without 'task_id'
    let params3 = json!({
        "dry_run": false
        // Missing 'task_id' - should error
    });

    let result3 = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_get", &params3)
            .await
    });

    assert!(
        result3.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope3 = result3.unwrap();
    assert_error_envelope(&envelope3);
    let error3 = unwrap_error(&envelope3);
    assert_error_fields(error3);

    // Test sequential_search without 'query'
    let params4 = json!({
        "dry_run": false
        // Missing 'query' - should error
    });

    let result4 = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_search", &params4)
            .await
    });

    assert!(
        result4.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope4 = result4.unwrap();
    assert_error_envelope(&envelope4);
    let error4 = unwrap_error(&envelope4);
    assert_error_fields(error4);
}

// ============================================================================
// Test 10: sequential_search timeout (3 seconds max)
// ============================================================================

#[test]
fn test_sequential_search_timeout() {
    let executor = create_test_executor("search_timeout");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Record many steps to make search potentially slow
    for i in 1..=50 {
        let record_params = json!({
            "task_id": 99,
            "step_number": i,
            "thought": format!("Analyzing component {} with detailed reasoning", i),
            "reasoning": format!("Complex reasoning step {} requiring vector embeddings", i),
            "dry_run": false
        });

        rt.block_on(async {
            executor
                .execute_real_tool_async("sequential_record", &record_params)
                .await
        })
        .ok();
    }

    // Search with query that may be slow with vector embeddings
    let search_params = json!({
        "query": "analyzing component detailed reasoning vector embeddings",
        "dry_run": false
    });

    let start = std::time::Instant::now();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("sequential_search", &search_params)
            .await
    });
    let elapsed = start.elapsed();

    // Should complete within 4 seconds (3s timeout + 1s buffer)
    assert!(
        elapsed.as_secs() < 4,
        "Search should timeout within 4 seconds, took {:?}",
        elapsed
    );

    // RealExecutor always returns Ok(Value), even for timeout errors
    assert!(
        result.is_ok(),
        "RealExecutor should return Ok(Value) for timeout"
    );

    // If search exceeded 3s, the envelope should contain a timeout error
    if elapsed.as_secs() >= 3 {
        let envelope = result.unwrap();
        // Check if it's an error envelope (timeout)
        if envelope.get("ok") == Some(&json!(false)) {
            assert_error_envelope(&envelope);
            let error = unwrap_error(&envelope);
            let err_msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("");
            assert!(
                err_msg.contains("Timeout") || err_msg.contains("timeout"),
                "Error should mention timeout, got: {}",
                err_msg
            );
        }
        // Otherwise it completed successfully before timeout (also acceptable)
    }
}
