//! Planning Engine Tests - PHASE 4 TDD
//!
//! Tests for the deterministic PlanningEngine that creates and refines
//! multi-step plans using existing memory/graph APIs.

use std::sync::Arc;
use syncore::agent::{ApreError, ApreResult, PlanNode, PlanTree, PlanningEngine};
use syncore::memory::Memory;
use syncore::raggraph::{HopGraphTransformer, RagGraphConfig};
use syncore::vector::VectorStore;

#[tokio::test]
async fn test_initial_plan_generation() -> ApreResult<()> {
    // GIVEN: A goal and available memory/graph services
    let memory = Arc::new(create_test_memory().await?);
    let vector_store = Arc::new(create_test_vector_store().await?);
    let hop_graph = HopGraphTransformer::new(create_test_rag_config());

    let mut planner = PlanningEngine::new(memory, vector_store, hop_graph);

    let goal = "Implement user authentication system";
    let constraints = vec!["Use existing database schema".to_string()];

    // WHEN: Generate initial plan
    let plan = planner.generate_initial_plan(goal, &constraints).await?;

    // THEN: Plan should be valid
    assert!(!plan.nodes.is_empty(), "Plan should have at least one node");
    assert_eq!(plan.nodes[0].task, goal, "Root node should match goal");
    assert_eq!(plan.nodes[0].dependencies.len(), 0, "Root node should have no dependencies");
    assert_eq!(plan.nodes[0].status, syncore::agent::PlanNodeStatus::Pending);

    // Plan should have subtasks for authentication
    let auth_tasks: Vec<_> = plan
        .nodes
        .iter()
        .filter(|node| node.task.contains("password") || node.task.contains("login"))
        .collect();
    assert!(!auth_tasks.is_empty(), "Plan should include authentication subtasks");

    Ok(())
}

#[tokio::test]
async fn test_plan_refinement_after_success() -> ApreResult<()> {
    // GIVEN: An existing plan and a successfully completed action
    let memory = Arc::new(create_test_memory().await?);
    let vector_store = Arc::new(create_test_vector_store().await?);
    let hop_graph = HopGraphTransformer::new(create_test_rag_config());

    let mut planner = PlanningEngine::new(memory, vector_store, hop_graph);

    let mut plan = create_test_plan().await?;
    let completed_node_id = plan.nodes[0].id.clone();

    // WHEN: Refine plan after successful action
    let refined_plan = planner
        .refine_plan_after_action(
            &mut plan,
            &completed_node_id,
            true, // success
            Some("Action completed successfully"),
            None, // no error
        )
        .await?;

    // THEN: Plan should be updated correctly
    let completed_node = refined_plan
        .nodes
        .iter()
        .find(|node| node.id == completed_node_id)
        .expect("Completed node should still exist");

    assert_eq!(completed_node.status, syncore::agent::PlanNodeStatus::Completed);

    // Next actionable nodes should be ready
    let ready_nodes: Vec<_> = refined_plan
        .nodes
        .iter()
        .filter(|node| node.status == syncore::agent::PlanNodeStatus::Ready)
        .collect();
    assert!(!ready_nodes.is_empty(), "Should have ready nodes after completion");

    Ok(())
}

#[tokio::test]
async fn test_plan_refinement_after_failure() -> ApreResult<()> {
    // GIVEN: An existing plan and a failed action
    let memory = Arc::new(create_test_memory().await?);
    let vector_store = Arc::new(create_test_vector_store().await?);
    let hop_graph = HopGraphTransformer::new(create_test_rag_config());

    let mut planner = PlanningEngine::new(memory, vector_store, hop_graph);

    let mut plan = create_test_plan().await?;
    let failed_node_id = plan.nodes[1].id.clone();

    // WHEN: Refine plan after failed action
    let error_msg = "Database connection failed";
    let refined_plan = planner
        .refine_plan_after_action(
            &mut plan,
            &failed_node_id,
            false, // failure
            None,  // no success message
            Some(error_msg),
        )
        .await?;

    // THEN: Plan should handle failure appropriately
    let failed_node = refined_plan
        .nodes
        .iter()
        .find(|node| node.id == failed_node_id)
        .expect("Failed node should still exist");

    assert_eq!(failed_node.status, syncore::agent::PlanNodeStatus::Failed);
    assert!(failed_node.error_message.is_some());

    // Should not mark dependent nodes as ready
    let dependent_ready: Vec<_> = refined_plan
        .nodes
        .iter()
        .filter(|node| {
            node.status == syncore::agent::PlanNodeStatus::Ready
                && node.dependencies.contains(&failed_node_id)
        })
        .collect();
    assert_eq!(dependent_ready.len(), 0, "Failed node's dependents should not be ready");

    Ok(())
}

