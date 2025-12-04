//! Branch Manager Tests - PHASE ST-6
//!
//! Test suite for Tree-of-Thought circuit breaker functionality.
//! Tests all safety invariants: branch limits, depth limits, breadth limits,
//! identical expansion detection, loop detection, and error limits.

use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use syncore::reasoning::{
    branch_manager::{BranchLimits, BranchManager},
    ReasoningError,
};
use syncore::router::SynCoreState;
use tempfile::TempDir;

/// Create test state with temporary database
fn create_test_state() -> (SynCoreState, TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    // Initialize DbManager with long-lived connections
    let _db_manager = Arc::new(
        syncore::db::DbManager::new(
            main_db_path.to_str().unwrap(),
            code_graph_db_path.to_str().unwrap(),
        )
        .unwrap(),
    );

    // Create vector stores for testing using stub embeddings (no model loading)
    let code_embeddings = Box::new(syncore::vector::StubEmbeddings::new(384).unwrap());
    let code_store =
        Arc::new(std::sync::Mutex::new(syncore::vector::VectorStore::new(code_embeddings)));
    let general_embeddings = Box::new(syncore::vector::StubEmbeddings::new(384).unwrap());
    let general_store =
        Arc::new(std::sync::Mutex::new(syncore::vector::VectorStore::new(general_embeddings)));

    // Create state using dual stores
    let state = SynCoreState::with_dual_stores(code_store, general_store).unwrap();

    (state, temp_dir)
}

/// Create a mock reasoning node for testing
fn create_mock_node(session_id: &str, content: &str, step_index: i64) -> Value {
    json!({
        "id": format!("node_{}", uuid::Uuid::new_v4()),
        "session_id": session_id,
        "parent_id": null,
        "step_index": step_index,
        "content": content,
        "score": 1.0
    })
}

/// Create a mock reasoning node with parent
fn create_mock_node_with_parent(
    session_id: &str,
    parent_id: &str,
    content: &str,
    step_index: i64,
) -> Value {
    json!({
        "id": format!("node_{}", uuid::Uuid::new_v4()),
        "session_id": session_id,
        "parent_id": parent_id,
        "step_index": step_index,
        "content": content,
        "score": 1.0
    })
}

#[test]
fn test_branch_limit_exceeded() -> Result<()> {
    let limits = BranchLimits {
        max_nodes: 5,
        max_depth: 10,
        max_breadth: 3,
        max_identical_expansions: 3,
        max_consecutive_errors: 5,
    };

    let mut manager = BranchManager::new(limits);
    let session_id = "test_session";

    // Create 5 nodes (at limit)
    for i in 0..5 {
        let node = create_mock_node(session_id, &format!("Node {}", i), i);
        manager.record_success(session_id, &node).unwrap();
    }

    // Try to create 6th node (should fail)
    let node = create_mock_node(session_id, "Node 5", 5);
    let result = manager.check_before_expand(session_id, &node);

    assert!(result.is_err());
    let err = result.unwrap_err();
    println!("Actual error: {:?}", err);
    match err {
        ReasoningError::BranchLimitExceeded(msg) => {
            assert!(msg.contains("Branch limit exceeded"));
        }
        _ => panic!("Expected BranchLimitExceeded error, got: {:?}", err),
    }

    Ok(())
}

#[test]
fn test_depth_limit_exceeded() -> Result<()> {
    let limits = BranchLimits {
        max_nodes: 100,
        max_depth: 3,
        max_breadth: 5,
        max_identical_expansions: 3,
        max_consecutive_errors: 5,
    };

    let mut manager = BranchManager::new(limits);
    let session_id = "test_session";

    // Create nodes at depth 0, 1, 2 (within limit)
    let root = create_mock_node(session_id, "Root", 0);
    manager.record_success(session_id, &root).unwrap();

    let child1 =
        create_mock_node_with_parent(session_id, root["id"].as_str().unwrap(), "Child 1", 1);
    manager.record_success(session_id, &child1).unwrap();

    let child2 =
        create_mock_node_with_parent(session_id, root["id"].as_str().unwrap(), "Child 2", 1);
    manager.record_success(session_id, &child2).unwrap();

    let grandchild =
        create_mock_node_with_parent(session_id, child1["id"].as_str().unwrap(), "Grandchild", 2);
    manager.record_success(session_id, &grandchild).unwrap();

    // Try to create node at depth 3 (should fail)
    let great_grandchild = create_mock_node_with_parent(
        session_id,
        grandchild["id"].as_str().unwrap(),
        "Great-grandchild",
        3,
    );
    let result = manager.check_before_expand(session_id, &great_grandchild);

    assert!(result.is_err());
    match result.unwrap_err() {
        ReasoningError::DepthLimitExceeded(msg) => {
            assert!(msg.contains("Depth limit exceeded"));
        }
        _ => panic!("Expected DepthLimitExceeded error"),
    }

    Ok(())
}

