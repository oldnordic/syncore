//! PHASE 4 STEP 5.3: Session Persistence Tests - TDD Failing Tests First
//!
//! These tests MUST fail initially. They define the session persistence requirements
//! that will be implemented in subsequent steps.
//!
//! FOR EACH FAILING TEST:
//! • read all affected code areas (ripgrep + tree-sitter)
//! • write minimal patch
//! • re-run tests
//! • fix next failing test
//!
//! Do NOT:
//! × invent behaviors not grounded in existing systems
//! × introduce new agent sub-systems
//! × write pseudo-code
//! × create 'later' comments or TODO markers

use std::sync::Arc;
use syncore::agent::{AgentSessionState, ApreResult, PlanningEngine, ReflectionEngine};
use syncore::memory::Memory;
use syncore::raggraph::{HopGraphTransformer, RagGraphConfig};
use syncore::vector::VectorStore;

#[cfg(test)]
mod session_persistence_tests {

    /// Test Case 1: Session state persists to memory correctly
    #[tokio::test]
    async fn test_session_state_persists_to_memory() {
        // GIVEN: AgentSessionState with active execution
        let mut session_state = create_test_session_state().await;

        let goal = "Implement error recovery for file operations";
        session_state.start_plan(goal, &[]).await.expect("Plan generation should succeed");

        // Execute some steps to create state
        for _ in 0..3 {
            if session_state.execute_next_step().await.is_err() {
                break;
            }
        }

        let session_id = session_state.id.clone();
        let step_count = session_state.step_count;
        let has_active_plan = session_state.active_plan.is_some();

        // WHEN: Storing session state to memory
        let store_result = session_state.store_to_memory().await;

        // THEN: Should persist successfully
        assert!(store_result.is_ok(), "Session should store to memory: {:?}", store_result.err());

        // Memory should contain session data
        let memory_key = format!("agent_session:{}", session_id);
        let memory_query =
            session_state.memory.query(&memory_key).await.expect("Memory query should succeed");
        assert!(!memory_query.is_empty(), "Memory should contain stored session");
    }

    /// Test Case 2: Session state loads from memory correctly
    #[tokio::test]
    async fn test_session_state_loads_from_memory() {
        // GIVEN: Stored session state in memory
        let mut original_session = create_test_session_state().await;

        let goal = "Refactor database connection handling";
        original_session.start_plan(goal, &[]).await.expect("Plan generation should succeed");

        // Execute step to create meaningful state
        original_session.execute_next_step().await.expect("Step execution should succeed");

        let original_id = original_session.id.clone();
        let original_step_count = original_session.step_count;
        let original_has_plan = original_session.active_plan.is_some();

        // Store session to memory
        original_session.store_to_memory().await.expect("Session should store to memory");

        // WHEN: Loading session state from memory
        let loaded_session =
            AgentSessionState::load_from_memory(&original_session.memory, &original_id)
                .await
                .expect("Session should load from memory");

        // THEN: Loaded session should match original
        assert_eq!(loaded_session.id, original_id, "Session ID should match");
        assert_eq!(loaded_session.step_count, original_step_count, "Step count should match");
        assert_eq!(
            loaded_session.active_plan.is_some(),
            original_has_plan,
            "Plan status should match"
        );

        // Execution state should be preserved
        assert!(!loaded_session.created_at.is_zero(), "Creation timestamp should be preserved");
        assert!(!loaded_session.last_activity.is_zero(), "Activity timestamp should be preserved");
    }

    /// Test Case 3: Session state handles missing sessions gracefully
    #[tokio::test]
    async fn test_session_state_handles_missing_sessions() {
        // GIVEN: Memory with no stored session
        let session_state = create_test_session_state().await;

        let non_existent_id = "non_existent_session_12345";

        // WHEN: Attempting to load missing session
        let load_result =
            AgentSessionState::load_from_memory(&session_state.memory, non_existent_id).await;

        // THEN: Should handle missing session gracefully
        assert!(load_result.is_err(), "Should fail to load non-existent session");

        match load_result.err().unwrap() {
            syncore::agent::ApreError::SessionNotFound(id) => {
                assert_eq!(id, non_existent_id, "Should return correct session ID");
            }
            other => panic!("Expected SessionNotFound error, got: {:?}", other),
        }
    }

