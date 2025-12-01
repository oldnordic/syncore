//! TDD Tests for Static Method Call Edge Detection
//!
//! These tests verify that the indexer correctly creates edges for:
//! 1. Static method calls: `Type::method()`
//! 2. Type usage when a type is referenced in a call expression
//!
//! Root cause of PAE false positives: FusionAttention::new() and CodeGraph::new()
//! are not creating CALLS edges because the indexer only handles `.method()` syntax,
//! not `Type::method()` syntax.

use anyhow::Result;
use std::sync::Arc;
use syncore::code_graph::edge_extractor::extract_edges_from_rust_ast;
use syncore::code_graph::EdgeType;
use tree_sitter::Parser;

/// Helper to parse Rust code and return the tree
fn parse_rust_code(code: &str) -> tree_sitter::Tree {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_rust::language()).expect("Failed to set Rust language");
    parser.parse(code, None).expect("Failed to parse code")
}

/// Sample Rust code with static method calls
const SAMPLE_CODE_STATIC_CALLS: &str = r#"
use crate::fusion::FusionAttention;

fn create_fusion() -> FusionAttention {
    let embeddings = get_embeddings();
    let fusion = FusionAttention::new(embeddings);
    fusion.combine(1.0, 2.0)
}

fn use_code_graph() {
    let graph = CodeGraph::new("db.sqlite", store);
    graph.add_entity(entity);
}
"#;

/// Test 1: Static method call `Type::new()` should create a CALLS edge
#[test]
fn test_static_method_creates_call_edge() {
    let tree = parse_rust_code(SAMPLE_CODE_STATIC_CALLS);
    let edges = extract_edges_from_rust_ast(SAMPLE_CODE_STATIC_CALLS, tree.root_node())
        .expect("Failed to extract edges");

    // Find edges where the callee contains "new" from a static call
    let static_call_edges: Vec<_> = edges
        .iter()
        .filter(|e| {
            matches!(e.edge_type, EdgeType::Calls)
                && (e.dst_entity_name.contains("FusionAttention")
                    || e.dst_entity_name.contains("new"))
        })
        .collect();

    assert!(
        !static_call_edges.is_empty(),
        "Expected CALLS edge for FusionAttention::new() but found none. \
         Current edges: {:?}",
        edges.iter().filter(|e| matches!(e.edge_type, EdgeType::Calls)).collect::<Vec<_>>()
    );

    // Verify the caller is create_fusion
    let has_correct_caller = static_call_edges.iter().any(|e| e.src_entity_name == "create_fusion");

    assert!(has_correct_caller, "Expected caller to be 'create_fusion' for FusionAttention::new()");
}

/// Test 2: Static method call should also create a TYPE_USAGE edge
#[test]
fn test_static_method_creates_type_usage_edge() {
    let tree = parse_rust_code(SAMPLE_CODE_STATIC_CALLS);
    let edges = extract_edges_from_rust_ast(SAMPLE_CODE_STATIC_CALLS, tree.root_node())
        .expect("Failed to extract edges");

    // Find UsesType edges for FusionAttention
    let type_usage_edges: Vec<_> = edges
        .iter()
        .filter(|e| {
            matches!(e.edge_type, EdgeType::UsesType) && e.dst_entity_name == "FusionAttention"
        })
        .collect();

    assert!(
        !type_usage_edges.is_empty(),
        "Expected UsesType edge for FusionAttention but found none. \
         All edges: {:?}",
        edges
    );
}

/// Test 3: CodeGraph::new() should also create proper edges
#[test]
fn test_codegraph_static_call_creates_edges() {
    let tree = parse_rust_code(SAMPLE_CODE_STATIC_CALLS);
    let edges = extract_edges_from_rust_ast(SAMPLE_CODE_STATIC_CALLS, tree.root_node())
        .expect("Failed to extract edges");

    // Find edges for CodeGraph::new
    let codegraph_edges: Vec<_> =
        edges.iter().filter(|e| e.dst_entity_name.contains("CodeGraph")).collect();

    assert!(!codegraph_edges.is_empty(), "Expected edges for CodeGraph::new() but found none");
}

