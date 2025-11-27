//! TDD Tests for Code Graph Edge Extraction
//!
//! Tests extraction of code relationships (edges) during indexing:
//! - Calls: Function/method calls
//! - Imports: Use declarations
//! - Uses: Struct/enum/trait type references
//! - References: Variable/constant references
//! - Inherits: Trait implementations
//!
//! REQUIREMENTS:
//! - Strict TDD (tests first, implementation after)
//! - Real tree-sitter parsing (no mocks)
//! - Verify edges in SQLite code_edges table
//! - Verify Neo4j sync works with extracted edges
//! - NO regressions (backward compatibility with R2-R5)

use anyhow::Result;
use std::io::Write;
use std::sync::{Arc, Mutex};
use syncore::code_graph::neo4j_sync::sync_relationships_to_neo4j;
use syncore::code_graph::{CodeGraph, EdgeType};
use syncore::graph::Neo4jClient;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};
use tempfile::Builder;

/// Helper to create test CodeGraph with in-memory database
fn create_test_code_graph() -> Result<CodeGraph> {
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    CodeGraph::new(":memory:", vector_store)
}

/// Helper to count edges by type in database
fn count_edges_by_type(code_graph: &CodeGraph, edge_type: EdgeType) -> Result<usize> {
    let db = code_graph.db_conn();
    let conn = db.lock().unwrap();

    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM code_edges WHERE edge_type = ?1",
        [edge_type.as_str()],
        |row| row.get(0),
    )?;

    Ok(count)
}

/// Helper to get total edge count
fn count_total_edges(code_graph: &CodeGraph) -> Result<usize> {
    let db = code_graph.db_conn();
    let conn = db.lock().unwrap();

    let count: usize = conn.query_row("SELECT COUNT(*) FROM code_edges", [], |row| row.get(0))?;

    Ok(count)
}

/// Helper to get Neo4j connection for integration tests
async fn get_neo4j_client() -> Result<Neo4jClient> {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    Neo4jClient::connect(&uri, &user, &pass).await
}

// ====================
// TEST 1: Calls Edges
// ====================

#[test]
fn test_extract_calls_edges() -> Result<()> {
    let mut code_graph = create_test_code_graph()?;

    // Create Rust file with function calls
    let mut temp_file = Builder::new()
        .prefix("test_calls_")
        .suffix(".rs")
        .tempfile()?;
    writeln!(temp_file, "fn main() {{")?;
    writeln!(temp_file, "    helper_function();")?;
    writeln!(temp_file, "    another_helper();")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "fn helper_function() {{")?;
    writeln!(temp_file, "    println!(\"Helper\");")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "fn another_helper() {{")?;
    writeln!(temp_file, "    helper_function();")?;
    writeln!(temp_file, "}}")?;
    temp_file.flush()?;

    // Index the file
    code_graph.index_file_with_neo4j(temp_file.path(), None)?;

    // Verify Calls edges were extracted
    let calls_count = count_edges_by_type(&code_graph, EdgeType::Calls)?;

    // Should have at least:
    // - main -> helper_function
    // - main -> another_helper
    // - another_helper -> helper_function
    assert!(
        calls_count >= 3,
        "Expected at least 3 Calls edges, got {}. Edge extraction not yet implemented.",
        calls_count
    );

    Ok(())
}

// =======================
// TEST 2: Imports Edges
// =======================

#[test]
fn test_extract_import_edges() -> Result<()> {
    let mut code_graph = create_test_code_graph()?;

    // Create Rust file with imports
    let mut temp_file = Builder::new()
        .prefix("test_imports_")
        .suffix(".rs")
        .tempfile()?;
    writeln!(temp_file, "use std::collections::HashMap;")?;
    writeln!(temp_file, "use anyhow::Result;")?;
    writeln!(temp_file, "use super::helper;")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "fn main() -> Result<()> {{")?;
    writeln!(
        temp_file,
        "    let map: HashMap<String, i32> = HashMap::new();"
    )?;
    writeln!(temp_file, "    Ok(())")?;
    writeln!(temp_file, "}}")?;
    temp_file.flush()?;

    // Index the file
    code_graph.index_file_with_neo4j(temp_file.path(), None)?;

    // Verify Imports edges were extracted
    let imports_count = count_edges_by_type(&code_graph, EdgeType::Imports)?;

    // Should have 3 import statements
    assert!(
        imports_count >= 3,
        "Expected at least 3 Imports edges, got {}. Import edge extraction not yet implemented.",
        imports_count
    );

    Ok(())
}

