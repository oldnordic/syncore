//! Pattern Parity Tests
//!
//! TDD-driven tests for ALL pattern combinations between Neo4j and SQLiteGraph backends.
//! Tests edge type filtering, label combinations, directionality, and property filtering.

use anyhow::Result;
use syncore::graph::{EntityResult, NodeLabel, NodeProperties, RelationType};
use syncore::graph::{GraphBackend, Neo4jBackend, SQLiteGraphBackend};
use tempfile::TempDir;
use tokio;

/// Test setup for dual backend comparison
struct DualBackendSetup {
    neo4j: Neo4jBackend,
    sqlite: SQLiteGraphBackend,
    _temp_dir: TempDir,
}

impl DualBackendSetup {
    async fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("test.db");

        // Setup SQLite backend
        let sqlite =
            SQLiteGraphBackend::connect(db_path.to_str().unwrap(), "", "", "code_syncore_default")
                .await?;

        // Setup Neo4j backend (using test configuration)
        let neo4j = Neo4jBackend::connect(
            "bolt://127.0.0.1:7687",
            "neo4j",
            "test_password",
            "code_syncore_default",
        )
        .await?;

        Ok(Self {
            neo4j,
            sqlite,
            _temp_dir: temp_dir,
        })
    }

    /// Create test entities on both backends
    async fn create_test_entities(&self, count: usize) -> Result<Vec<i64>> {
        let mut entity_ids = Vec::new();

        for i in 1..=count {
            let props = NodeProperties {
                id: i as i64,
                name: format!("entity_{}", i),
                path: Some(format!("/src/test_{}.rs", i)),
                start_line: Some(i as i64),
                end_line: Some((i + 10) as i64),
                signature: Some(format!("fn entity_{}() {{}}", i)),
                body_snippet: Some(format!("// Entity {} body", i)),
                docstring: Some(format!("Entity {} documentation", i)),
                hash: Some(format!("hash_{}", i)),
                language: Some("rust".to_string()),
                file_sha256: Some(format!("file_hash_{}", i)),
                mtime: Some((i * 1000) as i64),
                created_at: Some(format!("2024-01-{:02}T00:00:00Z", i % 28 + 1)),
                last_modified_at: Some(format!("2024-12-{:02}T00:00:00Z", i % 28 + 1)),
                change_count: Some(i as i64),
                author_count: Some((i % 3 + 1) as i64),
            };

            // Create on both backends
            self.neo4j.upsert_entity(NodeLabel::Function, props.clone()).await?;
            self.sqlite.upsert_entity(NodeLabel::Function, props).await?;

            entity_ids.push(i as i64);
        }

        Ok(entity_ids)
    }
}

/// Compare EntityResults from both backends for exact parity
fn compare_entity_results(
    neo4j_results: &[EntityResult],
    sqlite_results: &[EntityResult],
) -> Result<bool> {
    if neo4j_results.len() != sqlite_results.len() {
        println!("Count mismatch: Neo4j={}, SQLite={}", neo4j_results.len(), sqlite_results.len());
        return Ok(false);
    }

    for (i, (neo4j_entity, sqlite_entity)) in
        neo4j_results.iter().zip(sqlite_results.iter()).enumerate()
    {
        if neo4j_entity.id != sqlite_entity.id {
            println!(
                "ID mismatch at index {}: Neo4j={}, SQLite={}",
                i, neo4j_entity.id, sqlite_entity.id
            );
            return Ok(false);
        }

        if neo4j_entity.name != sqlite_entity.name {
            println!(
                "Name mismatch at index {}: Neo4j={}, SQLite={}",
                i, neo4j_entity.name, sqlite_entity.name
            );
            return Ok(false);
        }

        if neo4j_entity.label != sqlite_entity.label {
            println!(
                "Label mismatch at index {}: Neo4j={}, SQLite={}",
                i, neo4j_entity.label, sqlite_entity.label
            );
            return Ok(false);
        }
    }

    Ok(true)
}

