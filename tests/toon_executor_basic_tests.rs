//! TDD Tests for TOON Executor Basic Operations

use std::sync::{Arc, Mutex};
use syncore::memory_service::{
    MemoryEntry, MemoryService, ToonExecutor, ToonGraph, ToonInstr, ToonNode, ToonResult,
};

fn create_test_memory_service() -> Arc<Mutex<MemoryService>> {
    Arc::new(Mutex::new(MemoryService::new(128, 10)))
}

#[test]
fn test_executor_loadmemory_returns_correct_entry() {
    // Test that LoadMemory instruction loads entry from pointer store
    let memory = create_test_memory_service();

    // Build simple graph: Retrieve -> LoadMemory
    let mut graph = ToonGraph::new("retrieve".to_string());

    graph.add_node(ToonNode {
        id: "retrieve".to_string(),
        instr: ToonInstr::Retrieve {
            query: "test data".to_string(),
            k: 1,
        },
        next: vec!["load".to_string()],
    });

    graph.add_node(ToonNode {
        id: "load".to_string(),
        instr: ToonInstr::LoadMemory {
            id: "entry1".to_string(),
        },
        next: vec![],
    });

    // Pre-populate memory with test entry
    {
        let mut mem_lock = memory.lock().unwrap();
        let entry = MemoryEntry {
            id: "entry1".to_string(),
            summary: "Test entry".to_string(),
            importance: 0.5,
            tags: vec![],
            embedding: vec![0.5; 128],
        };
        mem_lock.store(entry).unwrap();
    }

    let mut executor = ToonExecutor::new(graph, Arc::clone(&memory));
    let results = executor.execute().unwrap();

    // First result should be Retrieved
    assert!(matches!(&results[0].result, ToonResult::Retrieved(_)));

    // Second result should be Loaded with correct entry
    if let ToonResult::Loaded(entry) = &results[1].result {
        assert_eq!(entry.id, "entry1");
    } else {
        panic!("Expected Loaded result");
    }
}

#[test]
fn test_executor_retrieve_executes_ram_then_ltm() {
    // Test that Retrieve instruction searches memory
    let memory = create_test_memory_service();

    // Store test entries
    {
        let mut mem_lock = memory.lock().unwrap();
        for i in 0..3 {
            let entry = MemoryEntry {
                id: format!("entry{}", i),
                summary: format!("Entry {}", i),
                importance: 0.5,
                tags: vec![],
                embedding: vec![i as f32 / 10.0; 128],
            };
            mem_lock.store(entry).unwrap();
        }
    }

    let mut graph = ToonGraph::new("retrieve".to_string());
    graph.add_node(ToonNode {
        id: "retrieve".to_string(),
        instr: ToonInstr::Retrieve {
            query: "test query".to_string(),
            k: 2,
        },
        next: vec![],
    });

    let mut executor = ToonExecutor::new(graph, Arc::clone(&memory));
    let results = executor.execute().unwrap();

    assert_eq!(results.len(), 1);
    if let ToonResult::Retrieved(entries) = &results[0].result {
        assert!(entries.len() > 0, "Should retrieve entries");
    } else {
        panic!("Expected Retrieved result");
    }
}

#[test]
fn test_executor_foldcontext_creates_new_ltm_summary() {
    // Test that FoldContext merges entries and creates new one
    let memory = create_test_memory_service();

    // Build graph: Retrieve -> FoldContext
    let mut graph = ToonGraph::new("retrieve".to_string());

    graph.add_node(ToonNode {
        id: "retrieve".to_string(),
        instr: ToonInstr::Retrieve {
            query: "context".to_string(),
            k: 2,
        },
        next: vec!["fold".to_string()],
    });

    graph.add_node(ToonNode {
        id: "fold".to_string(),
        instr: ToonInstr::FoldContext {
            context_ids: vec!["entry1".to_string(), "entry2".to_string()],
        },
        next: vec![],
    });

    // Pre-populate memory
    {
        let mut mem_lock = memory.lock().unwrap();
        mem_lock
            .store(MemoryEntry {
                id: "entry1".to_string(),
                summary: "First context".to_string(),
                importance: 0.5,
                tags: vec!["tag1".to_string()],
                embedding: vec![0.1; 128],
            })
            .unwrap();
        mem_lock
            .store(MemoryEntry {
                id: "entry2".to_string(),
                summary: "Second context".to_string(),
                importance: 0.5,
                tags: vec!["tag2".to_string()],
                embedding: vec![0.2; 128],
            })
            .unwrap();
    }

    let mut executor = ToonExecutor::new(graph, Arc::clone(&memory));
    let results = executor.execute().unwrap();

    // Should have Folded result
    let folded = results.iter().find(|r| matches!(r.result, ToonResult::Folded { .. }));
    assert!(folded.is_some(), "Should have Folded result");

    if let ToonResult::Folded {
        new_id,
    } = &folded.unwrap().result
    {
        assert!(new_id.starts_with("FOLD_"), "Folded ID should start with FOLD_");
    }
}

