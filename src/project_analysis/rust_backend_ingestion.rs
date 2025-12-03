//! Rust Backend Ingestion Layer
//!
//! Provides a clean API to run Rust diagnostics (clippy) and store them
//! via DiagnosticsManager without adding new MCP tools.

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use crate::db::DbManager;
use crate::project_analysis::diagnostics::{DiagnosticInput, DiagnosticsManager};

/// Rust ingestion status
#[derive(Debug, Clone)]
pub enum RustIngestionStatus {
    Success,
    ToolUnavailable,
    CommandFailed(String),
}

/// Rust ingestion summary
#[derive(Debug, Clone)]
pub struct RustIngestionSummary {
    pub total_diagnostics: usize,
    pub tool: String,
    pub status: RustIngestionStatus,
}

/// Rust backend ingestion wrapper
pub struct RustBackendIngestion {
    diagnostics: Arc<DiagnosticsManager>,
}

impl RustBackendIngestion {
    /// Create new rust backend ingestion
    pub fn new(db_manager: Arc<DbManager>) -> Self {
        Self {
            diagnostics: Arc::new(DiagnosticsManager::new(db_manager)),
        }
    }

    /// Run rust diagnostics for a project
    pub fn run_for_project(&self, project_root: &Path) -> Result<RustIngestionSummary> {
        // Check if Cargo.toml exists
        let cargo_toml = project_root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Ok(RustIngestionSummary {
                total_diagnostics: 0,
                tool: "clippy".to_string(),
                status: RustIngestionStatus::ToolUnavailable,
            });
        }

        // Check if cargo clippy is available
        let check_output =
            Command::new("cargo").args(["clippy", "--version"]).current_dir(project_root).output();