/// Test 1: Edge type filtering only
#[tokio::test]
#[ignore]
async fn test_edge_type_filtering_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;

    // Create test entities
    let entity_ids = setup.create_test_entities(10).await?;

    // Create different types of relationships
    for i in 0..entity_ids.len() - 1 {
        let src_id = entity_ids[i];
        let dst_id = entity_ids[i + 1];

        // Create different relationship types
        match i % 4 {
            0 => {
                setup.neo4j.create_relationship(src_id, dst_id, RelationType::Calls).await?;
                setup.sqlite.create_relationship(src_id, dst_id, RelationType::Calls).await?;
            }
            1 => {
                setup.neo4j.create_relationship(src_id, dst_id, RelationType::Contains).await?;
                setup.sqlite.create_relationship(src_id, dst_id, RelationType::Contains).await?;
            }
            2 => {
                setup.neo4j.create_relationship(src_id, dst_id, RelationType::DependsOn).await?;
                setup.sqlite.create_relationship(src_id, dst_id, RelationType::DependsOn).await?;
            }
            _ => {
                setup.neo4j.create_relationship(src_id, dst_id, RelationType::Uses).await?;
                setup.sqlite.create_relationship(src_id, dst_id, RelationType::Uses).await?;
            }
        }
    }

    // Test function callees (CALLS relationships)
    for &id in &entity_ids {
        let neo4j_callees = setup.neo4j.get_function_callees(id).await?;
        let sqlite_callees = setup.sqlite.get_function_callees(id).await?;

        // Should have same callees for both backends
        assert_eq!(
            neo4j_callees.len(),
            sqlite_callees.len(),
            "Callee count mismatch for entity {}",
            id
        );

        // Sort by ID for comparison
        let mut neo4j_sorted = neo4j_callees.clone();
        let mut sqlite_sorted = sqlite_callees.clone();
        neo4j_sorted.sort_by_key(|e| e.id);
        sqlite_sorted.sort_by_key(|e| e.id);

        for (neo4j_callee, sqlite_callee) in neo4j_sorted.iter().zip(sqlite_sorted.iter()) {
            assert_eq!(neo4j_callee.id, sqlite_callee.id, "Callee ID mismatch");
            assert_eq!(neo4j_callee.name, sqlite_callee.name, "Callee name mismatch");
        }
    }

    // Test function callers (incoming CALLS relationships)
    for &id in &entity_ids {
        let neo4j_callers = setup.neo4j.get_function_callers(id).await?;
        let sqlite_callers = setup.sqlite.get_function_callers(id).await?;

        assert_eq!(
            neo4j_callers.len(),
            sqlite_callers.len(),
            "Caller count mismatch for entity {}",
            id
        );

        // Sort by ID for comparison
        let mut neo4j_sorted = neo4j_callers.clone();
        let mut sqlite_sorted = sqlite_callers.clone();
        neo4j_sorted.sort_by_key(|e| e.id);
        sqlite_sorted.sort_by_key(|e| e.id);

        for (neo4j_caller, sqlite_caller) in neo4j_sorted.iter().zip(sqlite_sorted.iter()) {
            assert_eq!(neo4j_caller.id, sqlite_caller.id, "Caller ID mismatch");
            assert_eq!(neo4j_caller.name, sqlite_caller.name, "Caller name mismatch");
        }
    }

    Ok(())
}

