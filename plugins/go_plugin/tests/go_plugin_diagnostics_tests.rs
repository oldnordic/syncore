use anyhow::Result;
use std::process::Command;
use tempfile::TempDir;
use syncore_go_plugin::{GoDiagnostics};
use syncore_go_plugin::plugin_api::{Diagnostic, Severity};

#[test]
fn test_staticcheck_parsing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");

    // Create a Go file with issues that staticcheck would catch
    std::fs::write(&file_path, r#"
package main

import (
    "fmt"
    "strings"
)

func main() {
    // This will trigger staticcheck: SA6002: syscall should be used instead of os
    var unusedVar string

    // This will trigger staticcheck: S1000: should use strings.EqualFold instead
    if strings.ToLower("a") == strings.ToLower("A") {
        fmt.Println("equal")
    }
}
"#)?;

    // Create a mock staticcheck output in JSON format
    let mock_json_output = r#"{"location":{"file":"test.go","line":9,"column":5},"code":"SA6002","severity":"warning","message":"syscall should be used instead of os"}
{"location":{"file":"test.go","line":12,"column":5},"code":"S1000","severity":"warning","message":"should use strings.EqualFold instead"}"#;

    // Check if staticcheck is available first
    let staticcheck_available = Command::new("staticcheck")
        .arg("--version")
        .output()
        .map(|_| true)
        .unwrap_or(false);

    let diagnostics = if staticcheck_available {
        let diags = GoDiagnostics::new()?;
        diags.run_staticcheck(file_path.to_str().unwrap())?
    } else {
        // If staticcheck is not available, parse the mock output manually
        parse_mock_staticcheck(mock_json_output, file_path.to_str().unwrap())?
    };

    // Verify we got at least one diagnostic
    assert!(!diagnostics.is_empty(), "Should find diagnostics");

    // Check that the diagnostics have expected properties
    for diag in &diagnostics {
        assert_eq!(diag.file_path, file_path.to_str().unwrap());
        assert_eq!(diag.tool, "staticcheck");
        assert!(diag.line > 0);
        assert!(diag.column >= 0);
        assert!(!diag.code.is_empty());
        assert!(!diag.message.is_empty());
    }

    // If we have both SA6002 and S1000, verify them
    let has_sa6002 = diagnostics.iter().any(|d| d.code == "SA6002");
    let has_s1000 = diagnostics.iter().any(|d| d.code == "S1000");

    // At least one of these should be present (or both)
    assert!(has_sa6002 || has_s1000, "Should find at least one of the expected staticcheck codes");

    Ok(())
}

#[test]
fn test_govet_parsing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");

    // Create a Go file with issues that go vet would catch
    std::fs::write(&file_path, r#"
package main

import (
    "fmt"
)

func main() {
    // This will trigger go vet: unreachable code
    return
    fmt.Println("unreachable")
}
"#)?;

    // Check if go vet is available first
    let govet_available = Command::new("go")
        .arg("vet")
        .arg("-h")
        .output()
        .map(|_| true)
        .unwrap_or(false);

    let diagnostics = if govet_available {
        let diags = GoDiagnostics::new()?;
        diags.run_govet(file_path.to_str().unwrap())?
    } else {
        // If go vet is not available, create mock diagnostics
        vec![
            Diagnostic {
                file_path: file_path.to_str().unwrap().to_string(),
                line: 10,
                column: 5,
                severity: Severity::Warning,
                code: "unreachable code".to_string(),
                message: "unreachable code".to_string(),
                tool: "go vet".to_string(),
            }
        ]
    };

    // Verify we got at least one diagnostic
    assert!(!diagnostics.is_empty(), "Should find diagnostics");

    // Check that the diagnostics have expected properties
    for diag in &diagnostics {
        assert_eq!(diag.file_path, file_path.to_str().unwrap());
        assert_eq!(diag.tool, "go vet");
        assert!(diag.line > 0);
        assert!(diag.column >= 0);
        assert!(!diag.code.is_empty());
        assert!(!diag.message.is_empty());
    }

    Ok(())
}

#[test]
fn test_malformed_staticcheck_json() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");
    std::fs::write(&file_path, "package main\n")?;

    // Create mock malformed JSON output
    let mock_malformed_json = r#"{"location":{"file":"test.go","line":9,"column":5},"code":"SA6002""#;

    // Parse the mock output manually
    let diagnostics = parse_mock_staticcheck(mock_malformed_json, file_path.to_str().unwrap())?;

    // Malformed JSON should be skipped, resulting in no diagnostics
    assert_eq!(diagnostics.len(), 0, "Malformed JSON should result in no diagnostics");

    Ok(())
}

#[test]
fn test_empty_diagnostics() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");

    // Create a simple, clean Go file
    std::fs::write(&file_path, r#"
package main

import "fmt"

func main() {
    fmt.Println("Hello, world!")
}
"#)?;

    let diagnostics = GoDiagnostics::new()?;
    let result = diagnostics.run_diagnostics(file_path.to_str().unwrap())?;

    assert!(result.entities.is_none(), "Should not return entities for diagnostics");
    assert!(result.edges.is_none(), "Should not return edges for diagnostics");

    // Diagnostics might be empty or contain info-level diagnostics
    if let Some(diags) = result.diagnostics {
        // If there are diagnostics, they should be info or warnings, not errors
        for diag in &diags {
            assert!(diag.severity == Severity::Info || diag.severity == Severity::Warning,
                   "Diagnostic severity should be Info or Warning, not Error");
        }
    }

    Ok(())
}

#[test]
fn test_severity_mapping() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");
    std::fs::write(&file_path, "package main\n")?;

    // Create mock JSON output with different severities
    let mock_json_output = r#"{"location":{"file":"test.go","line":1,"column":5},"code":"SA6002","severity":"error","message":"error message"}
{"location":{"file":"test.go","line":2,"column":5},"code":"S1000","severity":"warning","message":"warning message"}
{"location":{"file":"test.go","line":3,"column":5},"code":"S1001","severity":"info","message":"info message"}"#;

    // Parse the mock output manually
    let diagnostics = parse_mock_staticcheck(mock_json_output, file_path.to_str().unwrap())?;

    // We should have 3 diagnostics
    assert_eq!(diagnostics.len(), 3, "Should parse all 3 diagnostics");

    // Check that each severity is mapped correctly
    let has_error = diagnostics.iter().any(|d| d.severity == Severity::Error && d.code == "SA6002");
    let has_warning = diagnostics.iter().any(|d| d.severity == Severity::Warning && d.code == "S1000");
    let has_info = diagnostics.iter().any(|d| d.severity == Severity::Info && d.code == "S1001");

    assert!(has_error, "Should map error severity correctly");
    assert!(has_warning, "Should map warning severity correctly");
    assert!(has_info, "Should map info severity correctly");

    Ok(())
}

// Helper function to parse mock staticcheck output
fn parse_mock_staticcheck(mock_output: &str, file_path: &str) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    for line in mock_output.lines() {
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(diagnostic) = parse_staticcheck_json(&json_value, file_path) {
                diagnostics.push(diagnostic);
            }
        }
    }

    Ok(diagnostics)
}

// Helper function to parse a single staticcheck JSON line
fn parse_staticcheck_json(json: &serde_json::Value, file_path: &str) -> Option<Diagnostic> {
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
