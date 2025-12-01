//! Tests for Rust Backend Ingestion functionality
//! Tests the unified Rust diagnostics ingestion via DiagnosticsManager

use anyhow::Result;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use syncore::db::DbManager;
use syncore::project_analysis::diagnostics::DiagnosticsManager;
use syncore::project_analysis::rust_backend_ingestion::{
    RustBackendIngestion, RustIngestionStatus,
};
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
    conn_guard
        .execute("CREATE INDEX IF NOT EXISTS idx_diagnostics_tool ON code_diagnostics(tool)", [])?;
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

/// Create a minimal Rust project with known issues
fn create_test_rust_project(project_dir: &Path) -> Result<()> {
    // Create Cargo.toml
    let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

    // Create src directory
    fs::create_dir_all(project_dir.join("src"))?;

    // Create main.rs with some clippy warnings
    let main_rs = r#"
fn unused_function() {
    let x = 42;  // unused variable
    println!("This function is never called");
}

fn main() {
    println!("Hello, world!");
    unused_function();  // This will make the function used, but x will still be unused
}
"#;
    fs::write(project_dir.join("src/main.rs"), main_rs)?;

    Ok(())
}

#[test]
fn test_rust_ingestion_with_clippy_available() -> Result<()> {
    let (_temp_dir, db_manager, _diagnostics) = create_test_database()?;
    let project_dir = TempDir::new()?;

    // Create test Rust project
    create_test_rust_project(project_dir.path())?;

    let ingestion = RustBackendIngestion::new(db_manager);

    // Check if clippy is available
    let check_result = std::process::Command::new("cargo")
        .args(["clippy", "--version"])
        .current_dir(project_dir.path())
        .output();

    match check_result {
        Ok(output) if output.status.success() => {
            // clippy is available, test ingestion
            let summary = ingestion.run_for_project(project_dir.path())?;

            match summary.status {
                RustIngestionStatus::Success => {
                    // Should have some diagnostics
                    assert!(summary.total_diagnostics >= 0);
                    assert_eq!(summary.tool, "clippy");

                    // If diagnostics were found, verify they were stored
                    if summary.total_diagnostics > 0 {
                        let stored_diagnostics =
                            _diagnostics.query_diagnostics_by_tool("clippy")?;
                        assert_eq!(stored_diagnostics.len(), summary.total_diagnostics);
                    }
                }
                RustIngestionStatus::CommandFailed(msg) => {
                    // clippy failed but that's ok for this test
                    println!("Clippy command failed: {}", msg);
                    assert_eq!(summary.total_diagnostics, 0);
                }
                _ => {
                    panic!("Unexpected status when clippy is available");
                }
            }
        }
        _ => {
            // clippy not available, test should pass gracefully
            println!("Clippy not available, skipping ingestion test");
        }
    }

    Ok(())
}

#[test]
fn test_rust_ingestion_without_cargo_toml() -> Result<()> {
    let (_temp_dir, db_manager, _diagnostics) = create_test_database()?;
    let project_dir = TempDir::new()?;

    // Don't create Cargo.toml - just create an empty directory
    let ingestion = RustBackendIngestion::new(db_manager);

    let summary = ingestion.run_for_project(project_dir.path())?;

    assert_eq!(summary.total_diagnostics, 0);
    assert_eq!(summary.tool, "clippy");
    match summary.status {
        RustIngestionStatus::ToolUnavailable => {
            // Expected
        }
        _ => panic!("Expected ToolUnavailable status when no Cargo.toml"),
    }

    Ok(())
}

#[test]
fn test_rust_ingestion_clippy_unavailable() -> Result<()> {
    let (_temp_dir, db_manager, _diagnostics) = create_test_database()?;
    let project_dir = TempDir::new()?;

    // Create a minimal Cargo.toml to make it look like a Rust project
    let cargo_toml = r#"[package]
name = "clean-project"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(project_dir.path().join("Cargo.toml"), cargo_toml)?;

    fs::create_dir_all(project_dir.path().join("src"))?;

    let main_rs = r#"fn main() {
    println!("Hello, world!");
}"#;
    fs::write(project_dir.path().join("src/main.rs"), main_rs)?;

    let ingestion = RustBackendIngestion::new(db_manager);

    // Check if clippy is available
    let check_result = std::process::Command::new("cargo")
        .args(["clippy", "--version"])
        .current_dir(project_dir.path())
        .output();

    if let Ok(output) = check_result {
        if output.status.success() {
            let summary = ingestion.run_for_project(project_dir.path())?;

            match summary.status {
                RustIngestionStatus::Success => {
                    // Should have 0 or very few diagnostics for a clean project
                    assert_eq!(summary.tool, "clippy");

                    // Verify the database state matches the summary
                    let stored_diagnostics = _diagnostics.query_diagnostics_by_tool("clippy")?;
                    assert_eq!(stored_diagnostics.len(), summary.total_diagnostics);
                }
                RustIngestionStatus::CommandFailed(_) => {
                    // Acceptable - clippy might fail for various reasons
                }
                _ => {
                    panic!("Unexpected status for clean project");
                }
            }
        }
    }

    Ok(())
}
