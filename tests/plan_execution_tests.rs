//! PHASE 4 STEP 5.2: Plan Execution Tests - TDD Failing Tests First
//!
//! These tests MUST fail initially. They define the plan execution requirements
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
use syncore::agent::{
    AgentSessionState, ApreResult, PlanNode, PlanNodeStatus, PlanTree, PlanningEngine,
};
use syncore::memory::Memory;
use syncore::raggraph::{HopGraphTransformer, RagGraphConfig};
use syncore::router::SynCoreState;
use syncore::vector::VectorStore;

#[cfg(test)]
mod plan_execution_tests {

    /// Test Case 1: AgentSessionState executes plan steps correctly
    #[tokio::test]
    async fn test_agent_session_state_executes_plan_steps() {
        // GIVEN: AgentSessionState with active plan
        let mut session_state = create_test_session_state().await;

        let goal = "Add error recovery to file operations";
        let constraints = vec!["Use existing error_recovery module".to_string()];

        // Generate initial plan
        let plan = session_state
            .start_plan(goal, &constraints)
            .await
            .expect("Plan generation should succeed");
        assert!(!plan.nodes.is_empty(), "Plan should have nodes");

        let initial_step_count = session_state.step_count;

        // WHEN: Executing next step
        let execution_result = session_state.execute_next_step().await;

        // THEN: Should execute successfully and update state
        assert!(
            execution_result.is_ok(),
            "Step execution should succeed: {:?}",
            execution_result.err()
        );

        let result = execution_result.unwrap();
        assert!(!result.is_empty(), "Execution result should not be empty");

        // Should have progressed
        assert_eq!(session_state.step_count, initial_step_count + 1, "Step count should increment");

        // At least one node should be completed
        let completed_nodes: Vec<_> =
            plan.nodes.iter().filter(|n| n.status == PlanNodeStatus::Completed).collect();
        assert!(!completed_nodes.is_empty(), "Should have completed nodes after execution");
    }

    /// Test Case 2: Plan execution updates node statuses correctly
    #[tokio::test]
    async fn test_plan_execution_updates_node_statuses() {
        // GIVEN: Plan with multiple nodes
        let mut session_state = create_test_session_state().await;

        let goal = "Refactor error handling across multiple files";
        let plan =
            session_state.start_plan(goal, &[]).await.expect("Plan generation should succeed");

        let initial_ready_nodes =
            plan.nodes.iter().filter(|n| n.status == PlanNodeStatus::Ready).count();
        let initial_completed_nodes =
            plan.nodes.iter().filter(|n| n.status == PlanNodeStatus::Completed).count();

        // WHEN: Executing a step
        session_state.execute_next_step().await.expect("Step execution should succeed");

        // THEN: Node statuses should be updated
        let current_plan = session_state.get_plan();
        let current_ready_nodes =
            current_plan.nodes.iter().filter(|n| n.status == PlanNodeStatus::Ready).count();
        let current_completed_nodes =
            current_plan.nodes.iter().filter(|n| n.status == PlanNodeStatus::Completed).count();

        // Should have fewer ready nodes and more completed nodes
        assert_eq!(
            current_ready_nodes,
            initial_ready_nodes - 1,
            "Should have one fewer ready node"
        );
        assert_eq!(
            current_completed_nodes,
            initial_completed_nodes + 1,
            "Should have one more completed node"
        );
    }