        match check_output {
            Ok(output) if output.status.success() => {
                // clippy is available, run diagnostics
                self.run_clippy_diagnostics(project_root)
            }
            Ok(_) => {
                // clippy command failed
                Ok(RustIngestionSummary {
                    total_diagnostics: 0,
                    tool: "clippy".to_string(),
                    status: RustIngestionStatus::ToolUnavailable,
                })
            }
            Err(_e) => {
                // cargo command not found
                Ok(RustIngestionSummary {
                    total_diagnostics: 0,
                    tool: "clippy".to_string(),
                    status: RustIngestionStatus::ToolUnavailable,
                })
            }
        }
    }

    /// Run cargo clippy and parse diagnostics
    fn run_clippy_diagnostics(&self, project_root: &Path) -> Result<RustIngestionSummary> {
        let output = Command::new("cargo")
            .args(["clippy", "--message-format=json", "--quiet"])
            .current_dir(project_root)
            .output()
            .context("Failed to run cargo clippy")?;

        // Check if cargo clippy ran (it may return non-zero exit code but still produce output)
        let stdout = String::from_utf8_lossy(&output.stdout);
        let _stderr = String::from_utf8_lossy(&output.stderr);

        // Parse JSON output line by line
        let mut diagnostic_inputs = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Try to parse each line as JSON
            match serde_json::from_str::<Value>(line) {
                Ok(json) => {
                    if let Some(diagnostic) = self.parse_clippy_diagnostic(json, project_root)? {
                        diagnostic_inputs.push(diagnostic);
                    }
                }
                Err(_) => {
                    // Skip lines that aren't valid JSON
                    continue;
                }
            }
        }

        // Store diagnostics using the new unified API
        let total_diagnostics = if diagnostic_inputs.is_empty() {
            0
        } else {
            match self.diagnostics.store_diagnostics(&diagnostic_inputs) {
                Ok(count) => count,
                Err(e) => {
                    return Ok(RustIngestionSummary {
                        total_diagnostics: 0,
                        tool: "clippy".to_string(),
                        status: RustIngestionStatus::CommandFailed(format!(
                            "Failed to store diagnostics: {}",
                            e
                        )),
                    });
                }
            }
        };

        let status = if output.status.success() {
            RustIngestionStatus::Success
        } else {
            // clippy found issues but still produced diagnostics
            RustIngestionStatus::Success
        };

        Ok(RustIngestionSummary {
            total_diagnostics,
            tool: "clippy".to_string(),
            status,
        })
    }

    /// Parse a single Clippy diagnostic from JSON into DiagnosticInput
    fn parse_clippy_diagnostic(
        &self,
        json: Value,
        project_root: &Path,
    ) -> Result<Option<DiagnosticInput>> {
        // Check if this is a diagnostic message
        if json.get("message").is_none() {
            return Ok(None);
        }

        // Extract basic fields
        let message_obj = &json["message"];
        let message = message_obj["message"].as_str().unwrap_or("Unknown message").to_string();

        let level = json["level"].as_str().unwrap_or("warning");

        let severity = match level {
            "error" => "error",
            "warning" => "warning",
            "note" => "note",
            "help" => "note",
            _ => "warning",
        };

        // Get file path and line number
        let empty_vec = vec![];
        let spans = message_obj["spans"].as_array().unwrap_or(&empty_vec);
        if spans.is_empty() {
            return Ok(None);
        }

        let span = &spans[0];
        let file_path_raw = span["file_name"].as_str().unwrap_or("");

        // Skip non-Rust files
        if !file_path_raw.ends_with(".rs") {
            return Ok(None);
        }

        // Convert to absolute path if relative
        let file_path = if file_path_raw.starts_with('/') {
            file_path_raw.to_string()
        } else {
            project_root.join(file_path_raw).to_string_lossy().to_string()
        };

        let line = span["line_start"].as_u64().unwrap_or(1) as u32;
        let column = span["column_start"].as_u64().unwrap_or(1) as u32;

        // Extract diagnostic code from message or use a default
        let code = if message.contains("dead_code") {
            Some("clippy::dead_code".to_string())
        } else if message.contains("unused") {
            Some("clippy::unused".to_string())
        } else if let Some(code) = json["code"].as_object() {
            code["code"].as_str().map(|s| s.to_string())
        } else {
            None
        };

        let diagnostic_input = DiagnosticInput {
            file_path,
            line,
            column,
            severity: severity.to_string(),
            tool: "clippy".to_string(),
            code,
            message,
        };

        Ok(Some(diagnostic_input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    #[test]
    fn test_parse_clippy_diagnostic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project_root = temp_dir.path();

        let ingestion =
            RustBackendIngestion::new(Arc::new(DbManager::new(":memory:", ":memory:")?));

        let json_str = "
        {
            \"message\": {
                \"message\": \"unused variable: `x`\",
                \"level\": \"warning\",
                \"spans\": [
                    {
                        \"file_name\": \"src/main.rs\",
                        \"line_start\": 10,
                        \"column_start\": 5,
                        \"line_end\": 10,
                        \"column_end\": 6
                    }
                ]
            },
            \"level\": \"warning\",
            \"code\": {
                \"code\": \"unused_variables\"
            }
        }
        ";

        let json: Value = serde_json::from_str(json_str)?;
        let diagnostic = ingestion
            .parse_clippy_diagnostic(json, project_root)?
            .expect("Should parse diagnostic");

        assert_eq!(diagnostic.file_path, project_root.join("src/main.rs").to_string_lossy());
        assert_eq!(diagnostic.line, 10);
        assert_eq!(diagnostic.column, 5);
        assert_eq!(diagnostic.severity, "warning");
        assert_eq!(diagnostic.tool, "clippy");
        assert_eq!(diagnostic.code, Some("unused_variables".to_string()));
        assert_eq!(diagnostic.message, "unused variable: `x`");

        Ok(())
    }

    #[test]
    fn test_skip_non_rust_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project_root = temp_dir.path();

        let ingestion =
            RustBackendIngestion::new(Arc::new(DbManager::new(":memory:", ":memory:")?));

        let json_str = "
        {
            \"message\": {
                \"message\": \"some warning\",
                \"level\": \"warning\",
                \"spans\": [
                    {
                        \"file_name\": \"Cargo.toml\",
                        \"line_start\": 1,
                        \"column_start\": 1,
                        \"line_end\": 1,
                        \"column_end\": 1
                    }
                ]
            },
            \"level\": \"warning\"
        }
        ";

        let json: Value = serde_json::from_str(json_str)?;
        let diagnostic = ingestion.parse_clippy_diagnostic(json, project_root)?;

        // Should skip non-.rs files
        assert!(diagnostic.is_none());

        Ok(())
    }

    #[test]
    fn test_tool_unavailable_no_cargo_toml() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let ingestion =
            RustBackendIngestion::new(Arc::new(DbManager::new(":memory:", ":memory:")?));

        let summary = ingestion.run_for_project(temp_dir.path())?;

        assert_eq!(summary.total_diagnostics, 0);
        assert_eq!(summary.tool, "clippy");
        match summary.status {
            RustIngestionStatus::ToolUnavailable => {
                // Expected
            }
            _ => panic!("Expected ToolUnavailable status"),
        }

        Ok(())
    }
}
