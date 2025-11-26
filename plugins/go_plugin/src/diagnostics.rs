use anyhow::{Context, Result};
use crate::plugin_api::{Diagnostic, Severity, PluginResult};
use serde_json::Value;
use std::process::Command;

pub struct GoDiagnostics {
    staticcheck_available: bool,
    govet_available: bool,
}

impl GoDiagnostics {
    pub fn new() -> Result<Self> {
        let staticcheck_available = Command::new("staticcheck")
            .arg("--version")
            .output()
            .map(|_| true)
            .unwrap_or(false);

        let govet_available = Command::new("go")
            .arg("vet")
            .arg("-h")
            .output()
            .map(|_| true)
            .unwrap_or(false);

        Ok(Self {
            staticcheck_available,
            govet_available,
        })
    }

    pub fn run_diagnostics(&self, file_path: &str) -> Result<PluginResult> {
        let mut diagnostics = Vec::new();

        if self.staticcheck_available {
            if let Ok(staticcheck_diagnostics) = self.run_staticcheck(file_path) {
                diagnostics.extend(staticcheck_diagnostics);
            }
        } else if self.govet_available {
            if let Ok(govet_diagnostics) = self.run_govet(file_path) {
                diagnostics.extend(govet_diagnostics);
            }
        }

        Ok(PluginResult {
            entities: None,
            edges: None,
            diagnostics: Some(diagnostics),
            meta: None,
        })
    }

    pub fn run_staticcheck(&self, file_path: &str) -> Result<Vec<Diagnostic>> {
        let output = Command::new("staticcheck")
            .arg("-f")
            .arg("json")
            .arg(file_path)
            .output()
            .context("Failed to run staticcheck")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut diagnostics = Vec::new();

        for line in stdout.lines() {
            if let Ok(json_value) = serde_json::from_str::<Value>(line) {
                if let Some(diagnostic) = self.parse_staticcheck_json(&json_value, file_path) {
                    diagnostics.push(diagnostic);
                }
            }
        }

        Ok(diagnostics)
    }

    fn parse_staticcheck_json(&self, json: &Value, file_path: &str) -> Option<Diagnostic> {
        let location = json.get("location")?;
        let line = location.get("line")?.as_u64()? as u32;
        let column = location.get("column")?.as_u64()? as u32;

        let severity = match json.get("severity")?.as_str()? {
            "error" => Severity::Error,
            "warning" => Severity::Warning,
            "info" => Severity::Info,
            _ => Severity::Info,
        };

        Some(Diagnostic {
            file_path: file_path.to_string(),
            line,
            column,
            severity,
            code: json.get("code")?.as_str()?.to_string(),
            message: json.get("message")?.as_str()?.to_string(),
            tool: "staticcheck".to_string(),
        })
    }

    pub fn run_govet(&self, file_path: &str) -> Result<Vec<Diagnostic>> {
        let output = Command::new("go")
            .arg("vet")
            .arg(file_path)
            .output()
            .context("Failed to run go vet")?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut diagnostics = Vec::new();

        for line in stderr.lines() {
            if let Some(diagnostic) = self.parse_govet_line(line, file_path) {
                diagnostics.push(diagnostic);
            }
        }

        Ok(diagnostics)
    }

    fn parse_govet_line(&self, line: &str, file_path: &str) -> Option<Diagnostic> {
        let re = regex::Regex::new(r"^([^:]+):(\d+):(\d+):\s+(.+)$").ok()?;
        let captures = re.captures(line)?;

        let line_num = captures.get(2)?.as_str().parse::<u32>().ok()?;
        let column = captures.get(3)?.as_str().parse::<u32>().ok()?;
        let message = captures.get(4)?.as_str();

        Some(Diagnostic {
            file_path: file_path.to_string(),
            line: line_num,
            column,
            severity: Severity::Warning,
            code: "go vet".to_string(),
            message: message.to_string(),
            tool: "go vet".to_string(),
        })
    }
}