    /// Test Case 3: Plan execution respects dependencies
    #[tokio::test]
    async fn test_plan_execution_respects_dependencies() {
        // GIVEN: Plan with dependency chain
        let mut session_state = create_test_session_state().await;

        let goal = "Implement multi-step refactoring with dependencies";
        let plan =
            session_state.start_plan(goal, &[]).await.expect("Plan generation should succeed");

        // Find nodes with dependencies
        let dependent_nodes: Vec<_> =
            plan.nodes.iter().filter(|n| !n.dependencies.is_empty()).collect();
        assert!(!dependent_nodes.is_empty(), "Plan should have dependent nodes");

        // WHEN: Executing steps
        let mut execution_results = vec![];
        for _ in 0..3 {
            match session_state.execute_next_step().await {
                Ok(result) => execution_results.push(result),
                Err(_) => break, // Stop if no more steps available
            }
        }

        // THEN: Should execute dependencies before dependent nodes
        let current_plan = session_state.get_plan();

        // Check that completed nodes include dependencies first
        let completed_node_ids: std::collections::HashSet<_> = current_plan
            .nodes
            .iter()
            .filter(|n| n.status == PlanNodeStatus::Completed)
            .map(|n| n.id)
            .collect();

        for node in &current_plan.nodes {
            if node.status == PlanNodeStatus::Completed && !node.dependencies.is_empty() {
                // All dependencies should be completed first
                for dep_id in &node.dependencies {
                    assert!(
                        completed_node_ids.contains(dep_id),
                        "Dependency {} should be completed before node {}",
                        dep_id,
                        node.id
                    );
                }
            }
        }
    }

    /// Test Case 4: Plan execution handles failures gracefully
    #[tokio::test]
    async fn test_plan_execution_handles_failures_gracefully() {
        // GIVEN: Plan that includes a potentially failing action
        let mut session_state = create_test_session_state().await;

        let goal = "Execute action that will fail";
        let constraints = vec!["Include failing test operation".to_string()];

        let plan = session_state
            .start_plan(goal, &constraints)
            .await
            .expect("Plan generation should succeed");

        // WHEN: Executing step that may fail
        let execution_result = session_state.execute_next_step().await;

        // THEN: Should handle failure without crashing
        // (The execution should either succeed or fail gracefully)
        match execution_result {
            Ok(result) => {
                // If it succeeds, result should be meaningful
                assert!(!result.is_empty(), "Successful result should not be empty");
            }
            Err(error) => {
                // If it fails, error should be meaningful
                let error_str = format!("{:?}", error);
                assert!(!error_str.is_empty(), "Error should provide meaningful information");
            }
        }

        // Session state should remain consistent
        assert!(session_state.step_count > 0, "Step count should be updated even on failure");
        assert!(!session_state.is_plan_complete(), "Plan should not be complete after single step");
    }

