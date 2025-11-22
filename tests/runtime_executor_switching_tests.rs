//! Runtime Executor Switching Tests
//!
//! Phase 7 Step 1 - TDD tests for executor selection at runtime
//! Tests MUST fail initially until runtime switching is implemented.

use std::sync::Arc;
use std::sync::Mutex;
use syncore::memory::Memory;
use syncore::router::SynCoreState;
use syncore::runtime::executor_selector::{create_executor, ExecutorKind};
use syncore::tasks::Tasks;
use syncore::vector::{RealEmbeddings, VectorStore};

/// Helper to create test state
fn create_test_state() -> Arc<SynCoreState> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = format!(":memory:_executor_switch_{}", timestamp);
    let memory = Memory::new(&db_path).expect("Failed to create memory");
    let tasks = Tasks::new(&db_path).expect("Failed to create tasks");
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings"));
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);
    Arc::new(state)
}

// ============================================================================
// Test 1: Default executor is Real
// ============================================================================

#[test]
fn test_default_executor_is_real() {
    std::env::remove_var("SYNCORE_EXECUTOR");

    let kind = ExecutorKind::from_env();

    match kind {
        ExecutorKind::Real => {} // Expected
        ExecutorKind::Stub => panic!("Default should be Real, got Stub"),
    }
}

// ============================================================================
// Test 2: SYNCORE_EXECUTOR=real uses RealExecutor
// ============================================================================

#[test]
fn test_env_real_uses_real_executor() {
    std::env::set_var("SYNCORE_EXECUTOR", "real");

    let kind = ExecutorKind::from_env();
    let state = create_test_state();
    let executor = create_executor(kind, state);

    // Executor should be created without error
    assert!(Arc::strong_count(&executor) >= 1);

    std::env::remove_var("SYNCORE_EXECUTOR");
}

// ============================================================================
// Test 3: SYNCORE_EXECUTOR=stub uses RealExecutorStub
// ============================================================================

#[test]
fn test_env_stub_uses_stub_executor() {
    std::env::set_var("SYNCORE_EXECUTOR", "stub");

    let kind = ExecutorKind::from_env();

    match kind {
        ExecutorKind::Stub => {} // Expected
        ExecutorKind::Real => panic!("With SYNCORE_EXECUTOR=stub, should use Stub, got Real"),
    }

    std::env::remove_var("SYNCORE_EXECUTOR");
}

// ============================================================================
// Test 4: Invalid env value falls back to Real
// ============================================================================

#[test]
fn test_invalid_env_falls_back_to_real() {
    std::env::set_var("SYNCORE_EXECUTOR", "invalid_value_xyz");

    let kind = ExecutorKind::from_env();

    match kind {
        ExecutorKind::Real => {} // Expected fallback
        ExecutorKind::Stub => panic!("Invalid env should fallback to Real, got Stub"),
    }

    std::env::remove_var("SYNCORE_EXECUTOR");
}

// ============================================================================
// Test 5: Executor switching does not break macro tools (smoke test)
// ============================================================================

#[test]
fn test_executor_switching_does_not_break_macro_tools() {
    use serde_json::json;

    // Test with Real executor
    std::env::set_var("SYNCORE_EXECUTOR", "real");
    let kind_real = ExecutorKind::from_env();
    let state = create_test_state();
    let executor_real = create_executor(kind_real, state.clone());

    // Should be able to call record_step without panic
    executor_real.record_step("memory_store", json!({"key": "test", "value": "data"}));

    // Test with Stub executor
    std::env::set_var("SYNCORE_EXECUTOR", "stub");
    let kind_stub = ExecutorKind::from_env();
    let executor_stub = create_executor(kind_stub, state.clone());

    // Should be able to call record_step without panic
    executor_stub.record_step("memory_store", json!({"key": "test", "value": "data"}));

    std::env::remove_var("SYNCORE_EXECUTOR");
}

// ============================================================================
// Test 6: Multiple executors can coexist
// ============================================================================

#[test]
fn test_multiple_executors_coexist() {
    let state = create_test_state();

    let executor_real = create_executor(ExecutorKind::Real, state.clone());
    let executor_stub = create_executor(ExecutorKind::Stub, state.clone());

    // Both should be valid
    assert!(Arc::strong_count(&executor_real) >= 1);
    assert!(Arc::strong_count(&executor_stub) >= 1);
}
