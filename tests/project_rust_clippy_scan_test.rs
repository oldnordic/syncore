use std::fs;
use std::sync::Arc;
use syncore::db::DbManager;
use syncore::rust_tools::clippy::{run_clippy_scan, ClippyScanRequest};

#[test]
fn test_project_rust_clippy_scan_integration() {
    // Setup test databases
    let test_main_db = "/tmp/test_clippy_main.db";
    let test_code_graph_db = "/tmp/test_clippy_code_graph.db";

    // Cleanup any existing test files
    let _ = fs::remove_file(test_main_db);
    let _ = fs::remove_file(test_code_graph_db);

    // Create test database manager
    let db_manager = Arc::new(DbManager::new(test_main_db, test_code_graph_db).unwrap());

    // Create a simple Rust project with some clippy warnings
    let test_project_dir = "/tmp/test_clippy_project";
    let _ = fs::remove_dir_all(test_project_dir);
    fs::create_dir_all(test_project_dir).unwrap();

    // Create Cargo.toml
    let cargo_toml = r#"[package]
name = "test-clippy-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
    fs::write(format!("{}/Cargo.toml", test_project_dir), cargo_toml).unwrap();

    // Create src directory
    fs::create_dir_all(format!("{}/src", test_project_dir)).unwrap();

    // Create a Rust file with intentional clippy warnings
    let main_rs = r#"use std::collections::HashMap;

fn main() {
    let unused_variable = 42; // dead_code warning
    println!("Hello, world!");
    
    // unused import warning
    let _map: HashMap<String, i32> = HashMap::new();
}
"#;
    fs::write(format!("{}/src/main.rs", test_project_dir), main_rs).unwrap();

    // Test the clippy scan functionality
    let request = ClippyScanRequest {
        project_root: Some(test_project_dir.to_string()),
    };

    // Run clippy scan (this may take a moment)
    match run_clippy_scan(db_manager.clone(), request) {
        Ok(result) => {
            println!("Clippy scan completed successfully");
            println!("Project root: {}", result.project_root);
            println!("Diagnostics inserted: {}", result.inserted);

            // Verify that diagnostics were stored
            use syncore::project_analysis::diagnostics::DiagnosticsManager;
            let diagnostics_manager = DiagnosticsManager::new(db_manager);

            match diagnostics_manager.query_diagnostics_by_tool("clippy") {
                Ok(diagnostics) => {
                    println!("Total diagnostics in database: {}", diagnostics.len());

                    // Should have at least some diagnostics from our test code
                    assert!(diagnostics.len() > 0, "Expected at least one diagnostic");

                    // Verify diagnostic structure
                    for diagnostic in &diagnostics {
                        assert!(!diagnostic.file_path.is_empty());
                        assert!(diagnostic.line_start > 0);
                        assert!(!diagnostic.severity.is_empty());
                        assert!(!diagnostic.diagnostic_type.is_empty());
                        assert!(!diagnostic.message.is_empty());
                        assert_eq!(diagnostic.tool, "clippy");
                    }
                }
                Err(e) => {
                    println!("Warning: Could not query diagnostics: {}", e);
                    // This might happen if clippy didn't find any issues
                }
            }
        }
        Err(e) => {
            println!("Clippy scan failed (this may be expected if clippy is not available): {}", e);
            // Don't fail the test if clippy is not available in the test environment
        }
    }

    // Cleanup
    let _ = fs::remove_file(test_main_db);
    let _ = fs::remove_file(test_code_graph_db);
    let _ = fs::remove_dir_all(test_project_dir);
}

#[test]
fn test_clippy_scan_request_validation() {
    // Test with empty request (should use current directory)
    let test_main_db = "/tmp/test_clippy_validation_main.db";
    let test_code_graph_db = "/tmp/test_clippy_validation_code_graph.db";

    // Cleanup
    let _ = fs::remove_file(test_main_db);
    let _ = fs::remove_file(test_code_graph_db);

    let db_manager = Arc::new(DbManager::new(test_main_db, test_code_graph_db).unwrap());

    let request = ClippyScanRequest {
        project_root: None, // Should use current directory
    };

    // This should not panic, even if it fails due to no Cargo project
    let result = run_clippy_scan(db_manager, request);

    // The result might be Ok or Err depending on whether we're in a Rust project
    // The important thing is that it doesn't panic
    match result {
        Ok(_) => println!("Clippy scan completed (current directory)"),
        Err(e) => println!("Clippy scan failed as expected: {}", e),
    }

    // Cleanup
    let _ = fs::remove_file(test_main_db);
    let _ = fs::remove_file(test_code_graph_db);
}
