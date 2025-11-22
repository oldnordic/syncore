//! Real Executor Agent Tools Tests
//!
//! Phase 6.7 - TDD tests for agent communication and registration tools
//! These tests validate that RealExecutor returns proper envelope-wrapped responses.
//!
//! Tools covered:
//! 1. agent_send
//! 2. agent_recv
//! 3. agent_register
//! 4. agent_list
//! 5. agent_status
//! 6. agent_task
//! 7. agent_result

mod real_executor_test_helpers;
use real_executor_test_helpers::{
    assert_error_envelope, assert_error_fields, assert_success_envelope, unwrap_data, unwrap_error,
};

use serde_json::json;
use std::sync::{Arc, Mutex};
use syncore::macro_tools::executor_real::RealExecutor;
use syncore::memory::Memory;
use syncore::message_bus::MessageBus;
use syncore::router::SynCoreState;
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};

/// Helper to create a RealExecutor with fresh state (message_bus = None by default)
fn create_test_executor_without_bus(suffix: &str) -> RealExecutor {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = format!(":memory:_agent_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);
    // Note: state.message_bus will be None

    RealExecutor::new(Arc::new(state))
}

/// Helper to create a RealExecutor with MessageBus enabled
fn create_test_executor_with_bus(suffix: &str) -> RealExecutor {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = format!(":memory:_agent_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let bus = MessageBus::new();
    let mut state = SynCoreState::new(memory, tasks, vector_store);
    state.message_bus = Some(Arc::new(bus));

    RealExecutor::new(Arc::new(state))
}

// ============================================================================
// Test 1: agent_send when message_bus unavailable
// ============================================================================

#[test]
fn test_agent_send_real_when_bus_unavailable() {
    let executor = create_test_executor_without_bus("send_unavail");

    let params = json!({
        "to": "test_agent",
        "message": "Hello",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_send", &params)
            .await
    });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(
        result.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope = result.unwrap();

    // Validate error envelope structure
    assert_error_envelope(&envelope);

    // Validate error details
    let error = unwrap_error(&envelope);
    assert_error_fields(error);

    // Check error message indicates unavailability
    let err_msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("");
    assert!(
        err_msg.contains("Agent")
            || err_msg.contains("bus")
            || err_msg.contains("unavailable")
            || err_msg.contains("not configured"),
        "Error should indicate agent system unavailable: {:?}",
        err_msg
    );
}

// ============================================================================
// Test 2: agent_send respects dry_run
// ============================================================================

#[test]
fn test_agent_send_respects_dry_run() {
    let executor = create_test_executor_with_bus("send_dry");

    let params = json!({
        "to": "test_agent",
        "message": "Test message",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_send", &params)
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
        data.get("dry_run").is_some()
            || data
                .get("message")
                .and_then(|m| m.as_str())
                .map(|s| s.contains("DRY RUN"))
                .unwrap_or(false)
            || data.get("sent").is_some(),
        "Dry run should return valid response: {:?}",
        data
    );
}

// ============================================================================
// Test 3: agent_send real message sent
// ============================================================================

#[test]
fn test_agent_send_real_message_sent() {
    let executor = create_test_executor_with_bus("send_real");

    let params = json!({
        "to": "target_agent",
        "message": "Real message",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_send", &params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real send should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("sent").and_then(|s| s.as_bool()).unwrap_or(false)
            || data
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false),
        "Result should indicate message sent: {:?}",
        data
    );
}

// ============================================================================
// Test 4: agent_recv when message_bus unavailable
// ============================================================================

#[test]
fn test_agent_recv_real_when_bus_unavailable() {
    let executor = create_test_executor_without_bus("recv_unavail");

    let params = json!({
        "agent": "test_agent",
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_recv", &params)
            .await
    });

    // RealExecutor returns Ok(Value) with error envelope, NOT Err
    assert!(
        result.is_ok(),
        "RealExecutor should return Ok(Value) even for errors"
    );
    let envelope = result.unwrap();

    // Validate error envelope structure
    assert_error_envelope(&envelope);

    // Validate error details
    let error = unwrap_error(&envelope);
    assert_error_fields(error);

    // Check error message indicates unavailability
    let err_msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("");
    assert!(
        err_msg.contains("Agent")
            || err_msg.contains("bus")
            || err_msg.contains("unavailable")
            || err_msg.contains("not configured"),
        "Error should indicate agent system unavailable: {:?}",
        err_msg
    );
}

// ============================================================================
// Test 5: agent_recv respects dry_run
// ============================================================================

#[test]
fn test_agent_recv_respects_dry_run() {
    let executor = create_test_executor_with_bus("recv_dry");

    let params = json!({
        "agent": "test_agent",
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_recv", &params)
            .await
    });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run response
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some()
            || data
                .get("message")
                .and_then(|m| m.as_str())
                .map(|s| s.contains("DRY RUN"))
                .unwrap_or(false)
            || data.get("messages").is_some(),
        "Dry run should return valid response: {:?}",
        data
    );
}

// ============================================================================
// Test 6: agent_register real
// ============================================================================

#[test]
fn test_agent_register_real() {
    let executor = create_test_executor_with_bus("register");

    let params = json!({
        "id": "test_agent_123",
        "capabilities": ["task_execution", "code_analysis"],
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_register", &params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real register should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("registered")
            .and_then(|r| r.as_bool())
            .unwrap_or(false)
            || data
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false),
        "Result should indicate registration success: {:?}",
        data
    );
}

// ============================================================================
// Test 7: agent_register respects dry_run
// ============================================================================

#[test]
fn test_agent_register_respects_dry_run() {
    let executor = create_test_executor_with_bus("register_dry");

    let params = json!({
        "id": "test_agent_dry",
        "capabilities": ["testing"],
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_register", &params)
            .await
    });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run response
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some()
            || data
                .get("message")
                .and_then(|m| m.as_str())
                .map(|s| s.contains("DRY RUN"))
                .unwrap_or(false),
        "Dry run should return valid response: {:?}",
        data
    );
}

// ============================================================================
// Test 8: agent_list real
// ============================================================================

#[test]
fn test_agent_list_real() {
    let executor = create_test_executor_with_bus("list");

    let params = json!({
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_list", &params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real list should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("agents").is_some(),
        "Result should have agents field: {:?}",
        data
    );
}

// ============================================================================
// Test 9: agent_status real
// ============================================================================

#[test]
fn test_agent_status_real() {
    let executor = create_test_executor_with_bus("status");

    let params = json!({
        "id": "test_agent",
        "status": {"state": "idle", "load": 0.2},
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_status", &params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real status update should succeed: {:?}",
        result.err()
    );
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
        "Result should indicate status updated: {:?}",
        data
    );
}

// ============================================================================
// Test 10: agent_status respects dry_run
// ============================================================================

#[test]
fn test_agent_status_respects_dry_run() {
    let executor = create_test_executor_with_bus("status_dry");

    let params = json!({
        "id": "test_agent",
        "status": {"state": "busy"},
        "dry_run": true
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_status", &params)
            .await
    });

    // Should succeed with synthetic response
    assert!(result.is_ok(), "Dry run should succeed");
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate dry run response
    let data = unwrap_data(&envelope);
    assert!(
        data.get("dry_run").is_some()
            || data
                .get("message")
                .and_then(|m| m.as_str())
                .map(|s| s.contains("DRY RUN"))
                .unwrap_or(false),
        "Dry run should return valid response: {:?}",
        data
    );
}

// ============================================================================
// Test 11: agent_task real
// ============================================================================

#[test]
fn test_agent_task_real() {
    let executor = create_test_executor_with_bus("task");

    let params = json!({
        "to": "worker_agent",
        "task_id": "task_001",
        "task_type": "code_analysis",
        "payload": {"file": "src/main.rs", "depth": 3},
        "dry_run": false
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_task", &params)
            .await
    });

    // Should succeed
    assert!(
        result.is_ok(),
        "Real task send should succeed: {:?}",
        result.err()
    );
    let envelope = result.unwrap();

    // Validate envelope structure
    assert_success_envelope(&envelope);

    // Unwrap data and validate contents
    let data = unwrap_data(&envelope);
    assert!(
        data.get("sent").and_then(|s| s.as_bool()).unwrap_or(false)
            || data
                .get("success")
                .and_then(|s| s.as_bool())
                .unwrap_or(false),
        "Result should indicate task sent: {:?}",
        data
    );
}

// ============================================================================
// Test 12: Error handling - missing required parameters
// ============================================================================

#[test]
fn test_agent_tools_error_handling() {
    let executor = create_test_executor_with_bus("errors");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Test agent_send without 'to'
    let params = json!({
        "message": "Hello"
        // Missing 'to' - should error
    });

    let result = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_send", &params)
            .await
    });

    assert!(
        result.is_ok(),
        "RealExecutor returns Ok(Value) even for errors"
    );
    let envelope = result.unwrap();
    assert_error_envelope(&envelope);

    // Test agent_recv without 'agent'
    let params2 = json!({
        "dry_run": false
        // Missing 'agent' - should error
    });

    let result2 = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_recv", &params2)
            .await
    });

    assert!(
        result2.is_ok(),
        "RealExecutor returns Ok(Value) even for errors"
    );
    let envelope2 = result2.unwrap();
    assert_error_envelope(&envelope2);

    // Test agent_register without 'id'
    let params3 = json!({
        "capabilities": ["test"]
        // Missing 'id' - should error
    });

    let result3 = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_register", &params3)
            .await
    });

    assert!(
        result3.is_ok(),
        "RealExecutor returns Ok(Value) even for errors"
    );
    let envelope3 = result3.unwrap();
    assert_error_envelope(&envelope3);

    // Test agent_status without 'id'
    let params4 = json!({
        "status": {"state": "idle"}
        // Missing 'id' - should error
    });

    let result4 = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_status", &params4)
            .await
    });

    assert!(
        result4.is_ok(),
        "RealExecutor returns Ok(Value) even for errors"
    );
    let envelope4 = result4.unwrap();
    assert_error_envelope(&envelope4);

    // Test agent_task without 'to'
    let params5 = json!({
        "task_id": "123",
        "task_type": "test",
        "payload": {}
        // Missing 'to' - should error
    });

    let result5 = rt.block_on(async {
        executor
            .execute_real_tool_async("agent_task", &params5)
            .await
    });

    assert!(
        result5.is_ok(),
        "RealExecutor returns Ok(Value) even for errors"
    );
    let envelope5 = result5.unwrap();
    assert_error_envelope(&envelope5);
}