// ===================
// TEST 3: Uses Edges
// ===================

#[test]
fn test_extract_uses_edges() -> Result<()> {
    let mut code_graph = create_test_code_graph()?;

    // Create Rust file with type usage
    let mut temp_file = Builder::new()
        .prefix("test_uses_")
        .suffix(".rs")
        .tempfile()?;
    writeln!(temp_file, "struct MyStruct {{")?;
    writeln!(temp_file, "    value: i32,")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "struct Container {{")?;
    writeln!(temp_file, "    item: MyStruct,")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "fn process(data: MyStruct) -> MyStruct {{")?;
    writeln!(temp_file, "    data")?;
    writeln!(temp_file, "}}")?;
    temp_file.flush()?;

    // Index the file
    code_graph.index_file_with_neo4j(temp_file.path(), None)?;

    // Verify Uses edges were extracted
    let uses_count = count_edges_by_type(&code_graph, EdgeType::Uses)?;

    // Should have:
    // - Container uses MyStruct (field type)
    // - process uses MyStruct (param type)
    // - process uses MyStruct (return type)
    assert!(
        uses_count >= 2,
        "Expected at least 2 Uses edges, got {}. Type usage edge extraction not yet implemented.",
        uses_count
    );

    Ok(())
}

// ==========================
// TEST 4: References Edges
// ==========================

#[test]
fn test_extract_references_edges() -> Result<()> {
    let mut code_graph = create_test_code_graph()?;

    // Create Rust file with references
    let mut temp_file = Builder::new()
        .prefix("test_refs_")
        .suffix(".rs")
        .tempfile()?;
    writeln!(temp_file, "const CONFIG: i32 = 100;")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "fn use_config() -> i32 {{")?;
    writeln!(temp_file, "    CONFIG")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "fn double_config() -> i32 {{")?;
    writeln!(temp_file, "    CONFIG * 2")?;
    writeln!(temp_file, "}}")?;
    temp_file.flush()?;

    // Index the file
    code_graph.index_file_with_neo4j(temp_file.path(), None)?;

    // Verify References edges were extracted
    let references_count = count_edges_by_type(&code_graph, EdgeType::References)?;

    // Should have at least:
    // - use_config references CONFIG
    // - double_config references CONFIG
    assert!(
        references_count >= 2,
        "Expected at least 2 References edges, got {}. Reference edge extraction not yet implemented.",
        references_count
    );

    Ok(())
}

// ========================
// TEST 5: Inherits Edges
// ========================

#[test]
fn test_extract_inherits_edges() -> Result<()> {
    let mut code_graph = create_test_code_graph()?;

    // Create Rust file with trait implementations
    let mut temp_file = Builder::new()
        .prefix("test_inherits_")
        .suffix(".rs")
        .tempfile()?;
    writeln!(temp_file, "trait Display {{")?;
    writeln!(temp_file, "    fn display(&self);")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "struct MyStruct {{")?;
    writeln!(temp_file, "    value: i32,")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "impl Display for MyStruct {{")?;
    writeln!(temp_file, "    fn display(&self) {{")?;
    writeln!(temp_file, "        println!(\"{{}}\", self.value);")?;
    writeln!(temp_file, "    }}")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "trait Clone {{")?;
    writeln!(temp_file, "    fn clone(&self) -> Self;")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "impl Clone for MyStruct {{")?;
    writeln!(temp_file, "    fn clone(&self) -> Self {{")?;
    writeln!(temp_file, "        MyStruct {{ value: self.value }}")?;
    writeln!(temp_file, "    }}")?;
    writeln!(temp_file, "}}")?;
    temp_file.flush()?;

    // Index the file
    code_graph.index_file_with_neo4j(temp_file.path(), None)?;

    // Verify Inherits edges were extracted
    let inherits_count = count_edges_by_type(&code_graph, EdgeType::Inherits)?;

    // Should have at least 2 trait implementations:
    // - MyStruct implements Display
    // - MyStruct implements Clone
    assert!(
        inherits_count >= 2,
        "Expected at least 2 Inherits edges, got {}. Trait implementation edge extraction not yet implemented.",
        inherits_count
    );

    Ok(())
}

