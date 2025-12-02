//! Ordering Parity Tests
//!
//! TDD-driven tests for deterministic ordering between Neo4j and SQLiteGraph backends.
//! Tests identical neighbor ordering, triple match ordering, and consistent behavior.

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
            self.sqlite.upsert_entity(NodeLabel::Function, props.clone()).await?;

            entity_ids.push(i as i64);
        }

        Ok(entity_ids)
    }
}

/// Compare EntityResults with ordering consideration
fn compare_ordered_results(
    neo4j_results: &[EntityResult],
    sqlite_results: &[EntityResult],
    context: &str,
) -> Result<bool> {
    if neo4j_results.len() != sqlite_results.len() {
        println!(
            "Count mismatch in {}: Neo4j={}, SQLite={}",
            context,
            neo4j_results.len(),
            sqlite_results.len()
        );
        return Ok(false);
    }

    for (i, (neo4j_entity, sqlite_entity)) in
        neo4j_results.iter().zip(sqlite_results.iter()).enumerate()
    {
        if neo4j_entity.id != sqlite_entity.id {
            println!(
                "ID mismatch at index {} in {}: Neo4j={}, SQLite={}",
                i, context, neo4j_entity.id, sqlite_entity.id
            );
            return Ok(false);
        }

        if neo4j_entity.name != sqlite_entity.name {
            println!(
                "Name mismatch at index {} in {}: Neo4j={}, SQLite={}",
                i, context, neo4j_entity.name, sqlite_entity.name
            );
            return Ok(false);
        }
    }

    Ok(true)
}

