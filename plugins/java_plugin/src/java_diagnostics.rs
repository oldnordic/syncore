use crate::plugin_api::{Diagnostic, PluginResult, Severity};
use anyhow::{Context, Result};
use std::process::Command;

pub struct JavaDiagnosticsRunner {
    javac_path: String,
}

impl JavaDiagnosticsRunner {
    pub fn new(javac_path: Option<String>) -> Self {
        Self {
            javac_path: javac_path.unwrap_or_else(|| "javac".to_string()),
        }
    }

    pub fn run_compiler_diagnostics(
        &self,
        project_root: &str,
        classpath: Option<String>,
    ) -> Result<PluginResult> {
        let mut diagnostics = Vec::new();
        
        // Find all Java files
        let java_files = find_java_files(project_root)?;
        
        if java_files.is_empty() {
            return Ok(PluginResult {
                entities: None,
                edges: None,
                diagnostics: Some(diagnostics),
                meta: None,
            });
        }

        // Create temporary directory for compilation
        let temp_dir = tempfile::tempdir()
            .context("Failed to create temporary directory")?;

        // Build javac command
        let mut cmd = Command::new(&self.javac_path);
        cmd.args(&["-Xlint:all", "-d"])
           .arg(temp_dir.path());
        
        if let Some(cp) = classpath {
            cmd.arg("-cp").arg(cp);
        }

        // Add all Java files
        for file in &java_files {
            cmd.arg(file);
        }

        // Run javac and capture output
        let output = cmd.output()
            .context("Failed to run javac")?;

        // Parse javac output
        if !output.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            diagnostics.extend(parse_javac_output(&stdout, "javac"));
        }

        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            diagnostics.extend(parse_javac_output(&stderr, "javac"));
        }

        Ok(PluginResult {
            entities: None,
            edges: None,
            diagnostics: Some(diagnostics),
            meta: None,
        })
    }

    pub fn run_errorprone(
        &self,
        project_root: &str,
        errorprone_jar: &str,
        javac_path: Option<String>,
    ) -> Result<PluginResult> {
        let javac = javac_path.unwrap_or_else(|| "javac".to_string());
        let java_files = find_java_files(project_root)?;
        
        if java_files.is_empty() {
            return Ok(PluginResult {
                entities: None,
                edges: None,
                diagnostics: Some(Vec::new()),
                meta: None,
            });
        }

        // Create temporary directory for compilation
        let temp_dir = tempfile::tempdir()
            .context("Failed to create temporary directory")?;

        // Build Error Prone command
        let mut cmd = Command::new(&javac);
        cmd.args(&[
            "-Xplugin:ErrorProne",
            "-J-Xep:AllDisabledChecksAsWarnings",
            "-J-Xep:MissingOverride:ERROR",
            "-d",
        ])
        .arg(temp_dir.path())
        .arg("-processorpath")
        .arg(errorprone_jar);

        // Add all Java files
        for file in &java_files {
            cmd.arg(file);
        }

        // Run Error Prone and capture output
        let output = cmd.output()
            .context("Failed to run Error Prone")?;

        let mut diagnostics = Vec::new();

        if !output.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            diagnostics.extend(parse_errorprone_output(&stdout));
        }

        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            diagnostics.extend(parse_errorprone_output(&stderr));
        }

        Ok(PluginResult {
            entities: None,
            edges: None,
            diagnostics: Some(diagnostics),
            meta: None,
        })
    }

    pub fn run_pmd(
        &self,
        project_root: &str,
        ruleset: &str,
        pmd_bin: &str,
    ) -> Result<PluginResult> {
        let java_files = find_java_files(project_root)?;
        
        if java_files.is_empty() {
            return Ok(PluginResult {
                entities: None,
                edges: None,
                diagnostics: Some(Vec::new()),
                meta: None,
            });
        }

        // Build PMD command
        let mut cmd = Command::new(pmd_bin);
        cmd.args(&[
            "-d",
            project_root,
            "-f",
            "text",
            "-r",
            ruleset,
            "-language",
            "java",
        ]);

        // Run PMD and capture output
        let output = cmd.output()
            .context("Failed to run PMD")?;

        let mut diagnostics = Vec::new();

        if !output.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            diagnostics.extend(parse_pmd_output(&stdout, project_root));
        }

        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            diagnostics.extend(parse_pmd_output(&stderr, project_root));
        }

        Ok(PluginResult {
            entities: None,
            edges: None,
            diagnostics: Some(diagnostics),
            meta: None,
        })
    }
}

