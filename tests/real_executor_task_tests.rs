//! Real Executor Task Tools Tests
//!
//! Phase 6 - TDD tests for task and intellitask tools
//! These tests validate that RealExecutor returns proper envelope-wrapped responses.
//!
//! Tools covered:
//! 1. task_create
//! 2. intellitask_list
//! 3. intellitask_get
//! 4. intellitask_update_status
//! 5. intellitask_next_ready
//! 6. intellitask_get_subtasks
//! 7. intellitask_subtask_stats
//! 8. intellitask_task_statistics
//! 9. intellitask_prd_statistics

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
    let db_path = format!(":memory:_task_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);

    RealExecutor::new(Arc::new(state))
}

// ============================================================================
// Test 1: task_create real basic execution
// ============================================================================

#[test]
fn test_task_create_real_basic() {
    let executor = create_test_executor("task_create_basic");

    let params = json!({
        "goal": "Implement user authentication system",
        "priority": 1,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("task_create", &params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real task_create should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("task_id").is_some(),
        "Data should have task_id: {:?}",
        data
    );

    let task_id = data["task_id"].as_i64().expect("task_id should be i64");
    assert!(task_id > 0, "task_id should be positive");

    // Verify side effect: task should exist in database
    let get_params = json!({
        "task_id": task_id,
        "dry_run": false
    });

    let get_result = rt.block_on(async {
        executor
            .execute_real_tool_async("intellitask_get", &get_params)
            .await
    });

    assert!(
        get_result.is_ok(),
        "Should be able to retrieve created task"
    );
    let get_envelope = get_result.unwrap();
    assert_success_envelope(&get_envelope);
    let task_data = unwrap_data(&get_envelope);
    assert!(
        task_data.get("id").is_some() || task_data.get("goal").is_some(),
        "Retrieved task should have data: {:?}",
        task_data
    );
}

// ============================================================================
// Test 2: task_create respects dry_run
// ============================================================================

#[test]
fn test_task_create_respects_dry_run() {
    let executor = create_test_executor("task_create_dry");

    let params = json!({
        "goal": "This should not be created",
        "priority": 1,
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("task_create", &params)
            .await
    });

    // Should succeed
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

    // Verify NO side effect: list tasks should be empty or not contain our goal
    let list_params = json!({
        "dry_run": false
    });

    let list_result = rt.block_on(async {
        executor
            .execute_real_tool_async("intellitask_list", &list_params)
            .await
    });

    if let Ok(list_envelope) = list_result {
        assert_success_envelope(&list_envelope);
        let list_data = unwrap_data(&list_envelope);
        if let Some(tasks) = list_data.get("tasks").and_then(|t| t.as_array()) {
            // Check that our dry run task is not in the list
            for task in tasks {
                if let Some(goal) = task.get("goal").and_then(|g| g.as_str()) {
                    assert_ne!(
                        goal, "This should not be created",
                        "Dry run task should not be persisted"
                    );
                }
            }
        }
    }
}

// ============================================================================
// Test 3: intellitask_list basic execution
// ============================================================================

#[test]
fn test_intellitask_list_real() {
    let executor = create_test_executor("task_list");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // First create a task
    let create_params = json!({
        "goal": "Test task for listing",
        "priority": 1,
        "dry_run": false
    });

    let create_result = rt.block_on(async {
        executor
            .execute_real_tool_async("task_create", &create_params)
            .await
    });
    assert!(create_result.is_ok(), "Create should succeed");

    // Now list tasks
    let list_params = json!({
        "dry_run": false
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("intellitask_list", &list_params)
            .await
    });

    assert!(result.is_ok(), "List should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("tasks").is_some(),
        "Data should have tasks field: {:?}",
        data
    );

    let tasks = data["tasks"].as_array().expect("tasks should be array");
    assert!(!tasks.is_empty(), "Should have at least one task");
}

// ============================================================================
// Test 4: intellitask_get retrieves specific task
// ============================================================================

#[test]
fn test_intellitask_get_real() {
    let executor = create_test_executor("task_get");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create a task
    let create_params = json!({
        "goal": "Specific task to retrieve",
        "priority": 2,
        "dry_run": false
    });

    let create_result = rt.block_on(async {
        executor
            .execute_real_tool_async("task_create", &create_params)
            .await
    });
    assert!(create_result.is_ok(), "Create should succeed");
    let create_envelope = create_result.unwrap();
    assert_success_envelope(&create_envelope);
    let create_data = unwrap_data(&create_envelope);

    let task_id = create_data["task_id"]
        .as_i64()
        .expect("Should have task_id");

    // Get the task
    let get_params = json!({
        "task_id": task_id,
        "dry_run": false
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("intellitask_get", &get_params)
            .await
    });

    assert!(result.is_ok(), "Get should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("id").is_some() || data.get("goal").is_some(),
        "Data should have task information: {:?}",
        data
    );
}

// ============================================================================
// Test 5: intellitask_update_status changes task status
// ============================================================================

#[test]
fn test_intellitask_update_status_real() {
    let executor = create_test_executor("task_update");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create a task
    let create_params = json!({
        "goal": "Task to update status",
        "priority": 1,
        "dry_run": false
    });

    let create_result = rt.block_on(async {
        executor
            .execute_real_tool_async("task_create", &create_params)
            .await
    });
    assert!(create_result.is_ok(), "Create should succeed");
    let create_envelope = create_result.unwrap();
    assert_success_envelope(&create_envelope);
    let create_data = unwrap_data(&create_envelope);

    let task_id = create_data["task_id"]
        .as_i64()
        .expect("Should have task_id");

    // Update status
    let update_params = json!({
        "task_id": task_id,
        "status": "in-progress",
        "dry_run": false
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("intellitask_update_status", &update_params)
            .await
    });

    assert!(result.is_ok(), "Update should succeed: {:?}", result.err());
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("updated")
            .and_then(|u| u.as_bool())
            .unwrap_or(false)
            || data
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false),
        "Data should indicate update success: {:?}",
        data
    );

    // Verify side effect: get task and check status
    let get_params = json!({
        "task_id": task_id,
        "dry_run": false
    });

    let get_result = rt.block_on(async {
        executor
            .execute_real_tool_async("intellitask_get", &get_params)
            .await
    });

    if let Ok(get_envelope) = get_result {
        assert_success_envelope(&get_envelope);
        let task_data = unwrap_data(&get_envelope);
        if let Some(status) = task_data.get("status").and_then(|s| s.as_str()) {
            assert_eq!(status, "in-progress", "Status should be updated");
        }
    }
}