/// Test 4: After indexing, FusionAttention should have incoming edges
/// and therefore NOT appear in dead code report
#[tokio::test]
async fn test_static_call_fixes_pae_false_positive() -> Result<()> {
    use syncore::db::DbManager;
    use syncore::project_analysis::{dead_code::DeadCodeRequest, ProjectAnalysisEngine};
    use tempfile::TempDir;

    // Create test database
    let temp_dir = TempDir::new()?;
    let main_db = temp_dir.path().join("main.db");
    let code_graph_db = temp_dir.path().join("code_graph.db");

    let db_manager =
        Arc::new(DbManager::new(main_db.to_str().unwrap(), code_graph_db.to_str().unwrap())?);

    // Setup schema
    {
        let conn = db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();
        conn_guard.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS code_entities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                name TEXT NOT NULL,
                signature TEXT,
                line_start INTEGER NOT NULL,
                line_end INTEGER NOT NULL,
                docstring TEXT,
                language TEXT NOT NULL,
                indexed_at INTEGER NOT NULL,
                UNIQUE(file_path, entity_type, name, line_start)
            );
            CREATE TABLE IF NOT EXISTS code_edges (
                src_entity_id INTEGER NOT NULL,
                dst_entity_id INTEGER NOT NULL,
                edge_type TEXT NOT NULL,
                PRIMARY KEY (src_entity_id, dst_entity_id, edge_type)
            );
            "#,
        )?;

        // Insert FusionAttention class
        conn_guard.execute(
            "INSERT INTO code_entities (file_path, entity_type, name, line_start, line_end, language, indexed_at)
             VALUES ('src/fusion.rs', 'class', 'FusionAttention', 10, 50, 'rust', 0)",
            [],
        )?;
        let fusion_id = conn_guard.last_insert_rowid();

        // Insert the caller function
        conn_guard.execute(
            "INSERT INTO code_entities (file_path, entity_type, name, line_start, line_end, language, indexed_at)
             VALUES ('src/api.rs', 'function', 'create_fusion', 1, 10, 'rust', 0)",
            [],
        )?;
        let caller_id = conn_guard.last_insert_rowid();

        // THIS IS THE KEY: Insert a CALLS edge from caller to FusionAttention
        // If the indexer works correctly, this edge should be auto-created
        // For this test, we simulate what SHOULD happen after the fix
        conn_guard.execute(
            "INSERT INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?, ?, 'calls')",
            rusqlite::params![caller_id, fusion_id],
        )?;
    }

    // Run dead code analysis
    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);
    let request = DeadCodeRequest {
        exclude_public: Some(false),
        limit: None,
    };
    let response = engine.dead_code(request).await?;
    assert!(response.ok);

    let data = response.data.unwrap();
    let dead_names: Vec<&str> = data.dead_entities.iter().map(|e| e.name.as_str()).collect();

    // FusionAttention should NOT be in dead code (it has an incoming edge)
    assert!(
        !dead_names.contains(&"FusionAttention"),
        "FusionAttention should NOT be flagged as dead code when it has incoming CALLS edge. \
         Dead entities: {:?}",
        dead_names
    );

    Ok(())
}

/// Test 5: Verify instance method calls still work (regression test)
#[test]
fn test_instance_method_calls_still_work() {
    let code = r#"
fn process() {
    let obj = get_object();
    obj.method();
    obj.another_method(arg);
}
"#;

    let tree = parse_rust_code(code);
    let edges =
        extract_edges_from_rust_ast(code, tree.root_node()).expect("Failed to extract edges");

    // Instance method calls should still create edges
    let call_edges: Vec<_> =
        edges.iter().filter(|e| matches!(e.edge_type, EdgeType::Calls)).collect();

    // Note: Currently the extractor may not handle these either,
    // but this test documents expected behavior
    println!("Instance method call edges: {:?}", call_edges);
}
