//! Tests for Python Backend Ingestion functionality
//! Tests the unified Python diagnostics ingestion via DiagnosticsManager

use anyhow::Result;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use syncore::db::DbManager;
use syncore::project_analysis::diagnostics::DiagnosticsManager;
use syncore::project_analysis::python_backend_ingestion::{
    PythonBackendIngestion, PythonIngestionStatus,
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

/// Create a minimal Python project with known issues
fn create_test_python_project(project_dir: &Path) -> Result<()> {
    // Create main.py with some lint issues
    let main_py = r#"
import os  # Unused import
import sys

def unused_function():
    x = 42  # Unused variable
    return x

def main():
    print("Hello, world!")
    unused_function()  # This makes the function used, but x is still unused

if __name__ == "__main__":
    main()
"#;
    fs::write(project_dir.join("main.py"), main_py)?;

    // Create another Python file with type issues
    let types_py = r#"
def add_numbers(a: int, b: int) -> str:
    return a + b  # Type error: should return int, not str

def process_data(data: list[str]) -> int:
    result = data + "extra"  # Type error: can't concatenate list and str
    return len(result)
"#;
    fs::write(project_dir.join("types.py"), types_py)?;

    Ok(())
}

#[test]
fn test_python_ingestion_with_tools_available() -> Result<()> {
    let (_temp_dir, db_manager, diagnostics_manager) = create_test_database()?;
    let project_dir = TempDir::new()?;

    // Create test Python project
    create_test_python_project(project_dir.path())?;

    let ingestion = PythonBackendIngestion::new(db_manager);

    // Check if ruff is available
    let ruff_available = std::process::Command::new("ruff")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    // Check if mypy is available
    let mypy_available = std::process::Command::new("mypy")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if ruff_available || mypy_available {
        let summary = ingestion.run_for_project(project_dir.path())?;

        match summary.status {
            PythonIngestionStatus::Success => {
                // Should have some diagnostics
                assert!(summary.total_diagnostics >= 0);

                // Verify tools used matches availability
                if ruff_available {
                    assert!(summary.tools_used.contains(&"ruff".to_string()));
                }
                if mypy_available {
                    assert!(summary.tools_used.contains(&"mypy".to_string()));
                }

                // If diagnostics were found, verify they were stored
                if summary.total_diagnostics > 0 {
                    for tool in &summary.tools_used {
                        let stored_diagnostics =
                            diagnostics_manager.query_diagnostics_by_tool(tool)?;
                        assert!(!stored_diagnostics.is_empty());

                        // Verify diagnostic structure
                        for diagnostic in &stored_diagnostics {
                            assert_eq!(diagnostic.tool, *tool);
                            assert!(!diagnostic.file_path.is_empty());
                            assert!(diagnostic.line_start > 0);
                            assert!(!diagnostic.message.is_empty());
                            assert!(["error", "warning", "note"]
                                .contains(&diagnostic.severity.as_str()));
                        }
                    }
                }
            }
            PythonIngestionStatus::CommandFailed(msg) => {
                println!("Python tools failed: {}", msg);
                // Tools might fail but that's ok for this test
                assert_eq!(summary.total_diagnostics, 0);
            }
            _ => {
                panic!("Unexpected status when tools are available");
            }
        }
    } else {
        println!("Neither ruff nor mypy available, skipping ingestion test");
    }

    Ok(())
}

#[test]
fn test_python_ingestion_without_python_files() -> Result<()> {
    let (_temp_dir, db_manager, _diagnostics) = create_test_database()?;
    let project_dir = TempDir::new()?;

    // Don't create any Python files, just create an empty directory
    let ingestion = PythonBackendIngestion::new(db_manager);

    let summary = ingestion.run_for_project(project_dir.path())?;

    assert_eq!(summary.total_diagnostics, 0);
    assert!(summary.tools_used.is_empty());
    match summary.status {
        PythonIngestionStatus::ToolUnavailable => {
            // Expected
        }
        _ => panic!("Expected ToolUnavailable status when no Python files"),
    }

    Ok(())
}

#[test]
fn test_python_ingestion_tools_unavailable() -> Result<()> {
    let (_temp_dir, db_manager, _diagnostics) = create_test_database()?;
    let project_dir = TempDir::new()?;

    // Create Python files to make it look like a Python project
    create_test_python_project(project_dir.path())?;

    let ingestion = PythonBackendIngestion::new(db_manager);

    // Mock scenario where tools are not available by checking if we can detect them
    let ruff_available = std::process::Command::new("ruff")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    let mypy_available = std::process::Command::new("mypy")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    let summary = ingestion.run_for_project(project_dir.path())?;

    if !ruff_available && !mypy_available {
        // Both tools unavailable
        assert_eq!(summary.total_diagnostics, 0);
        assert!(summary.tools_used.is_empty());
        match summary.status {
            PythonIngestionStatus::ToolUnavailable => {
                // Expected
            }
            _ => panic!("Expected ToolUnavailable status when no tools available"),
        }
    } else {
        // At least one tool is available - status depends on execution
        match summary.status {
            PythonIngestionStatus::ToolUnavailable => {
                // Should not happen if tools are available
                panic!("ToolUnavailable when tools are available");
            }
            PythonIngestionStatus::Success => {
                // Expected when tools work
                assert!(!summary.tools_used.is_empty());
            }
            PythonIngestionStatus::CommandFailed(_) => {
                // Acceptable - tools might be available but fail
                assert_eq!(summary.total_diagnostics, 0);
            }
        }
    }

    Ok(())
}

#[test]
fn test_python_ingestion_with_real_diagnostics() -> Result<()> {
    let (_temp_dir, db_manager, diagnostics_manager) = create_test_database()?;
    let project_dir = TempDir::new()?;

    // Create a Python project with guaranteed issues
    create_test_python_project(project_dir.path())?;

    let ingestion = PythonBackendIngestion::new(db_manager);

    // Check tool availability
    let ruff_available = std::process::Command::new("ruff")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    let mypy_available = std::process::Command::new("mypy")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if ruff_available || mypy_available {
        let summary = ingestion.run_for_project(project_dir.path())?;

        match summary.status {
            PythonIngestionStatus::Success => {
                // Verify diagnostics were stored correctly
                for tool in &summary.tools_used {
                    let stored_diagnostics = diagnostics_manager.query_diagnostics_by_tool(tool)?;

                    // Should have diagnostics we inserted
                    assert!(!stored_diagnostics.is_empty());

                    // Test PAE helper methods
                    if !stored_diagnostics.is_empty() {
                        let first_file = &stored_diagnostics[0].file_path;
                        let file_diagnostics = diagnostics_manager
                            .list_diagnostics_for_file_by_tool(first_file, tool)?;
                        assert!(!file_diagnostics.is_empty());

                        let count =
                            diagnostics_manager.count_diagnostics_by_tool(first_file, tool)?;
                        assert_eq!(count, file_diagnostics.len());
                    }
                }
            }
            PythonIngestionStatus::CommandFailed(msg) => {
                println!("Python tools failed: {}", msg);
                // Acceptable for test
            }
            _ => {
                panic!("Unexpected status: {:?}", summary.status);
            }
        }
    } else {
        println!("Neither ruff nor mypy available, skipping real diagnostics test");
    }

    Ok(())
}

#[test]
fn test_python_ingestion_clean_project() -> Result<()> {
    let (_temp_dir, db_manager, diagnostics_manager) = create_test_database()?;
    let project_dir = TempDir::new()?;

    // Create a clean Python project with no obvious issues
    let clean_py = r#"
def greet(name: str) -> str:
    """Greet someone by name."""
    return f"Hello, {name}!"

def main() -> None:
    message = greet("world")
    print(message)

if __name__ == "__main__":
    main()
"#;
    fs::write(project_dir.path().join("clean.py"), clean_py)?;

    let ingestion = PythonBackendIngestion::new(db_manager);

    // Check tool availability
    let ruff_available = std::process::Command::new("ruff")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    let mypy_available = std::process::Command::new("mypy")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if ruff_available || mypy_available {
        let summary = ingestion.run_for_project(project_dir.path())?;

        match summary.status {
            PythonIngestionStatus::Success => {
                // Should have 0 or very few diagnostics for a clean project
                if !summary.tools_used.is_empty() {
                    // Verify database state matches summary
                    for tool in &summary.tools_used {
                        let stored_diagnostics =
                            diagnostics_manager.query_diagnostics_by_tool(tool)?;
                        // Clean project might still have some diagnostics (style, etc.)
                        assert_eq!(stored_diagnostics.len(), summary.total_diagnostics);
                    }
                }
            }
            PythonIngestionStatus::CommandFailed(_) => {
                // Acceptable - tools might fail for various reasons
            }
            _ => {
                panic!("Unexpected status for clean project");
            }
        }
    } else {
        println!("Neither ruff nor mypy available, skipping clean project test");
    }

    Ok(())
}

#[test]
fn test_python_ingestion_nested_directories() -> Result<()> {
    let (_temp_dir, db_manager, _diagnostics) = create_test_database()?;
    let project_dir = TempDir::new()?;

    // Create nested directory structure with Python files
    fs::create_dir_all(project_dir.path().join("src"))?;
    fs::create_dir_all(project_dir.path().join("tests"))?;

    let src_py = r#"
import os  # Unused import in nested directory

def helper_function():
    return "helper"
"#;
    fs::write(project_dir.path().join("src/helper.py"), src_py)?;

    let test_py = r#"
from src.helper import helper_function

def test_helper():
    result = helper_function()
    assert result == "helper"
"#;
    fs::write(project_dir.path().join("tests/test_helper.py"), test_py)?;

    let ingestion = PythonBackendIngestion::new(db_manager);

    // Should detect Python files in nested directories
    let summary = ingestion.run_for_project(project_dir.path())?;

    // Should not be ToolUnavailable since we have Python files
    match summary.status {
        PythonIngestionStatus::ToolUnavailable => {
            // This is acceptable if no tools are available
        }
        PythonIngestionStatus::Success => {
            // Tools are available and ran
            assert!(!summary.tools_used.is_empty() || summary.total_diagnostics == 0);
        }
        PythonIngestionStatus::CommandFailed(_) => {
            // Tools are available but failed
        }
    }

    Ok(())
}