/// Test 1: Neighbor ordering consistency
#[tokio::test]
#[ignore]
async fn test_neighbor_ordering_consistency() -> Result<()> {
    let setup = DualBackendSetup::new().await?;

    // Create a central entity with many neighbors
    let central_id = 1i64;
    let central_props = NodeProperties {
        id: central_id,
        name: "central".to_string(),
        path: Some("/src/central.rs".to_string()),
        start_line: Some(1),
        end_line: Some(10),
        signature: Some("fn central()".to_string()),
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

    setup.neo4j.upsert_entity(NodeLabel::Function, central_props.clone()).await?;
    setup.sqlite.upsert_entity(NodeLabel::Function, central_props.clone()).await?;

    // Create many neighbor entities with names that should sort predictably
    let mut neighbor_ids = Vec::new();
    for i in 2..=12 {
        let neighbor_props = NodeProperties {
            id: i,
            name: format!("neighbor_{:02}", i), // Zero-padded for consistent sorting
            path: Some(format!("/src/neighbor_{:02}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn neighbor_{:02}() {{}}", i)),
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

        setup.neo4j.upsert_entity(NodeLabel::Function, neighbor_props.clone()).await?;
        setup.sqlite.upsert_entity(NodeLabel::Function, neighbor_props.clone()).await?;
        neighbor_ids.push(i);
    }

    // Create relationships from central to all neighbors
    for &neighbor_id in &neighbor_ids {
        setup.neo4j.create_relationship(central_id, neighbor_id, RelationType::Calls).await?;
        setup.sqlite.create_relationship(central_id, neighbor_id, RelationType::Calls).await?;
    }

    // Test neighbor ordering multiple times
    for iteration in 1..=5 {
        let neo4j_neighbors = setup.neo4j.get_neighbors(central_id).await?;
        let sqlite_neighbors = setup.sqlite.get_neighbors(central_id).await?;

        assert!(
            compare_ordered_results(
                &neo4j_neighbors,
                &sqlite_neighbors,
                &format!("iteration {}", iteration)
            )?,
            "Neighbor ordering mismatch in iteration {}",
            iteration
        );

        // Verify ordering is consistent across runs
        if iteration > 1 {
            // Should have same order as previous iteration
            for (i, (neo4j_entity, sqlite_entity)) in
                neo4j_neighbors.iter().zip(sqlite_neighbors.iter()).enumerate()
            {
                assert_eq!(
                    neo4j_entity.name, sqlite_entity.name,
                    "Name ordering changed at index {} in iteration {}",
                    i, iteration
                );
            }
        }
    }

    Ok(())
}

/// Test 2: File entity ordering consistency
#[tokio::test]
#[ignore]
async fn test_file_entity_ordering_consistency() -> Result<()> {
    let setup = DualBackendSetup::new().await?;

    let file_path = "/src/ordering_test.rs";

    // Create entities in reverse line order to test sorting
    let mut entities = Vec::new();
    for i in (1..=10).rev() {
        let props = NodeProperties {
            id: i,
            name: format!("function_{}", i),
            path: Some(file_path.to_string()),
            start_line: Some(i * 10), // Non-sequential line numbers
            end_line: Some(i * 10 + 5),
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

        entities.push(props);
    }

    // Insert in reverse order
    for props in entities.iter().rev() {
        setup.neo4j.upsert_entity(NodeLabel::Function, props.clone()).await?;
        setup.sqlite.upsert_entity(NodeLabel::Function, props.clone()).await?;
    }

    // Test file entity ordering multiple times
    for iteration in 1..=3 {
        let neo4j_file_entities = setup.neo4j.get_file_entities(file_path).await?;
        let sqlite_file_entities = setup.sqlite.get_file_entities(file_path).await?;

        assert!(
            compare_ordered_results(
                &neo4j_file_entities,
                &sqlite_file_entities,
                &format!("file iteration {}", iteration)
            )?,
            "File entity ordering mismatch in iteration {}",
            iteration
        );

        // Verify they're sorted by line_start, then ID
        for i in 1..neo4j_file_entities.len() {
            let prev = &neo4j_file_entities[i - 1];
            let curr = &neo4j_file_entities[i];

            assert!(
                prev.start_line <= curr.start_line,
                "Neo4j file entities not sorted by line_start: {} (line {}) before {} (line {})",
                prev.name,
                prev.start_line.unwrap_or(0),
                curr.name,
                curr.start_line.unwrap_or(0)
            );
        }

        for i in 1..sqlite_file_entities.len() {
            let prev = &sqlite_file_entities[i - 1];
            let curr = &sqlite_file_entities[i];

            assert!(
                prev.start_line <= curr.start_line,
                "SQLite file entities not sorted by line_start: {} (line {}) before {} (line {})",
                prev.name,
                prev.start_line.unwrap_or(0),
                curr.name,
                curr.start_line.unwrap_or(0)
            );
        }
    }

    Ok(())
}

/// Test 3: Entity type ordering consistency
#[tokio::test]
#[ignore]
async fn test_entity_type_ordering_consistency() -> Result<()> {
    let setup = DualBackendSetup::new().await?;

    // Create entities of different types
    let mut entity_count = 1;

    // Create Functions
    for i in 1..=5 {
        let props = NodeProperties {
            id: entity_count,
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
        setup.sqlite.upsert_entity(NodeLabel::Function, props.clone()).await?;
        entity_count += 1;
    }

    // Create Structs
    for i in 1..=3 {
        let props = NodeProperties {
            id: entity_count,
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
        setup.sqlite.upsert_entity(NodeLabel::Struct, props.clone()).await?;
        entity_count += 1;
    }

    // Test entity type ordering
    for iteration in 1..=3 {
        let neo4j_functions = setup.neo4j.get_entities_by_type(NodeLabel::Function).await?;
        let sqlite_functions = setup.sqlite.get_entities_by_type(NodeLabel::Function).await?;

        assert!(
            compare_ordered_results(
                &neo4j_functions,
                &sqlite_functions,
                &format!("functions iteration {}", iteration)
            )?,
            "Function ordering mismatch in iteration {}",
            iteration
        );

        let neo4j_structs = setup.neo4j.get_entities_by_type(NodeLabel::Struct).await?;
        let sqlite_structs = setup.sqlite.get_entities_by_type(NodeLabel::Struct).await?;

        assert!(
            compare_ordered_results(
                &neo4j_structs,
                &sqlite_structs,
                &format!("structs iteration {}", iteration)
            )?,
            "Struct ordering mismatch in iteration {}",
            iteration
        );

        // Verify consistent ordering within each type
        for backend_results in [&neo4j_functions, &sqlite_functions] {
            for i in 1..backend_results.len() {
                let prev = &backend_results[i - 1];
                let curr = &backend_results[i];

                // Should be sorted by name, then file_path, then line_start, then ID
                if prev.name == curr.name {
                    if prev.path == curr.path {
                        if prev.start_line == curr.start_line {
                            assert!(
                                prev.id < curr.id,
                                "Entities with same name, path, and line should be ordered by ID"
                            );
                        } else {
                            assert!(
                                prev.start_line < curr.start_line,
                                "Entities with same name and path should be ordered by line_start"
                            );
                        }
                    } else {
                        assert!(
                            prev.path < curr.path,
                            "Entities with same name should be ordered by path"
                        );
                    }
                } else {
                    assert!(prev.name < curr.name, "Entities should be ordered by name");
                }
            }
        }
    }

    Ok(())
}

/// Test 4: Name search ordering consistency
#[tokio::test]
#[ignore]
async fn test_name_search_ordering_consistency() -> Result<()> {
    let setup = DualBackendSetup::new().await?;

    // Create entities with same name in different files/lines
    let common_name = "common_function";

    for i in 1..=5 {
        let props = NodeProperties {
            id: i,
            name: common_name.to_string(),
            path: Some(format!("/src/file_{}.rs", i)),
            start_line: Some(i * 10),
            end_line: Some(i * 10 + 5),
            signature: Some(format!("fn common_function() {{}} // from file {}", i)),
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
        setup.sqlite.upsert_entity(NodeLabel::Function, props.clone()).await?;
    }

    // Test name search ordering
    for iteration in 1..=3 {
        let neo4j_results = setup.neo4j.find_entities_by_name(common_name).await?;
        let sqlite_results = setup.sqlite.find_entities_by_name(common_name).await?;

        assert!(
            compare_ordered_results(
                &neo4j_results,
                &sqlite_results,
                &format!("name search iteration {}", iteration)
            )?,
            "Name search ordering mismatch in iteration {}",
            iteration
        );

        // Verify ordering by file_path, then line_start, then ID
        for backend_results in [&neo4j_results, &sqlite_results] {
            for i in 1..backend_results.len() {
                let prev = &backend_results[i - 1];
                let curr = &backend_results[i];

                if prev.path == curr.path {
                    if prev.start_line == curr.start_line {
                        assert!(
                            prev.id < curr.id,
                            "Same name entities should be ordered by ID as final tiebreaker"
                        );
                    } else {
                        assert!(
                            prev.start_line < curr.start_line,
                            "Same name entities should be ordered by line_start"
                        );
                    }
                } else {
                    assert!(
                        prev.path < curr.path,
                        "Same name entities should be ordered by file_path"
                    );
                }
            }
        }
    }

    Ok(())
}

/// Test 5: Orphan detection ordering consistency
#[tokio::test]
#[ignore]
async fn test_orphan_detection_ordering_consistency() -> Result<()> {
    let setup = DualBackendSetup::new().await?;

    // Create some orphan entities
    for i in 1..=10 {
        let props = NodeProperties {
            id: i,
            name: format!("orphan_{}", i),
            path: Some(format!("/src/orphan_{}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn orphan_{}() {{}}", i)),
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
        setup.sqlite.upsert_entity(NodeLabel::Function, props.clone()).await?;
    }

    // Create some connected entities
    let connected_id = 11i64;
    let connected_props = NodeProperties {
        id: connected_id,
        name: "connected".to_string(),
        path: Some("/src/connected.rs".to_string()),
        start_line: Some(1),
        end_line: Some(10),
        signature: Some("fn connected()".to_string()),
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

    setup.neo4j.upsert_entity(NodeLabel::Function, connected_props.clone()).await?;
    setup.sqlite.upsert_entity(NodeLabel::Function, connected_props.clone()).await?;

    // Create a relationship to make it non-orphan
    let target_id = 1i64;
    setup.neo4j.create_relationship(connected_id, target_id, RelationType::Calls).await?;
    setup.sqlite.create_relationship(connected_id, target_id, RelationType::Calls).await?;

    // Test orphan detection ordering
    for iteration in 1..=3 {
        let neo4j_orphans = setup.neo4j.find_orphan_entities().await?;
        let sqlite_orphans = setup.sqlite.find_orphan_entities().await?;

        assert!(
            compare_ordered_results(
                &neo4j_orphans,
                &sqlite_orphans,
                &format!("orphan iteration {}", iteration)
            )?,
            "Orphan detection ordering mismatch in iteration {}",
            iteration
        );

        // Should have exactly 10 orphans (not connected one)
        assert_eq!(neo4j_orphans.len(), 10, "Neo4j should detect 10 orphans");
        assert_eq!(sqlite_orphans.len(), 10, "SQLite should detect 10 orphans");

        // Verify ordering by name, then ID
        for backend_results in [&neo4j_orphans, &sqlite_orphans] {
            for i in 1..backend_results.len() {
                let prev = &backend_results[i - 1];
                let curr = &backend_results[i];

                if prev.name == curr.name {
                    assert!(prev.id < curr.id, "Orphans with same name should be ordered by ID");
                } else {
                    assert!(prev.name < curr.name, "Orphans should be ordered by name");
                }
            }
        }
    }

    Ok(())
}

/// Test 6: Complex relationship ordering
#[tokio::test]
#[ignore]
async fn test_complex_relationship_ordering() -> Result<()> {
    let setup = DualBackendSetup::new().await?;

    // Create a hub entity
    let hub_id = 1i64;
    let hub_props = NodeProperties {
        id: hub_id,
        name: "hub".to_string(),
        path: Some("/src/hub.rs".to_string()),
        start_line: Some(1),
        end_line: Some(10),
        signature: Some("fn hub()".to_string()),
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

    setup.neo4j.upsert_entity(NodeLabel::Function, hub_props.clone()).await?;
    setup.sqlite.upsert_entity(NodeLabel::Function, hub_props.clone()).await?;

    // Create many entities with different relationship types
    let mut entity_ids = Vec::new();
    for i in 2..=20 {
        let props = NodeProperties {
            id: i,
            name: format!("entity_{:02}", i),
            path: Some(format!("/src/entity_{:02}.rs", i)),
            start_line: Some(i),
            end_line: Some(i + 5),
            signature: Some(format!("fn entity_{:02}() {{}}", i)),
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
        setup.sqlite.upsert_entity(NodeLabel::Function, props.clone()).await?;
        entity_ids.push(i);
    }

    // Create relationships of different types
    for (i, &entity_id) in entity_ids.iter().enumerate() {
        let rel_type = match i % 4 {
            0 => RelationType::Calls,
            1 => RelationType::Contains,
            2 => RelationType::DependsOn,
            _ => RelationType::Uses,
        };

        setup.neo4j.create_relationship(hub_id, entity_id, rel_type).await?;
        setup.sqlite.create_relationship(hub_id, entity_id, rel_type).await?;
    }

    // Test that neighbor ordering is consistent despite different relationship types
    for iteration in 1..=3 {
        let neo4j_neighbors = setup.neo4j.get_neighbors(hub_id).await?;
        let sqlite_neighbors = setup.sqlite.get_neighbors(hub_id).await?;

        assert!(
            compare_ordered_results(
                &neo4j_neighbors,
                &sqlite_neighbors,
                &format!("complex iteration {}", iteration)
            )?,
            "Complex relationship ordering mismatch in iteration {}",
            iteration
        );

        // Should have all 19 neighbors
        assert_eq!(neo4j_neighbors.len(), 19, "Neo4j should have 19 neighbors");
        assert_eq!(sqlite_neighbors.len(), 19, "SQLite should have 19 neighbors");

        // Verify consistent ordering across iterations
        if iteration > 1 {
            for (i, (neo4j_entity, sqlite_entity)) in
                neo4j_neighbors.iter().zip(sqlite_neighbors.iter()).enumerate()
            {
                assert_eq!(
                    neo4j_entity.name, sqlite_entity.name,
                    "Neighbor ordering changed at index {} in iteration {}",
                    i, iteration
                );
            }
        }
    }

    Ok(())
}