/// Test 2: Entity type filtering with relationships
#[tokio::test]
#[ignore]
async fn test_entity_type_filtering_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;

    // Create different types of entities
    let mut function_ids = Vec::new();
    let mut struct_ids = Vec::new();

    // Create Functions
    for i in 1..=5 {
        let props = NodeProperties {
            id: i,
            name: format!("function_{}", i),
            path: Some(format!("/src/fn_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn function_{}() {{}}", i)),
            body_snippet: None,
            docstring: None,
            hash: None,
            language: Some("rust".to_string()),
            file_sha256: None,
            mtime: None,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        };

        setup.neo4j.upsert_entity(NodeLabel::Function, props.clone()).await?;
        setup.sqlite.upsert_entity(NodeLabel::Function, props).await?;
        function_ids.push(i);
    }

    // Create Structs
    for i in 6..=10 {
        let props = NodeProperties {
            id: i,
            name: format!("struct_{}", i),
            path: Some(format!("/src/struct_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("struct struct_{} {{}}", i)),
            body_snippet: None,
            docstring: None,
            hash: None,
            language: Some("rust".to_string()),
            file_sha256: None,
            mtime: None,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        };

        setup.neo4j.upsert_entity(NodeLabel::Struct, props.clone()).await?;
        setup.sqlite.upsert_entity(NodeLabel::Struct, props).await?;
        struct_ids.push(i);
    }

    // Test entity type filtering
    let neo4j_functions = setup.neo4j.get_entities_by_type(NodeLabel::Function).await?;
    let sqlite_functions = setup.sqlite.get_entities_by_type(NodeLabel::Function).await?;

    assert_eq!(neo4j_functions.len(), sqlite_functions.len(), "Function count mismatch");

    let neo4j_structs = setup.neo4j.get_entities_by_type(NodeLabel::Struct).await?;
    let sqlite_structs = setup.sqlite.get_entities_by_type(NodeLabel::Struct).await?;

    assert_eq!(neo4j_structs.len(), sqlite_structs.len(), "Struct count mismatch");

    // Create cross-type relationships (Functions calling Structs)
    for (i, &func_id) in function_ids.iter().enumerate() {
        let struct_id = struct_ids[i % struct_ids.len()];
        setup.neo4j.create_relationship(func_id, struct_id, RelationType::Uses).await?;
        setup.sqlite.create_relationship(func_id, struct_id, RelationType::Uses).await?;
    }

    // Verify relationships are preserved regardless of entity type filtering
    for &func_id in &function_ids {
        let neo4j_neighbors = setup.neo4j.get_neighbors(func_id).await?;
        let sqlite_neighbors = setup.sqlite.get_neighbors(func_id).await?;

        assert_eq!(
            neo4j_neighbors.len(),
            sqlite_neighbors.len(),
            "Neighbor count mismatch for function {}",
            func_id
        );
    }

    Ok(())
}

/// Test 3: Directionality parity (incoming vs outgoing)
#[tokio::test]
#[ignore]
async fn test_directionality_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;

    // Create a linear chain: A -> B -> C -> D
    let entity_ids = setup.create_test_entities(4).await?;

    // Create directed relationships
    for i in 0..entity_ids.len() - 1 {
        let src_id = entity_ids[i];
        let dst_id = entity_ids[i + 1];

        setup.neo4j.create_relationship(src_id, dst_id, RelationType::Calls).await?;
        setup.sqlite.create_relationship(src_id, dst_id, RelationType::Calls).await?;
    }

    // Test outgoing relationships (callees)
    let first_entity = entity_ids[0];
    let neo4j_outgoing = setup.neo4j.get_function_callees(first_entity).await?;
    let sqlite_outgoing = setup.sqlite.get_function_callees(first_entity).await?;

    assert_eq!(neo4j_outgoing.len(), sqlite_outgoing.len(), "Outgoing relationship count mismatch");

    // Test incoming relationships (callers)
    let last_entity = entity_ids[entity_ids.len() - 1];
    let neo4j_incoming = setup.neo4j.get_function_callers(last_entity).await?;
    let sqlite_incoming = setup.sqlite.get_function_callers(last_entity).await?;

    assert_eq!(neo4j_incoming.len(), sqlite_incoming.len(), "Incoming relationship count mismatch");

    // Test bidirectional neighbors (should include both incoming and outgoing)
    let middle_entity = entity_ids[1];
    let neo4j_neighbors = setup.neo4j.get_neighbors(middle_entity).await?;
    let sqlite_neighbors = setup.sqlite.get_neighbors(middle_entity).await?;

    assert_eq!(
        neo4j_neighbors.len(),
        sqlite_neighbors.len(),
        "Bidirectional neighbor count mismatch"
    );

    Ok(())
}

