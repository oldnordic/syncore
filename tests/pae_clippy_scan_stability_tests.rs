//! PAE Clippy Scan Stability Tests
//!
//! Validates correct behavior with corrupted or partial clippy output:
//! - Broken JSON line (should skip)
//! - Empty output
//! - Warning-level severity only
//! - Unexpected fields (should not panic)

use anyhow::Result;
use std::sync::Arc;
use syncore::db::DbManager;
use syncore::project_analysis::diagnostics::{CodeDiagnostic, DiagnosticsManager};
use tempfile::TempDir;

#[test]
fn test_clippy_broken_json_line_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let diagnostics = DiagnosticsManager::new(db_manager);

    // Mock clippy output with broken JSON lines mixed with valid ones
    let mock_clippy_output = vec![
        // Valid JSON line
        r#"{"file_path":"src/main.rs","line_start":10,"severity":"warning","diagnostic_type":"clippy::dead_code","message":"unused function","tool":"clippy"}"#,
        // Broken JSON line - missing closing brace
        r#"{"file_path":"src/main.rs","line_start":15,"severity":"error","diagnostic_type":"clippy::unimplemented","message":"unimplemented code","tool":"clippy""#,
        // Valid JSON line
        r#"{"file_path":"src/lib.rs","line_start":20,"severity":"warning","diagnostic_type":"clippy::unused_import","message":"unused import","tool":"clippy"}"#,
        // Completely broken line
        "not json at all",
        // Valid JSON line with extra whitespace
        r#"  {"file_path":"src/utils.rs","line_start":25,"severity":"note","diagnostic_type":"clippy::info","message":"info message","tool":"clippy"}  "#,
        // Broken JSON - missing quotes
        r#"{file_path:"src/broken.rs","line_start":30,"severity":"warning","diagnostic_type":"clippy::dead_code","message":"broken json","tool":"clippy"}"#,
    ];

    // Parse valid JSON lines and create diagnostics
    let mut valid_diagnostics = Vec::new();
    for line in mock_clippy_output {
        if let Ok(diagnostic_value) = serde_json::from_str::<serde_json::Value>(line) {
            if let (
                Some(file_path),
                Some(line_start),
                Some(severity),
                Some(diagnostic_type),
                Some(message),
                Some(tool),
            ) = (
                diagnostic_value.get("file_path").and_then(|v| v.as_str()),
                diagnostic_value.get("line_start").and_then(|v| v.as_i64()),
                diagnostic_value.get("severity").and_then(|v| v.as_str()),
                diagnostic_value.get("diagnostic_type").and_then(|v| v.as_str()),
                diagnostic_value.get("message").and_then(|v| v.as_str()),
                diagnostic_value.get("tool").and_then(|v| v.as_str()),
            ) {
                valid_diagnostics.push(CodeDiagnostic::new(
                    file_path.to_string(),
                    line_start,
                    severity.to_string(),
                    diagnostic_type.to_string(),
                    message.to_string(),
                    tool.to_string(),
                ));
            }
        }
    }

    // Should have parsed 3 valid diagnostics out of 6 lines
    assert_eq!(valid_diagnostics.len(), 3);

    // Insert valid diagnostics
    let inserted = diagnostics.insert_diagnostics(&valid_diagnostics)?;
    assert_eq!(inserted, 3);

    // Verify only valid diagnostics were inserted
    let all_diagnostics = diagnostics.query_diagnostics_by_tool("clippy")?;
    assert_eq!(all_diagnostics.len(), 3);

    // Verify content of valid diagnostics
    let file_paths: Vec<String> = all_diagnostics.iter().map(|d| d.file_path.clone()).collect();
    assert!(file_paths.contains(&"src/main.rs".to_string()));
    assert!(file_paths.contains(&"src/lib.rs".to_string()));
    assert!(file_paths.contains(&"src/utils.rs".to_string()));

    Ok(())
}

#[test]
fn test_clippy_empty_output_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let diagnostics = DiagnosticsManager::new(db_manager);

    // Test with empty diagnostics list
    let empty_diagnostics: Vec<CodeDiagnostic> = vec![];

    // Insert empty diagnostics
    let inserted = diagnostics.insert_diagnostics(&empty_diagnostics)?;
    assert_eq!(inserted, 0);

    // Verify no diagnostics were inserted
    let all_diagnostics = diagnostics.query_diagnostics_by_tool("clippy")?;
    assert_eq!(all_diagnostics.len(), 0);

    // Verify count is zero
    let count = diagnostics.count_diagnostics_for_tool("clippy")?;
    assert_eq!(count, 0);

    Ok(())
}

#[test]
fn test_clippy_warning_level_severity_only() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let diagnostics = DiagnosticsManager::new(db_manager);

    // Create diagnostics with only warning-level severity
    let warning_diagnostics = vec![
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
            "warning".to_string(),
            "clippy::unused_import".to_string(),
            "unused import".to_string(),
            "clippy".to_string(),
        ),
        CodeDiagnostic::new(
            "src/file3.rs".to_string(),
            15,
            "warning".to_string(),
            "clippy::too_many_arguments".to_string(),
            "too many function arguments".to_string(),
            "clippy".to_string(),
        ),
    ];

    // Insert warning diagnostics
    let inserted = diagnostics.insert_diagnostics(&warning_diagnostics)?;
    assert_eq!(inserted, 3);

    // Verify all diagnostics are warnings
    let all_diagnostics = diagnostics.query_diagnostics_by_tool("clippy")?;
    assert_eq!(all_diagnostics.len(), 3);

    for diagnostic in &all_diagnostics {
        assert_eq!(diagnostic.severity, "warning");
    }

    // Verify count by severity
    let warning_count = all_diagnostics.iter().filter(|d| d.severity == "warning").count();
    assert_eq!(warning_count, 3);

    // Verify no other severities exist
    let error_count = all_diagnostics.iter().filter(|d| d.severity == "error").count();
    let note_count = all_diagnostics.iter().filter(|d| d.severity == "note").count();

    assert_eq!(error_count, 0);
    assert_eq!(note_count, 0);

    Ok(())
}

