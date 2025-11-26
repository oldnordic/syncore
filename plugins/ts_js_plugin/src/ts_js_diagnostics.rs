use anyhow::{Context, Result};
use crate::plugin_api::{Diagnostic, PluginResult, Severity};
use async_process::{Command, Stdio};
use crate::config::TsJsConfig;
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};

pub struct TsJsDiagnosticsRunner {
    tsserver_path: Option<String>,
    eslint_path: Option<String>,
    prettier_path: Option<String>,
}

impl TsJsDiagnosticsRunner {
    pub fn new(tsserver_path: Option<String>, eslint_path: Option<String>, prettier_path: Option<String>) -> Self {
        Self {
            tsserver_path,
            eslint_path,
            prettier_path,
        }
    }

    pub async fn run_tsserver_diagnostics(&self, project_root: &str) -> Result<PluginResult> {
        match &self.tsserver_path {
            Some(tsserver_path) => {
                let diagnostics = self.run_tsserver(project_root, tsserver_path).await
                    .context("Failed to run tsserver diagnostics")?;
                
                Ok(PluginResult {
                    entities: None,
                    edges: None,
                    diagnostics: Some(diagnostics),
                    meta: None,
                })
            }
            None => {
                // Skip tsserver if not configured
                Ok(PluginResult {
                    entities: None,
                    edges: None,
                    diagnostics: Some(Vec::new()),
                    meta: None,
                })
            }
        }
    }

    pub async fn run_eslint_diagnostics(&self, project_root: &str, eslint_config: Option<&str>) -> Result<PluginResult> {
        match &self.eslint_path {
            Some(eslint_path) => {
                let diagnostics = self.run_eslint(project_root, eslint_path, eslint_config).await
                    .context("Failed to run ESLint diagnostics")?;
                
                Ok(PluginResult {
                    entities: None,
                    edges: None,
                    diagnostics: Some(diagnostics),
                    meta: None,
                })
            }
            None => {
                // Skip eslint if not configured
                Ok(PluginResult {
                    entities: None,
                    edges: None,
                    diagnostics: Some(Vec::new()),
                    meta: None,
                })
            }
        }
    }

    pub async fn run_prettier_diagnostics(&self, project_root: &str) -> Result<PluginResult> {
        match &self.prettier_path {
            Some(prettier_path) => {
                let diagnostics = self.run_prettier(project_root, prettier_path).await
                    .context("Failed to run Prettier diagnostics")?;
                
                Ok(PluginResult {
                    entities: None,
                    edges: None,
                    diagnostics: Some(diagnostics),
                    meta: None,
                })
            }
            None => {
                // Skip prettier if not configured
                Ok(PluginResult {
                    entities: None,
                    edges: None,
                    diagnostics: Some(Vec::new()),
                    meta: None,
                })
            }
        }
    }

    async fn run_tsserver(&self, project_root: &str, tsserver_path: &str) -> Result<Vec<Diagnostic>> {
        // This is a simplified implementation
        // In a real implementation, you would need to communicate with tsserver via its protocol
        // For now, we'll use tsc (TypeScript compiler) as a fallback
        
        let mut cmd = Command::new("npx");
        cmd.arg("tsc")
           .arg("--noEmit")
           .arg("--skipLibCheck")
           .arg("--project")
           .arg(project_root)
           .current_dir(project_root)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let output = cmd.output().await
            .context("Failed to run TypeScript compiler")?;

        let mut diagnostics = Vec::new();
        
        // Parse TypeScript compiler output
        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            diagnostics.extend(self.parse_tsc_errors(&stderr, project_root)?);
        }

