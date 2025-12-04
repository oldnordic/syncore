//! S9-DE-STUB: Tests to verify removal of synthetic logic from cognition + reasoning + executor pipeline
//! These tests should FAIL with current stub behavior and PASS after de-stubbing.

use syncore::cognition::orchestrator::CognitionOrchestrator;
use syncore::macro_tools::executor_real::RealExecutor;
use syncore::reasoning::engine::ReasoningEngine;
use syncore::reasoning::node::ReasoningNode;
use syncore::llm::{LanguageModel, Prompt};
use syncore::models::gguf_engine::GGUFEngine;
use syncore::state::SynCoreState;
use anyhow::Result;

#[tokio::test]
async fn test_orchestrator_returns_real_data() {
    // GIVEN: A CognitionOrchestrator with real state
    let state = SynCoreState::new_test().await;
    let orchestrator = CognitionOrchestrator::new(&state);

    // WHEN: Querying the orchestrator
    let query = "test query for real data";
    let result = orchestrator.query(query).await;

    // THEN: Should not return placeholder JSON
    assert!(result.is_ok(), "Orchestrator query should succeed");
    let response_text = result.unwrap();

    // ASSERT: Response should NOT contain placeholder indicators
    assert!(!response_text.contains("placeholder"),
            "Response should not contain 'placeholder': {}", response_text);
    assert!(!response_text.contains("Placeholder"),
            "Response should not contain 'Placeholder': {}", response_text);
    assert!(!response_text.contains("debug_info"),
            "Response should not contain debug placeholder: {}", response_text);

    // ASSERT: Response should contain real data indicators
    assert!(response_text.contains("memory") || response_text.contains("graph") || response_text.contains("reasoning"),
            "Response should contain real data fields: {}", response_text);
}

#[test]
fn test_executor_real_no_stub_paths() {
    // GIVEN: Read the executor_real.rs source
    let executor_source = include_str!("../src/macro_tools/executor_real.rs");

    // THEN: Should contain NO stub markers
    assert!(!executor_source.contains("synthetic"),
            "executor_real.rs should not contain 'synthetic'");
    assert!(!executor_source.contains("SYNTHETIC"),
            "executor_real.rs should not contain 'SYNTHETIC'");
    assert!(!executor_source.contains("TEMPORARY"),
            "executor_real.rs should not contain 'TEMPORARY'");
    assert!(!executor_source.contains("stub result"),
            "executor_real.rs should not contain 'stub result'");
    assert!(!executor_source.contains("executor_stub"),
            "executor_real.rs should not contain 'executor_stub'");
}

#[tokio::test]
async fn test_reasoning_engine_node_expansion_real() {
    // GIVEN: A ReasoningEngine with real LLM
    let llm = Box::new(GGUFEngine::new_test());
    let mut engine = ReasoningEngine::new(llm);

    // WHEN: Expanding a node
    let node = ReasoningNode {
        id: "test_node_1".to_string(),
        content: "test node content for expansion".to_string(),
        node_type: "question".to_string(),
        confidence: 0.5,
        children: vec![],
        metadata: serde_json::json!({}),
    };

    let result = engine.expand_node(&node).await;

    // THEN: Should succeed and not use stub expansion
    assert!(result.is_ok(), "Node expansion should succeed");
    let expanded_node = result.unwrap();

    // ASSERT: Should NOT contain stub indicators
    let node_json = serde_json::to_string(&expanded_node).unwrap();
    assert!(!node_json.contains("stub"),
            "Expanded node should not contain 'stub': {}", node_json);
    assert!(!node_json.contains("deterministic fallback"),
            "Expanded node should not contain 'deterministic fallback': {}", node_json);

    // ASSERT: Should have real expansion (children added)
    assert!(!expanded_node.children.is_empty(),
            "Expanded node should have children from real LLM expansion");
}

#[test]
fn test_node_prompt_builder_no_stub() {
    // GIVEN: Create a ReasoningNode with test data
    let node = ReasoningNode {
        id: "test_node_1".to_string(),
        content: "test node content for prompt building".to_string(),
        node_type: "question".to_string(),
        confidence: 0.5,
        children: vec![],
        metadata: serde_json::json!({}),
    };

    // WHEN: Building prompt context
    let prompt_context = node.prepare_llm_prompt_context();

    // THEN: Should NOT contain stub/placeholder text
    assert!(!prompt_context.contains("stub"),
            "Prompt context should not contain 'stub': {}", prompt_context);
    assert!(!prompt_context.contains("placeholder"),
            "Prompt context should not contain 'placeholder': {}", prompt_context);
    assert!(!prompt_context.contains("ST-3"),
            "Prompt context should not contain 'ST-3': {}", prompt_context);

    // ASSERT: Should contain actual node content
    assert!(prompt_context.contains("test node content"),
            "Prompt context should contain actual node content: {}", prompt_context);
}

#[tokio::test]
async fn test_node_evaluation_no_stub() {
    // GIVEN: A ReasoningNode and real LLM
    let llm = Box::new(GGUFEngine::new_test());
    let node = ReasoningNode {
        id: "test_node_1".to_string(),
        content: "test node content for evaluation".to_string(),
        node_type: "answer".to_string(),
        confidence: 0.5,
        children: vec![],
        metadata: serde_json::json!({}),
    };

    // WHEN: Evaluating node quality
    let result = node.evaluate_quality(&*llm).await;

    // THEN: Should succeed and not use stub evaluation
    assert!(result.is_ok(), "Node evaluation should succeed");
    let quality_score = result.unwrap();

    // ASSERT: Should be a real score (not stub value)
    assert!(quality_score > 0.0 && quality_score <= 1.0,
            "Quality score should be in valid range: {}", quality_score);
}

#[tokio::test]
async fn test_intellitask_and_sequential_tools_unchanged() {
    // GIVEN: A SynCoreState with all tool suites
    let state = SynCoreState::new_test().await;

    // WHEN: Accessing tool suites
    let memory_suite = &state.memory_suite;
    let intellitask = &state.intellitask_engine;
    let sequential = &state.sequential_engine;

    // THEN: All tool suites should be accessible
    assert!(memory_suite.is_some(), "Memory suite should be available");
    assert!(intellitask.is_some(), "IntelliTask engine should be available");
    assert!(sequential.is_some(), "Sequential engine should be available");

    // WHEN: Testing basic tool functionality
    if let Some(ref memory) = memory_suite {
        let result = memory.store("test_key", "test_value").await;
        assert!(result.is_ok(), "Memory store should work");
    }

    // Note: We can't test IntelliTask and Sequential tools without a real LLM backend
    // but we verify they're properly initialized and not stubbed
}