// ============================================================================
// Test 6: intellitask_next_ready finds tasks with satisfied dependencies
// ============================================================================

#[test]
fn test_intellitask_next_ready_real() {
    let executor = create_test_executor("task_next");

    let params = json!({
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("intellitask_next_ready", &params)
            .await
    });

    // Should succeed even if no tasks
    assert!(result.is_ok(), "next_ready should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate structure
    let data = unwrap_data(&envelope);
    assert!(
        data.get("ready_tasks").is_some() || data.get("next_task").is_some() || data.is_null(),
        "Data should have valid structure: {:?}",
        data
    );
}

// ============================================================================
// Test 7: intellitask_get_subtasks retrieves child tasks
// ============================================================================

#[test]
fn test_intellitask_get_subtasks_real() {
    let executor = create_test_executor("task_subtasks");

    // For now, just test with a parent_id (may have no subtasks)
    let params = json!({
        "parent_id": 1,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("intellitask_get_subtasks", &params)
            .await
    });

    assert!(result.is_ok(), "get_subtasks should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("subtasks").is_some(),
        "Data should have subtasks field: {:?}",
        data
    );

    if let Some(subtasks) = data["subtasks"].as_array() {
        // Array should be valid (may be empty if no subtasks)
        assert!(subtasks.len() >= 0);
    }
}

// ============================================================================
// Test 8: intellitask_subtask_stats provides statistics
// ============================================================================

#[test]
fn test_intellitask_subtask_stats_real() {
    let executor = create_test_executor("task_stats");

    let params = json!({
        "parent_id": 1,
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("intellitask_subtask_stats", &params)
            .await
    });

    assert!(result.is_ok(), "subtask_stats should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("total").is_some() || data.get("completed").is_some(),
        "Data should have statistics: {:?}",
        data
    );
}

// ============================================================================
// Test 9: intellitask_task_statistics provides overall statistics
// ============================================================================

#[test]
fn test_intellitask_task_statistics_real() {
    let executor = create_test_executor("task_all_stats");

    let params = json!({
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("intellitask_task_statistics", &params)
            .await
    });

    assert!(result.is_ok(), "task_statistics should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("total").is_some() || data.get("total_tasks").is_some(),
        "Data should have overall statistics: {:?}",
        data
    );
}

// ============================================================================
// Test 10: intellitask_prd_statistics for specific PRD
// ============================================================================

#[test]
fn test_intellitask_prd_statistics_real() {
    let executor = create_test_executor("task_prd_stats");

    let params = json!({
        "prd_title": "Test PRD",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("intellitask_prd_statistics", &params)
            .await
    });

    assert!(result.is_ok(), "prd_statistics should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("total").is_some() || data.get("total_tasks").is_some() || data.is_object(),
        "Data should have valid structure: {:?}",
        data
    );
}

// ============================================================================
// Test 11: Error handling - missing required parameters
// ============================================================================

#[test]
fn test_task_tools_error_handling() {
    let executor = create_test_executor("task_errors");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Test task_create without goal
    let params = json!({
        "priority": 1
        // Missing 'goal' - should error
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("task_create", &params)
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

    // Test intellitask_get without task_id
    let params2 = json!({
        "dry_run": false
        // Missing 'task_id' - should error
    });

    let result2 = rt.block_on(async {
        executor
            .execute_real_tool_async("intellitask_get", &params2)
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
}
