//! PAE Diagnostics Tests
//!
//! Validates DiagnosticsManager + code_diagnostics table:
//! - Insert diagnostic
//! - Query by file
//! - Query by severity
//! - Query by diagnostic type
//! - Adding multiple diagnostics for same file

use anyhow::Result;
use std::sync::Arc;
use syncore::db::DbManager;
use syncore::project_analysis::diagnostics::{CodeDiagnostic, DiagnosticsManager};
use tempfile::TempDir;

#[test]
fn test_diagnostics_insert_and_query_by_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let diagnostics = DiagnosticsManager::new(db_manager);

    // Create test diagnostics for different files
    let test_diagnostics = vec![
        CodeDiagnostic::new(
            "src/main.rs".to_string(),
            10,
            "warning".to_string(),
            "clippy::dead_code".to_string(),
            "unused function 'test_fn'".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/main.rs".to_string(),
            25,
            "error".to_string(),
            "clippy::unimplemented".to_string(),
            "unimplemented code".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/lib.rs".to_string(),
            15,
            "warning".to_string(),
            "clippy::unused_import".to_string(),
            "unused import".to_string(),
            "clippy".to_string(),
        ),
    ];

    // Insert diagnostics
    let inserted = diagnostics.insert_diagnostics(&test_diagnostics)?;
    assert_eq!(inserted, 3);

    // Query diagnostics by file
    let main_rs_diagnostics = diagnostics.query_diagnostics_by_file("src/main.rs")?;
    assert_eq!(main_rs_diagnostics.len(), 2);

    // Verify ordering by line_start
    assert_eq!(main_rs_diagnostics[0].line_start, 10);
    assert_eq!(main_rs_diagnostics[1].line_start, 25);

    // Verify content
    assert_eq!(main_rs_diagnostics[0].diagnostic_type, "clippy::dead_code");
    assert_eq!(main_rs_diagnostics[1].diagnostic_type, "clippy::unimplemented");

    let lib_rs_diagnostics = diagnostics.query_diagnostics_by_file("src/lib.rs")?;
    assert_eq!(lib_rs_diagnostics.len(), 1);
    assert_eq!(lib_rs_diagnostics[0].diagnostic_type, "clippy::unused_import");

    Ok(())
}

#[test]
fn test_diagnostics_query_by_severity() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let diagnostics = DiagnosticsManager::new(db_manager);

    // Create test diagnostics with different severities
    let test_diagnostics = vec![
        CodeDiagnostic::new(
            "src/file1.rs".to_string(),
            5,
            "error".to_string(),
            "clippy::unimplemented".to_string(),
            "error message".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/file1.rs".to_string(),
            10,
            "warning".to_string(),
            "clippy::dead_code".to_string(),
            "warning message".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/file2.rs".to_string(),
            15,
            "warning".to_string(),
            "clippy::unused_import".to_string(),
            "another warning".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/file3.rs".to_string(),
            20,
            "note".to_string(),
            "clippy::info".to_string(),
            "info message".to_string(),
            "clippy".to_string(),
        ),
    ];

    // Insert diagnostics
    let inserted = diagnostics.insert_diagnostics(&test_diagnostics)?;
    assert_eq!(inserted, 4);

    // Query all diagnostics and filter by severity manually
    let all_diagnostics = diagnostics.query_diagnostics_by_tool("clippy")?;
    assert_eq!(all_diagnostics.len(), 4);

    // Count by severity
    let error_count = all_diagnostics.iter().filter(|d| d.severity == "error").count();
    let warning_count = all_diagnostics.iter().filter(|d| d.severity == "warning").count();
    let note_count = all_diagnostics.iter().filter(|d| d.severity == "note").count();

    assert_eq!(error_count, 1);
    assert_eq!(warning_count, 2);
    assert_eq!(note_count, 1);

    Ok(())
}

#[test]
fn test_diagnostics_query_by_diagnostic_type() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let diagnostics = DiagnosticsManager::new(db_manager);

    // Create test diagnostics with different types
    let test_diagnostics = vec![
        CodeDiagnostic::new(
            "src/file1.rs".to_string(),
            5,
            "warning".to_string(),
            "clippy::dead_code".to_string(),
            "unused function".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/file1.rs".to_string(),
            10,
            "warning".to_string(),
            "clippy::dead_code".to_string(),
            "another unused function".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/file2.rs".to_string(),
            15,
            "error".to_string(),
            "clippy::unimplemented".to_string(),
            "unimplemented code".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/file3.rs".to_string(),
            20,
            "warning".to_string(),
            "clippy::unused_import".to_string(),
            "unused import".to_string(),
            "clippy".to_string(),
        ),
    ];

    // Insert diagnostics
    let inserted = diagnostics.insert_diagnostics(&test_diagnostics)?;
    assert_eq!(inserted, 4);

    // Query all diagnostics and filter by type manually
    let all_diagnostics = diagnostics.query_diagnostics_by_tool("clippy")?;
    assert_eq!(all_diagnostics.len(), 4);

    // Count by diagnostic type
    let dead_code_count =
        all_diagnostics.iter().filter(|d| d.diagnostic_type == "clippy::dead_code").count();
    let unimplemented_count =
        all_diagnostics.iter().filter(|d| d.diagnostic_type == "clippy::unimplemented").count();
    let unused_import_count =
        all_diagnostics.iter().filter(|d| d.diagnostic_type == "clippy::unused_import").count();

    assert_eq!(dead_code_count, 2);
    assert_eq!(unimplemented_count, 1);
    assert_eq!(unused_import_count, 1);

    Ok(())
}

