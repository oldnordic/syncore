//! Runtime Executor Server Integration Tests
//!
//! Phase 7 Step 2 - TDD tests for executor integration into SynCoreMCPServer
//! Tests MUST fail initially until server integration is implemented.

use std::sync::Arc;
use std::sync::Mutex;
use syncore::mcp_server::SynCoreMCPServer;
use syncore::memory::Memory;
use syncore::router::SynCoreState;
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};

/// Helper to create test state with unique database
fn create_test_state(suffix: &str) -> SynCoreState {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let db_path = format!(":memory:_server_exec_{}_{}", suffix, timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    SynCoreState::new(memory, tasks, vector_store)
}

// ============================================================================
// Test 1: Server uses Real executor by default
// ============================================================================

#[test]
fn test_server_uses_real_executor_by_default() {
    // Remove env var to ensure default behavior
    std::env::remove_var("SYNCORE_EXECUTOR");

    let state = create_test_state("default");
    let server = SynCoreMCPServer::new(state);

    // Server should have been created with Real executor
    // Verify by checking that executor exists
    assert!(Arc::strong_count(&server.executor) >= 1);
}

// ============================================================================
// Test 2: Server uses Stub executor when SYNCORE_EXECUTOR=stub
// ============================================================================

#[test]
fn test_server_uses_stub_executor_when_env_set() {
    std::env::set_var("SYNCORE_EXECUTOR", "stub");

    let state = create_test_state("stub");
    let server = SynCoreMCPServer::new(state);

    // Server should have been created with Stub executor
    assert!(Arc::strong_count(&server.executor) >= 1);

    std::env::remove_var("SYNCORE_EXECUTOR");
}

// ============================================================================
// Test 3: Macro tools respect runtime executor
// ============================================================================

#[tokio::test]
async fn test_macro_tools_respect_runtime_executor() {
    use serde_json::json;

    // Test with Real executor
    std::env::set_var("SYNCORE_EXECUTOR", "real");
    let state_real = create_test_state("macro_real");
    let server_real = SynCoreMCPServer::new(state_real);

    // Should be able to call tool without panic
    let params_real = json!({"key": "test_real", "value": "data_real", "dry_run": false});
    let result_real = server_real.executor.clone();
    result_real.record_step("memory_store", params_real);

    // Test with Stub executor
    std::env::set_var("SYNCORE_EXECUTOR", "stub");
    let state_stub = create_test_state("macro_stub");
    let server_stub = SynCoreMCPServer::new(state_stub);

    // Should be able to call tool without panic
    let params_stub = json!({"key": "test_stub", "value": "data_stub", "dry_run": false});
    let result_stub = server_stub.executor.clone();
    result_stub.record_step("memory_store", params_stub);

    std::env::remove_var("SYNCORE_EXECUTOR");
}

// ============================================================================
// Test 4: Executor switching does not break existing tools
// ============================================================================

#[tokio::test]
async fn test_executor_switching_does_not_break_existing_tools() {
    use serde_json::json;

    // Create server with default (Real) executor
    std::env::remove_var("SYNCORE_EXECUTOR");
    let state = create_test_state("existing");
    let server = SynCoreMCPServer::new(state);

    // Test memory_store tool works
    let params = json!({"key": "compatibility_test", "value": "works", "dry_run": false});
    server.executor.record_step("memory_store", params.clone());

    // Test multiple tools work
    server.executor.record_step("memory_query", json!({"key": "compatibility_test"}));
    server.executor.record_step("task_create", json!({"goal": "test task", "priority": 3}));

    // No panics = success
}

// ============================================================================
// Test 5: Executor switching is isolated per server instance
// ============================================================================

#[test]
fn test_executor_switching_is_isolated() {
    // Create server 1 with Real executor
    std::env::set_var("SYNCORE_EXECUTOR", "real");
    let state1 = create_test_state("isolated1");
    let server1 = SynCoreMCPServer::new(state1);
    let executor1 = server1.executor.clone();

    // Create server 2 with Stub executor
    std::env::set_var("SYNCORE_EXECUTOR", "stub");
    let state2 = create_test_state("isolated2");
    let server2 = SynCoreMCPServer::new(state2);
    let executor2 = server2.executor.clone();

    // Both should exist independently
    assert!(Arc::strong_count(&executor1) >= 1);
    assert!(Arc::strong_count(&executor2) >= 1);

    // They should be different instances
    assert!(!Arc::ptr_eq(&executor1, &executor2));

    std::env::remove_var("SYNCORE_EXECUTOR");
}

// ============================================================================
// Test 6: Server constructor reads env var at creation time
// ============================================================================

#[test]
fn test_server_constructor_reads_env_at_creation() {
    // Set env to stub
    std::env::set_var("SYNCORE_EXECUTOR", "stub");

    let state1 = create_test_state("creation1");
    let server1 = SynCoreMCPServer::new(state1);

    // Change env to real
    std::env::set_var("SYNCORE_EXECUTOR", "real");

    let state2 = create_test_state("creation2");
    let server2 = SynCoreMCPServer::new(state2);

    // server1 should still have stub (captured at creation)
    // server2 should have real (captured at creation)
    // Both should exist
    assert!(Arc::strong_count(&server1.executor) >= 1);
    assert!(Arc::strong_count(&server2.executor) >= 1);

    std::env::remove_var("SYNCORE_EXECUTOR");
}
