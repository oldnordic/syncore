use std::path::Path;
use std::process::Command;
use anyhow::{anyhow, Result};
use serde_json::Value;
use serde_yaml;
// Use Path from earlier import

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub severity: String,
    pub rule: Option<String>,
    pub source: String,
    pub fixes: Vec<Fix>,
}

#[derive(Debug, Clone)]
pub struct Fix {
    pub file_path: String,
    pub offset: u32,
    pub length: u32,
    pub replacement: String,
}

pub struct CppDiagnostics {
    #[allow(dead_code)]
    workspace_root: Option<String>,
}

impl CppDiagnostics {
    pub fn new() -> Self {
        Self {
            workspace_root: None,
        }
    }

    #[allow(dead_code)]
    pub fn set_workspace(&mut self, workspace: &str) {
        self.workspace_root = Some(workspace.to_string());
    }

    pub fn run_clangd(&mut self, file_path: &str, mock_response: Option<Value>) -> Result<Vec<Diagnostic>> {
        if let Some(mock) = mock_response {
            return self.parse_clangd_mock_response(&mock, file_path);
        }

        // Check if clangd is available
        if !self.check_clangd_available() {
            return Ok(Vec::new()); // Return empty list if clangd is not available
        }

        // For this implementation, we'll return empty diagnostics
        // In a real implementation, you would launch clangd and communicate via LSP protocol
        let diagnostics = Vec::new();

        Ok(diagnostics)
    }

    pub fn run_clang_tidy(&mut self, file_path: &str, mock_response: Option<String>) -> Result<Vec<Diagnostic>> {
        if let Some(mock) = mock_response {
            return self.parse_clang_tidy_mock_response(&mock, file_path);
        }

        // Check if clang-tidy is available
        if !self.check_clang_tidy_available() {
            return Ok(Vec::new()); // Return empty list if clang-tidy is not available
        }

        // Create a temporary file for fixes
        let temp_fixes_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow!("Failed to create temp file: {}", e))?;
        let fixes_path = temp_fixes_file.path().to_string_lossy().to_string();

        // Run clang-tidy
        let output = std::process::Command::new("clang-tidy")
            .arg(file_path)
            .arg("--export-fixes")
            .arg(&fixes_path)
            .output()
            .map_err(|e| anyhow!("Failed to run clang-tidy: {}", e))?;

        let _stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse YAML output
        let mut diagnostics = Vec::new();
        if !output.status.success() && !stderr.contains("No config file") {
            // Check if we have fixes to parse
            if Path::new(&fixes_path).exists() {
                let fixes_content = std::fs::read_to_string(&fixes_path)?;
                if let Ok(parsed_fixes) = self.parse_clang_tidy_fixes(&fixes_content, file_path) {
                    diagnostics.extend(parsed_fixes);
                }
            }
        }

        // Parse diagnostics from stderr
        diagnostics.extend(self.parse_clang_tidy_stderr(&stderr, file_path));

