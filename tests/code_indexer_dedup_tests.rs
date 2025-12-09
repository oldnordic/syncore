//! Tests for code indexer UNIQUE constraint deduplication logic
//!
//! These tests specifically target the bug where const/function name collisions
//! cause UNIQUE constraint violations on (file_path, entity_type, name, line_start)

use syncore::code_graph::{CodeGraph, EntityType};
use syncore::vector::{VectorStore, HuggingFaceEmbeddings};
use std::io::Write;
use std::sync::{Arc, Mutex};
use tempfile::Builder;

/// Regression test for UNIQUE constraint violation when const and function share same name
///
/// Bug: Constants were stored with EntityType::Function and line_start=0, causing
/// collisions with functions of the same name due to UNIQUE(file_path, entity_type, name, line_start)
#[test]
fn test_const_function_name_collision_no_unique_violation() {
    // Create a temporary Rust file with const and function sharing the same name
    let mut temp_file = Builder::new().prefix("const_fn_collision").suffix(".rs").tempfile().unwrap();

    // Write content that would trigger the bug
    writeln!(temp_file, "const FOO: i32 = 42;").unwrap();
    writeln!(temp_file, "").unwrap();
    writeln!(temp_file, "fn FOO() -> i32 {{").unwrap();
    writeln!(temp_file, "    123").unwrap();
    writeln!(temp_file, "}}").unwrap();
    temp_file.flush().unwrap();

    // Setup CodeGraph with in-memory database
    let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(":memory:", vector_store).unwrap();

    // This should succeed without UNIQUE constraint violation
    let result = code_graph.index_file(temp_file.path());

    assert!(result.is_ok(), "Indexing should succeed without UNIQUE constraint violation. Error: {:?}", result.unwrap_err());

    let entities_indexed = result.unwrap();
    assert!(entities_indexed > 0, "Should index some entities");

    // Verify we can query the indexed entities without errors
    let db = code_graph.db.lock().unwrap();
    let entities: Result<Vec<_>, _> = db.prepare(
        "SELECT entity_type, name, line_start FROM code_entities WHERE file_path = ? ORDER BY line_start"
    ).unwrap().query_map([temp_file.path().to_str().unwrap()], |row| {
        Ok((
            row.get::<_, String>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
            row.get::<_, i64>(2).unwrap()
        ))
    }).unwrap().collect();

    assert!(entities.is_ok(), "Should be able to query indexed entities");
    let entities = entities.unwrap();

    // Should have both const and function entities
    assert_eq!(entities.len(), 2, "Should have indexed both const and function");

    // Find const (stored as Constant entity)
    let const_entity = entities.iter().find(|(entity_type, name, _)| {
        entity_type == "constant" && name == "FOO"
    }).expect("Should find const FOO entity");

    // Find function
    let fn_entity = entities.iter().find(|(entity_type, name, _)| {
        entity_type == "function" && name == "FOO"
    }).expect("Should find function FOO entity");

    // They should have different entity_type values to avoid collision
    assert_ne!(
        const_entity.0, fn_entity.0,
        "Const and function should have different entity_type values ('constant' vs 'function') to avoid UNIQUE constraint collision"
    );
}

/// Test for trait line_start extraction - traits should not all use line_start=0
#[test]
fn test_trait_line_start_extraction() {
    let mut temp_file = Builder::new().prefix("trait_test").suffix(".rs").tempfile().unwrap();

    writeln!(temp_file, "trait MyTrait {{").unwrap();
    writeln!(temp_file, "    fn do_something(&self);").unwrap();
    writeln!(temp_file, "}}").unwrap();
    writeln!(temp_file, "").unwrap();
    writeln!(temp_file, "struct MyStruct;").unwrap();
    writeln!(temp_file, "").unwrap();
    writeln!(temp_file, "impl MyTrait for MyStruct {{").unwrap();
    writeln!(temp_file, "    fn do_something(&self) {{}}").unwrap();
    writeln!(temp_file, "}}").unwrap();
    temp_file.flush().unwrap();

    let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(":memory:", vector_store).unwrap();

    let result = code_graph.index_file(temp_file.path());
    assert!(result.is_ok(), "Should index trait file without UNIQUE constraint violation");

    let entities_indexed = result.unwrap();
    assert!(entities_indexed > 0, "Should index some entities");

    // Verify trait entity has proper line_start
    let db = code_graph.db.lock().unwrap();
    let trait_entity: Option<_> = db.prepare(
        "SELECT line_start FROM code_entities WHERE entity_type = 'trait' AND name = 'MyTrait'"
    ).unwrap().query_row([], |row| {
        row.get::<_, i64>(0)
    }).optional().unwrap();

    assert!(trait_entity.is_some(), "Should find trait entity");
    let trait_line_start = trait_entity.unwrap();

    // Trait should have real line_start, not hardcoded 0
    assert!(trait_line_start > 0, "Trait should have real line_start, not 0. Got: {}", trait_line_start);
}

/// Test multiple constants with same name in different scopes don't collide
#[test]
fn test_multiple_constants_same_name_different_scopes() {
    let mut temp_file = Builder::new().prefix("multi_const").suffix(".rs").tempfile().unwrap();

    writeln!(temp_file, "mod outer {{").unwrap();
    writeln!(temp_file, "    pub const MAX_SIZE: usize = 100;").unwrap();
    writeln!(temp_file, "    mod inner {{").unwrap();
    writeln!(temp_file, "        pub const MAX_SIZE: usize = 50;").unwrap();
    writeln!(temp_file, "    }}").unwrap();
    writeln!(temp_file, "}}").unwrap();
    temp_file.flush().unwrap();

    let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(":memory:", vector_store).unwrap();

    let result = code_graph.index_file(temp_file.path());
    assert!(result.is_ok(), "Should handle multiple constants with same name without collision");

    let entities_indexed = result.unwrap();
    assert!(entities_indexed >= 2, "Should index both constants");

    // Verify we have distinct entities for the two constants
    let db = code_graph.db.lock().unwrap();
    let const_count: i64 = db.prepare(
        "SELECT COUNT(*) FROM code_entities WHERE name = 'MAX_SIZE' AND entity_type = 'constant' AND signature = 'const'"
    ).unwrap().query_row([], |row| {
        row.get(0)
    }).unwrap();

    assert_eq!(const_count, 2, "Should have two distinct MAX_SIZE constants");
}