        Ok(diagnostics)
    }

    async fn run_eslint(&self, project_root: &str, eslint_path: &str, eslint_config: Option<&str>) -> Result<Vec<Diagnostic>> {
        let mut cmd = if eslint_path == "eslint" {
            Command::new("npx")
        } else {
            Command::new(eslint_path)
        };

        cmd.arg("--format=json")
           .arg(project_root);

        if let Some(config) = eslint_config {
            cmd.arg("--config").arg(config);
        }

        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let output = cmd.output().await
            .context("Failed to run ESLint")?;

        let mut diagnostics = Vec::new();
        
        if !output.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            diagnostics.extend(self.parse_eslint_output(&stdout)?);
        }

        Ok(diagnostics)
    }

    async fn run_prettier(&self, project_root: &str, prettier_path: &str) -> Result<Vec<Diagnostic>> {
        let mut cmd = if prettier_path == "prettier" {
            Command::new("npx")
        } else {
            Command::new(prettier_path)
        };

        cmd.arg("--check")
           .arg("**/*.{ts,tsx,js,jsx}")
           .current_dir(project_root)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let output = cmd.output().await
            .context("Failed to run Prettier")?;

        let mut diagnostics = Vec::new();
        
        if !output.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            diagnostics.extend(self.parse_prettier_output(&stdout, project_root)?);
        }

        Ok(diagnostics)
    }

    pub fn parse_tsc_errors(&self, output: &str, project_root: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();
        
        for line in output.lines() {
            if let Some(diagnostic) = self.parse_tsc_line(line, project_root) {
                diagnostics.push(diagnostic);
            }
        }

        Ok(diagnostics)
    }

    fn parse_tsc_line(&self, line: &str, project_root: &str) -> Option<Diagnostic> {
        // TypeScript error format: file.ts(line,col): error TS123: message
        // Split by `:` gives: [file(line,col), error/warning TSxxxx, message parts...]
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 3 {
            return None;
        }

        let file_part = parts[0].trim();
        let code_part = parts[1].trim(); // "error TS2322" or "warning TS6133"
        let message = parts[2..].join(":").trim().to_string(); // Join remaining parts (message may contain colons)

        // Parse file path and location
        let file_parts: Vec<&str> = file_part.split('(').collect();
        if file_parts.len() != 2 {
            return None;
        }

        let file_path = file_parts[0].trim();
        let location_part = file_parts[1].replace(')', "");
        let location_parts: Vec<&str> = location_part.split(',').collect();

        if location_parts.len() != 2 {
            return None;
        }

        let line = location_parts[0].trim().parse().unwrap_or(1);
        let column = location_parts[1].trim().parse().unwrap_or(1);

        // Determine severity based on code prefix
        let severity = if code_part.starts_with("error") {
            Severity::Error
        } else {
            Severity::Warning
        };

        Some(Diagnostic {
            file_path: format!("{}/{}", project_root, file_path),
            line,
            column,
            severity,
            code: code_part.to_string(),
            message,
            tool: "tsserver".to_string(),
        })
    }

    pub fn parse_eslint_output(&self, output: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();
        
        // ESLint JSON format
        let eslint_results: Vec<EslintFileResult> = serde_json::from_str(output)
            .context("Failed to parse ESLint JSON output")?;

        for file_result in eslint_results {
            let file_path = file_result.file_path;
            
            for message in file_result.messages {
                let severity = match message.severity {
                    1 => Severity::Warning,
                    2 => Severity::Error,
                    _ => Severity::Info,
                };

                let diagnostic = Diagnostic {
                    file_path: file_path.clone(),
                    line: message.line,
                    column: message.column,
                    severity,
                    code: message.rule_id.unwrap_or_else(|| "unknown".to_string()),
                    message: message.message,
                    tool: "eslint".to_string(),
                };
                
                diagnostics.push(diagnostic);
            }
        }

        Ok(diagnostics)
    }

    pub fn parse_prettier_output(&self, output: &str, project_root: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();
        
        // Prettier output format: filename.ts
        // Some code...
        
        let mut current_file = None;
        for line in output.lines() {
            if line.ends_with(".ts") || line.ends_with(".tsx") || line.ends_with(".js") || line.ends_with(".jsx") {
                current_file = Some(line.trim());
            } else if current_file.is_some() && !line.trim().is_empty() {
                // This is a formatting issue
                let diagnostic = Diagnostic {
                    file_path: format!("{}/{}", project_root, current_file.as_ref().unwrap()),
                    line: 1, // Prettier doesn't give line numbers in check mode
                    column: 1,
                    severity: Severity::Info,
                    code: "prettier-format".to_string(),
                    message: "File needs formatting".to_string(),
                    tool: "prettier".to_string(),
                };
                
                diagnostics.push(diagnostic);
                current_file = None;
            }
        }

        Ok(diagnostics)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EslintFileResult {
    file_path: String,
    messages: Vec<EslintMessage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EslintMessage {
    rule_id: Option<String>,
    message: String,
    line: u32,
    column: u32,
    severity: u32,
}