#[test]
fn test_clippy_unexpected_fields_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let diagnostics = DiagnosticsManager::new(db_manager);

    // Mock clippy output with unexpected fields
    let mock_clippy_with_extra_fields = vec![
        // Valid JSON with extra fields
        r#"{"file_path":"src/main.rs","line_start":10,"severity":"warning","diagnostic_type":"clippy::dead_code","message":"unused function","tool":"clippy","extra_field":"extra_value","nested":{"field":"value"},"array_field":[1,2,3]}"#,
        // Valid JSON with different extra fields
        r#"{"file_path":"src/lib.rs","line_start":20,"severity":"error","diagnostic_type":"clippy::unimplemented","message":"unimplemented code","tool":"clippy","unexpected_numeric":123,"unexpected_bool":true,"unexpected_null":null}"#,
        // Valid JSON with missing some expected fields (should be filtered out)
        r#"{"file_path":"src/utils.rs","line_start":30,"severity":"warning","diagnostic_type":"clippy::info","tool":"clippy"}"#,
    ];

    // Parse JSON and extract only expected fields
    let mut valid_diagnostics = Vec::new();
    for line in mock_clippy_with_extra_fields {
        if let Ok(diagnostic_value) = serde_json::from_str::<serde_json::Value>(line) {
            // Extract only the expected fields
            if let (
                Some(file_path),
                Some(line_start),
                Some(severity),
                Some(diagnostic_type),
                Some(message),
                Some(tool),
            ) = (
                diagnostic_value.get("file_path").and_then(|v| v.as_str()),
                diagnostic_value.get("line_start").and_then(|v| v.as_i64()),
                diagnostic_value.get("severity").and_then(|v| v.as_str()),
                diagnostic_value.get("diagnostic_type").and_then(|v| v.as_str()),
                diagnostic_value.get("message").and_then(|v| v.as_str()),
                diagnostic_value.get("tool").and_then(|v| v.as_str()),
            ) {
                valid_diagnostics.push(CodeDiagnostic::new(
                    file_path.to_string(),
                    line_start,
                    severity.to_string(),
                    diagnostic_type.to_string(),
                    message.to_string(),
                    tool.to_string(),
                ));
            }
        }
    }

    // Should have parsed 2 valid diagnostics (third one missing message field)
    assert_eq!(valid_diagnostics.len(), 2);

    // Insert valid diagnostics
    let inserted = diagnostics.insert_diagnostics(&valid_diagnostics)?;
    assert_eq!(inserted, 2);

    // Verify diagnostics were inserted correctly
    let all_diagnostics = diagnostics.query_diagnostics_by_tool("clippy")?;
    assert_eq!(all_diagnostics.len(), 2);

    // Verify content
    let diagnostic_types: Vec<String> =
        all_diagnostics.iter().map(|d| d.diagnostic_type.clone()).collect();
    assert!(diagnostic_types.contains(&"clippy::dead_code".to_string()));
    assert!(diagnostic_types.contains(&"clippy::unimplemented".to_string()));

    Ok(())
}

#[test]
fn test_clippy_malformed_input_no_panic() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let main_db_path = temp_dir.path().join("main.db");
    let code_graph_db_path = temp_dir.path().join("code_graph.db");

    let db_manager = Arc::new(DbManager::new(
        main_db_path.to_str().unwrap(),
        code_graph_db_path.to_str().unwrap(),
    )?);

    let diagnostics = DiagnosticsManager::new(db_manager);

    // Various malformed inputs that should not cause panic
    let malformed_inputs = vec![
        // Empty string
        "",
        // Only whitespace
        "   \n\t  ",
        // Incomplete JSON structures
        "{",
        "}",
        "[",
        "]",
        // Invalid JSON syntax
        "{invalid json}",
        "{'single quotes'}",
        "{unquoted_key: 'value'}",
        // Null values
        "null",
        // Numbers only
        "123",
        // Boolean only
        "true",
        "false",
        // Arrays without objects
        "[]",
        "[1,2,3]",
        // Mixed content
        "some text {\"valid\": \"json\"} more text",
    ];

    // Try to parse each malformed input - none should panic
    // The goal is to verify no panics occur, not that parsing fails
    for input in malformed_inputs {
        let _result = serde_json::from_str::<serde_json::Value>(input);
        // Parsing may succeed or fail - the important thing is no panic
    }

    // Test with empty diagnostics list (edge case)
    let empty_result = diagnostics.insert_diagnostics(&[]);
    assert!(empty_result.is_ok());
    assert_eq!(empty_result.unwrap(), 0);

    // Verify database is still functional after malformed input handling
    let test_diagnostic = CodeDiagnostic::new(
        "src/test.rs".to_string(),
        1,
        "warning".to_string(),
        "clippy::test".to_string(),
        "test message".to_string(),
        "clippy".to_string(),
    );

    let insert_result = diagnostics.insert_diagnostics(&[test_diagnostic]);
    assert!(insert_result.is_ok());
    assert_eq!(insert_result.unwrap(), 1);

    let query_result = diagnostics.query_diagnostics_by_tool("clippy");
    assert!(query_result.is_ok());
    assert_eq!(query_result.unwrap().len(), 1);

    Ok(())
}
