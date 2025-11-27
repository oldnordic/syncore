//! Python Backend Ingestion Layer
//!
//! Provides diagnostics ingestion from Python tools (ruff, mypy) and stores them
//! via DiagnosticsManager.

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use crate::db::DbManager;
use crate::project_analysis::diagnostics::{DiagnosticInput, DiagnosticsManager};

/// Python ingestion status
#[derive(Debug, Clone)]
pub enum PythonIngestionStatus {
    Success,
    ToolUnavailable,
    CommandFailed(String),
}

/// Python ingestion summary
#[derive(Debug, Clone)]
pub struct PythonIngestionSummary {
    pub total_diagnostics: usize,
    pub tools_used: Vec<String>,
    pub status: PythonIngestionStatus,
}

/// Python backend ingestion wrapper
pub struct PythonBackendIngestion {
    diagnostics: Arc<DiagnosticsManager>,
}

impl PythonBackendIngestion {
    /// Create new python backend ingestion
    pub fn new(db_manager: Arc<DbManager>) -> Self {
        Self {
            diagnostics: Arc::new(DiagnosticsManager::new(db_manager)),
        }
    }

    /// Run python diagnostics for a project
    pub fn run_for_project(&self, project_root: &Path) -> Result<PythonIngestionSummary> {
        // Check if project contains Python files
        if !self.has_python_files(project_root)? {
            return Ok(PythonIngestionSummary {
                total_diagnostics: 0,
                tools_used: vec![],
                status: PythonIngestionStatus::ToolUnavailable,
            });
        }

        let mut total_diagnostics = 0;
        let mut tools_used = Vec::new();
        let mut overall_status = PythonIngestionStatus::Success;

        // Try ruff
        if self.is_tool_available("ruff") {
            match self.run_ruff_diagnostics(project_root) {
                Ok(count) => {
                    total_diagnostics += count;
                    tools_used.push("ruff".to_string());
                }
                Err(e) => {
                    overall_status =
                        PythonIngestionStatus::CommandFailed(format!("ruff failed: {}", e));
                }
            }
        }

        // Try mypy
        if self.is_tool_available("mypy") {
            match self.run_mypy_diagnostics(project_root) {
                Ok(count) => {
                    total_diagnostics += count;
                    tools_used.push("mypy".to_string());
                }
                Err(e) => {
                    overall_status =
                        PythonIngestionStatus::CommandFailed(format!("mypy failed: {}", e));
                }
            }
        }

        // If no tools were available
        if tools_used.is_empty() {
            return Ok(PythonIngestionSummary {
                total_diagnostics: 0,
                tools_used: vec![],
                status: PythonIngestionStatus::ToolUnavailable,
            });
        }

        Ok(PythonIngestionSummary {
            total_diagnostics,
            tools_used,
            status: overall_status,
        })
    }

