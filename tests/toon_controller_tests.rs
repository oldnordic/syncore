//! TDD Tests for TOON Controller

use std::sync::{Arc, Mutex};
use syncore::memory_service::{
    MemoryEntry, MemoryService, ToonController, ToonGraph, ToonInstr, ToonNode, ToonResult,
};

fn create_test_memory_service() -> Arc<Mutex<MemoryService>> {
    Arc::new(Mutex::new(MemoryService::new(128, 10)))
}

#[test]
fn test_controller_builds_prompt() {
    // Test that controller can build LLM prompts
    let memory = create_test_memory_service();

    let mut graph = ToonGraph::new("start".to_string());
    graph.add_node(ToonNode {
        id: "start".to_string(),
        instr: ToonInstr::NoOp,
        next: vec![],
    });

    let controller = ToonController::new(graph, memory, 5000);
    let prompt = controller.build_llm_prompt();

    assert!(!prompt.is_empty(), "Should generate non-empty prompt");
    assert!(prompt.contains("start"), "Should include graph entry");
}

#[test]
fn test_controller_decodes_and_executes_ops() {
    // Test that controller can decode LLM output and execute ops
    let memory = create_test_memory_service();

    // Pre-populate memory
    {
        let mut mem_lock = memory.lock().unwrap();
        mem_lock
            .store(MemoryEntry {
                id: "test_entry".to_string(),
                summary: "Test data".to_string(),
                importance: 0.5,
                tags: vec![],
                embedding: vec![0.5; 128],
            })
            .unwrap();
    }

    let graph = ToonGraph::new("start".to_string());
    let mut controller = ToonController::new(graph, Arc::clone(&memory), 5000);

    let llm_output = r#"{
        "ops": [
            {"type": "retrieve", "query": "test", "k": 1}
        ]
    }"#;

    let result = controller.step_llm(llm_output);
    assert!(result.is_ok(), "Should execute LLM ops successfully");

    let steps = result.unwrap();
    assert!(steps.len() > 0, "Should have execution results");
}

#[test]
fn test_controller_appends_new_nodes() {
    // Test that LLM ops are appended as nodes to the graph
    let memory = create_test_memory_service();

    let graph = ToonGraph::new("start".to_string());
    let mut controller = ToonController::new(graph, memory, 5000);

    let llm_output = r#"{
        "ops": [
            {"type": "emit_pointer", "id": "P1"},
            {"type": "emit_pointer", "id": "P2"}
        ]
    }"#;

    controller.step_llm(llm_output).unwrap();

    // Verify nodes were added (indirectly through successful execution)
    // The controller should have executed the new nodes
}

#[test]
fn test_controller_returns_step_results() {
    // Test that controller returns proper step results
    let memory = create_test_memory_service();

    let graph = ToonGraph::new("start".to_string());
    let mut controller = ToonController::new(graph, memory, 5000);

    let llm_output = r#"{
        "ops": [
            {"type": "emit_pointer", "id": "PTR1"}
        ]
    }"#;

    let results = controller.step_llm(llm_output).unwrap();

    assert_eq!(results.len(), 1);

    // Check result type
    if let ToonResult::Pointer(id) = &results[0].result {
        assert_eq!(id, "PTR1");
    } else {
        panic!("Expected Pointer result");
    }
}

#[test]
fn test_controller_trigger_folding() {
    // Test that controller can trigger automatic folding
    let memory = create_test_memory_service();

    // Pre-populate with many entries to trigger folding
    {
        let mut mem_lock = memory.lock().unwrap();
        for i in 0..15 {
            // Exceeds capacity of 10
            mem_lock
                .store(MemoryEntry {
                    id: format!("entry{}", i),
                    summary: format!("Entry {}", i),
                    importance: 0.5,
                    tags: vec![],
                    embedding: vec![i as f32 / 20.0; 128],
                })
                .unwrap();
        }
    }

    let graph = ToonGraph::new("start".to_string());
    let mut controller = ToonController::new(graph, memory, 5000);

    // Execute some operations to populate pointer store
    let llm_output = r#"{
        "ops": [
            {"type": "retrieve", "query": "data", "k": 5}
        ]
    }"#;

    controller.step_llm(llm_output).unwrap();

    // Attempt folding
    let fold_result = controller.fold_if_required();
    assert!(fold_result.is_ok(), "Folding should not error");

    // If folding occurred, should have new ID
    if let Ok(Some(folded_id)) = fold_result {
        assert!(folded_id.starts_with("FOLD_"), "Folded ID should have FOLD_ prefix");
    }
}

#[test]
fn test_controller_does_not_fold_when_not_needed() {
    // Test that controller doesn't fold when memory is within limits
    let memory = create_test_memory_service();

    // Only add a few entries (well under capacity)
    {
        let mut mem_lock = memory.lock().unwrap();
        for i in 0..3 {
            mem_lock
                .store(MemoryEntry {
                    id: format!("e{}", i),
                    summary: format!("Entry {}", i),
                    importance: 0.5,
                    tags: vec![],
                    embedding: vec![0.1; 128],
                })
                .unwrap();
        }
    }

    let graph = ToonGraph::new("start".to_string());
    let mut controller = ToonController::new(graph, memory, 5000);

    let fold_result = controller.fold_if_required();
    assert!(fold_result.is_ok(), "Should complete without error");

    // Should not fold when not needed
    if let Ok(fold_id) = fold_result {
        assert!(fold_id.is_none(), "Should not fold when memory is under capacity");
    }
}

#[test]
fn test_controller_handles_multiple_step_llm_calls() {
    // Test that controller can handle multiple successive LLM interactions
    let memory = create_test_memory_service();

    let graph = ToonGraph::new("start".to_string());
    let mut controller = ToonController::new(graph, Arc::clone(&memory), 5000);

    // First LLM call
    let output1 = r#"{"ops": [{"type": "emit_pointer", "id": "P1"}]}"#;
    let result1 = controller.step_llm(output1);
    assert!(result1.is_ok(), "First step should succeed");

    // Second LLM call
    let output2 = r#"{"ops": [{"type": "emit_pointer", "id": "P2"}]}"#;
    let result2 = controller.step_llm(output2);
    assert!(result2.is_ok(), "Second step should succeed");

    // Both should produce results
    assert_eq!(result1.unwrap().len(), 1);
    assert_eq!(result2.unwrap().len(), 1);
}

#[test]
fn test_controller_rejects_invalid_llm_output() {
    // Test that controller properly handles malformed LLM output
    let memory = create_test_memory_service();

    let graph = ToonGraph::new("start".to_string());
    let mut controller = ToonController::new(graph, memory, 5000);

    let invalid_output = "This is not JSON";

    let result = controller.step_llm(invalid_output);
    assert!(result.is_err(), "Should reject non-JSON output");
}

#[test]
fn test_controller_prompt_includes_previous_results() {
    // Test that prompt reflects state after execution
    let memory = create_test_memory_service();

    let graph = ToonGraph::new("start".to_string());
    let mut controller = ToonController::new(graph, Arc::clone(&memory), 5000);

    // Get initial prompt
    let prompt1 = controller.build_llm_prompt();

    // Execute an operation
    let llm_output = r#"{"ops": [{"type": "emit_pointer", "id": "PTR"}]}"#;
    controller.step_llm(llm_output).unwrap();

    // Get updated prompt
    let prompt2 = controller.build_llm_prompt();

    // Prompts should differ (second should reflect new state)
    assert_ne!(prompt1, prompt2, "Prompt should update after execution");
}