    /// Test Case 4: Session state preserves plan tree structure
    #[tokio::test]
    async fn test_session_state_preserves_plan_tree_structure() {
        // GIVEN: Session with complex plan tree
        let mut session_state = create_test_session_state().await;

        let goal = "Implement comprehensive error handling across multiple modules";
        let constraints = vec![
            "Use existing error_recovery module".to_string(),
            "Maintain backward compatibility".to_string(),
            "Add comprehensive logging".to_string(),
        ];

        let original_plan = session_state
            .start_plan(goal, &constraints)
            .await
            .expect("Plan generation should succeed");

        // Execute some steps to modify plan state
        for _ in 0..2 {
            if session_state.execute_next_step().await.is_err() {
                break;
            }
        }

        let original_node_count = original_plan.nodes.len();
        let original_completed_count = original_plan
            .nodes
            .iter()
            .filter(|n| n.status == syncore::agent::PlanNodeStatus::Completed)
            .count();

        // Store and reload session
        session_state.store_to_memory().await.expect("Session should store to memory");

        let loaded_session =
            AgentSessionState::load_from_memory(&session_state.memory, &session_state.id)
                .await
                .expect("Session should load from memory");

        // WHEN: Examining loaded plan structure
        let loaded_plan =
            loaded_session.active_plan.as_ref().expect("Loaded session should have active plan");

        // THEN: Plan tree structure should be preserved
        assert_eq!(loaded_plan.nodes.len(), original_node_count, "Node count should match");

        let loaded_completed_count = loaded_plan
            .nodes
            .iter()
            .filter(|n| n.status == syncore::agent::PlanNodeStatus::Completed)
            .count();
        assert_eq!(
            loaded_completed_count, original_completed_count,
            "Completed node count should match"
        );

        // Node relationships should be preserved
        for original_node in &original_plan.nodes {
            let loaded_node = loaded_plan
                .nodes
                .iter()
                .find(|n| n.id == original_node.id)
                .expect("Node should exist in loaded plan");
            assert_eq!(loaded_node.task, original_node.task, "Node task should match");
            assert_eq!(
                loaded_node.dependencies, original_node.dependencies,
                "Dependencies should match"
            );
        }
    }

    /// Test Case 5: Session state persists reflection reports
    #[tokio::test]
    async fn test_session_state_persists_reflection_reports() {
        // GIVEN: Session with reflection data
        let mut session_state = create_test_session_state().await;

        // Trigger a reflection by failing an action
        let goal = "Task that will trigger reflection";
        let constraints = vec!["Include failing operation".to_string()];

        session_state.start_plan(goal, &constraints).await.expect("Plan generation should succeed");

        // Execute step that may fail and trigger reflection
        let _ = session_state.execute_next_step().await;

        let has_reflection_before = session_state.last_reflection.is_some();

        // WHEN: Storing and loading session
        session_state.store_to_memory().await.expect("Session should store to memory");

        let loaded_session =
            AgentSessionState::load_from_memory(&session_state.memory, &session_state.id)
                .await
                .expect("Session should load from memory");

        // THEN: Reflection data should be preserved
        assert_eq!(
            loaded_session.last_reflection.is_some(),
            has_reflection_before,
            "Reflection presence should be preserved"
        );

        if let (Some(original_reflection), Some(loaded_reflection)) =
            (&session_state.last_reflection, &loaded_session.last_reflection)
        {
            assert_eq!(
                loaded_reflection.action_description, original_reflection.action_description,
                "Reflection action should match"
            );
            assert_eq!(
                loaded_reflection.error_summary, original_reflection.error_summary,
                "Reflection error summary should match"
            );
        }
    }