#[tokio::test]
async fn test_detect_deadlocks() -> ApreResult<()> {
    // GIVEN: A plan with circular dependencies
    let memory = Arc::new(create_test_memory().await?);
    let vector_store = Arc::new(create_test_vector_store().await?);
    let hop_graph = HopGraphTransformer::new(create_test_rag_config());

    let mut planner = PlanningEngine::new(memory, vector_store, hop_graph);

    let mut plan = create_circular_dependency_plan().await?;

    // WHEN: Check for deadlocks
    let deadlocks = planner.detect_deadlocks(&plan).await?;

    // THEN: Should detect circular dependencies
    assert!(!deadlocks.is_empty(), "Should detect deadlock in circular dependency");

    // Deadlock should contain the cycle
    assert!(deadlocks[0].len() >= 2, "Deadlock cycle should have at least 2 nodes");

    Ok(())
}

#[tokio::test]
async fn test_next_action_order() -> ApreResult<()> {
    // GIVEN: A complex plan with multiple ready nodes
    let memory = Arc::new(create_test_memory().await?);
    let vector_store = Arc::new(create_test_vector_store().await?);
    let hop_graph = HopGraphTransformer::new(create_test_rag_config());

    let mut planner = PlanningEngine::new(memory, vector_store, hop_graph);

    let mut plan = create_complex_plan().await?;

    // WHEN: Get next action
    let next_action = planner.next_action(&mut plan).await?;

    // THEN: Should return the highest priority ready node
    assert!(next_action.is_some(), "Should have a next action");

    let action = next_action.unwrap();
    assert_eq!(action.status, syncore::agent::PlanNodeStatus::Ready);

    // Should be the highest priority ready node
    let ready_nodes: Vec<_> = plan
        .nodes
        .iter()
        .filter(|node| node.status == syncore::agent::PlanNodeStatus::Ready)
        .collect();

    if !ready_nodes.is_empty() {
        let max_priority = ready_nodes.iter().map(|node| node.priority).max().unwrap();
        assert_eq!(action.priority, max_priority, "Should select highest priority node");
    }

    Ok(())
}

// Test helper functions (these will need to be implemented)
async fn create_test_memory() -> ApreResult<Memory> {
    // This should create a test memory instance
    // Implementation needed
    Err(ApreError::PlanningFailed("Test helper not implemented".to_string()))
}

async fn create_test_vector_store() -> ApreResult<VectorStore> {
    // This should create a test vector store
    // Implementation needed
    Err(ApreError::PlanningFailed("Test helper not implemented".to_string()))
}

fn create_test_rag_config() -> RagGraphConfig {
    // This should create a test RagGraphConfig
    // Implementation needed
    todo!("Test helper not implemented")
}

async fn create_test_plan() -> ApreResult<PlanTree> {
    // This should create a test plan for testing
    // Implementation needed
    Err(ApreError::PlanningFailed("Test helper not implemented".to_string()))
}

async fn create_circular_dependency_plan() -> ApreResult<PlanTree> {
    // This should create a plan with circular dependencies
    // Implementation needed
    Err(ApreError::PlanningFailed("Test helper not implemented".to_string()))
}

async fn create_complex_plan() -> ApreResult<PlanTree> {
    // This should create a complex plan for testing priority ordering
    // Implementation needed
    Err(ApreError::PlanningFailed("Test helper not implemented".to_string()))
}
