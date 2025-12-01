//! TDD Tests for TOON Graph

use syncore::memory_service::toon_engine::{ToonGraph, ToonInstr, ToonNode};

#[test]
fn test_toon_graph_add_and_retrieve_nodes() {
    // Test that we can add nodes and retrieve them
    let mut graph = ToonGraph::new("start".to_string());

    let node1 = ToonNode {
        id: "start".to_string(),
        instr: ToonInstr::Retrieve {
            query: "test query".to_string(),
            k: 5,
        },
        next: vec!["node2".to_string()],
    };

    let node2 = ToonNode {
        id: "node2".to_string(),
        instr: ToonInstr::EmitPointer {
            id: "P123".to_string(),
        },
        next: vec![],
    };

    graph.add_node(node1);
    graph.add_node(node2);

    // Retrieve nodes
    let retrieved1 = graph.get_node("start");
    assert!(retrieved1.is_some(), "Should find start node");
    assert_eq!(retrieved1.unwrap().id, "start");

    let retrieved2 = graph.get_node("node2");
    assert!(retrieved2.is_some(), "Should find node2");
    assert_eq!(retrieved2.unwrap().id, "node2");

    // Check entry point
    assert_eq!(graph.entry(), "start");
}

#[test]
fn test_toon_graph_respects_deterministic_ordering() {
    // Test that next[] maintains deterministic order
    let mut graph = ToonGraph::new("root".to_string());

    let node = ToonNode {
        id: "root".to_string(),
        instr: ToonInstr::NoOp,
        next: vec!["c".to_string(), "a".to_string(), "b".to_string()],
    };

    graph.add_node(node);

    let retrieved = graph.get_node("root").unwrap();

    // Order should be preserved exactly as inserted
    assert_eq!(retrieved.next.len(), 3);
    assert_eq!(retrieved.next[0], "c");
    assert_eq!(retrieved.next[1], "a");
    assert_eq!(retrieved.next[2], "b");
}

#[test]
fn test_toon_graph_missing_node_errors() {
    // Test that querying missing nodes returns None
    let graph = ToonGraph::new("start".to_string());

    let result = graph.get_node("nonexistent");
    assert!(result.is_none(), "Should return None for missing node");
}

#[test]
fn test_toon_graph_can_update_existing_node() {
    // Test that adding a node with same ID updates it
    let mut graph = ToonGraph::new("node1".to_string());

    let node_v1 = ToonNode {
        id: "node1".to_string(),
        instr: ToonInstr::NoOp,
        next: vec![],
    };

    graph.add_node(node_v1);

    let node_v2 = ToonNode {
        id: "node1".to_string(),
        instr: ToonInstr::EmitPointer {
            id: "P456".to_string(),
        },
        next: vec!["next_node".to_string()],
    };

    graph.add_node(node_v2);

    // Should have updated version
    let retrieved = graph.get_node("node1").unwrap();
    match &retrieved.instr {
        ToonInstr::EmitPointer {
            id,
        } => {
            assert_eq!(id, "P456");
        }
        _ => panic!("Expected EmitPointer instruction"),
    }
    assert_eq!(retrieved.next.len(), 1);
}

#[test]
fn test_toon_instr_variants() {
    // Test that all ToonInstr variants can be created
    let instr1 = ToonInstr::LoadMemory {
        id: "M123".to_string(),
    };
    let instr2 = ToonInstr::Retrieve {
        query: "test".to_string(),
        k: 10,
    };
    let instr3 = ToonInstr::FoldContext {
        context_ids: vec!["id1".to_string(), "id2".to_string()],
    };
    let instr4 = ToonInstr::EmitPointer {
        id: "P789".to_string(),
    };
    let instr5 = ToonInstr::NoOp;

    // Just verify they compile and can be pattern matched
    match instr1 {
        ToonInstr::LoadMemory {
            ..
        } => {}
        _ => panic!("Expected LoadMemory"),
    }
    match instr2 {
        ToonInstr::Retrieve {
            ..
        } => {}
        _ => panic!("Expected Retrieve"),
    }
    match instr3 {
        ToonInstr::FoldContext {
            ..
        } => {}
        _ => panic!("Expected FoldContext"),
    }
    match instr4 {
        ToonInstr::EmitPointer {
            ..
        } => {}
        _ => panic!("Expected EmitPointer"),
    }
    match instr5 {
        ToonInstr::NoOp => {}
        _ => panic!("Expected NoOp"),
    }
}

#[test]
fn test_toon_graph_entry_point() {
    // Test that graph has correct entry point
    let graph1 = ToonGraph::new("entry1".to_string());
    assert_eq!(graph1.entry(), "entry1");

    let graph2 = ToonGraph::new("start_here".to_string());
    assert_eq!(graph2.entry(), "start_here");
}

#[test]
fn test_toon_graph_empty_next_list() {
    // Test nodes with no successors
    let mut graph = ToonGraph::new("terminal".to_string());

    let terminal_node = ToonNode {
        id: "terminal".to_string(),
        instr: ToonInstr::NoOp,
        next: vec![],
    };

    graph.add_node(terminal_node);

    let retrieved = graph.get_node("terminal").unwrap();
    assert_eq!(retrieved.next.len(), 0, "Terminal node should have empty next list");
}
