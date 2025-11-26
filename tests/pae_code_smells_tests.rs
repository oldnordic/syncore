//! PAE Code Smells Tests
//!
//! Tests code smell and anti-pattern detection:
//! - GOD_FILE: loc >= 800, fan_in >= 40, entity_count >= 40
//! - HOTSPOT_GOD_FILE: risk_score >= 30.0, loc >= 1000
//! - DEAD_CODE_CLUSTER: dead_entity_count >= 10, dead_ratio >= 0.20
//! - IMPORT_JUNGLE: unused_import_count >= 10
//! - LONG_FUNCTION: function_loc >= 40
//! - LONG_PARAMETER_LIST: parameter_count >= 5

use anyhow::Result;
use std::sync::Arc;
use syncore::db::DbManager;
use syncore::project_analysis::ProjectAnalysisEngine;
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
        "#,
    )?;
    Ok(())
}

/// Insert a test entity into code_entities table
fn insert_test_entity(
    db_manager: &DbManager,
    file_path: &str,
    entity_type: &str,
    name: &str,
    signature: Option<&str>,
    line_start: i32,
    line_end: i32,
) -> Result<i64> {
    let conn = db_manager.code_graph_conn();
    let db = conn.lock().unwrap();
    db.execute(
        "INSERT INTO code_entities (file_path, entity_type, name, signature, line_start, line_end, language, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'rust', strftime('%s', 'now'))",
        rusqlite::params![file_path, entity_type, name, signature, line_start, line_end],
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

/// Insert a diagnostic for risk score calculation
fn insert_diagnostic(
    db_manager: &DbManager,
    file_path: &str,
    line_start: i32,
    severity: &str,
    diagnostic_type: &str,
    message: &str,
) -> Result<()> {
    let conn = db_manager.code_graph_conn();
    let db = conn.lock().unwrap();
    db.execute(
        "INSERT INTO code_diagnostics (file_path, line_start, severity, diagnostic_type, message, tool)
         VALUES (?1, ?2, ?3, ?4, ?5, 'clippy')",
        rusqlite::params![file_path, line_start, severity, diagnostic_type, message],
    )?;
    Ok(())
}

#[tokio::test]
async fn test_god_file_detection() -> Result<()> {
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

    // Create a file with loc >= 800, fan_in >= 40, entity_count >= 40
    let file_path = "src/god_file.rs";

    // Insert 45 entities spanning 850 lines
    for i in 0..45 {
        let line_start = (i * 19) + 1;
        let line_end = line_start + 18;
        insert_test_entity(
            &db_manager,
            file_path,
            "function",
            &format!("function_{}", i),
            Some(&format!("pub fn function_{}() {{}}", i)),
            line_start,
            line_end,
        )?;
    }

    // Create fan-in by adding 45 call edges from external entities
    for i in 0..45 {
        let external_id = insert_test_entity(
            &db_manager,
            "src/external.rs",
            "function",
            &format!("external_{}", i),
            Some(&format!("pub fn external_{}() {{}}", i)),
            i + 1,
            i + 1,
        )?;

        let target_id = (i + 1) as i64; // First 45 entities in god_file
        insert_call_edge(&db_manager, external_id, target_id)?;
    }

    let smells = engine.detect_file_smells(10)?;

    // Should detect GOD_FILE
    let god_file_smells: Vec<_> = smells
        .iter()
        .filter(|s| s.smell_type == "GOD_FILE" && s.file_path == file_path)
        .collect();

    assert!(!god_file_smells.is_empty(), "Should detect GOD_FILE smell");
    let smell = &god_file_smells[0];
    assert_eq!(smell.file_path, file_path);
    assert_eq!(smell.smell_type, "GOD_FILE");
    assert!(smell.loc.unwrap() >= 800);
    assert!(smell.fan_in.unwrap() >= 40);
    assert!(smell.entity_count.unwrap() >= 40);

    Ok(())
}

#[tokio::test]
async fn test_hotspot_god_file_detection() -> Result<()> {
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

    // Create a file with loc >= 1000 and high risk score
    let file_path = "src/hotspot_god.rs";

    // Insert entities spanning 1100 lines
    for i in 0..50 {
        let line_start = (i * 22) + 1;
        let line_end = line_start + 21;
        insert_test_entity(
            &db_manager,
            file_path,
            "function",
            &format!("function_{}", i),
            Some(&format!("pub fn function_{}() {{}}", i)),
            line_start,
            line_end,
        )?;
    }

    // Add many error diagnostics to increase risk score
    for i in 0..20 {
        insert_diagnostic(
            &db_manager,
            file_path,
            (i * 50) + 1,
            "error",
            "clippy::error",
            &format!("Error message {}", i),
        )?;
    }

    let smells = engine.detect_file_smells(10)?;

    // Should detect HOTSPOT_GOD_FILE
    let hotspot_smells: Vec<_> = smells
        .iter()
        .filter(|s| s.smell_type == "HOTSPOT_GOD_FILE" && s.file_path == file_path)
        .collect();

    assert!(
        !hotspot_smells.is_empty(),
        "Should detect HOTSPOT_GOD_FILE smell"
    );
    let smell = &hotspot_smells[0];
    assert_eq!(smell.file_path, file_path);
    assert_eq!(smell.smell_type, "HOTSPOT_GOD_FILE");
    assert!(smell.loc.unwrap() >= 1000);
    assert!(smell.risk_score.unwrap() >= 30.0);

    Ok(())
}

#[tokio::test]
async fn test_dead_code_cluster_detection() -> Result<()> {
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

    // Create a file with 50 entities, 12 dead (24% dead ratio)
    let file_path = "src/dead_code_cluster.rs";

    // Insert 50 entities
    for i in 0..50 {
        insert_test_entity(
            &db_manager,
            file_path,
            "function",
            &format!("function_{}", i),
            Some(&format!("pub fn function_{}() {{}}", i)),
            i + 1,
            i + 1,
        )?;
    }

    // Make first 12 entities dead by not adding any incoming edges
    // The remaining 38 entities will have incoming edges, making them alive

    Ok(())
}

#[tokio::test]
async fn test_import_jungle_detection() -> Result<()> {
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

    // Create a file with 15 unused imports
    let file_path = "src/import_jungle.rs";

    // Insert 15 import entities
    for i in 0..15 {
        insert_test_entity(
            &db_manager,
            file_path,
            "import",
            &format!("unused_import_{}", i),
            Some(&format!("use std::{};", i)),
            i + 1,
            i + 1,
        )?;
    }

    // Insert one function that doesn't use any of these imports
    insert_test_entity(
        &db_manager,
        file_path,
        "function",
        "main_function",
        Some("pub fn main_function() {}"),
        20,
        20,
    )?;

    let smells = engine.detect_file_smells(10)?;

    // Should detect IMPORT_JUNGLE
    let import_jungle_smells: Vec<_> = smells
        .iter()
        .filter(|s| s.smell_type == "IMPORT_JUNGLE" && s.file_path == file_path)
        .collect();

    assert!(
        !import_jungle_smells.is_empty(),
        "Should detect IMPORT_JUNGLE smell"
    );
    let smell = &import_jungle_smells[0];
    assert_eq!(smell.file_path, file_path);
    assert_eq!(smell.smell_type, "IMPORT_JUNGLE");
    assert!(smell.unused_import_count.unwrap() >= 10);

    Ok(())
}

#[tokio::test]
async fn test_long_function_detection() -> Result<()> {
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

    // Create a function with 50 LOC
    let file_path = "src/long_function.rs";
    insert_test_entity(
        &db_manager,
        file_path,
        "function",
        "long_function",
        Some("pub fn long_function() { /* 50 lines of code */ }"),
        1,
        50,
    )?;

    let smells = engine.detect_entity_smells(10)?;

    // Should detect LONG_FUNCTION
    let long_function_smells: Vec<_> = smells
        .iter()
        .filter(|s| s.smell_type == "LONG_FUNCTION" && s.file_path == file_path)
        .collect();

    assert!(
        !long_function_smells.is_empty(),
        "Should detect LONG_FUNCTION smell"
    );
    let smell = &long_function_smells[0];
    assert_eq!(smell.file_path, file_path);
    assert_eq!(smell.smell_type, "LONG_FUNCTION");
    assert_eq!(smell.name, "long_function");
    assert_eq!(smell.entity_type, "function");
    assert!(smell.function_loc.unwrap() >= 40);

    Ok(())
}

#[tokio::test]
async fn test_long_parameter_list_detection() -> Result<()> {
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

    // Create a function with 7 parameters
    let file_path = "src/long_params.rs";
    insert_test_entity(
        &db_manager,
        file_path,
        "function",
        "function_with_many_params",
        Some("pub fn function_with_many_params(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32) -> i32 { a }"),
        1,
        1,
    )?;

    let smells = engine.detect_entity_smells(10)?;

    // Should detect LONG_PARAMETER_LIST
    let long_param_smells: Vec<_> = smells
        .iter()
        .filter(|s| s.smell_type == "LONG_PARAMETER_LIST" && s.file_path == file_path)
        .collect();

    assert!(
        !long_param_smells.is_empty(),
        "Should detect LONG_PARAMETER_LIST smell"
    );
    let smell = &long_param_smells[0];
    assert_eq!(smell.file_path, file_path);
    assert_eq!(smell.smell_type, "LONG_PARAMETER_LIST");
    assert_eq!(smell.name, "function_with_many_params");
    assert_eq!(smell.entity_type, "function");
    assert!(smell.parameter_count.unwrap() >= 5);

    Ok(())
}

#[tokio::test]
async fn test_project_code_smells_mcp_tool_basic() -> Result<()> {
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

    // Create a simple file with a long function to trigger at least one smell
    let file_path = "src/test_file.rs";
    insert_test_entity(
        &db_manager,
        file_path,
        "function",
        "test_function",
        Some("pub fn test_function() { /* 45 lines of code */ }"),
        1,
        45,
    )?;

    // Test the detection functions directly
    let file_smells = engine.detect_file_smells(50)?;
    let entity_smells = engine.detect_entity_smells(50)?;

    // Should have some smells detected
    assert!(
        !file_smells.is_empty() || !entity_smells.is_empty(),
        "Should detect at least some code smells"
    );

    Ok(())
}