fn find_java_files(project_root: &str) -> Result<Vec<String>> {
    let mut java_files = Vec::new();
    
    for entry in walkdir::WalkDir::new(project_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "java").unwrap_or(false))
    {
        java_files.push(entry.path().to_string_lossy().to_string());
    }
    
    Ok(java_files)
}

fn parse_javac_output(output: &str, tool: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    for line in output.lines() {
        if let Some(diagnostic) = parse_javac_line(line, tool) {
            diagnostics.push(diagnostic);
        }
    }
    
    diagnostics
}

fn parse_javac_line(line: &str, tool: &str) -> Option<Diagnostic> {
    // javac output format: filename:line: error: message
    // or filename:line: warning: message
    let re = regex::Regex::new(r"^(.+?):(\d+):\s*(error|warning|info):\s*(.+)$").ok()?;
    
    let captures = re.captures(line)?;
    let file_path = captures.get(1)?.as_str().to_string();
    let line_num: u32 = captures.get(2)?.as_str().parse().ok()?;
    let severity_str = captures.get(3)?.as_str();
    let message = captures.get(4)?.as_str().to_string();
    
    let severity = match severity_str {
        "error" => Severity::Error,
        "warning" => Severity::Warning,
        "info" => Severity::Info,
        _ => Severity::Info,
    };
    
    Some(Diagnostic {
        file_path,
        line: line_num,
        column: 1, // javac doesn't always provide column info
        severity,
        code: format!("javac-{}", severity_str),
        message,
        tool: tool.to_string(),
    })
}

fn parse_errorprone_output(output: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    for line in output.lines() {
        if let Some(diagnostic) = parse_errorprone_line(line) {
            diagnostics.push(diagnostic);
        }
    }
    
    diagnostics
}

fn parse_errorprone_line(line: &str) -> Option<Diagnostic> {
    // Error Prone output format similar to javac but with specific error codes
    let re = regex::Regex::new(r"^(.+?):(\d+):\s*(error|warning|info):\s*\[([^\]]+)\]\s*(.+)$").ok()?;
    
    let captures = re.captures(line)?;
    let file_path = captures.get(1)?.as_str().to_string();
    let line_num: u32 = captures.get(2)?.as_str().parse().ok()?;
    let severity_str = captures.get(3)?.as_str();
    let code = captures.get(4)?.as_str().to_string();
    let message = captures.get(5)?.as_str().to_string();
    
    let severity = match severity_str {
        "error" => Severity::Error,
        "warning" => Severity::Warning,
        "info" => Severity::Info,
        _ => Severity::Info,
    };
    
    Some(Diagnostic {
        file_path,
        line: line_num,
        column: 1,
        severity,
        code: format!("EP-{}", code),
        message,
        tool: "errorprone".to_string(),
    })
}

fn parse_pmd_output(output: &str, project_root: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    for line in output.lines() {
        if let Some(diagnostic) = parse_pmd_line(line, project_root) {
            diagnostics.push(diagnostic);
        }
    }
    
    diagnostics
}

fn parse_pmd_line(line: &str, _project_root: &str) -> Option<Diagnostic> {
    // PMD output format: filepath:line: description [rule]
    let re = regex::Regex::new(r"^(.+?):(\d+):\s*(.+?)\s*\[([^\]]+)\]$").ok()?;
    
    let captures = re.captures(line)?;
    let file_path = captures.get(1)?.as_str().to_string();
    let line_num: u32 = captures.get(2)?.as_str().parse().ok()?;
    let message = captures.get(3)?.as_str().to_string();
    let rule = captures.get(4)?.as_str().to_string();
    
    Some(Diagnostic {
        file_path,
        line: line_num,
        column: 1,
        severity: Severity::Warning, // PMD typically produces warnings
        code: format!("PMD-{}", rule),
        message,
        tool: "pmd".to_string(),
    })
}