#[test]
fn test_executor_emitpointer_emits_expected_id() {
    // Test that EmitPointer emits the specified ID
    let memory = create_test_memory_service();

    let mut graph = ToonGraph::new("emit".to_string());
    graph.add_node(ToonNode {
        id: "emit".to_string(),
        instr: ToonInstr::EmitPointer {
            id: "P123".to_string(),
        },
        next: vec![],
    });

    let mut executor = ToonExecutor::new(graph, Arc::clone(&memory));
    let results = executor.execute().unwrap();

    assert_eq!(results.len(), 1);
    if let ToonResult::Pointer(id) = &results[0].result {
        assert_eq!(id, "P123");
    } else {
        panic!("Expected Pointer result");
    }
}

#[test]
fn test_executor_detects_execution_loops() {
    // Test that executor detects and prevents infinite loops
    let memory = create_test_memory_service();

    // Build cyclic graph: A -> B -> A
    let mut graph = ToonGraph::new("nodeA".to_string());

    graph.add_node(ToonNode {
        id: "nodeA".to_string(),
        instr: ToonInstr::NoOp,
        next: vec!["nodeB".to_string()],
    });

    graph.add_node(ToonNode {
        id: "nodeB".to_string(),
        instr: ToonInstr::NoOp,
        next: vec!["nodeA".to_string()], // Loop back
    });

    let mut executor = ToonExecutor::new(graph, Arc::clone(&memory));
    let result = executor.execute();

    assert!(result.is_err(), "Should detect execution loop");
}

#[test]
fn test_executor_returns_sequence_of_results() {
    // Test that executor returns ordered sequence of step results
    let memory = create_test_memory_service();

    // Build linear graph: A -> B -> C
    let mut graph = ToonGraph::new("A".to_string());

    graph.add_node(ToonNode {
        id: "A".to_string(),
        instr: ToonInstr::EmitPointer {
            id: "PA".to_string(),
        },
        next: vec!["B".to_string()],
    });

    graph.add_node(ToonNode {
        id: "B".to_string(),
        instr: ToonInstr::EmitPointer {
            id: "PB".to_string(),
        },
        next: vec!["C".to_string()],
    });

    graph.add_node(ToonNode {
        id: "C".to_string(),
        instr: ToonInstr::EmitPointer {
            id: "PC".to_string(),
        },
        next: vec![],
    });

    let mut executor = ToonExecutor::new(graph, Arc::clone(&memory));
    let results = executor.execute().unwrap();

    assert_eq!(results.len(), 3, "Should have 3 step results");
    assert_eq!(results[0].node_id, "A");
    assert_eq!(results[1].node_id, "B");
    assert_eq!(results[2].node_id, "C");

    // Check pointer values
    if let ToonResult::Pointer(id) = &results[0].result {
        assert_eq!(id, "PA");
    }
    if let ToonResult::Pointer(id) = &results[1].result {
        assert_eq!(id, "PB");
    }
    if let ToonResult::Pointer(id) = &results[2].result {
        assert_eq!(id, "PC");
    }
}

#[test]
fn test_executor_noop_completes() {
    // Test that NoOp instruction completes successfully
    let memory = create_test_memory_service();

    let mut graph = ToonGraph::new("noop".to_string());
    graph.add_node(ToonNode {
        id: "noop".to_string(),
        instr: ToonInstr::NoOp,
        next: vec![],
    });

    let mut executor = ToonExecutor::new(graph, Arc::clone(&memory));
    let results = executor.execute().unwrap();

    assert_eq!(results.len(), 1);
    assert!(matches!(results[0].result, ToonResult::Completed));
}
