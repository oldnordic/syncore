//! TDD Tests for TOON Pointer Resolution

use std::sync::{Arc, Mutex};
use syncore::memory_service::{
    MemoryEntry, MemoryService, ToonExecutor, ToonGraph, ToonInstr, ToonNode, ToonResult,
};

fn create_test_memory_service() -> Arc<Mutex<MemoryService>> {
    Arc::new(Mutex::new(MemoryService::new(128, 10)))
}

#[test]
fn test_pointer_resolution_valid() {
    // Test that valid pointers resolve correctly
    let memory = create_test_memory_service();

    // Build graph that retrieves entries (populates pointer store)
    let mut graph = ToonGraph::new("retrieve".to_string());
    graph.add_node(ToonNode {
        id: "retrieve".to_string(),
        instr: ToonInstr::Retrieve {
            query: "test".to_string(),
            k: 2,
        },
        next: vec![],
    });

    // Pre-populate memory
    {
        let mut mem_lock = memory.lock().unwrap();
        mem_lock
            .store(MemoryEntry {
                id: "entry1".to_string(),
                summary: "Test entry 1".to_string(),
                importance: 0.5,
                tags: vec![],
                embedding: vec![0.5; 128],
            })
            .unwrap();
    }

    let mut executor = ToonExecutor::new(graph, Arc::clone(&memory));
    executor.execute().unwrap();

    // Now resolve pointer
    let resolved = executor.resolve_pointer("entry1");
    assert!(resolved.is_some(), "Should resolve valid pointer");
    assert_eq!(resolved.unwrap().id, "entry1");
}

#[test]
fn test_pointer_resolution_invalid() {
    // Test that invalid pointers return None
    let memory = create_test_memory_service();

    let mut graph = ToonGraph::new("start".to_string());
    graph.add_node(ToonNode {
        id: "start".to_string(),
        instr: ToonInstr::NoOp,
        next: vec![],
    });

    let executor = ToonExecutor::new(graph, Arc::clone(&memory));

    let resolved = executor.resolve_pointer("nonexistent");
    assert!(resolved.is_none(), "Should return None for invalid pointer");
}

#[test]
fn test_pointer_resolution_does_not_mutate_memory() {
    // Test that resolving pointers doesn't modify memory service
    let memory = create_test_memory_service();

    // Get initial memory stats
    let initial_size = {
        let mem_lock = memory.lock().unwrap();
        mem_lock.stats().ram_size
    };

    // Build and execute graph
    let mut graph = ToonGraph::new("retrieve".to_string());
    graph.add_node(ToonNode {
        id: "retrieve".to_string(),
        instr: ToonInstr::Retrieve {
            query: "test".to_string(),
            k: 1,
        },
        next: vec![],
    });

    let mut executor = ToonExecutor::new(graph, Arc::clone(&memory));
    executor.execute().unwrap();

    // Resolve pointers
    executor.resolve_pointer("entry1");
    executor.resolve_pointer("entry2");
    executor.resolve_pointer("nonexistent");

    // Memory size should not have changed from pointer resolution
    let final_size = {
        let mem_lock = memory.lock().unwrap();
        mem_lock.stats().ram_size
    };

    assert_eq!(initial_size, final_size, "Pointer resolution should not mutate memory");
}

#[test]
fn test_pointer_store_populated_by_retrieve() {
    // Test that Retrieve instruction populates pointer store
    let memory = create_test_memory_service();

    // Pre-populate memory with entries
    {
        let mut mem_lock = memory.lock().unwrap();
        for i in 0..3 {
            mem_lock
                .store(MemoryEntry {
                    id: format!("entry{}", i),
                    summary: format!("Entry {}", i),
                    importance: 0.5,
                    tags: vec![],
                    embedding: vec![i as f32 / 10.0; 128],
                })
                .unwrap();
        }
    }

    let mut graph = ToonGraph::new("retrieve".to_string());
    graph.add_node(ToonNode {
        id: "retrieve".to_string(),
        instr: ToonInstr::Retrieve {
            query: "test query".to_string(),
            k: 3,
        },
        next: vec![],
    });

    let mut executor = ToonExecutor::new(graph, Arc::clone(&memory));
    let results = executor.execute().unwrap();

    // Get retrieved IDs
    let retrieved_ids: Vec<String> = if let ToonResult::Retrieved(entries) = &results[0].result {
        entries.iter().map(|e| e.id.clone()).collect()
    } else {
        vec![]
    };

    // All retrieved entries should be resolvable via pointers
    for id in retrieved_ids {
        let resolved = executor.resolve_pointer(&id);
        assert!(resolved.is_some(), "Retrieved entry {} should be resolvable", id);
    }
}

#[test]
fn test_pointer_store_populated_by_foldcontext() {
    // Test that FoldContext adds folded entry to pointer store
    let memory = create_test_memory_service();

    // Pre-populate memory
    {
        let mut mem_lock = memory.lock().unwrap();
        mem_lock
            .store(MemoryEntry {
                id: "ctx1".to_string(),
                summary: "Context 1".to_string(),
                importance: 0.5,
                tags: vec![],
                embedding: vec![0.1; 128],
            })
            .unwrap();
        mem_lock
            .store(MemoryEntry {
                id: "ctx2".to_string(),
                summary: "Context 2".to_string(),
                importance: 0.5,
                tags: vec![],
                embedding: vec![0.2; 128],
            })
            .unwrap();
    }

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
            context_ids: vec!["ctx1".to_string(), "ctx2".to_string()],
        },
        next: vec![],
    });

    let mut executor = ToonExecutor::new(graph, Arc::clone(&memory));
    let results = executor.execute().unwrap();

    // Get folded ID
    let folded_id = if let Some(result) =
        results.iter().find(|r| matches!(r.result, ToonResult::Folded { .. }))
    {
        if let ToonResult::Folded {
            new_id,
        } = &result.result
        {
            new_id.clone()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Folded entry should be resolvable
    let resolved = executor.resolve_pointer(&folded_id);
    assert!(resolved.is_some(), "Folded entry should be resolvable");
    assert_eq!(resolved.unwrap().id, folded_id);
}

#[test]
fn test_loadmemory_requires_pointer_in_store() {
    // Test that LoadMemory fails gracefully for pointers not in store
    let memory = create_test_memory_service();

    let mut graph = ToonGraph::new("load".to_string());
    graph.add_node(ToonNode {
        id: "load".to_string(),
        instr: ToonInstr::LoadMemory {
            id: "missing_entry".to_string(),
        },
        next: vec![],
    });

    let mut executor = ToonExecutor::new(graph, Arc::clone(&memory));
    let result = executor.execute();

    // Should fail because pointer not in store
    assert!(result.is_err(), "LoadMemory should fail for missing pointer");
}