#[test]
fn test_diagnostics_multiple_for_same_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let diagnostics = DiagnosticsManager::new(db_manager);

    // Create multiple diagnostics for the same file
    let test_diagnostics = vec![
        CodeDiagnostic::new(
            "src/main.rs".to_string(),
            5,
            "error".to_string(),
            "clippy::unimplemented".to_string(),
            "first error".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/main.rs".to_string(),
            10,
            "warning".to_string(),
            "clippy::dead_code".to_string(),
            "first warning".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/main.rs".to_string(),
            15,
            "warning".to_string(),
            "clippy::unused_import".to_string(),
            "second warning".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/main.rs".to_string(),
            20,
            "note".to_string(),
            "clippy::info".to_string(),
            "info message".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/main.rs".to_string(),
            25,
            "error".to_string(),
            "clippy::panic".to_string(),
            "second error".to_string(),
            "clippy".to_string(),
        ),
    ];

    // Insert diagnostics
    let inserted = diagnostics.insert_diagnostics(&test_diagnostics)?;
    assert_eq!(inserted, 5);

    // Query diagnostics for the file
    let file_diagnostics = diagnostics.query_diagnostics_by_file("src/main.rs")?;
    assert_eq!(file_diagnostics.len(), 5);

    // Verify correct ordering by line_start
    for (i, diagnostic) in file_diagnostics.iter().enumerate() {
        assert_eq!(diagnostic.line_start, ((i + 1) * 5) as i64);
    }

    // Verify grouping by severity
    let error_count = file_diagnostics.iter().filter(|d| d.severity == "error").count();
    let warning_count = file_diagnostics.iter().filter(|d| d.severity == "warning").count();
    let note_count = file_diagnostics.iter().filter(|d| d.severity == "note").count();

    assert_eq!(error_count, 2);
    assert_eq!(warning_count, 2);
    assert_eq!(note_count, 1);

    // Verify count function works correctly
    let count = diagnostics.count_diagnostics_for_file("src/main.rs", "clippy")?;
    assert_eq!(count, 5);

    Ok(())
}

#[test]
fn test_diagnostics_correct_count_and_grouping() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let diagnostics = DiagnosticsManager::new(db_manager);

    // Create diagnostics for multiple files and tools
    let clippy_diagnostics = vec![
        CodeDiagnostic::new(
            "src/file1.rs".to_string(),
            5,
            "warning".to_string(),
            "clippy::dead_code".to_string(),
            "unused function".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/file2.rs".to_string(),
            10,
            "error".to_string(),
            "clippy::unimplemented".to_string(),
            "unimplemented".to_string(),
            "clippy".to_string(),
        ),
    ];

    let rustc_diagnostics = vec![CodeDiagnostic::new(
        "src/file1.rs".to_string(),
        15,
        "error".to_string(),
        "E0382".to_string(),
        "borrow checker error".to_string(),
        "rustc".to_string(),
    )];

    // Insert clippy diagnostics
    let clippy_inserted = diagnostics.insert_diagnostics(&clippy_diagnostics)?;
    assert_eq!(clippy_inserted, 2);

    // Insert rustc diagnostics
    let rustc_inserted = diagnostics.insert_diagnostics(&rustc_diagnostics)?;
    assert_eq!(rustc_inserted, 1);

    // Verify counts by tool
    let clippy_count = diagnostics.count_diagnostics_for_tool("clippy")?;
    let rustc_count = diagnostics.count_diagnostics_for_tool("rustc")?;

    assert_eq!(clippy_count, 2);
    assert_eq!(rustc_count, 1);

    // Verify retrieval by tool
    let clippy_results = diagnostics.query_diagnostics_by_tool("clippy")?;
    let rustc_results = diagnostics.query_diagnostics_by_tool("rustc")?;

    assert_eq!(clippy_results.len(), 2);
    assert_eq!(rustc_results.len(), 1);

    // Verify all diagnostics are from correct tool
    assert!(clippy_results.iter().all(|d| d.tool == "clippy"));
    assert!(rustc_results.iter().all(|d| d.tool == "rustc"));

    // Verify correct retrieval order (by file_path, then line_start)
    for results in [&clippy_results, &rustc_results] {
        let mut prev_file = "";
        let mut prev_line = 0;

        for diagnostic in results {
            if diagnostic.file_path == *prev_file {
                assert!(
                    diagnostic.line_start >= prev_line,
                    "Line numbers should be ordered within same file"
                );
            } else {
                assert!(
                    diagnostic.file_path.as_str() > prev_file,
                    "Files should be ordered alphabetically"
                );
            }
            prev_file = diagnostic.file_path.as_str();
            prev_line = diagnostic.line_start;
        }
    }

    Ok(())
}
