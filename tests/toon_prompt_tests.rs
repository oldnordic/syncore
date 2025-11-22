//! TDD Tests for TOON Prompt Builder

use std::collections::HashMap;
use syncore::memory_service::{MemoryEntry, ToonGraph, ToonInstr, ToonNode, ToonPromptBuilder};

#[test]
fn test_prompt_includes_graph() {
    // Test that prompt includes serialized graph structure
    let builder = ToonPromptBuilder::new(10000);

    let mut graph = ToonGraph::new("start".to_string());
    graph.add_node(ToonNode {
        id: "start".to_string(),
        instr: ToonInstr::Retrieve {
            query: "test query".to_string(),
            k: 5,
        },
        next: vec!["next_node".to_string()],
    });

    let prompt = builder.build_prompt(&graph, &[], &HashMap::new());

    // Should include entry point
    assert!(
        prompt.contains("start"),
        "Prompt should include entry node ID"
    );
    // Should include instruction type
    assert!(
        prompt.contains("Retrieve") || prompt.contains("retrieve"),
        "Prompt should reference Retrieve instruction"
    );
}

#[test]
fn test_prompt_includes_memory_entries() {
    // Test that prompt includes memory entries
    let builder = ToonPromptBuilder::new(10000);

    let graph = ToonGraph::new("start".to_string());

    let memory = vec![MemoryEntry {
        id: "mem1".to_string(),
        summary: "Test memory entry".to_string(),
        importance: 0.8,
        tags: vec!["tag1".to_string()],
        embedding: vec![0.5; 128],
    }];

    let prompt = builder.build_prompt(&graph, &memory, &HashMap::new());

    assert!(
        prompt.contains("mem1"),
        "Prompt should include memory entry ID"
    );
    assert!(
        prompt.contains("Test memory entry"),
        "Prompt should include summary"
    );
}

#[test]
fn test_prompt_includes_pointer_store() {
    // Test that prompt includes pointer store contents
    let builder = ToonPromptBuilder::new(10000);

    let graph = ToonGraph::new("start".to_string());

    let mut pointer_store = HashMap::new();
    pointer_store.insert(
        "ptr1".to_string(),
        MemoryEntry {
            id: "ptr1".to_string(),
            summary: "Pointer entry".to_string(),
            importance: 0.5,
            tags: vec![],
            embedding: vec![0.3; 128],
        },
    );

    let prompt = builder.build_prompt(&graph, &[], &pointer_store);

    assert!(prompt.contains("ptr1"), "Prompt should include pointer ID");
    assert!(
        prompt.contains("Pointer entry"),
        "Prompt should include pointer summary"
    );
}

#[test]
fn test_prompt_obeys_max_context_tokens() {
    // Test that prompt respects max_context_tokens limit
    let builder = ToonPromptBuilder::new(100); // Very small limit

    let graph = ToonGraph::new("start".to_string());

    // Create many memory entries
    let memory: Vec<MemoryEntry> = (0..100)
        .map(|i| MemoryEntry {
            id: format!("entry{}", i),
            summary: format!(
                "This is a very long summary for entry {} with lots of text",
                i
            ),
            importance: 0.5,
            tags: vec![],
            embedding: vec![0.1; 128],
        })
        .collect();

    let prompt = builder.build_prompt(&graph, &memory, &HashMap::new());

    // Prompt should be bounded (not include all 100 entries)
    let entry_count = prompt.matches("entry").count();
    assert!(
        entry_count < 100,
        "Should limit number of entries due to token limit"
    );
}

#[test]
fn test_prompt_is_deterministic() {
    // Test that multiple calls produce identical prompts
    let builder = ToonPromptBuilder::new(5000);

    let mut graph = ToonGraph::new("node1".to_string());
    graph.add_node(ToonNode {
        id: "node1".to_string(),
        instr: ToonInstr::EmitPointer {
            id: "P123".to_string(),
        },
        next: vec![],
    });

    let memory = vec![
        MemoryEntry {
            id: "m1".to_string(),
            summary: "Entry 1".to_string(),
            importance: 0.5,
            tags: vec![],
            embedding: vec![0.1; 128],
        },
        MemoryEntry {
            id: "m2".to_string(),
            summary: "Entry 2".to_string(),
            importance: 0.5,
            tags: vec![],
            embedding: vec![0.2; 128],
        },
    ];

    let mut pointer_store = HashMap::new();
    pointer_store.insert(
        "p1".to_string(),
        MemoryEntry {
            id: "p1".to_string(),
            summary: "Pointer".to_string(),
            importance: 0.5,
            tags: vec![],
            embedding: vec![0.3; 128],
        },
    );

    let prompt1 = builder.build_prompt(&graph, &memory, &pointer_store);
    let prompt2 = builder.build_prompt(&graph, &memory, &pointer_store);
    let prompt3 = builder.build_prompt(&graph, &memory, &pointer_store);

    assert_eq!(prompt1, prompt2, "Prompts should be identical");
    assert_eq!(prompt2, prompt3, "Prompts should be identical");
}

#[test]
fn test_prompt_has_static_header() {
    // Test that prompt includes a static header section
    let builder = ToonPromptBuilder::new(10000);

    let graph = ToonGraph::new("start".to_string());
    let prompt = builder.build_prompt(&graph, &[], &HashMap::new());

    // Should have some kind of header/instructions
    assert!(!prompt.is_empty(), "Prompt should not be empty");
    assert!(prompt.len() > 50, "Prompt should have substantial content");
}

#[test]
fn test_prompt_handles_empty_inputs() {
    // Test that prompt handles empty memory and pointer store gracefully
    let builder = ToonPromptBuilder::new(10000);

    let graph = ToonGraph::new("empty".to_string());
    let prompt = builder.build_prompt(&graph, &[], &HashMap::new());

    assert!(
        !prompt.is_empty(),
        "Should produce valid prompt even with empty inputs"
    );
    assert!(prompt.contains("empty"), "Should still include entry node");
}

#[test]
fn test_prompt_includes_all_instruction_types() {
    // Test that prompt can represent all instruction types
    let builder = ToonPromptBuilder::new(10000);

    let mut graph = ToonGraph::new("n1".to_string());

    // Add nodes with all instruction types
    graph.add_node(ToonNode {
        id: "n1".to_string(),
        instr: ToonInstr::LoadMemory {
            id: "M1".to_string(),
        },
        next: vec!["n2".to_string()],
    });
    graph.add_node(ToonNode {
        id: "n2".to_string(),
        instr: ToonInstr::Retrieve {
            query: "q".to_string(),
            k: 3,
        },
        next: vec!["n3".to_string()],
    });
    graph.add_node(ToonNode {
        id: "n3".to_string(),
        instr: ToonInstr::FoldContext {
            context_ids: vec!["c1".to_string()],
        },
        next: vec!["n4".to_string()],
    });
    graph.add_node(ToonNode {
        id: "n4".to_string(),
        instr: ToonInstr::EmitPointer {
            id: "P1".to_string(),
        },
        next: vec!["n5".to_string()],
    });
    graph.add_node(ToonNode {
        id: "n5".to_string(),
        instr: ToonInstr::NoOp,
        next: vec![],
    });

    let prompt = builder.build_prompt(&graph, &[], &HashMap::new());

    // Check that all instruction types are represented
    assert!(
        prompt.contains("n1")
            && prompt.contains("n2")
            && prompt.contains("n3")
            && prompt.contains("n4")
            && prompt.contains("n5"),
        "Should include all nodes"
    );
}