    /// Test Case 6: Session state persists performance metrics
    #[tokio::test]
    async fn test_session_state_persists_performance_metrics() {
        // GIVEN: Session with accumulated metrics
        let mut session_state = create_test_session_state().await;

        let goal = "Generate task for metrics testing";
        session_state.start_plan(goal, &[]).await.expect("Plan generation should succeed");

        // Execute multiple steps to accumulate metrics
        let initial_actions = session_state.metrics.actions_executed;
        let initial_planning_time = session_state.metrics.total_planning_time_ms;

        // Execute steps
        for _ in 0..3 {
            if session_state.execute_next_step().await.is_err() {
                break;
            }
        }

        let final_actions = session_state.metrics.actions_executed;
        let final_planning_time = session_state.metrics.total_planning_time_ms;

        assert!(final_actions > initial_actions, "Should have executed actions");
        assert!(final_planning_time > initial_planning_time, "Should have recorded planning time");

        // WHEN: Storing and loading session
        session_state.store_to_memory().await.expect("Session should store to memory");

        let loaded_session =
            AgentSessionState::load_from_memory(&session_state.memory, &session_state.id)
                .await
                .expect("Session should load from memory");

        // THEN: Performance metrics should be preserved
        assert_eq!(
            loaded_session.metrics.actions_executed, final_actions,
            "Actions executed should match"
        );
        assert_eq!(
            loaded_session.metrics.total_planning_time_ms, final_planning_time,
            "Planning time should match"
        );
        assert!(
            loaded_session.metrics.avg_action_time_ms >= 0.0,
            "Average action time should be preserved"
        );
    }

    /// Test Case 7: Session state handles corruption gracefully
    #[tokio::test]
    async fn test_session_state_handles_corruption_gracefully() {
        // GIVEN: Memory with corrupted session data
        let session_state = create_test_session_state().await;
        let session_id = "corrupted_session_test";

        // Store invalid JSON in memory
        let corrupted_key = format!("agent_session:{}", session_id);
        let corrupted_data = "{ invalid json data [broken";

        session_state
            .memory
            .store(&corrupted_key, corrupted_data)
            .expect("Memory should store corrupted data");

        // WHEN: Attempting to load corrupted session
        let load_result =
            AgentSessionState::load_from_memory(&session_state.memory, session_id).await;

        // THEN: Should handle corruption gracefully
        assert!(load_result.is_err(), "Should fail to load corrupted session");

        match load_result.err().unwrap() {
            syncore::agent::ApreError::MemoryError(_) => {
                // Expected error type
            }
            other => panic!("Expected MemoryError for corrupted data, got: {:?}", other),
        }
    }

    /// Test Case 8: Session state persistence is performant
    #[tokio::test]
    async fn test_session_state_persistence_is_performant() {
        // GIVEN: Session with substantial data
        let mut session_state = create_test_session_state().await;

        // Create substantial plan data
        let goal = "Large-scale refactoring with many steps";
        session_state.start_plan(goal, &[]).await.expect("Plan generation should succeed");

        // Execute steps to accumulate data
        for _ in 0..10 {
            if session_state.execute_next_step().await.is_err() {
                break;
            }
        }

        // WHEN: Measuring persistence performance
        let store_start = std::time::Instant::now();
        session_state.store_to_memory().await.expect("Session should store to memory");
        let store_duration = store_start.elapsed();

        let load_start = std::time::Instant::now();
        let _loaded_session =
            AgentSessionState::load_from_memory(&session_state.memory, &session_state.id)
                .await
                .expect("Session should load from memory");
        let load_duration = load_start.elapsed();

        // THEN: Operations should be reasonably fast
        assert!(store_duration.as_millis() < 1000, "Store should complete within 1 second");
        assert!(load_duration.as_millis() < 1000, "Load should complete within 1 second");

        // Memory usage should be reasonable (basic check)
        let memory_key = format!("agent_session:{}", session_state.id);
        let stored_data =
            session_state.memory.query(&memory_key).await.expect("Memory query should succeed");
        assert!(!stored_data.is_empty(), "Should have stored data");
        assert!(stored_data[0].value.len() < 1_000_000, "Stored data should be reasonably sized");
        // < 1MB
    }
}

// Helper function to create test session state (this will fail to compile initially)
async fn create_test_session_state() -> AgentSessionState {
    // This should use existing Memory, PlanningEngine, and ReflectionEngine
    // NOTE: This helper uses existing APIs - no new infrastructure

    unimplemented!("Session state creation needs integration with existing systems")
}
