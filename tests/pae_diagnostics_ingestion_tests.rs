//! Tests for DiagnosticsManager ingestion functionality
//! Tests the unified DiagnosticInput API and PAE helper methods

use anyhow::Result;
use std::fs;
use std::sync::Arc;
use syncore::db::DbManager;
use syncore::project_analysis::diagnostics::{DiagnosticInput, DiagnosticsManager};
use tempfile::TempDir;

/// Create a test database with the code_diagnostics schema
fn create_test_database() -> Result<(TempDir, Arc<DbManager>, DiagnosticsManager)> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    // Create the code_diagnostics table manually using the same SQL as production
    let conn = db_manager.code_graph_conn();
    let conn_guard = conn.lock().unwrap();
    conn_guard.execute(
        r#"
        CREATE TABLE IF NOT EXISTS code_diagnostics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            line_start INTEGER NOT NULL,
            severity TEXT NOT NULL,
            diagnostic_type TEXT NOT NULL,
            message TEXT NOT NULL,
            tool TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        [],
    )?;

    // Create indexes
    conn_guard.execute(
        "CREATE INDEX IF NOT EXISTS idx_diagnostics_file ON code_diagnostics(file_path)",
        [],
    )?;
    conn_guard.execute(
        "CREATE INDEX IF NOT EXISTS idx_diagnostics_tool ON code_diagnostics(tool)",
        [],
    )?;
    conn_guard.execute(
        "CREATE INDEX IF NOT EXISTS idx_diagnostics_type ON code_diagnostics(diagnostic_type)",
        [],
    )?;
    conn_guard.execute(
        "CREATE INDEX IF NOT EXISTS idx_diagnostics_severity ON code_diagnostics(severity)",
        [],
    )?;

    drop(conn_guard);

    let diagnostics_manager = DiagnosticsManager::new(db_manager.clone());

    Ok((temp_dir, db_manager, diagnostics_manager))
}

#[test]
fn test_store_diagnostics_basic() -> Result<()> {
    let (_temp_dir, _db_manager, diagnostics) = create_test_database()?;

    // Create test diagnostic inputs
    let diagnostic_inputs = vec![
        DiagnosticInput {
            file_path: "src/main.rs".to_string(),
            line: 10,
            column: 5,
            severity: "warning".to_string(),
            tool: "clippy".to_string(),
            code: Some("clippy::dead_code".to_string()),
            message: "unused function".to_string(),
        },
        DiagnosticInput {
            file_path: "src/main.rs".to_string(),
            line: 20,
            column: 10,
            severity: "error".to_string(),
            tool: "clippy".to_string(),
            code: Some("clippy::unimplemented".to_string()),
            message: "unimplemented code".to_string(),
        },
    ];

    // Store diagnostics
    let inserted = diagnostics.store_diagnostics(&diagnostic_inputs)?;
    assert_eq!(inserted, 2);

    // Verify diagnostics were stored correctly
    let stored = diagnostics.query_diagnostics_by_tool("clippy")?;
    assert_eq!(stored.len(), 2);

    // Check first diagnostic
    assert_eq!(stored[0].file_path, "src/main.rs");
    assert_eq!(stored[0].line_start, 10);
    assert_eq!(stored[0].severity, "warning");
    assert_eq!(stored[0].diagnostic_type, "clippy::dead_code");
    assert_eq!(stored[0].message, "unused function");
    assert_eq!(stored[0].tool, "clippy");

    // Check second diagnostic
    assert_eq!(stored[1].file_path, "src/main.rs");
    assert_eq!(stored[1].line_start, 20);
    assert_eq!(stored[1].severity, "error");
    assert_eq!(stored[1].diagnostic_type, "clippy::unimplemented");
    assert_eq!(stored[1].message, "unimplemented code");
    assert_eq!(stored[1].tool, "clippy");

    Ok(())
}

#[test]
fn test_store_diagnostics_without_code() -> Result<()> {
    let (_temp_dir, _db_manager, diagnostics) = create_test_database()?;

    // Create diagnostic input without code field
    let diagnostic_input = DiagnosticInput {
        file_path: "src/test.rs".to_string(),
        line: 15,
        column: 8,
        severity: "warning".to_string(),
        tool: "ruff".to_string(),
        code: None,
        message: "unused import".to_string(),
    };

    let inserted = diagnostics.store_diagnostics(&[diagnostic_input])?;
    assert_eq!(inserted, 1);

    // Verify diagnostic was stored with default diagnostic_type
    let stored = diagnostics.query_diagnostics_by_tool("ruff")?;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].diagnostic_type, "ruff::unknown");
    assert_eq!(stored[0].message, "unused import");

    Ok(())
}

#[test]
fn test_list_diagnostics_for_file_by_tool() -> Result<()> {
    let (_temp_dir, _db_manager, diagnostics) = create_test_database()?;

    // Insert diagnostics from different tools for the same file
    let clippy_diagnostic = DiagnosticInput {
        file_path: "src/shared.rs".to_string(),
        line: 5,
        column: 1,
        severity: "warning".to_string(),
        tool: "clippy".to_string(),
        code: Some("clippy::dead_code".to_string()),
        message: "clippy warning".to_string(),
    };

    let ruff_diagnostic = DiagnosticInput {
        file_path: "src/shared.rs".to_string(),
        line: 10,
        column: 1,
        severity: "error".to_string(),
        tool: "ruff".to_string(),
        code: Some("F401".to_string()),
        message: "ruff error".to_string(),
    };

    diagnostics.store_diagnostics(&[clippy_diagnostic, ruff_diagnostic])?;

    // Test listing by file and tool
    let clippy_diagnostics =
        diagnostics.list_diagnostics_for_file_by_tool("src/shared.rs", "clippy")?;
    assert_eq!(clippy_diagnostics.len(), 1);
    assert_eq!(clippy_diagnostics[0].tool, "clippy");
    assert_eq!(clippy_diagnostics[0].message, "clippy warning");

    let ruff_diagnostics =
        diagnostics.list_diagnostics_for_file_by_tool("src/shared.rs", "ruff")?;
    assert_eq!(ruff_diagnostics.len(), 1);
    assert_eq!(ruff_diagnostics[0].tool, "ruff");
    assert_eq!(ruff_diagnostics[0].message, "ruff error");

    // Test non-existent tool
    let mypy_diagnostics =
        diagnostics.list_diagnostics_for_file_by_tool("src/shared.rs", "mypy")?;
    assert_eq!(mypy_diagnostics.len(), 0);

    Ok(())
}