    /// Test Case 5: Plan execution stops when complete
    #[tokio::test]
    async fn test_plan_execution_stops_when_complete() {
        // GIVEN: AgentSessionState with plan
        let mut session_state = create_test_session_state().await;

        let goal = "Simple single-step task";
        let plan =
            session_state.start_plan(goal, &[]).await.expect("Plan generation should succeed");

        // WHEN: Executing all steps until completion
        let mut execution_results = vec![];
        loop {
            match session_state.execute_next_step().await {
                Ok(result) => {
                    execution_results.push(result);
                    if session_state.is_plan_complete() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        // THEN: Should complete successfully
        assert!(!execution_results.is_empty(), "Should have executed at least one step");
        assert!(session_state.is_plan_complete(), "Plan should be marked complete");

        let current_plan = session_state.get_plan();
        let all_nodes_completed =
            current_plan.nodes.iter().all(|n| n.status == PlanNodeStatus::Completed);
        assert!(all_nodes_completed, "All nodes should be completed when plan is complete");
    }

    /// Test Case 6: Plan execution maintains session metrics
    #[tokio::test]
    async fn test_plan_execution_maintains_session_metrics() {
        // GIVEN: AgentSessionState
        let mut session_state = create_test_session_state().await;

        let initial_metrics = session_state.metrics.clone();
        assert_eq!(initial_metrics.actions_executed, 0, "Initial actions should be zero");

        // WHEN: Executing multiple steps
        let goal = "Multi-step task for metrics testing";
        session_state.start_plan(goal, &[]).await.expect("Plan generation should succeed");

        for _ in 0..3 {
            if session_state.execute_next_step().await.is_err() {
                break;
            }
        }

        // THEN: Metrics should be updated
        let updated_metrics = session_state.metrics;
        assert!(
            updated_metrics.actions_executed > initial_metrics.actions_executed,
            "Actions executed should increase"
        );

        assert!(updated_metrics.total_planning_time_ms > 0, "Planning time should be recorded");

        // Average action time should be reasonable
        if updated_metrics.actions_executed > 0 {
            assert!(
                updated_metrics.avg_action_time_ms >= 0.0,
                "Average action time should be non-negative"
            );
        }
    }

    /// Test Case 7: Plan execution integrates with existing memory
    #[tokio::test]
    async fn test_plan_execution_integrates_existing_memory() {
        // GIVEN: AgentSessionState with memory integration
        let mut session_state = create_test_session_state().await;

        // Store execution context in memory
        let memory_key = "execution_context";
        let memory_value = r#"{
            "target_files": ["src/parser.rs", "src/error.rs"],
            "required_patterns": ["anyhow::Result", "? operator"],
            "avoid_patterns": ["panic!", "unwrap()"]
        }"#;

        session_state.memory.store(memory_key, memory_value).expect("Memory store should succeed");

        // WHEN: Executing plan that should use memory context
        let goal = "Update error patterns based on stored context";
        session_state.start_plan(goal, &[]).await.expect("Plan generation should succeed");

        let execution_result = session_state.execute_next_step().await;

        // THEN: Execution should leverage memory context
        assert!(execution_result.is_ok(), "Execution should succeed with memory context");

        // Memory should retain execution information
        let memory_query = session_state
            .memory
            .query("execution_context")
            .await
            .expect("Memory query should succeed");
        assert!(!memory_query.is_empty(), "Memory should still contain context");
    }

    /// Test Case 8: Plan execution can be paused and resumed
    #[tokio::test]
    async fn test_plan_execution_can_be_paused_and_resumed() {
        // GIVEN: AgentSessionState with active plan
        let mut session_state = create_test_session_state().await;

        let goal = "Long-running task that can be paused";
        session_state.start_plan(goal, &[]).await.expect("Plan generation should succeed");

        // Execute first step
        session_state.execute_next_step().await.expect("First step should succeed");

        // WHEN: Pausing execution
        session_state.pause();
        assert_ne!(session_state.execution_state, syncore::agent::PlanExecutionState::Ready);

        // Try to execute while paused
        let paused_execution_result = session_state.execute_next_step().await;

        // THEN: Should not execute while paused
        assert!(paused_execution_result.is_err(), "Should not execute while paused");

        // WHEN: Resuming execution
        session_state.resume();

        // THEN: Should be able to execute again
        let resumed_execution_result = session_state.execute_next_step().await;
        assert!(resumed_execution_result.is_ok(), "Should execute successfully after resume");
    }

    /// Test Case 9: Plan execution persists session state
    #[tokio::test]
    async fn test_plan_execution_persists_session_state() {
        // GIVEN: AgentSessionState with active execution
        let mut session_state = create_test_session_state().await;

        let goal = "Task with persistence testing";
        session_state.start_plan(goal, &[]).await.expect("Plan generation should succeed");

        // Execute some steps
        for _ in 0..2 {
            if session_state.execute_next_step().await.is_err() {
                break;
            }
        }

        let session_id = session_state.id.clone();
        let step_count_before = session_state.step_count;

        // WHEN: Storing session state to memory
        session_state.store_to_memory().await.expect("Session state should store to memory");

        // THEN: Should be able to load session state from memory
        let loaded_session =
            AgentSessionState::load_from_memory(&session_state.memory, &session_id)
                .await
                .expect("Should load session from memory");

        assert_eq!(loaded_session.id, session_id, "Session ID should match");
        assert_eq!(loaded_session.step_count, step_count_before, "Step count should be preserved");
        assert!(loaded_session.active_plan.is_some(), "Plan should be preserved");
    }
}

// Helper function to create test session state (this will fail to compile initially)
async fn create_test_session_state() -> AgentSessionState {
    // This should use existing PlanningEngine and ReflectionEngine
    // NOTE: This helper uses existing APIs - no new infrastructure

    unimplemented!("Session state creation needs integration with existing systems")
}