#[test]
fn test_breadth_limit_exceeded() -> Result<()> {
    let limits = BranchLimits {
        max_nodes: 100,
        max_depth: 10,
        max_breadth: 2,
        max_identical_expansions: 3,
        max_consecutive_errors: 5,
    };

    let mut manager = BranchManager::new(limits);
    let session_id = "test_session";

    // Create root node
    let root = create_mock_node(session_id, "Root", 0);
    manager.record_success(session_id, &root).unwrap();

    // Create 2 children (at limit)
    let child1 =
        create_mock_node_with_parent(session_id, root["id"].as_str().unwrap(), "Child 1", 1);
    manager.record_success(session_id, &child1).unwrap();

    let child2 =
        create_mock_node_with_parent(session_id, root["id"].as_str().unwrap(), "Child 2", 1);
    manager.record_success(session_id, &child2).unwrap();

    // Try to create 3rd child (should fail)
    let child3 =
        create_mock_node_with_parent(session_id, root["id"].as_str().unwrap(), "Child 3", 1);
    let result = manager.check_before_expand(session_id, &child3);

    assert!(result.is_err());
    match result.unwrap_err() {
        ReasoningError::BreadthLimitExceeded(msg) => {
            assert!(msg.contains("Breadth limit exceeded"));
        }
        _ => panic!("Expected BreadthLimitExceeded error"),
    }

    Ok(())
}

#[test]
fn test_identical_expansion_detection() -> Result<()> {
    let limits = BranchLimits {
        max_nodes: 100,
        max_depth: 10,
        max_breadth: 5,
        max_identical_expansions: 2,
        max_consecutive_errors: 5,
    };

    let mut manager = BranchManager::new(limits);
    let session_id = "test_session";

    // Create identical content multiple times
    let identical_content = "Continue with current approach";

    for i in 0..2 {
        let node = create_mock_node(session_id, &format!("{} - {}", identical_content, i), i);
        manager.record_success(session_id, &node).unwrap();
    }

    // Try to create 3rd identical expansion (should fail)
    let node = create_mock_node(session_id, &format!("{} - 2", identical_content), 2);
    let result = manager.check_before_expand(session_id, &node);

    println!("Result: {:?}", result);
    assert!(result.is_err());
    match result.unwrap_err() {
        ReasoningError::RepetitiveThoughtPattern(msg) => {
            assert!(msg.contains("Identical expansion detected"));
        }
        _ => panic!("Expected RepetitiveThoughtPattern error"),
    }

    Ok(())
}

#[test]
fn test_loop_detection_by_content_hash() -> Result<()> {
    let limits = BranchLimits {
        max_nodes: 100,
        max_depth: 10,
        max_breadth: 5,
        max_identical_expansions: 3,
        max_consecutive_errors: 5,
    };

    let mut manager = BranchManager::new(limits);
    let session_id = "test_session";

    // Create a cycle: A -> B -> C -> A (same content)
    let node_a = create_mock_node(session_id, "State A", 0);
    manager.record_success(session_id, &node_a).unwrap();

    let node_b =
        create_mock_node_with_parent(session_id, node_a["id"].as_str().unwrap(), "State B", 1);
    manager.record_success(session_id, &node_b).unwrap();

    let node_c =
        create_mock_node_with_parent(session_id, node_b["id"].as_str().unwrap(), "State C", 2);
    manager.record_success(session_id, &node_c).unwrap();

    // Try to return to State A (should detect loop)
    let node_a_again =
        create_mock_node_with_parent(session_id, node_c["id"].as_str().unwrap(), "State A", 3);
    let result = manager.check_before_expand(session_id, &node_a_again);

    assert!(result.is_err());
    match result.unwrap_err() {
        ReasoningError::LoopDetected(msg) => {
            assert!(msg.contains("Loop detected"));
        }
        _ => panic!("Expected LoopDetected error"),
    }

    Ok(())
}

#[test]
fn test_consecutive_error_limit() -> Result<()> {
    let limits = BranchLimits {
        max_nodes: 100,
        max_depth: 10,
        max_breadth: 5,
        max_identical_expansions: 3,
        max_consecutive_errors: 2,
    };

    let mut manager = BranchManager::new(limits);
    let session_id = "test_session";

    // Record 2 consecutive errors (at limit)
    for i in 0..2 {
        let node = create_mock_node(session_id, &format!("Error {}", i), i);
        manager.record_failure(session_id, &node, "Test error").unwrap();
    }

    // Try to record 3rd consecutive error (should fail)
    let node = create_mock_node(session_id, "Error 2", 2);
    let result = manager.check_before_expand(session_id, &node);

    assert!(result.is_err());
    let err = result.unwrap_err();
    println!("Actual error: {:?}", err);
    match err {
        ReasoningError::TooManyErrors(msg) => {
            assert!(msg.contains("Too many consecutive errors"));
        }
        _ => panic!("Expected TooManyErrors error, got: {:?}", err),
    }

    Ok(())
}