/// Test 4: Complex graph patterns (diamond, cycles, etc.)
#[tokio::test]
#[ignore]
async fn test_complex_graph_patterns_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;

    // Create entities for complex patterns
    let entity_ids = setup.create_test_entities(6).await?;

    // Pattern 1: Diamond shape
    // A -> B, A -> C, B -> D, C -> D
    let (a, b, c, d) = (entity_ids[0], entity_ids[1], entity_ids[2], entity_ids[3]);

    setup.neo4j.create_relationship(a, b, RelationType::Calls).await?;
    setup.sqlite.create_relationship(a, b, RelationType::Calls).await?;

    setup.neo4j.create_relationship(a, c, RelationType::Calls).await?;
    setup.sqlite.create_relationship(a, c, RelationType::Calls).await?;

    setup.neo4j.create_relationship(b, d, RelationType::Calls).await?;
    setup.sqlite.create_relationship(b, d, RelationType::Calls).await?;

    setup.neo4j.create_relationship(c, d, RelationType::Calls).await?;
    setup.sqlite.create_relationship(c, d, RelationType::Calls).await?;

    // Verify diamond pattern
    let neo4j_a_callees = setup.neo4j.get_function_callees(a).await?;
    let sqlite_a_callees = setup.sqlite.get_function_callees(a).await?;

    assert_eq!(neo4j_a_callees.len(), 2, "A should call 2 entities");
    assert_eq!(sqlite_a_callees.len(), 2, "A should call 2 entities");

    let neo4j_d_callers = setup.neo4j.get_function_callers(d).await?;
    let sqlite_d_callers = setup.sqlite.get_function_callers(d).await?;

    assert_eq!(neo4j_d_callers.len(), 2, "D should have 2 callers");
    assert_eq!(sqlite_d_callers.len(), 2, "D should have 2 callers");

    // Pattern 2: Cycle
    // E -> F -> E
    let (e, f) = (entity_ids[4], entity_ids[5]);

    setup.neo4j.create_relationship(e, f, RelationType::Calls).await?;
    setup.sqlite.create_relationship(e, f, RelationType::Calls).await?;

    setup.neo4j.create_relationship(f, e, RelationType::Calls).await?;
    setup.sqlite.create_relationship(f, e, RelationType::Calls).await?;

    // Verify cycle
    let neo4j_e_callees = setup.neo4j.get_function_callees(e).await?;
    let sqlite_e_callees = setup.sqlite.get_function_callees(e).await?;

    assert_eq!(neo4j_e_callees.len(), 1, "E should call 1 entity");
    assert_eq!(sqlite_e_callees.len(), 1, "E should call 1 entity");

    let neo4j_f_callees = setup.neo4j.get_function_callees(f).await?;
    let sqlite_f_callees = setup.sqlite.get_function_callees(f).await?;

    assert_eq!(neo4j_f_callees.len(), 1, "F should call 1 entity");
    assert_eq!(sqlite_f_callees.len(), 1, "F should call 1 entity");

    // Pattern 3: Self-loop
    setup.neo4j.create_relationship(a, a, RelationType::Calls).await?;
    setup.sqlite.create_relationship(a, a, RelationType::Calls).await?;

    let neo4j_a_neighbors = setup.neo4j.get_neighbors(a).await?;
    let sqlite_a_neighbors = setup.sqlite.get_neighbors(a).await?;

    // Should include self in neighbors
    assert!(
        neo4j_a_neighbors.iter().any(|n| n.id == a),
        "A should be in its own neighbors (Neo4j)"
    );
    assert!(
        sqlite_a_neighbors.iter().any(|n| n.id == a),
        "A should be in its own neighbors (SQLite)"
    );

    Ok(())
}