        Ok(diagnostics)
    }

    fn check_clangd_available(&self) -> bool {
        Command::new("clangd")
            .arg("--version")
            .output()
            .is_ok()
    }

    fn check_clang_tidy_available(&self) -> bool {
        Command::new("clang-tidy")
            .arg("--version")
            .output()
            .is_ok()
    }

    fn parse_clangd_mock_response(&self, mock: &Value, file_path: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();

        if let Some(diags) = mock.get("params")
            .and_then(|p| p.get("diagnostics"))
            .and_then(|d| d.as_array()) {

            for diag in diags {
                if let Ok(parsed) = self.parse_clangd_diagnostic(diag, file_path) {
                    diagnostics.push(parsed);
                }
            }
        }

        Ok(diagnostics)
    }

    fn parse_clangd_diagnostic(&self, diag: &Value, file_path: &str) -> Result<Diagnostic> {
        let range = diag.get("range")
            .ok_or_else(|| anyhow!("Missing range in diagnostic"))?;

        let start = range.get("start")
            .ok_or_else(|| anyhow!("Missing start in range"))?;

        let line = start.get("line")
            .and_then(|l| l.as_u64())
            .ok_or_else(|| anyhow!("Missing or invalid line"))? as u32;

        let column = start.get("character")
            .and_then(|c| c.as_u64())
            .ok_or_else(|| anyhow!("Missing or invalid character"))? as u32;

        let message = diag.get("message")
            .and_then(|m| m.as_str())
            .ok_or_else(|| anyhow!("Missing message"))?
            .to_string();

        let severity = diag.get("severity")
            .and_then(|s| s.as_u64())
            .map(|s| match s {
                1 => "error",
                2 => "warning",
                3 => "info",
                4 => "hint",
                _ => "unknown"
            })
            .unwrap_or("unknown")
            .to_string();

        let rule = diag.get("code")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        Ok(Diagnostic {
            file_path: file_path.to_string(),
            line,
            column,
            message,
            severity,
            rule,
            source: "clangd".to_string(),
            fixes: Vec::new(),
        })
    }

    fn parse_clang_tidy_mock_response(&self, mock: &str, file_path: &str) -> Result<Vec<Diagnostic>> {
        // Parse YAML format
        let yaml_docs: Vec<Value> = serde_yaml::from_str(mock)
            .map_err(|e| anyhow!("Failed to parse YAML: {}", e))?;

        let mut diagnostics = Vec::new();
        for doc in yaml_docs {
            if let Some(diag_array) = doc.get("Diagnostics").and_then(|d| d.as_array()) {
                for diag in diag_array {
                    if let Ok(parsed) = self.parse_clang_tidy_yaml_diagnostic(diag, file_path) {
                        diagnostics.push(parsed);
                    }
                }
            }
        }

        Ok(diagnostics)
    }

    fn parse_clang_tidy_stderr(&self, stderr: &str, file_path: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Parse clang-tidy output format:
        // file_path:line:column: warning: message [rule]
        for line in stderr.lines() {
            if line.starts_with(file_path) {
                let parts: Vec<&str> = line.splitn(5, ':').collect();
                if parts.len() >= 4 {
                    let line_num = parts[1].trim().parse::<u32>().unwrap_or(0);
                    let column = parts[2].trim().parse::<u32>().unwrap_or(0);
                    let remainder = parts[3..].join(":");

                    if let Some(message_end) = remainder.find('[') {
                        let message = remainder[..message_end].trim();
                        let rule_part = remainder[message_end..].trim();

                        let rule = if rule_part.starts_with('[') && rule_part.ends_with(']') {
                            Some(rule_part[1..rule_part.len()-1].to_string())
                        } else {
                            None
                        };

                        let severity = if remainder.contains("error") {
                            "error"
                        } else {
                            "warning"
                        }.to_string();

                        diagnostics.push(Diagnostic {
                            file_path: file_path.to_string(),
                            line: line_num,
                            column,
                            message: message.to_string(),
                            severity,
                            rule,
                            source: "clang-tidy".to_string(),
                            fixes: Vec::new(),
                        });
                    }
                }
            }
        }

        diagnostics
    }

    fn parse_clang_tidy_yaml_diagnostic(&self, diag: &Value, file_path: &str) -> Result<Diagnostic> {
        let name = diag.get("DiagnosticName")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow!("Missing DiagnosticName"))?;

        let message = diag.get("Message")
            .and_then(|m| m.as_str())
            .ok_or_else(|| anyhow!("Missing Message"))?
            .to_string();

        let file_offset = diag.get("FileOffset")
            .and_then(|f| f.as_u64())
            .ok_or_else(|| anyhow!("Missing FileOffset"))? as u32;

        // Convert file offset to line and column
        let content = std::fs::read_to_string(file_path)?;
        let (line, column) = self.offset_to_line_column(&content, file_offset);

        let mut fixes = Vec::new();
        if let Some(replacements) = diag.get("Replacements").and_then(|r| r.as_array()) {
            for repl in replacements {
                if let Ok(fix) = self.parse_clang_tidy_fix(repl, file_path) {
                    fixes.push(fix);
                }
            }
        }

        Ok(Diagnostic {
            file_path: file_path.to_string(),
            line,
            column,
            message: format!("{} [{}]", message, name),
            severity: "warning".to_string(), // Default to warning for clang-tidy
            rule: Some(name.to_string()),
            source: "clang-tidy".to_string(),
            fixes,
        })
    }

    fn parse_clang_tidy_fixes(&self, fixes_content: &str, file_path: &str) -> Result<Vec<Diagnostic>> {
        let yaml_value: Value = serde_yaml::from_str(fixes_content)
            .map_err(|e| anyhow!("Failed to parse fixes YAML: {}", e))?;

        let mut diagnostics = Vec::new();

        if let Some(_main_source_file) = yaml_value.get("MainSourceFile").and_then(|f| f.as_str()) {
            if let Some(diags) = yaml_value.get("Diagnostics").and_then(|d| d.as_array()) {
                for diag in diags {
                    if let Ok(parsed) = self.parse_clang_tidy_yaml_diagnostic(diag, file_path) {
                        diagnostics.push(parsed);
                    }
                }
            }
        }

        Ok(diagnostics)
    }

    fn parse_clang_tidy_fix(&self, fix: &Value, file_path: &str) -> Result<Fix> {
        let file_path = fix.get("FilePath")
            .and_then(|p| p.as_str())
            .unwrap_or(file_path)
            .to_string();

        let offset = fix.get("Offset")
            .and_then(|o| o.as_u64())
            .ok_or_else(|| anyhow!("Missing Offset"))? as u32;

        let length = fix.get("Length")
            .and_then(|l| l.as_u64())
            .ok_or_else(|| anyhow!("Missing Length"))? as u32;

        let replacement = fix.get("ReplacementText")
            .and_then(|r| r.as_str())
            .ok_or_else(|| anyhow!("Missing ReplacementText"))?
            .to_string();

        Ok(Fix {
            file_path,
            offset,
            length,
            replacement,
        })
    }

    fn offset_to_line_column(&self, content: &str, offset: u32) -> (u32, u32) {
        let offset = offset as usize;
        if offset > content.len() {
            return (0, 0);
        }

        let (mut line, mut column) = (1, 0);
        for (i, c) in content.char_indices() {
            if i >= offset {
                break;
            }
            if c == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }

        (line, column)
    }
}

impl Default for Diagnostic {
    fn default() -> Self {
        Diagnostic {
            file_path: String::new(),
            line: 0,
            column: 0,
            message: String::new(),
            severity: "unknown".to_string(),
            rule: None,
            source: "unknown".to_string(),
            fixes: Vec::new(),
        }
    }
}
