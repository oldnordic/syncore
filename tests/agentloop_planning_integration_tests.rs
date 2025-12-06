//! AgentLoop Planning Integration Tests - PHASE 4 TDD
//!
//! Tests for the full PLAN → ACT → REFLECT integration with existing ToT engine
//! and memory/graph systems.

use std::sync::Arc;
use syncore::agent::{
    AgentSessionState, ApreError, ApreResult, PlanExecutionState, PlanningEngine, ReflectionEngine,
};
use syncore::memory::Memory;
use syncore::raggraph::{HopGraphTransformer, RagGraphConfig};
use syncore::reasoning::{ReasoningSessionManager, ToTEngine};
use syncore::vector::VectorStore;

#[tokio::test]
async fn test_agentloop_runs_multi_step_plan() -> ApreResult<()> {
    // GIVEN: A complete AgentLoop setup with multi-step plan
    let memory = Arc::new(create_test_memory().await?);
    let vector_store = Arc::new(create_test_vector_store().await?);
    let hop_graph = HopGraphTransformer::new(create_test_rag_config());

    let mut planning_engine = PlanningEngine::new(memory.clone(), vector_store, hop_graph);

    let reasoning_engine = create_test_reasoning_engine().await?;

    let reflection_engine = ReflectionEngine::new(
        memory.clone(),
        HopGraphTransformer::new(create_test_rag_config()),
        Arc::new(reasoning_engine),
    );

    let mut agent_state = AgentSessionState::new(planning_engine, reflection_engine);

    // Create a multi-step plan
    let goal = "Build a REST API with authentication";
    let constraints = vec!["Use Rust".to_string(), "Include JWT auth".to_string()];
    agent_state.active_plan =
        Some(agent_state.planning_engine.generate_initial_plan(goal, &constraints).await?);

    // WHEN: Execute multiple steps
    let mut step_results = Vec::new();
    let max_steps = 5;

    for step in 0..max_steps {
        let result = agent_state.execute_next_step().await;

        match result {
            Ok(output) => {
                step_results.push(output);

                // Check if plan is complete
                if agent_state.is_plan_complete() {
                    break;
                }
            }
            Err(ApreError::ExecutionFailed(msg)) if msg.contains("No ready actions") => {
                // No more ready actions, plan might be complete or blocked
                break;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    // THEN: Should execute multiple steps successfully
    assert!(!step_results.is_empty(), "Should execute at least one step");

    // Should progress through the plan
    let completed_nodes = agent_state
        .get_plan()
        .nodes
        .iter()
        .filter(|node| node.status == syncore::agent::PlanNodeStatus::Completed)
        .count();
    assert!(completed_nodes > 0, "Should have completed some plan nodes");

    Ok(())
}

#[tokio::test]
async fn test_agentloop_updates_plan_after_each_step() -> ApreResult<()> {
    // GIVEN: AgentLoop with a plan
    let memory = Arc::new(create_test_memory().await?);
    let vector_store = Arc::new(create_test_vector_store().await?);
    let hop_graph = HopGraphTransformer::new(create_test_rag_config());

    let mut planning_engine = PlanningEngine::new(memory.clone(), vector_store, hop_graph);

    let reflection_engine = ReflectionEngine::new(
        memory.clone(),
        HopGraphTransformer::new(create_test_rag_config()),
        Arc::new(create_test_reasoning_engine().await?),
    );

    let mut agent_state = AgentSessionState::new(planning_engine, reflection_engine);

    let goal = "Test goal";
    agent_state.active_plan =
        Some(agent_state.planning_engine.generate_initial_plan(goal, &[]).await?);

    let initial_plan = agent_state.get_plan().clone();

    // WHEN: Execute a step
    let step_result = agent_state.execute_next_step().await?;

    // THEN: Plan should be updated after the step
    let updated_plan = agent_state.get_plan();

    // At least one node should have changed status
    let status_changes = initial_plan
        .nodes
        .iter()
        .zip(updated_plan.nodes.iter())
        .filter(|(initial, updated)| initial.status != updated.status)
        .count();

    assert!(status_changes > 0, "Plan should be updated after step execution");

    // Should have step result
    assert!(!step_result.is_empty(), "Step should produce output");

    // Should have reflection if step failed
    if agent_state.last_reflection.is_some() {
        assert!(
            !agent_state.last_reflection.as_ref().unwrap().root_causes.is_empty(),
            "Reflection should include analysis"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_agentloop_reflection_integration() -> ApreResult<()> {
    // GIVEN: AgentLoop setup that will experience a failure
    let memory = Arc::new(create_test_memory().await?);
    let vector_store = Arc::new(create_test_vector_store().await?);
    let hop_graph = HopGraphTransformer::new(create_test_rag_config());

    let mut planning_engine = PlanningEngine::new(memory.clone(), vector_store, hop_graph);

    // Create a reflection engine that will detect failures
    let reflection_engine = ReflectionEngine::new(
        memory.clone(),
        HopGraphTransformer::new(create_test_rag_config()),
        Arc::new(create_test_reasoning_engine().await?),
    );

    let mut agent_state = AgentSessionState::new(planning_engine, reflection_engine);

    // Create a plan with a step that will fail
    let goal = "Simulate failure scenario";
    agent_state.active_plan = Some(create_failing_test_plan().await?);

    // WHEN: Execute the failing step
    let _step_result = agent_state.execute_next_step().await;

    // THEN: Should have generated a reflection
    assert!(agent_state.last_reflection.is_some(), "Should have reflection after failure");

    let reflection = agent_state.last_reflection.as_ref().unwrap();
    assert!(reflection.failure_detected, "Reflection should detect failure");
    assert!(!reflection.recovery_actions.is_empty(), "Should suggest recovery actions");

    // Should store reflection in memory
    let memory_key = format!("reflection:{}", reflection.plan_id);
    let stored = memory.query(&memory_key).await?;
    assert!(!stored.is_empty(), "Reflection should be stored in memory");

    Ok(())
}

#[tokio::test]
async fn test_agentloop_uses_graph_memory_for_planning() -> ApreResult<()> {
    // GIVEN: AgentLoop with graph and memory services
    let memory = Arc::new(create_test_memory().await?);
    let vector_store = Arc::new(create_test_vector_store().await?);

    // Set up RagGraph for semantic retrieval during planning
    let mut rag_config = create_test_rag_config();
    rag_config.backend_mode = syncore::raggraph::RaggraphBackendMode::Real;

    let hop_graph = HopGraphTransformer::with_storage(
        rag_config,
        Arc::new(create_test_storage_adapter().await?),
    );

    let mut planning_engine = PlanningEngine::new(memory.clone(), vector_store, hop_graph);

    let reflection_engine = ReflectionEngine::new(
        memory.clone(),
        HopGraphTransformer::new(create_test_rag_config()),
        Arc::new(create_test_reasoning_engine().await?),
    );

    let mut agent_state = AgentSessionState::new(planning_engine, reflection_engine);

    // Pre-populate memory with relevant context
    populate_test_memory(&memory).await?;

    // WHEN: Generate plan using graph/memory
    let goal = "Implement user registration with email verification";
    let constraints = vec!["Use existing email service".to_string()];

    agent_state.active_plan =
        Some(agent_state.planning_engine.generate_initial_plan(goal, &constraints).await?);

    // THEN: Plan should leverage graph and memory
    let plan = agent_state.get_plan();

    // Should include steps that reference stored knowledge
    let email_steps: Vec<_> =
        plan.nodes.iter().filter(|node| node.task.to_lowercase().contains("email")).collect();
    assert!(!email_steps.is_empty(), "Plan should include email-related steps");

    // Should have retrieved relevant context from memory
    let memory_queries = memory.query("registration").await?;
    assert!(!memory_queries.is_empty(), "Should have queried memory for planning context");

    Ok(())
}

// Test helper functions (these will need to be implemented)
async fn create_test_memory() -> ApreResult<Memory> {
    Err(ApreError::ExecutionFailed("Test helper not implemented".to_string()))
}

async fn create_test_vector_store() -> ApreResult<VectorStore> {
    Err(ApreError::ExecutionFailed("Test helper not implemented".to_string()))
}

async fn create_test_neo4j_client() -> ApreResult<Neo4jClient> {
    Err(ApreError::ExecutionFailed("Test helper not implemented".to_string()))
}

fn create_test_rag_config() -> RagGraphConfig {
    todo!("Test helper not implemented")
}

fn create_test_language_model() -> Arc<dyn syncore::llm::LanguageModel> {
    todo!("Test helper not implemented")
}

async fn create_test_reasoning_engine() -> ApreResult<ToTEngine> {
    use syncore::graph::{SQLiteGraphBackend, GraphBackend};
    use tempfile::tempdir;

    // Create a temporary SQLite database for testing
    let temp_dir = tempdir()
        .map_err(|e| ApreError::ExecutionFailed(format!("Failed to create temp dir: {}", e)))?;
    let db_path = temp_dir.path().join("test.db");
    let sqlite_backend = SQLiteGraphBackend::connect(
        db_path.to_str().ok_or_else(|| ApreError::ExecutionFailed("Invalid path".to_string()))?,
        "",
        "",
        "test_namespace",
    )
    .await
    .map_err(|e| ApreError::ExecutionFailed(format!("Failed to create SQLite backend: {}", e)))?;

    // Use the new sqlitegraph-only constructor
    ToTEngine::with_sqlitegraph(sqlite_backend)
        .await
        .map_err(|e| ApreError::ExecutionFailed(format!("Failed to create ToT engine: {}", e)))
}

async fn create_failing_test_plan() -> ApreResult<syncore::agent::PlanTree> {
    Err(ApreError::ExecutionFailed("Test helper not implemented".to_string()))
}

async fn create_test_storage_adapter() -> ApreResult<dyn syncore::raggraph::StorageAdapter> {
    Err(ApreError::ExecutionFailed("Test helper not implemented".to_string()))
}

async fn populate_test_memory(memory: &Arc<Memory>) -> ApreResult<()> {
    Err(ApreError::ExecutionFailed("Test helper not implemented".to_string()))
}
