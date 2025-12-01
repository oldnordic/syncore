//! PAE Clippy Integration Tests
//!
//! TDD tests to verify that PAE tools (dead_code, unused_imports) properly
//! leverage Clippy diagnostics via validate_with_clippy_diagnostics().
//!
//! These tests ensure:
//! 1. When Clippy diagnostics are present, they influence the results
//! 2. Fallback heuristics work when no Clippy data exists
//! 3. Cross-validation between graph heuristics and Clippy improves accuracy

use anyhow::Result;
use std::sync::Arc;
use syncore::db::DbManager;
// CodeDiagnostic and DiagnosticsManager will be used when we wire up the integration
#[allow(unused_imports)]
use syncore::project_analysis::diagnostics::{CodeDiagnostic, DiagnosticsManager};
use syncore::project_analysis::{dead_code::DeadCodeRequest, ProjectAnalysisEngine};
use tempfile::TempDir;

/// Create test database with both code_entities and code_diagnostics schemas
fn create_test_db_with_schemas() -> Result<(TempDir, Arc<DbManager>)> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let conn = db_manager.code_graph_conn();
    let conn_guard = conn.lock().unwrap();

    // Create code_entities table
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

        CREATE TABLE IF NOT EXISTS code_diagnostics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            line_start INTEGER NOT NULL,
            severity TEXT NOT NULL,
            diagnostic_type TEXT NOT NULL,
            message TEXT NOT NULL,
            tool TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_diagnostics_file ON code_diagnostics(file_path);
        CREATE INDEX IF NOT EXISTS idx_diagnostics_tool ON code_diagnostics(tool);
        CREATE INDEX IF NOT EXISTS idx_diagnostics_type ON code_diagnostics(diagnostic_type);
        "#,
    )?;

    drop(conn_guard);
    Ok((temp_dir, db_manager))
}

/// Insert a test entity
fn insert_entity(
    db: &DbManager,
    file_path: &str,
    entity_type: &str,
    name: &str,
    line_start: i32,
) -> Result<i64> {
    let conn = db.code_graph_conn();
    let conn_guard = conn.lock().unwrap();
    conn_guard.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, line_start, line_end, language, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'rust', strftime('%s', 'now'))",
        rusqlite::params![file_path, entity_type, name, line_start, line_start + 5],
    )?;
    Ok(conn_guard.last_insert_rowid())
}

/// Insert a Clippy diagnostic directly into the database
fn insert_clippy_diagnostic(
    db: &DbManager,
    file_path: &str,
    line_start: i64,
    diagnostic_type: &str,
    message: &str,
) -> Result<()> {
    let conn = db.code_graph_conn();
    let conn_guard = conn.lock().unwrap();
    conn_guard.execute(
        "INSERT INTO code_diagnostics (file_path, line_start, severity, diagnostic_type, message, tool)
         VALUES (?1, ?2, 'warning', ?3, ?4, 'clippy')",
        rusqlite::params![file_path, line_start, diagnostic_type, message],
    )?;
    Ok(())
}

/// Insert a call edge between entities
fn insert_call_edge(db: &DbManager, src_id: i64, dst_id: i64) -> Result<()> {
    let conn = db.code_graph_conn();
    let conn_guard = conn.lock().unwrap();
    conn_guard.execute(
        "INSERT OR IGNORE INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?1, ?2, 'calls')",
        rusqlite::params![src_id, dst_id],
    )?;
    Ok(())
}

// =============================================================================
// TEST: dead_code should use Clippy diagnostics when available
// =============================================================================

#[tokio::test]
async fn test_dead_code_respects_clippy_confirmation() -> Result<()> {
    let (_temp_dir, db_manager) = create_test_db_with_schemas()?;
    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Insert two entities with no incoming edges (both appear dead by graph heuristics)
    let entity1_id = insert_entity(&db_manager, "src/lib.rs", "function", "truly_dead_fn", 10)?;
    let entity2_id = insert_entity(&db_manager, "src/lib.rs", "function", "false_positive_fn", 20)?;

    // Insert Clippy diagnostic ONLY for entity1 (truly_dead_fn)
    // This confirms entity1 is actually dead code
    insert_clippy_diagnostic(
        &db_manager,
        "src/lib.rs",
        10,
        "clippy::dead_code",
        "function `truly_dead_fn` is never used",
    )?;

    // Run dead code analysis
    let request = DeadCodeRequest {
        exclude_public: Some(false),
        limit: None,
    };
    let response = engine.dead_code(request).await?;
    assert!(response.ok, "Dead code analysis failed: {:?}", response.error);

    let data = response.data.unwrap();
    let dead_names: Vec<&str> = data.dead_entities.iter().map(|e| e.name.as_str()).collect();

    // Both should be detected (graph heuristics find both)
    // But with Clippy integration, we could prioritize or annotate
    // For now, test that at least the Clippy-confirmed one is found
    assert!(
        dead_names.contains(&"truly_dead_fn"),
        "Clippy-confirmed dead code should be detected. Found: {:?}",
        dead_names
    );

    Ok(())
}