/// Test 5: Multi-label node handling
#[tokio::test]
#[ignore]
async fn test_multi_label_handling_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;

    // Create entities with different labels
    let function_id = 1i64;
    let struct_id = 2i64;
    let file_id = 3i64;

    // Function entity
    let func_props = NodeProperties {
        id: function_id,
        name: "test_function".to_string(),
        path: Some("/src/test.rs".to_string()),
        start_line: Some(1),
        end_line: Some(10),
        signature: Some("fn test_function()".to_string()),
        body_snippet: None,
        docstring: None,
        hash: None,
        language: Some("rust".to_string()),
        file_sha256: None,
        mtime: None,
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    // Struct entity
    let struct_props = NodeProperties {
        id: struct_id,
        name: "TestStruct".to_string(),
        path: Some("/src/test.rs".to_string()),
        start_line: Some(12),
        end_line: Some(20),
        signature: Some("struct TestStruct {}".to_string()),
        body_snippet: None,
        docstring: None,
        hash: None,
        language: Some("rust".to_string()),
        file_sha256: None,
        mtime: None,
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    // File entity
    let file_props = NodeProperties {
        id: file_id,
        name: "test.rs".to_string(),
        path: Some("/src/test.rs".to_string()),
        start_line: None,
        end_line: None,
        signature: None,
        body_snippet: None,
        docstring: None,
        hash: None,
        language: None,
        file_sha256: None,
        mtime: None,
        created_at: None,
        last_modified_at: None,
        change_count: None,
        author_count: None,
    };

    // Create entities on both backends
    setup.neo4j.upsert_entity(NodeLabel::Function, func_props.clone()).await?;
    setup.sqlite.upsert_entity(NodeLabel::Function, func_props).await?;

    setup.neo4j.upsert_entity(NodeLabel::Struct, struct_props.clone()).await?;
    setup.sqlite.upsert_entity(NodeLabel::Struct, struct_props).await?;

    setup.neo4j.upsert_entity(NodeLabel::File, file_props.clone()).await?;
    setup.sqlite.upsert_entity(NodeLabel::File, file_props).await?;

    // Create relationships between different types
    setup.neo4j.create_relationship(function_id, struct_id, RelationType::Uses).await?;
    setup.sqlite.create_relationship(function_id, struct_id, RelationType::Uses).await?;

    setup.neo4j.create_relationship(struct_id, file_id, RelationType::Contains).await?;
    setup.sqlite.create_relationship(struct_id, file_id, RelationType::Contains).await?;

    // Test entity retrieval by type
    let neo4j_functions = setup.neo4j.get_entities_by_type(NodeLabel::Function).await?;
    let sqlite_functions = setup.sqlite.get_entities_by_type(NodeLabel::Function).await?;

    assert_eq!(neo4j_functions.len(), 1, "Should have 1 function");
    assert_eq!(sqlite_functions.len(), 1, "Should have 1 function");
    assert_eq!(neo4j_functions[0].label, "Function", "Function label should be correct");
    assert_eq!(sqlite_functions[0].label, "Function", "Function label should be correct");

    let neo4j_structs = setup.neo4j.get_entities_by_type(NodeLabel::Struct).await?;
    let sqlite_structs = setup.sqlite.get_entities_by_type(NodeLabel::Struct).await?;

    assert_eq!(neo4j_structs.len(), 1, "Should have 1 struct");
    assert_eq!(sqlite_structs.len(), 1, "Should have 1 struct");
    assert_eq!(neo4j_structs[0].label, "Struct", "Struct label should be correct");
    assert_eq!(sqlite_structs[0].label, "Struct", "Struct label should be correct");

    // Test cross-type relationships
    let neo4j_func_neighbors = setup.neo4j.get_neighbors(function_id).await?;
    let sqlite_func_neighbors = setup.sqlite.get_neighbors(function_id).await?;

    assert_eq!(neo4j_func_neighbors.len(), 1, "Function should have 1 neighbor");
    assert_eq!(sqlite_func_neighbors.len(), 1, "Function should have 1 neighbor");

    Ok(())
}

/// Test 6: Property-based filtering
#[tokio::test]
#[ignore]
async fn test_property_filtering_parity() -> Result<()> {
    let setup = DualBackendSetup::new().await?;

    // Create entities with different properties
    for i in 1..=10 {
        let props = NodeProperties {
            id: i,
            name: format!("entity_{}", i),
            path: Some(format!("/src/file_{}.rs", i % 3 + 1)), // 3 different files
            start_line: Some(i * 10),
            end_line: Some(i * 10 + 5),
            signature: Some(format!("fn entity_{}() -> i32 {{}}", i)),
            body_snippet: Some(format!("return {};", i)),
            docstring: if i % 2 == 0 {
                Some(format!("Documentation for {}", i))
            } else {
                None
            },
            hash: Some(format!("hash_{:x}", i)),
            language: Some("rust".to_string()),
            file_sha256: Some(format!("file_hash_{:x}", i)),
            mtime: Some((i * 1000) as i64),
            created_at: Some(format!("2024-01-{:02}T00:00:00Z", i % 28 + 1)),
            last_modified_at: Some(format!("2024-12-{:02}T00:00:00Z", i % 28 + 1)),
            change_count: Some((i % 5 + 1) as i64),
            author_count: Some((i % 3 + 1) as i64),
        };

        setup.neo4j.upsert_entity(NodeLabel::Function, props.clone()).await?;
        setup.sqlite.upsert_entity(NodeLabel::Function, props).await?;
    }

    // Test file-based filtering
    let file_1_entities_neo4j = setup.neo4j.get_file_entities("/src/file_1.rs").await?;
    let file_1_entities_sqlite = setup.sqlite.get_file_entities("/src/file_1.rs").await?;

    assert_eq!(
        file_1_entities_neo4j.len(),
        file_1_entities_sqlite.len(),
        "File 1 entity count mismatch"
    );

    // Test name-based filtering
    let entity_5_neo4j = setup.neo4j.find_entities_by_name("entity_5").await?;
    let entity_5_sqlite = setup.sqlite.find_entities_by_name("entity_5").await?;

    assert_eq!(entity_5_neo4j.len(), entity_5_sqlite.len(), "Name-based search count mismatch");

    if !entity_5_neo4j.is_empty() && !entity_5_sqlite.is_empty() {
        assert_eq!(entity_5_neo4j[0].id, entity_5_sqlite[0].id, "Entity 5 ID mismatch");
        assert_eq!(entity_5_neo4j[0].name, entity_5_sqlite[0].name, "Entity 5 name mismatch");
    }

    // Test orphan detection
    let neo4j_orphans = setup.neo4j.find_orphan_entities().await?;
    let sqlite_orphans = setup.sqlite.find_orphan_entities().await?;

    assert_eq!(neo4j_orphans.len(), sqlite_orphans.len(), "Orphan count mismatch");

    Ok(())
}
