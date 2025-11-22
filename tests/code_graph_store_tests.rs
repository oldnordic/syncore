//! TDD Tests for CodeGraphStore
//! Verifies SQLite storage, Neo4j sync, FAISS embedding, and cross-linked queries.

use std::path::PathBuf;
use syncore::portfolio::code_graph_extractor::{
    CallEdge, CodeGraph, FunctionNode, ImplementationEdge, ImportNode, StructNode, TraitNode,
};
use syncore::portfolio::code_graph_store::{CodeGraphStore, GraphQuery, GraphResult};
use tempfile::TempDir;

/// Helper to create isolated test store
fn create_test_store() -> (TempDir, CodeGraphStore) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let vectors_dir = temp_dir.path().join("vectors");
    std::fs::create_dir_all(&vectors_dir).unwrap();

    // Set test namespace
    std::env::set_var("GRAPH_NAMESPACE", "test_code_graph");

    let store =
        CodeGraphStore::new_with_paths(&db_path, &vectors_dir).expect("Should create store");

    (temp_dir, store)
}

/// Helper to create a sample CodeGraph
fn create_sample_graph() -> CodeGraph {
    CodeGraph {
        file_path: PathBuf::from("src/lib.rs"),
        imports: vec![
            ImportNode {
                path: "std::collections::HashMap".to_string(),
                line: 1,
            },
            ImportNode {
                path: "crate::memory::MemoryStore".to_string(),
                line: 2,
            },
        ],
        functions: vec![
            FunctionNode {
                name: "process_data".to_string(),
                qualified_path: "lib::process_data".to_string(),
                is_public: true,
                is_async: false,
                parent_type: None,
                line_start: 10,
                line_end: 20,
            },
            FunctionNode {
                name: "helper".to_string(),
                qualified_path: "lib::helper".to_string(),
                is_public: false,
                is_async: false,
                parent_type: None,
                line_start: 22,
                line_end: 25,
            },
        ],
        calls: vec![CallEdge {
            from: "process_data".to_string(),
            to: "helper".to_string(),
            line: 15,
        }],
        structs: vec![StructNode {
            name: "DataProcessor".to_string(),
            is_public: true,
            line: 30,
        }],
        traits: vec![TraitNode {
            name: "Processable".to_string(),
            is_public: true,
            line: 40,
        }],
        implementations: vec![ImplementationEdge {
            struct_name: "DataProcessor".to_string(),
            trait_name: Some("Processable".to_string()),
            line: 50,
        }],
    }
}

#[test]
fn test_insert_function_nodes_into_sqlite() {
    let (_temp, mut store) = create_test_store();
    let graph = create_sample_graph();

    store.insert_graph(&graph).expect("Should insert graph");

    // Query SQLite for functions
    let functions = store
        .get_functions("src/lib.rs")
        .expect("Should get functions");

    assert_eq!(functions.len(), 2, "Should have 2 functions");
    assert!(functions.iter().any(|f| f.name == "process_data"));
    assert!(functions.iter().any(|f| f.name == "helper"));
}

#[test]
fn test_insert_call_edges_into_sqlite() {
    let (_temp, mut store) = create_test_store();
    let graph = create_sample_graph();

    store.insert_graph(&graph).expect("Should insert graph");

    // Query callgraph edges
    let callers = store.get_callers("helper").expect("Should get callers");

    assert!(
        callers.iter().any(|c| c == "process_data"),
        "process_data should call helper"
    );
}

#[test]
fn test_insert_struct_trait_edges() {
    let (_temp, mut store) = create_test_store();
    let graph = create_sample_graph();

    store.insert_graph(&graph).expect("Should insert graph");

    // Query implementations
    let impls = store
        .get_implementations("DataProcessor")
        .expect("Should get implementations");

    assert!(
        impls.iter().any(|t| t == "Processable"),
        "DataProcessor should implement Processable"
    );
}

