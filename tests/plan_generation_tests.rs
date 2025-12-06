//! PHASE 4 STEP 5.2: Planning Generation Tests - TDD Failing Tests First
//!
//! These tests MUST fail initially. They define the planning-first architecture
//! requirements that will be implemented in subsequent steps.
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

#[cfg(test)]
mod planning_generation_tests {
    use std::sync::Arc;
    use syncore::agent::planner::PlanNodeStatus;
    use syncore::agent::{ApreResult, PlanNode, PlanTree, PlanningEngine};
    use syncore::memory::Memory;
    use syncore::raggraph::{HopGraphTransformer, RagGraphConfig};
    use syncore::router::SynCoreState;
    use syncore::vector::VectorStore;

    /// Test Case 1: PlanningEngine generates valid initial plan from goal
    #[tokio::test]
    async fn test_planning_engine_generates_valid_initial_plan() {
        // GIVEN: Existing SynCoreState with Memory, VectorStore, HopGraph
        let state = create_test_state().await;

        // WHEN: PlanningEngine generates initial plan from concrete goal
        // Create HopGraph using existing systems
        let hop_graph = HopGraphTransformer::new(RagGraphConfig::default());
        let mut planner =
            PlanningEngine::new(state.memory.clone(), state.code_store.clone(), hop_graph);

        let goal = "Add error recovery to all file operations in src/parser.rs";
        let constraints = vec![
            "Use existing error_recovery module".to_string(),
            "Maintain backward compatibility".to_string(),
        ];

        let plan_result = planner.generate_initial_plan(goal, &constraints).await;

        // THEN: Should generate valid PlanTree with concrete structure
        assert!(plan_result.is_ok(), "Plan generation should succeed: {:?}", plan_result.err());

        let plan = plan_result.unwrap();
        assert!(!plan.nodes.is_empty(), "Plan should contain nodes");

        // First node should be READY
        let first_node = &plan.nodes[0];
        assert_eq!(first_node.status, PlanNodeStatus::Ready);

        // Plan should have root dependency node
        let root_nodes: Vec<_> = plan.nodes.iter().filter(|n| n.dependencies.is_empty()).collect();
        assert!(!root_nodes.is_empty(), "Plan should have root nodes");

        // All nodes should have valid priorities
        for node in &plan.nodes {
            assert!(node.priority > 0, "Node priority should be positive");
            assert!(!node.task.is_empty(), "Node task should not be empty");
        }
    }

    /// Test Case 2: PlanningEngine refuses invalid goals
    #[tokio::test]
    async fn test_planning_engine_refuses_invalid_goals() {
        // GIVEN: Existing SynCoreState
        let state = create_test_state().await;

        // WHEN: PlanningEngine receives invalid goal
        // Create HopGraph using existing systems
        let hop_graph = HopGraphTransformer::new(RagGraphConfig::default());
        let mut planner =
            PlanningEngine::new(state.memory.clone(), state.code_store.clone(), hop_graph);

        let invalid_goals = vec![
            "",               // Empty goal
            " ",              // Whitespace only
            "fix everything", // Too vague
            "make it better", // Non-specific
        ];

        for invalid_goal in invalid_goals {
            let plan_result = planner.generate_initial_plan(invalid_goal, &[]).await;

            // THEN: Should reject invalid goals
            assert!(plan_result.is_err(), "Should reject invalid goal: '{}'", invalid_goal);
        }
    }

    /// Test Case 3: PlanningEngine uses existing memory for context
    #[tokio::test]
    async fn test_planning_engine_uses_existing_memory_context() {
        // GIVEN: Existing Memory with relevant context
        let state = create_test_state().await;

        // Store relevant context in memory
        let memory_key = "parser_error_handling";
        let memory_value = r#"{
            "file": "src/parser.rs",
            "current_error_handling": "Result<T, ParseError>",
            "needs_recovery": true,
            "functions_requiring_update": ["parse_function", "parse_struct"]
        }"#;

        state.memory.store(memory_key, memory_value).expect("Memory store should succeed");

        // WHEN: PlanningEngine generates plan for related goal
        // Create HopGraph using existing systems
        let hop_graph = HopGraphTransformer::new(RagGraphConfig::default());
        let mut planner =
            PlanningEngine::new(state.memory.clone(), state.code_store.clone(), hop_graph);

        let goal = "Update error handling in parser.rs with recovery patterns";
        let plan_result = planner.generate_initial_plan(goal, &[]).await;

        // THEN: Plan should reflect memory context
        assert!(plan_result.is_ok(), "Plan generation should succeed with memory context");

        let plan = plan_result.unwrap();

