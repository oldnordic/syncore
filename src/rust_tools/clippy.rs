//! Clippy Integration Tool
//!
//! Runs cargo clippy and stores diagnostics in code_diagnostics table.

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use crate::db::DbManager;
use crate::path_resolver::PathResolver; // APEX v1.7 Phase 6
use crate::project_analysis::diagnostics::{CodeDiagnostic, DiagnosticsManager};

/// Run cargo clippy on a project and store diagnostics
pub fn run_clippy_and_store_diagnostics(
    db_manager: Arc<DbManager>,
    project_root: &Path,
) -> Result<usize> {
    // Run cargo clippy with JSON output
    let output = Command::new("cargo")
        .args(["clippy", "--message-format=json"])
        .current_dir(project_root)
        .output()
        .context("Failed to run cargo clippy")?;

    // Check if cargo clippy succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("cargo clippy failed: {}", stderr));
    }

    // Parse JSON output line by line
    let stdout =
        String::from_utf8(output.stdout).context("Failed to parse clippy output as UTF-8")?;

    let mut diagnostics = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Try to parse each line as JSON
        match serde_json::from_str::<Value>(line) {
            Ok(json) => {
                if let Some(diagnostic) = parse_clippy_diagnostic(json, project_root)? {
                    diagnostics.push(diagnostic);
                }
            }
            Err(_) => {
                // Skip lines that aren't valid JSON
                continue;
            }
        }
    }

    // Store diagnostics in database
    let diagnostics_manager = DiagnosticsManager::new(db_manager);
    let inserted = diagnostics_manager.insert_diagnostics(&diagnostics)?;

    Ok(inserted)
}

/// Parse a single Clippy diagnostic from JSON
fn parse_clippy_diagnostic(json: Value, project_root: &Path) -> Result<Option<CodeDiagnostic>> {
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

    let line_start = span["line_start"].as_u64().unwrap_or(1) as i64;

    // Extract diagnostic type
    let diagnostic_type = if message.contains("dead_code") {
        "clippy::dead_code".to_string()
    } else if message.contains("unused") {
        "clippy::unused".to_string()
    } else {
        "clippy::unknown".to_string()
    };

    let diagnostic = CodeDiagnostic {
        file_path,
        line_start,
        severity: severity.to_string(),
        diagnostic_type,
        message,
        tool: "clippy".to_string(),
    };

    Ok(Some(diagnostic))
}

/// Request structure for clippy scan
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ClippyScanRequest {
    pub project_root: Option<String>,
}

/// Response structure for clippy scan
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ClippyScanData {
    pub inserted: usize,
    pub project_root: String,
}

/// Run clippy scan with request/response pattern
pub fn run_clippy_scan(db: Arc<DbManager>, request: ClippyScanRequest) -> Result<ClippyScanData> {
    // APEX v1.7 Phase 6: Use PathResolver instead of current_dir()
    let project_root =
        request.project_root.map(|p| Path::new(&p).to_path_buf()).unwrap_or_else(|| {
            let mut resolver = PathResolver::new();
            resolver
                .resolve_workspace_root(Path::new("."))
                .ok()
                .flatten()
                .unwrap_or_else(|| Path::new(".").to_path_buf())
        });

    let project_root_str = project_root.to_string_lossy().to_string();

    let inserted = run_clippy_and_store_diagnostics(db, &project_root)?;

    Ok(ClippyScanData {
        inserted,
        project_root: project_root_str,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clippy_diagnostic() -> Result<()> {
        let json_str = r#"
        {
            "message": {
                "message": "unused variable: `x`",
                "level": "warning",
                "spans": [
                    {
                        "file_name": "src/main.rs",
                        "line_start": 10,
                        "line_end": 10
                    }
                ]
            },
            "level": "warning"
        }
        "#;

        let json: Value = serde_json::from_str(json_str)?;
        let project_root = Path::new("/test");

        let diagnostic =
            parse_clippy_diagnostic(json, project_root)?.expect("Should parse diagnostic");

        assert_eq!(diagnostic.file_path, "/test/src/main.rs");
        assert_eq!(diagnostic.line_start, 10);
        assert_eq!(diagnostic.severity, "warning");
        assert_eq!(diagnostic.message, "unused variable: `x`");
        assert_eq!(diagnostic.tool, "clippy");

        Ok(())
    }

    #[test]
    fn test_skip_non_rust_files() -> Result<()> {
        let json_str = r#"
        {
            "message": {
                "message": "some warning",
                "level": "warning",
                "spans": [
                    {
                        "file_name": "Cargo.toml",
                        "line_start": 1,
                        "line_end": 1
                    }
                ]
            }
        }
        "#;

        let json: Value = serde_json::from_str(json_str)?;
        let project_root = Path::new("/test");

        let diagnostic = parse_clippy_diagnostic(json, project_root)?;

        // Should skip non-.rs files
        assert!(diagnostic.is_none());

        Ok(())
    }
}