#[test]
fn test_sync_to_neo4j_function_calls() {
    let (_temp, mut store) = create_test_store();
    let graph = create_sample_graph();

    store.insert_graph(&graph).expect("Should insert graph");

    // Sync to Neo4j (if available, otherwise skip gracefully)
    match store.sync_to_neo4j() {
        Ok(_) => {
            // Verify Neo4j has the relationship
            let neo4j_calls = store
                .query_neo4j_calls("process_data")
                .expect("Should query Neo4j");
            assert!(neo4j_calls.iter().any(|c| c == "helper"));
        }
        Err(e) if e.to_string().contains("connection") => {
            // Neo4j not available in test environment - acceptable
            eprintln!("Neo4j not available: {}", e);
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[test]
fn test_sync_to_neo4j_implementations() {
    let (_temp, mut store) = create_test_store();
    let graph = create_sample_graph();

    store.insert_graph(&graph).expect("Should insert graph");

    match store.sync_to_neo4j() {
        Ok(_) => {
            let impls = store
                .query_neo4j_implementations("DataProcessor")
                .expect("Should query implementations");
            assert!(impls.iter().any(|t| t == "Processable"));
        }
        Err(e) if e.to_string().contains("connection") => {
            eprintln!("Neo4j not available: {}", e);
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[test]
fn test_embed_functions_in_faiss() {
    let (_temp, mut store) = create_test_store();
    let graph = create_sample_graph();

    store.insert_graph(&graph).expect("Should insert graph");

    // Embed function signatures/names using FAISS
    store.embed_functions().expect("Should embed functions");

    // Search for semantically similar functions
    let results = store
        .search_similar_functions("data processing helper", 5)
        .expect("Should search similar");

    assert!(!results.is_empty(), "Should find similar functions");
    // process_data and helper should have high similarity to query
}

#[test]
fn test_cross_linked_query_results() {
    let (_temp, mut store) = create_test_store();
    let graph = create_sample_graph();

    store.insert_graph(&graph).expect("Should insert graph");
    store.embed_functions().expect("Should embed functions");

    // Query with cross-linking
    let query = GraphQuery {
        function_name: "process_data".to_string(),
        include_callers: true,
        include_callees: true,
        include_semantic: true,
        semantic_limit: 3,
    };

    let result = store
        .query_cross_linked(&query)
        .expect("Should query cross-linked");

    assert_eq!(result.function.name, "process_data");
    assert!(result.callees.iter().any(|c| c == "helper"));
    assert!(!result.semantic_neighbors.is_empty());
}

#[test]
fn test_namespace_isolation() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let vectors_dir = temp_dir.path().join("vectors");
    std::fs::create_dir_all(&vectors_dir).unwrap();

    // Create two stores with different namespaces
    std::env::set_var("GRAPH_NAMESPACE", "project_a");
    let mut store_a = CodeGraphStore::new_with_paths(&db_path, &vectors_dir).unwrap();

    std::env::set_var("GRAPH_NAMESPACE", "project_b");
    let mut store_b = CodeGraphStore::new_with_paths(&db_path, &vectors_dir).unwrap();

    // Insert different data in each namespace
    let graph_a = CodeGraph {
        file_path: PathBuf::from("src/a.rs"),
        functions: vec![FunctionNode {
            name: "func_a".to_string(),
            qualified_path: "a::func_a".to_string(),
            is_public: true,
            is_async: false,
            parent_type: None,
            line_start: 1,
            line_end: 5,
        }],
        ..Default::default()
    };

    let graph_b = CodeGraph {
        file_path: PathBuf::from("src/b.rs"),
        functions: vec![FunctionNode {
            name: "func_b".to_string(),
            qualified_path: "b::func_b".to_string(),
            is_public: true,
            is_async: false,
            parent_type: None,
            line_start: 1,
            line_end: 5,
        }],
        ..Default::default()
    };

    store_a.insert_graph(&graph_a).unwrap();
    store_b.insert_graph(&graph_b).unwrap();

    // Each store should only see its own namespace
    let funcs_a = store_a.get_all_functions().unwrap();
    let funcs_b = store_b.get_all_functions().unwrap();

    assert!(funcs_a.iter().any(|f| f.name == "func_a"));
    assert!(!funcs_a.iter().any(|f| f.name == "func_b"));

    assert!(funcs_b.iter().any(|f| f.name == "func_b"));
    assert!(!funcs_b.iter().any(|f| f.name == "func_a"));
}

#[test]
fn test_get_file_imports() {
    let (_temp, mut store) = create_test_store();
    let graph = create_sample_graph();

    store.insert_graph(&graph).expect("Should insert graph");

    let imports = store.get_imports("src/lib.rs").expect("Should get imports");

    assert_eq!(imports.len(), 2);
    assert!(imports.iter().any(|i| i.contains("HashMap")));
    assert!(imports.iter().any(|i| i.contains("MemoryStore")));
}

#[test]
fn test_message_bus_events() {
    let (_temp, mut store) = create_test_store();
    let graph = create_sample_graph();

    // Subscribe to events (simplified check)
    let events_before = store.get_event_count();

    store.insert_graph(&graph).expect("Should insert graph");

    let events_after = store.get_event_count();

    // Should have emitted "code_graph_indexed" event
    assert!(
        events_after > events_before,
        "Should emit MessageBus events on insert"
    );
}