// ====================================
// TEST 6: Combined Edge Extraction
// ====================================

#[test]
fn test_combined_edge_extraction() -> Result<()> {
    let mut code_graph = create_test_code_graph()?;

    // Comprehensive Rust sample with multiple edge types
    let mut temp_file = Builder::new()
        .prefix("test_combined_")
        .suffix(".rs")
        .tempfile()?;
    writeln!(temp_file, "use std::fmt;")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "trait Processor {{")?;
    writeln!(temp_file, "    fn process(&self) -> i32;")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "struct DataProcessor {{")?;
    writeln!(temp_file, "    value: i32,")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "impl Processor for DataProcessor {{")?;
    writeln!(temp_file, "    fn process(&self) -> i32 {{")?;
    writeln!(temp_file, "        compute(self.value)")?;
    writeln!(temp_file, "    }}")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "fn compute(x: i32) -> i32 {{")?;
    writeln!(temp_file, "    x * 2")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "fn main() {{")?;
    writeln!(
        temp_file,
        "    let processor = DataProcessor {{ value: 42 }};"
    )?;
    writeln!(temp_file, "    let result = processor.process();")?;
    writeln!(temp_file, "    println!(\"{{}}\", result);")?;
    writeln!(temp_file, "}}")?;
    temp_file.flush()?;

    // Index the file
    code_graph.index_file_with_neo4j(temp_file.path(), None)?;

    // Verify multiple edge types were extracted
    let calls_count = count_edges_by_type(&code_graph, EdgeType::Calls)?;
    let imports_count = count_edges_by_type(&code_graph, EdgeType::Imports)?;
    let inherits_count = count_edges_by_type(&code_graph, EdgeType::Inherits)?;

    // Should have:
    // - Calls: process -> compute, main -> process
    // - Imports: use std::fmt
    // - Inherits: DataProcessor implements Processor
    assert!(
        calls_count >= 1,
        "Expected Calls edges, got {}",
        calls_count
    );
    assert!(
        imports_count >= 1,
        "Expected Imports edges, got {}",
        imports_count
    );
    assert!(
        inherits_count >= 1,
        "Expected Inherits edges, got {}",
        inherits_count
    );

    // Verify total edges > 0
    let total_edges = count_total_edges(&code_graph)?;
    assert!(
        total_edges >= 3,
        "Expected total edges >= 3, got {}",
        total_edges
    );

    Ok(())
}

// ===========================================
// TEST 7: Edges Written to SQLite
// ===========================================

#[test]
fn test_edges_written_to_sqlite() -> Result<()> {
    let mut code_graph = create_test_code_graph()?;

    let mut temp_file = Builder::new()
        .prefix("test_sqlite_")
        .suffix(".rs")
        .tempfile()?;
    writeln!(temp_file, "fn main() {{")?;
    writeln!(temp_file, "    helper();")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "fn helper() {{")?;
    writeln!(temp_file, "    println!(\"Helper\");")?;
    writeln!(temp_file, "}}")?;
    temp_file.flush()?;

    // Index the file
    code_graph.index_file_with_neo4j(temp_file.path(), None)?;

    // Verify edges are in database
    let db = code_graph.db_conn();
    let conn = db.lock().unwrap();

    let mut stmt =
        conn.prepare("SELECT src_entity_id, dst_entity_id, edge_type FROM code_edges")?;
    let all_edges: Vec<(i64, i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    assert!(
        !all_edges.is_empty(),
        "Expected edges to be written to SQLite, but got none. Edge extraction not implemented."
    );

    // Verify edge structure
    for (src_id, dst_id, edge_type) in &all_edges {
        assert!(*src_id > 0, "Source entity ID should be positive");
        assert!(*dst_id > 0, "Destination entity ID should be positive");
        assert!(!edge_type.is_empty(), "Edge type should not be empty");
    }

    Ok(())
}