#[test]
fn test_count_diagnostics_by_tool() -> Result<()> {
    let (_temp_dir, _db_manager, diagnostics) = create_test_database()?;

    // Insert multiple diagnostics from different tools
    let diagnostics_input = vec![
        DiagnosticInput {
            file_path: "src/file1.rs".to_string(),
            line: 1,
            column: 1,
            severity: "warning".to_string(),
            tool: "clippy".to_string(),
            code: Some("clippy::dead_code".to_string()),
            message: "clippy 1".to_string(),
        },
        DiagnosticInput {
            file_path: "src/file1.rs".to_string(),
            line: 2,
            column: 1,
            severity: "warning".to_string(),
            tool: "clippy".to_string(),
            code: Some("clippy::unused".to_string()),
            message: "clippy 2".to_string(),
        },
        DiagnosticInput {
            file_path: "src/file1.rs".to_string(),
            line: 3,
            column: 1,
            severity: "error".to_string(),
            tool: "ruff".to_string(),
            code: Some("F401".to_string()),
            message: "ruff 1".to_string(),
        },
        DiagnosticInput {
            file_path: "src/file2.rs".to_string(),
            line: 1,
            column: 1,
            severity: "warning".to_string(),
            tool: "clippy".to_string(),
            code: Some("clippy::style".to_string()),
            message: "clippy 3".to_string(),
        },
    ];

    diagnostics.store_diagnostics(&diagnostics_input)?;

    // Test counts
    let clippy_count_file1 = diagnostics.count_diagnostics_by_tool("src/file1.rs", "clippy")?;
    assert_eq!(clippy_count_file1, 2);

    let ruff_count_file1 = diagnostics.count_diagnostics_by_tool("src/file1.rs", "ruff")?;
    assert_eq!(ruff_count_file1, 1);

    let clippy_count_file2 = diagnostics.count_diagnostics_by_tool("src/file2.rs", "clippy")?;
    assert_eq!(clippy_count_file2, 1);

    let mypy_count_file1 = diagnostics.count_diagnostics_by_tool("src/file1.rs", "mypy")?;
    assert_eq!(mypy_count_file1, 0);

    Ok(())
}

#[test]
fn test_store_diagnostics_empty_batch() -> Result<()> {
    let (_temp_dir, _db_manager, diagnostics) = create_test_database()?;

    // Store empty batch
    let inserted = diagnostics.store_diagnostics(&[])?;
    assert_eq!(inserted, 0);

    // Verify no diagnostics were added
    let all_diagnostics = diagnostics.query_diagnostics_by_tool("any_tool")?;
    assert_eq!(all_diagnostics.len(), 0);

    Ok(())
}

#[test]
fn test_store_diagnostics_replace_behavior() -> Result<()> {
    let (_temp_dir, _db_manager, diagnostics) = create_test_database()?;

    // Insert initial diagnostics for clippy
    let initial_diagnostics = vec![DiagnosticInput {
        file_path: "src/test.rs".to_string(),
        line: 10,
        column: 1,
        severity: "warning".to_string(),
        tool: "clippy".to_string(),
        code: Some("clippy::old".to_string()),
        message: "old message".to_string(),
    }];

    let inserted1 = diagnostics.store_diagnostics(&initial_diagnostics)?;
    assert_eq!(inserted1, 1);

    let initial_count = diagnostics.count_diagnostics_for_tool("clippy")?;
    assert_eq!(initial_count, 1);

    // Insert new diagnostics for the same tool (should replace)
    let new_diagnostics = vec![
        DiagnosticInput {
            file_path: "src/test.rs".to_string(),
            line: 20,
            column: 1,
            severity: "error".to_string(),
            tool: "clippy".to_string(),
            code: Some("clippy::new".to_string()),
            message: "new message".to_string(),
        },
        DiagnosticInput {
            file_path: "src/other.rs".to_string(),
            line: 30,
            column: 1,
            severity: "warning".to_string(),
            tool: "clippy".to_string(),
            code: Some("clippy::other".to_string()),
            message: "other message".to_string(),
        },
    ];

    let inserted2 = diagnostics.store_diagnostics(&new_diagnostics)?;
    assert_eq!(inserted2, 2);

    // Verify old diagnostics are gone and new ones are present
    let final_count = diagnostics.count_diagnostics_for_tool("clippy")?;
    assert_eq!(final_count, 2);

    let all_clippy = diagnostics.query_diagnostics_by_tool("clippy")?;
    assert_eq!(all_clippy.len(), 2);
    assert!(all_clippy.iter().all(|d| d.message != "old message"));
    assert!(all_clippy.iter().any(|d| d.message == "new message"));
    assert!(all_clippy.iter().any(|d| d.message == "other message"));

    Ok(())
}