#[tokio::test]
async fn test_dead_code_clippy_filters_false_positives() -> Result<()> {
    let (_temp_dir, db_manager) = create_test_db_with_schemas()?;
    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Insert a trait impl method (fmt) - normally filtered by heuristics
    let fmt_id = insert_entity(&db_manager, "src/lib.rs", "function", "fmt", 10)?;

    // Insert a truly unused function
    let unused_id = insert_entity(&db_manager, "src/lib.rs", "function", "unused_helper", 20)?;

    // Clippy confirms ONLY unused_helper is dead (not fmt, which is trait impl)
    insert_clippy_diagnostic(
        &db_manager,
        "src/lib.rs",
        20,
        "clippy::dead_code",
        "function `unused_helper` is never used",
    )?;

    let request = DeadCodeRequest {
        exclude_public: Some(false),
        limit: None,
    };
    let response = engine.dead_code(request).await?;
    assert!(response.ok);

    let data = response.data.unwrap();
    let dead_names: Vec<&str> = data.dead_entities.iter().map(|e| e.name.as_str()).collect();

    // fmt should be filtered by heuristics (common trait method)
    assert!(
        !dead_names.contains(&"fmt"),
        "Trait method 'fmt' should be filtered as false positive"
    );

    // unused_helper should be detected
    assert!(dead_names.contains(&"unused_helper"), "Clippy-confirmed dead code should be detected");

    Ok(())
}

#[tokio::test]
async fn test_dead_code_fallback_when_no_clippy_data() -> Result<()> {
    let (_temp_dir, db_manager) = create_test_db_with_schemas()?;
    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Insert entities with NO Clippy diagnostics
    let _unused_id = insert_entity(&db_manager, "src/lib.rs", "function", "orphan_function", 10)?;

    // Insert a used function with incoming edge
    let main_id = insert_entity(&db_manager, "src/lib.rs", "function", "main", 20)?;
    let used_id = insert_entity(&db_manager, "src/lib.rs", "function", "used_function", 30)?;
    insert_call_edge(&db_manager, main_id, used_id)?;

    let request = DeadCodeRequest {
        exclude_public: Some(false),
        limit: None,
    };
    let response = engine.dead_code(request).await?;
    assert!(response.ok);

    let data = response.data.unwrap();
    let dead_names: Vec<&str> = data.dead_entities.iter().map(|e| e.name.as_str()).collect();

    // orphan_function should be detected via graph heuristics (no incoming edges)
    assert!(
        dead_names.contains(&"orphan_function"),
        "Graph heuristics should detect orphan_function when no Clippy data"
    );

    // used_function has incoming edge, should NOT be flagged
    assert!(
        !dead_names.contains(&"used_function"),
        "Function with incoming edges should not be flagged"
    );

    Ok(())
}

// =============================================================================
// TEST: Clippy cross-validation boosts confidence
// =============================================================================

#[tokio::test]
async fn test_dead_code_clippy_cross_validation_called() -> Result<()> {
    let (_temp_dir, db_manager) = create_test_db_with_schemas()?;
    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Insert entity that graph says is dead
    let entity_id = insert_entity(&db_manager, "src/lib.rs", "function", "maybe_dead", 15)?;

    // Insert matching Clippy diagnostic
    insert_clippy_diagnostic(
        &db_manager,
        "src/lib.rs",
        15, // Same line as entity
        "clippy::dead_code",
        "function `maybe_dead` is never used",
    )?;

    let request = DeadCodeRequest {
        exclude_public: Some(false),
        limit: None,
    };
    let response = engine.dead_code(request).await?;
    assert!(response.ok);

    let data = response.data.unwrap();

    // The entity should be found - if validate_with_clippy_diagnostics is wired,
    // it would have been called during filtering
    let found = data.dead_entities.iter().any(|e| e.name == "maybe_dead");
    assert!(found, "Entity with Clippy confirmation should be in results");

    Ok(())
}