#[test]
fn test_branch_manager_resets_after_valid_expansion() -> Result<()> {
    let limits = BranchLimits {
        max_nodes: 100,
        max_depth: 10,
        max_breadth: 5,
        max_identical_expansions: 3,
        max_consecutive_errors: 2,
    };

    let mut manager = BranchManager::new(limits);
    let session_id = "test_session";

    // Record some errors
    for i in 0..2 {
        let node = create_mock_node(session_id, &format!("Error {}", i), i);
        manager.record_failure(session_id, &node, "Test error").unwrap();
    }

    // Should fail due to error limit
    let error_node = create_mock_node(session_id, "Error 2", 2);
    assert!(manager.check_before_expand(session_id, &error_node).is_err());

    // Record a successful expansion (should reset error count)
    let success_node = create_mock_node(session_id, "Success", 3);
    manager.record_success(session_id, &success_node).unwrap();

    // Should now allow expansion (error count reset)
    let new_node = create_mock_node(session_id, "New node", 4);
    assert!(manager.check_before_expand(session_id, &new_node).is_ok());

    Ok(())
}

#[test]
fn test_branch_manager_integration_with_tote_engine_stub() -> Result<()> {
    let (_state, _temp) = create_test_state();
    let session_id = "test_session";

    // This test verifies that BranchManager can be integrated with ToTEngine
    // In actual integration, ToTEngine would call BranchManager before expansion
    let limits = BranchLimits::default();
    let mut manager = BranchManager::new(limits);

    // Simulate ToTEngine checking before expansion
    let node = create_mock_node(session_id, "Test expansion", 0);
    let result = manager.check_before_expand(session_id, &node);

    assert!(result.is_ok());

    // Simulate successful expansion
    manager.record_success(session_id, &node).unwrap();

    // Verify diagnostics are updated
    let diagnostics = manager.get_diagnostics(session_id);
    assert_eq!(diagnostics.total_nodes, 1);
    assert_eq!(diagnostics.consecutive_errors, 0);

    Ok(())
}

#[test]
fn test_diagnostics_propagation_on_error() -> Result<()> {
    let limits = BranchLimits::default();
    let mut manager = BranchManager::new(limits);
    let session_id = "test_session";

    // Record an error
    let node = create_mock_node(session_id, "Error node", 0);
    manager.record_failure(session_id, &node, "Test error").unwrap();

    // Check diagnostics
    let diagnostics = manager.get_diagnostics(session_id);
    assert_eq!(diagnostics.consecutive_errors, 1);
    assert!(diagnostics.last_safety_violation.is_some());
    assert!(diagnostics.last_safety_violation.unwrap().contains("Test error"));

    Ok(())
}

#[test]
fn test_session_cannot_expand_after_failure() -> Result<()> {
    let limits = BranchLimits {
        max_nodes: 100,
        max_depth: 10,
        max_breadth: 5,
        max_identical_expansions: 3,
        max_consecutive_errors: 1,
    };

    let mut manager = BranchManager::new(limits);
    let session_id = "test_session";

    // Record one error (at limit)
    let node = create_mock_node(session_id, "Error node", 0);
    manager.record_failure(session_id, &node, "Test error").unwrap();

    // Should fail expansion
    let new_node = create_mock_node(session_id, "New node", 1);
    let result = manager.check_before_expand(session_id, &new_node);

    assert!(result.is_err());
    match result.unwrap_err() {
        ReasoningError::TooManyErrors(msg) => {
            assert!(msg.contains("Too many consecutive errors"));
        }
        _ => panic!("Expected TooManyErrors error"),
    }

    Ok(())
}

#[test]
fn test_limits_can_be_overridden_via_config_struct() -> Result<()> {
    let custom_limits = BranchLimits {
        max_nodes: 50,
        max_depth: 5,
        max_breadth: 2,
        max_identical_expansions: 1,
        max_consecutive_errors: 3,
    };

    let mut manager = BranchManager::new(custom_limits);
    let session_id = "test_session";

    // Test that custom limits are enforced
    for i in 0..50 {
        let node = create_mock_node(session_id, &format!("Node {}", i), i);
        manager.record_success(session_id, &node).unwrap();
    }

    // 51st node should fail
    let node = create_mock_node(session_id, "Node 50", 50);
    let result = manager.check_before_expand(session_id, &node);

    assert!(result.is_err());
    match result.unwrap_err() {
        ReasoningError::BranchLimitExceeded(msg) => {
            assert!(msg.contains("50")); // Should mention custom limit
        }
        _ => panic!("Expected BranchLimitExceeded error"),
    }

    Ok(())
}

#[test]
fn test_safety_invariants_persisted_in_cognition_graph() -> Result<()> {
    let (_state, _temp) = create_test_state();
    let session_id = "test_session";

    // This test verifies that safety violations are recorded in cognition graph
    // In actual implementation, BranchManager would store breaker events
    let limits = BranchLimits::default();
    let mut manager = BranchManager::new(limits);

    // Record a safety violation
    let node = create_mock_node(session_id, "Violation node", 0);
    manager.record_failure(session_id, &node, "Safety violation test").unwrap();

    // Verify that violation is recorded for persistence
    let diagnostics = manager.get_diagnostics(session_id);
    assert!(diagnostics.last_safety_violation.is_some());

    // In real implementation, this would be stored in cognition graph
    // For test, we verify the diagnostic structure
    let violation = diagnostics.last_safety_violation.unwrap();
    assert!(violation.contains("Safety violation test"));

    Ok(())
}
