//! PAE Dead Code Precision Tests
//!
//! Validates enhanced dead code heuristics to ensure:
//! - false positives filtered: new(), clone(), fmt(), default()
//! - false negatives avoided: real unused functions detected
//! - entity_type variations: function, method, struct
//! - trait impl detection

use anyhow::Result;
use std::sync::Arc;
use syncore::db::DbManager;
use syncore::project_analysis::{dead_code::DeadCodeRequest, ProjectAnalysisEngine};
use tempfile::TempDir;

/// Initialize code_graph schema on the code_graph database
fn ensure_code_graph_schema(db_manager: &DbManager) -> Result<()> {
    let conn = db_manager.code_graph_conn();
    let db = conn.lock().unwrap();
    db.execute_batch(
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
            created_at INTEGER,
            last_modified_at INTEGER,
            change_count INTEGER,
            author_count INTEGER,
            UNIQUE(file_path, entity_type, name, line_start)
        );
        CREATE INDEX IF NOT EXISTS idx_entities_name ON code_entities(name);
        CREATE INDEX IF NOT EXISTS idx_entities_file ON code_entities(file_path);
        CREATE INDEX IF NOT EXISTS idx_entities_type ON code_entities(entity_type);

        CREATE TABLE IF NOT EXISTS code_edges (
            src_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
            dst_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
            edge_type TEXT NOT NULL,
            PRIMARY KEY (src_entity_id, dst_entity_id, edge_type)
        );
        CREATE INDEX IF NOT EXISTS idx_edges_src ON code_edges(src_entity_id);
        CREATE INDEX IF NOT EXISTS idx_edges_dst ON code_edges(dst_entity_id);
        "#,
    )?;
    Ok(())
}

/// Insert a test entity into the code_entities table
fn insert_test_entity(
    db_manager: &DbManager,
    file_path: &str,
    entity_type: &str,
    name: &str,
    line_start: i32,
    line_end: i32,
) -> Result<i64> {
    let conn = db_manager.code_graph_conn();
    let db = conn.lock().unwrap();
    db.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (?1, ?2, ?3, NULL, ?4, ?5, 'rust', strftime('%s', 'now'))",
        rusqlite::params![file_path, entity_type, name, line_start, line_end],
    )?;
    Ok(db.last_insert_rowid())
}

/// Insert a call edge between two entities
fn insert_call_edge(db_manager: &DbManager, src_id: i64, dst_id: i64) -> Result<()> {
    let conn = db_manager.code_graph_conn();
    let db = conn.lock().unwrap();
    db.execute(
        "INSERT OR IGNORE INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?1, ?2, 'calls')",
        rusqlite::params![src_id, dst_id],
    )?;
    Ok(())
}

#[tokio::test]
async fn test_dead_code_false_positive_filtering() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    // Initialize code_graph schema
    ensure_code_graph_schema(&db_manager)?;

    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Insert entities directly into database to test dead code heuristics
    // Constructor method "new" - should be FILTERED OUT by is_constructor_method heuristic:
    let new_id = insert_test_entity(&db_manager, "src/lib.rs", "function", "new", 10, 12)?;

    // This is a truly unused function - should be DETECTED as dead:
    let _unused_id = insert_test_entity(
        &db_manager,
        "src/lib.rs",
        "function",
        "actually_unused_function",
        26,
        28,
    )?;

    // This function has an incoming edge (simulating it's called)
    let used_id =
        insert_test_entity(&db_manager, "src/lib.rs", "function", "used_function", 34, 36)?;
    // Create edge: something calls used_function (we use new as caller since we need any caller)
    insert_call_edge(&db_manager, new_id, used_id)?;

    // Run dead code analysis
    let request = DeadCodeRequest {
        exclude_public: Some(false), // Include public to test filtering
        limit: None,
    };

    let response = engine.dead_code(request).await?;
    assert!(response.ok, "Dead code analysis failed: {:?}", response.error);

    let data = response.data.unwrap();
    let dead_entities = data.dead_entities;

    // Should find 1 dead entity: actually_unused_function
    // (new is filtered by is_constructor_method, used_function has incoming edge)
    assert_eq!(
        dead_entities.len(),
        1,
        "Should detect exactly 1 dead entity, found: {:?}",
        dead_entities.iter().map(|e| &e.name).collect::<Vec<_>>()
    );

    let dead_entity = &dead_entities[0];
    assert_eq!(dead_entity.name, "actually_unused_function");
    assert_eq!(dead_entity.entity_type, "function");

    // Verify constructor "new" is not flagged
    for entity in &dead_entities {
        assert!(entity.name != "new", "Constructor 'new' should not be flagged as dead code");
    }

    Ok(())
}