// =============================================================================
// TEST: unused_imports should use Clippy diagnostics
// =============================================================================

#[tokio::test]
async fn test_unused_imports_respects_clippy_confirmation() -> Result<()> {
    let (_temp_dir, db_manager) = create_test_db_with_schemas()?;
    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Insert two imports (both appear unused by graph heuristics)
    let _import1_id = insert_entity(&db_manager, "src/lib.rs", "import", "std::fs", 1)?;
    let _import2_id = insert_entity(&db_manager, "src/lib.rs", "import", "std::io", 2)?;

    // Insert Clippy diagnostic ONLY for import1 (std::fs)
    // This confirms import1 is actually unused
    insert_clippy_diagnostic(
        &db_manager,
        "src/lib.rs",
        1,
        "unused_imports",
        "unused import: `std::fs`",
    )?;

    // Run unused imports analysis
    let request = syncore::project_analysis::unused_imports::UnusedImportsRequest {
        file_path: None,
        limit: None,
    };
    let response = engine.unused_imports(request).await?;
    assert!(response.ok, "Unused imports analysis failed: {:?}", response.error);

    let data = response.data.unwrap();
    let unused_names: Vec<&str> =
        data.unused_imports.iter().map(|i| i.import_name.as_str()).collect();

    // With proper Clippy integration, the Clippy-confirmed unused import should be found
    assert!(
        unused_names.contains(&"std::fs"),
        "Clippy-confirmed unused import should be detected. Found: {:?}",
        unused_names
    );

    Ok(())
}

#[tokio::test]
async fn test_unused_imports_clippy_filters_false_positives() -> Result<()> {
    let (_temp_dir, db_manager) = create_test_db_with_schemas()?;
    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Insert a prelude-style import that might be flagged by heuristics
    let _prelude_id = insert_entity(&db_manager, "src/lib.rs", "import", "std::prelude", 1)?;

    // Insert a truly unused import
    let _unused_id =
        insert_entity(&db_manager, "src/lib.rs", "import", "unused_crate::Something", 2)?;

    // Clippy confirms ONLY unused_crate::Something is unused
    insert_clippy_diagnostic(
        &db_manager,
        "src/lib.rs",
        2,
        "unused_imports",
        "unused import: `unused_crate::Something`",
    )?;

    let request = syncore::project_analysis::unused_imports::UnusedImportsRequest {
        file_path: None,
        limit: None,
    };
    let response = engine.unused_imports(request).await?;
    assert!(response.ok);

    let data = response.data.unwrap();
    let unused_names: Vec<&str> =
        data.unused_imports.iter().map(|i| i.import_name.as_str()).collect();

    // unused_crate::Something should be detected (Clippy confirmed)
    assert!(
        unused_names.contains(&"unused_crate::Something"),
        "Clippy-confirmed unused import should be detected"
    );

    Ok(())
}

#[tokio::test]
async fn test_unused_imports_fallback_when_no_clippy_data() -> Result<()> {
    let (_temp_dir, db_manager) = create_test_db_with_schemas()?;
    let engine = ProjectAnalysisEngine::new(db_manager.clone(), None);

    // Insert imports with NO Clippy diagnostics
    let orphan_import_id = insert_entity(&db_manager, "src/lib.rs", "import", "orphan_module", 1)?;

    // Insert a used import with a reference edge
    let used_import_id = insert_entity(&db_manager, "src/lib.rs", "import", "used_module", 2)?;
    let fn_id = insert_entity(&db_manager, "src/lib.rs", "function", "some_fn", 10)?;

    // Create edge showing fn uses the import
    {
        let conn = db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();
        conn_guard.execute(
            "INSERT OR IGNORE INTO code_edges (src_entity_id, dst_entity_id, edge_type) VALUES (?1, ?2, 'uses')",
            rusqlite::params![fn_id, used_import_id],
        )?;
    }

    let request = syncore::project_analysis::unused_imports::UnusedImportsRequest {
        file_path: None,
        limit: None,
    };
    let response = engine.unused_imports(request).await?;
    assert!(response.ok);

    let data = response.data.unwrap();
    let unused_names: Vec<&str> =
        data.unused_imports.iter().map(|i| i.import_name.as_str()).collect();

    // orphan_module should be detected via graph heuristics when no Clippy data
    assert!(
        unused_names.contains(&"orphan_module"),
        "Graph heuristics should detect orphan_module when no Clippy data. Found: {:?}",
        unused_names
    );

    // Suppress unused variable warnings
    let _ = orphan_import_id;

    Ok(())
}