        // Should contain nodes related to parser functions
        let parser_nodes: Vec<_> = plan
            .nodes
            .iter()
            .filter(|n| n.task.contains("parser") || n.task.contains("parse_function"))
            .collect();
        assert!(!parser_nodes.is_empty(), "Plan should reference parser context from memory");
    }

    /// Test Case 4: PlanningEngine respects constraints from existing codebase
    #[tokio::test]
    async fn test_planning_engine_respects_existing_constraints() {
        // GIVEN: Existing codebase with established patterns
        let state = create_test_state().await;

        // Store constraint information
        state
            .memory
            .store("error_handling_pattern", "use anyhow::Result for all operations")
            .expect("Memory store should succeed");

        // WHEN: PlanningEngine generates plan with constraints
        // Create HopGraph using existing systems
        let hop_graph = HopGraphTransformer::new(RagGraphConfig::default());
        let mut planner =
            PlanningEngine::new(state.memory.clone(), state.code_store.clone(), hop_graph);

        let goal = "Add error handling to file operations";
        let constraints = vec![
            "Use existing anyhow::Result pattern".to_string(),
            "Follow existing error propagation".to_string(),
            "Maintain compatibility with current API".to_string(),
        ];

        let plan_result = planner.generate_initial_plan(goal, &constraints).await;

        // THEN: Plan should respect constraints
        assert!(plan_result.is_ok(), "Plan generation should succeed");

        let plan = plan_result.unwrap();

        // Plan nodes should reference constraint compliance
        let constraint_compliant_nodes: Vec<_> = plan
            .nodes
            .iter()
            .filter(|n| {
                n.task.contains("anyhow")
                    || n.task.contains("existing")
                    || n.task.contains("compatible")
            })
            .collect();

        assert!(
            !constraint_compliant_nodes.is_empty(),
            "Plan should contain constraint-compliant actions"
        );
    }

    /// Test Case 5: PlanningEngine integrates with existing VectorStore
    #[tokio::test]
    async fn test_planning_engine_integrates_vector_store() {
        // GIVEN: VectorStore with indexed code context
        let state = create_test_state().await;

        // Insert relevant code context into VectorStore
        let code_context = "Error handling patterns in Rust:
use anyhow::{Result, Context};
fn safe_operation() -> Result<String> {
    // Implementation with proper error handling
}";

        state
            .code_store
            .lock()
            .unwrap()
            .insert_text(1, Some(1), code_context, "error_handling_examples")
            .expect("Vector store insert should succeed");

        // WHEN: PlanningEngine generates related plan
        // Create HopGraph using existing systems
        let hop_graph = HopGraphTransformer::new(RagGraphConfig::default());
        let mut planner =
            PlanningEngine::new(state.memory.clone(), state.code_store.clone(), hop_graph);

        let goal = "Implement error handling for file operations";
        let plan_result = planner.generate_initial_plan(goal, &[]).await;

        // THEN: Plan should leverage vector store context
        assert!(plan_result.is_ok(), "Plan generation should succeed with vector store context");

        let plan = plan_result.unwrap();

        // Should reference patterns found in vector store
        let pattern_nodes: Vec<_> = plan
            .nodes
            .iter()
            .filter(|n| {
                n.task.contains("anyhow") || n.task.contains("Context") || n.task.contains("Result")
            })
            .collect();

        assert!(!pattern_nodes.is_empty(), "Plan should leverage patterns from vector store");
    }

    /// Test Case 6: PlanningEngine creates executable action items
    #[tokio::test]
    async fn test_planning_engine_creates_executable_actions() {
        // GIVEN: Existing SynCoreState
        let state = create_test_state().await;

        // WHEN: PlanningEngine generates plan
        // Create HopGraph using existing systems
        let hop_graph = HopGraphTransformer::new(RagGraphConfig::default());
        let mut planner =
            PlanningEngine::new(state.memory.clone(), state.code_store.clone(), hop_graph);

        let goal = "Add unit tests for error handling in parser.rs";
        let plan_result = planner.generate_initial_plan(goal, &[]).await;

        // THEN: Plan nodes should be executable actions
        assert!(plan_result.is_ok(), "Plan generation should succeed");

        let plan = plan_result.unwrap();

        for node in &plan.nodes {
            // Each node should represent a concrete, executable action
            assert!(!node.task.is_empty(), "Task should not be empty");
            assert!(node.task.len() > 10, "Task should be descriptive");

            // Task should start with action verb
            let action_verbs = ["add", "update", "create", "implement", "modify", "refactor"];
            let starts_with_action =
                action_verbs.iter().any(|verb| node.task.to_lowercase().starts_with(verb));
            assert!(starts_with_action, "Task should start with action verb: '{}'", node.task);
        }
    }

    /// Test Case 7: PlanningEngine handles dependency chains correctly
    #[tokio::test]
    async fn test_planning_engine_handles_dependency_chains() {
        // GIVEN: Complex goal requiring multiple steps
        let state = create_test_state().await;

        // WHEN: PlanningEngine generates plan for complex goal
        // Create HopGraph using existing systems
        let hop_graph = HopGraphTransformer::new(RagGraphConfig::default());
        let mut planner =
            PlanningEngine::new(state.memory.clone(), state.code_store.clone(), hop_graph);

        let goal = "Refactor parser.rs to use async error handling with proper propagation";
        let plan_result = planner.generate_initial_plan(goal, &[]).await;

        // THEN: Plan should have logical dependency chains
        assert!(plan_result.is_ok(), "Plan generation should succeed");

        let plan = plan_result.unwrap();

        // Should have multiple nodes with dependencies
        assert!(plan.nodes.len() > 1, "Complex goal should require multiple nodes");

        // Dependencies should be valid (refer to existing nodes)
        for node in &plan.nodes {
            for dep_id in &node.dependencies {
                let dep_exists = plan.nodes.iter().any(|n| n.id == *dep_id);
                assert!(dep_exists, "Dependency {} should exist in plan", dep_id);
            }
        }

        // Should have at least one root node (no dependencies)
        let root_nodes: Vec<_> = plan.nodes.iter().filter(|n| n.dependencies.is_empty()).collect();
        assert!(!root_nodes.is_empty(), "Plan should have root nodes");
    }

    /// Test Case 8: PlanningEngine plan is deterministic
    #[tokio::test]
    async fn test_planning_engine_plan_is_deterministic() {
        // GIVEN: Identical state and inputs
        let state = create_test_state().await;

        // WHEN: PlanningEngine generates plan twice with same inputs
        let goal = "Add input validation to public API functions";
        let constraints = vec!["Use existing validation patterns".to_string()];

        // Create HopGraph using existing systems
        let hop_graph = HopGraphTransformer::new(RagGraphConfig::default());

        let mut planner1 =
            PlanningEngine::new(state.memory.clone(), state.code_store.clone(), hop_graph.clone());

        let hop_graph2 = HopGraphTransformer::new(RagGraphConfig::default());
        let mut planner2 =
            PlanningEngine::new(state.memory.clone(), state.code_store.clone(), hop_graph2);

        let plan1_result = planner1.generate_initial_plan(goal, &constraints).await;
        let plan2_result = planner2.generate_initial_plan(goal, &constraints).await;

        // THEN: Plans should be identical
        assert!(plan1_result.is_ok(), "First plan generation should succeed");
        assert!(plan2_result.is_ok(), "Second plan generation should succeed");

        let plan1 = plan1_result.unwrap();
        let plan2 = plan2_result.unwrap();

        assert_eq!(plan1.nodes.len(), plan2.nodes.len(), "Plans should have same number of nodes");

        // Node ordering and content should be identical
        for (i, (node1, node2)) in plan1.nodes.iter().zip(plan2.nodes.iter()).enumerate() {
            assert_eq!(node1.task, node2.task, "Node {} tasks should be identical", i);
            assert_eq!(node1.priority, node2.priority, "Node {} priorities should be identical", i);
            assert_eq!(
                node1.dependencies, node2.dependencies,
                "Node {} dependencies should be identical",
                i
            );
        }
    }

    // Helper function to create test state using existing systems only
    async fn create_test_state() -> SynCoreState {
        use std::sync::{Arc, Mutex};
        use syncore::memory::Memory;
        use syncore::raggraph::{HopGraphTransformer, RagGraphConfig};
        use syncore::vector::{RealEmbeddings, VectorStore};

        // Create memory using existing Memory API with test database path
        let memory = Arc::new(Memory::new(":memory:").expect("Memory creation should succeed"));

        // Create vector store using existing VectorStore API with RealEmbeddings
        let embeddings =
            Box::new(RealEmbeddings::new(384).expect("RealEmbeddings creation should succeed"));
        let code_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        // Create HopGraph using existing HopGraphTransformer API
        let hop_graph = HopGraphTransformer::new(RagGraphConfig::default());

        // Build SynCoreState using existing constructor pattern
        let state = syncore::router::SynCoreState::with_dual_stores(
            code_store.clone(),
            code_store.clone(), // Use same store for both code and general
        )
        .expect("SynCoreState creation should succeed");

        state
    }
}