// ==============================================
// TEST 8: Neo4j Sync Integration (async)
// ==============================================

#[tokio::test]
async fn test_edges_sync_to_neo4j() -> Result<()> {
    // Skip if Neo4j not available
    let neo4j = match get_neo4j_client().await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("Neo4j not available, skipping integration test");
            return Ok(());
        }
    };

    let mut code_graph = create_test_code_graph()?;

    let mut temp_file = Builder::new()
        .prefix("test_neo4j_")
        .suffix(".rs")
        .tempfile()?;
    writeln!(temp_file, "fn main() {{")?;
    writeln!(temp_file, "    helper();")?;
    writeln!(temp_file, "}}")?;
    writeln!(temp_file, "")?;
    writeln!(temp_file, "fn helper() {{")?;
    writeln!(temp_file, "    println!(\"Helper\");")?;
    writeln!(temp_file, "}}")?;
    temp_file.flush()?;

    // Index the file
    code_graph.index_file_with_neo4j(temp_file.path(), None)?;

    // Create entity nodes in Neo4j (sync needs them)
    let db = code_graph.db_conn();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, file_path, entity_type, name, signature, line_start, line_end, language FROM code_entities")?;
    let entities: Vec<_> = stmt
        .query_map([], |row| {
            Ok(syncore::code_graph::CodeEntity {
                id: Some(row.get(0)?),
                file_path: row.get(1)?,
                entity_type: syncore::code_graph::EntityType::Function,
                name: row.get(3)?,
                signature: row.get(4)?,
                line_start: row.get(5)?,
                line_end: row.get(6)?,
                docstring: None,
                language: row.get(7)?,
                body_snippet: None, // APEX v1.7 Phase 3
                created_at: None,
                last_modified_at: None,
                change_count: None,
                author_count: None,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);

    // Create nodes in Neo4j
    for entity in entities {
        let entity_id = entity.id.unwrap();
        syncore::code_graph::neo4j_writer::create_code_entity_node(&neo4j, entity_id, &entity)
            .await?;
    }

    // Sync edges to Neo4j
    let summary = sync_relationships_to_neo4j(code_graph.db_conn(), &neo4j, None, None).await?;

    // Verify edges were synced
    assert!(
        summary.edges_processed > 0,
        "Expected edges to be processed, got: {:?}. Edge extraction not yet implemented.",
        summary
    );
    assert!(
        summary.edges_created > 0,
        "Expected edges to be created in Neo4j, got: {:?}",
        summary
    );

    Ok(())
}

// ============================================
// TEST 9: Backward Compatibility (R2-R5)
// ============================================

#[tokio::test]
async fn test_backward_compatibility_r2_to_r5() -> Result<()> {
    use syncore::code_graph::fusion_simple::FusionSimple;
    use syncore::cognition::intent_classifier::{classify_intent, QueryIntent};
    use syncore::cognition::router_logic::route_query;

    // R2.2: CodeGraph indexing still works
    let mut code_graph = create_test_code_graph()?;
    let mut temp_file = Builder::new()
        .prefix("backward_")
        .suffix(".rs")
        .tempfile()?;
    writeln!(temp_file, "fn test() {{")?;
    writeln!(temp_file, "    println!(\"test\");")?;
    writeln!(temp_file, "}}")?;
    temp_file.flush()?;
    code_graph.index_file_with_neo4j(temp_file.path(), None)?;

    // R2.4: Fusion still works
    let fusion = FusionSimple::new(0.6, 0.3, 0.1, 0.0);
    let score = fusion.combine(0.8, 0.4, 0.0, 0.0);
    assert!((score - 0.64).abs() < 0.001);

    // R3.1: Intent classification still works
    let intent = classify_intent("format_string");
    assert_eq!(intent, QueryIntent::Symbolic);

    // R3.1: Router still works
    let decision = route_query(&QueryIntent::Semantic, "explain test");
    assert!(decision.should_call_raggraph);

    Ok(())
}