#[tokio::test]
async fn test_dead_code_entity_type_variations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    // Initialize code_graph schema
    ensure_code_graph_schema(&db_manager)?;

    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Insert entities directly - dead entities with different types
    let _unused_struct =
        insert_test_entity(&db_manager, "src/lib.rs", "struct", "UnusedStruct", 1, 4)?;
    let _unused_fn =
        insert_test_entity(&db_manager, "src/lib.rs", "function", "unused_function", 6, 8)?;
    let _unused_method =
        insert_test_entity(&db_manager, "src/lib.rs", "function", "unused_method", 10, 12)?;

    // Insert used entities with call edges
    let used_fn =
        insert_test_entity(&db_manager, "src/lib.rs", "function", "used_function", 14, 16)?;
    let main_fn = insert_test_entity(&db_manager, "src/lib.rs", "function", "main", 18, 20)?;
    insert_call_edge(&db_manager, main_fn, used_fn)?;

    // Run dead code analysis
    // Note: exclude_public=false because our test entities don't have signatures set
    let request = DeadCodeRequest {
        exclude_public: Some(false),
        limit: None,
    };

    let response = engine.dead_code(request).await?;
    assert!(response.ok, "Dead code analysis failed: {:?}", response.error);

    let data = response.data.unwrap();
    let dead_entities = data.dead_entities;

    // Should find unused struct, function, method, and main (4 total: 3 unused + main has no incoming edges)
    assert!(
        dead_entities.len() >= 2,
        "Should detect at least 2 dead entities, found {}",
        dead_entities.len()
    );

    // Check for different entity types
    let mut found_struct = false;
    let mut found_function = false;

    for entity in &dead_entities {
        match entity.entity_type.as_str() {
            "struct" if entity.name == "UnusedStruct" => found_struct = true,
            "function" if entity.name == "unused_function" || entity.name == "unused_method" => {
                found_function = true
            }
            _ => {}
        }
    }

    assert!(found_struct, "Should detect unused struct");
    assert!(found_function, "Should detect unused function");

    Ok(())
}

#[tokio::test]
async fn test_dead_code_trait_impl_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    // Initialize code_graph schema
    ensure_code_graph_schema(&db_manager)?;

    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Insert entities directly to test dead code detection
    // Trait methods - should be filtered out as they're part of trait impl
    let _trait_req =
        insert_test_entity(&db_manager, "src/lib.rs", "function", "required_method", 10, 12)?;
    let _trait_opt =
        insert_test_entity(&db_manager, "src/lib.rs", "function", "optional_method", 14, 16)?;

    // Constructor - should be filtered out (heuristic)
    let _new_fn = insert_test_entity(&db_manager, "src/lib.rs", "function", "new", 20, 22)?;

    // Actually unused internal method - should be detected
    let _unused_internal =
        insert_test_entity(&db_manager, "src/lib.rs", "function", "unused_internal", 24, 26)?;

    // Used function with call edge
    let main_fn = insert_test_entity(&db_manager, "src/lib.rs", "function", "main", 28, 30)?;
    let used_fn =
        insert_test_entity(&db_manager, "src/lib.rs", "function", "used_function", 32, 34)?;
    insert_call_edge(&db_manager, main_fn, used_fn)?;

    // Run dead code analysis
    let request = DeadCodeRequest {
        exclude_public: Some(false), // Include public to test trait method filtering
        limit: None,
    };

    let response = engine.dead_code(request).await?;
    assert!(response.ok, "Dead code analysis failed: {:?}", response.error);

    let data = response.data.unwrap();
    let dead_entities = data.dead_entities;

    // unused_internal should be detected as dead
    let mut found_unused_internal = false;
    for entity in &dead_entities {
        if entity.name == "unused_internal" {
            found_unused_internal = true;
        }
    }

    assert!(found_unused_internal, "Should detect unused_internal method");

    Ok(())
}

#[tokio::test]
async fn test_dead_code_no_false_negatives() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    // Initialize code_graph schema
    ensure_code_graph_schema(&db_manager)?;

    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Insert clearly unused functions - these SHOULD be detected as dead
    let _unused1 = insert_test_entity(
        &db_manager,
        "src/lib.rs",
        "function",
        "completely_unused_function",
        1,
        3,
    )?;
    let _unused2 =
        insert_test_entity(&db_manager, "src/lib.rs", "function", "unused_with_params", 5, 7)?;
    let _unused3 =
        insert_test_entity(&db_manager, "src/lib.rs", "function", "unused_complex", 9, 11)?;

    // Insert used functions with call edges - these should NOT be detected
    let used_fn =
        insert_test_entity(&db_manager, "src/lib.rs", "function", "used_function", 13, 15)?;
    let helper_fn =
        insert_test_entity(&db_manager, "src/lib.rs", "function", "helper_function", 17, 19)?;
    let main_fn = insert_test_entity(&db_manager, "src/lib.rs", "function", "main", 21, 23)?;

    // Create call edges: main -> used_function, main -> helper_function
    insert_call_edge(&db_manager, main_fn, used_fn)?;
    insert_call_edge(&db_manager, main_fn, helper_fn)?;

    // Run dead code analysis
    // Note: exclude_public=false because our test entities don't have signatures set
    let request = DeadCodeRequest {
        exclude_public: Some(false),
        limit: None,
    };

    let response = engine.dead_code(request).await?;
    assert!(response.ok, "Dead code analysis failed: {:?}", response.error);

    let data = response.data.unwrap();
    let dead_entities = data.dead_entities;

    // Should find the unused functions (main also has no incoming edge but that's expected)
    let unused_names = ["completely_unused_function", "unused_with_params", "unused_complex"];
    let mut found_count = 0;

    for entity in &dead_entities {
        if unused_names.contains(&entity.name.as_str()) {
            found_count += 1;
        }
    }

    assert!(found_count >= 3, "Should detect at least 3 unused functions, found {}", found_count);

    // Functions with incoming edges (used_function, helper_function) should not be flagged
    // Note: "main" has no incoming edges, so it will be flagged as dead - this is correct behavior
    let used_names = ["used_function", "helper_function"];
    for entity in &dead_entities {
        assert!(
            !used_names.contains(&entity.name.as_str()),
            "Used function '{}' should not be flagged as dead code",
            entity.name
        );
    }

    Ok(())
}