    /// Check if project contains Python files
    fn has_python_files(&self, project_root: &Path) -> Result<bool> {
        let entries = std::fs::read_dir(project_root)
            .with_context(|| format!("Failed to read directory: {}", project_root.display()))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "py" {
                        return Ok(true);
                    }
                }
            } else if path.is_dir() {
                // Recursively check subdirectories (but limit depth to avoid infinite loops)
                if path
                    .file_name()
                    .map_or(false, |name| name != "target" && name != "node_modules")
                    && self.has_python_files(&path)?
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if a tool is available
    fn is_tool_available(&self, tool: &str) -> bool {
        Command::new(tool)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Run ruff diagnostics
    fn run_ruff_diagnostics(&self, project_root: &Path) -> Result<usize> {
        let output = Command::new("ruff")
            .args(["check", "--format=json"])
            .current_dir(project_root)
            .output()
            .context("Failed to run ruff")?;

        if !output.status.success() && !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("ruff failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut diagnostic_inputs = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<Value>(line) {
                Ok(json) => {
                    if let Some(diagnostic) = self.parse_ruff_diagnostic(json, project_root)? {
                        diagnostic_inputs.push(diagnostic);
                    }
                }
                Err(_) => {
                    // Skip lines that aren't valid JSON
                    continue;
                }
            }
        }

        let count = if diagnostic_inputs.is_empty() {
            0
        } else {
            self.diagnostics.store_diagnostics(&diagnostic_inputs)?
        };

        Ok(count)
    }

    /// Run mypy diagnostics
    fn run_mypy_diagnostics(&self, project_root: &Path) -> Result<usize> {
        let output = Command::new("mypy")
            .args(["--show-error-codes", "--no-error-summary"])
            .current_dir(project_root)
            .output()
            .context("Failed to run mypy")?;

        // mypy returns non-zero exit code when it finds issues, but still produces output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined_output = format!("{}\n{}", stdout, stderr);

        let mut diagnostic_inputs = Vec::new();

        for line in combined_output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Some(diagnostic) = self.parse_mypy_diagnostic_line(line, project_root)? {
                diagnostic_inputs.push(diagnostic);
            }
        }

        let count = if diagnostic_inputs.is_empty() {
            0
        } else {
            self.diagnostics.store_diagnostics(&diagnostic_inputs)?
        };

        Ok(count)
    }

    /// Parse a single ruff diagnostic from JSON
    fn parse_ruff_diagnostic(
        &self,
        json: Value,
        project_root: &Path,
    ) -> Result<Option<DiagnosticInput>> {
        let filename = json.get("filename").and_then(|v| v.as_str()).unwrap_or("");
        if !filename.ends_with(".py") {
            return Ok(None);
        }

        let line = json
            .get("location")
            .and_then(|loc| loc.get("row"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;

        let column = json
            .get("location")
            .and_then(|loc| loc.get("column"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;

        let message = json
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown message")
            .to_string();

        let code = json
            .get("code")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let severity = match json.get("level").and_then(|v| v.as_str()) {
            Some("error") => "error",
            Some("warning") => "warning",
            Some("info") => "note",
            _ => "warning",
        };

        let file_path = if filename.starts_with('/') {
            filename.to_string()
        } else {
            project_root.join(filename).to_string_lossy().to_string()
        };

        Ok(Some(DiagnosticInput {
            file_path,
            line,
            column,
            severity: severity.to_string(),
            tool: "ruff".to_string(),
            code,
            message,
        }))
    }

    /// Parse a mypy diagnostic from text line
    fn parse_mypy_diagnostic_line(
        &self,
        line: &str,
        project_root: &Path,
    ) -> Result<Option<DiagnosticInput>> {
        // mypy format: filename:line:severity: message [error-code]
        // Example: main.py:5: error: Incompatible types in assignment  [assignment]
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 4 {
            return Ok(None);
        }

        let filename = parts[0].trim();
        if !filename.ends_with(".py") {
            return Ok(None);
        }

        let line_num = parts[1].trim().parse::<u32>().unwrap_or(1);
        let severity_part = parts[2].trim();
        let message_part = parts[3].trim();

        let (severity, message_with_code) = if severity_part == "error" {
            ("error", message_part)
        } else if severity_part == "warning" {
            ("warning", message_part)
        } else if severity_part == "note" {
            ("note", message_part)
        } else {
            ("warning", message_part)
        };

        // Extract error code from message if present
        let (message, code) = if let Some(code_start) = message_with_code.rfind('[') {
            if let Some(code_end) = message_with_code.rfind(']') {
                let code_str = &message_with_code[code_start + 1..code_end];
                let message = &message_with_code[..code_start].trim();
                (message.to_string(), Some(code_str.to_string()))
            } else {
                (message_with_code.to_string(), None)
            }
        } else {
            (message_with_code.to_string(), None)
        };

        let file_path = if filename.starts_with('/') {
            filename.to_string()
        } else {
            project_root.join(filename).to_string_lossy().to_string()
        };

        Ok(Some(DiagnosticInput {
            file_path,
            line: line_num,
            column: 1, // mypy doesn't provide column info
            severity: severity.to_string(),
            tool: "mypy".to_string(),
            code,
            message,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_ruff_diagnostic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project_root = temp_dir.path();

        let ingestion =
            PythonBackendIngestion::new(Arc::new(DbManager::new(":memory:", ":memory:")?));

        let json_str = r#"
        {
            "filename": "src/main.py",
            "location": {
                "row": 10,
                "column": 5
            },
            "message": "Unused import `os`",
            "code": "F401",
            "level": "error"
        }
        "#;

        let json: Value = serde_json::from_str(json_str)?;
        let diagnostic = ingestion
            .parse_ruff_diagnostic(json, project_root)?
            .expect("Should parse diagnostic");

        assert_eq!(
            diagnostic.file_path,
            project_root.join("src/main.py").to_string_lossy()
        );
        assert_eq!(diagnostic.line, 10);
        assert_eq!(diagnostic.column, 5);
        assert_eq!(diagnostic.severity, "error");
        assert_eq!(diagnostic.tool, "ruff");
        assert_eq!(diagnostic.code, Some("F401".to_string()));
        assert_eq!(diagnostic.message, "Unused import `os`");

        Ok(())
    }

    #[test]
    fn test_parse_mypy_diagnostic_line() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project_root = temp_dir.path();

        let ingestion =
            PythonBackendIngestion::new(Arc::new(DbManager::new(":memory:", ":memory:")?));

        let line = "main.py:15: error: Incompatible types in assignment  [assignment]";
        let diagnostic = ingestion
            .parse_mypy_diagnostic_line(line, project_root)?
            .expect("Should parse diagnostic");

        assert_eq!(
            diagnostic.file_path,
            project_root.join("main.py").to_string_lossy()
        );
        assert_eq!(diagnostic.line, 15);
        assert_eq!(diagnostic.column, 1);
        assert_eq!(diagnostic.severity, "error");
        assert_eq!(diagnostic.tool, "mypy");
        assert_eq!(diagnostic.code, Some("assignment".to_string()));
        assert_eq!(diagnostic.message, "Incompatible types in assignment");

        Ok(())
    }

    #[test]
    fn test_has_python_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let ingestion =
            PythonBackendIngestion::new(Arc::new(DbManager::new(":memory:", ":memory:")?));

        // Empty directory should return false
        assert!(!ingestion.has_python_files(temp_dir.path())?);

        // Create a Python file
        fs::write(temp_dir.path().join("test.py"), "print('hello')")?;
        assert!(ingestion.has_python_files(temp_dir.path())?);

        Ok(())
    }

    #[test]
    fn test_tool_unavailable_no_python_files() -> Result<()> {
        let (_temp_dir, db_manager, _diagnostics) = create_test_database()?;
        let project_dir = TempDir::new()?;

        // Don't create any Python files
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

    fn create_test_database() -> Result<(
        TempDir,
        Arc<DbManager>,
        crate::project_analysis::diagnostics::DiagnosticsManager,
    )> {
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

        let diagnostics_manager =
            crate::project_analysis::diagnostics::DiagnosticsManager::new(db_manager.clone());

        Ok((temp_dir, db_manager, diagnostics_manager))
    }
}
