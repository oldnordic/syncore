//! APEX 2.5-LSP: JSON-RPC 2.0 Message Framing & Parsing
//!
//! Handles Content-Length header framing and publishDiagnostics parsing.

use super::LspDiagnosticEvent;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

// ============================================================================
// Message Framing
// ============================================================================

/// Frame a JSON-RPC message with Content-Length header
///
/// Format: `Content-Length: N\r\n\r\n<json>`
pub fn frame_jsonrpc_message(json: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", json.len(), json)
}

/// Parse a framed JSON-RPC message, extracting the JSON payload
///
/// Expects format: `Content-Length: N\r\n\r\n<json>`
pub fn parse_jsonrpc_message(framed: &str) -> Result<String> {
    // Find Content-Length header
    let header_end = framed.find("\r\n\r\n").context("Missing header separator (\\r\\n\\r\\n)")?;

    let header = &framed[..header_end];
    let json_start = header_end + 4; // Skip "\r\n\r\n"

    // Extract content length
    let content_length_str =
        header.strip_prefix("Content-Length:").context("Missing Content-Length prefix")?.trim();

    let content_length: usize =
        content_length_str.parse().context("Invalid Content-Length value")?;

    // Extract JSON payload
    let json = &framed[json_start..];

    if json.len() < content_length {
        anyhow::bail!("Incomplete message: expected {} bytes, got {}", content_length, json.len());
    }

    Ok(json[..content_length].to_string())
}

// ============================================================================
// publishDiagnostics Parsing
// ============================================================================

/// Parse LSP textDocument/publishDiagnostics notification
///
/// Returns vector of normalized diagnostic events.
pub fn parse_publish_diagnostics(json: &str) -> Result<Vec<LspDiagnosticEvent>> {
    let value: serde_json::Value = serde_json::from_str(json).context("Failed to parse JSON")?;

    // Check if this is a publishDiagnostics notification
    let method = value.get("method").and_then(|m| m.as_str()).unwrap_or("");

    if method != "textDocument/publishDiagnostics" {
        return Ok(Vec::new()); // Not a diagnostic message
    }

    // Extract params
    let params = value.get("params").context("Missing params in publishDiagnostics")?;

    // Extract URI
    let uri = params.get("uri").and_then(|u| u.as_str()).context("Missing uri in params")?;

    let path = uri_to_path(uri)?;

    // Extract diagnostics array
    let diagnostics = params
        .get("diagnostics")
        .and_then(|d| d.as_array())
        .context("Missing or invalid diagnostics array")?;

    // Parse each diagnostic
    let mut events = Vec::new();
    for diag in diagnostics {
        if let Some(event) = parse_diagnostic(&path, diag)? {
            events.push(event);
        }
    }

    Ok(events)
}

/// Parse a single diagnostic object
fn parse_diagnostic(path: &Path, diag: &serde_json::Value) -> Result<Option<LspDiagnosticEvent>> {
    // Extract range
    let range = diag.get("range").context("Missing range in diagnostic")?;

    let start = range.get("start").context("Missing start in range")?;
    let end = range.get("end").context("Missing end in range")?;

    let start_line =
        start.get("line").and_then(|l| l.as_u64()).context("Missing or invalid start.line")? as u32;

    let start_char = start
        .get("character")
        .and_then(|c| c.as_u64())
        .context("Missing or invalid start.character")? as u32;

    let end_line =
        end.get("line").and_then(|l| l.as_u64()).context("Missing or invalid end.line")? as u32;

    let end_char =
        end.get("character").and_then(|c| c.as_u64()).context("Missing or invalid end.character")?
            as u32;

    // Extract message
    let message = diag.get("message").and_then(|m| m.as_str()).unwrap_or("").to_string();

    // Extract severity (optional)
    let severity = diag.get("severity").and_then(|s| s.as_u64()).map(|s| s as u32);

    // Extract code (optional)
    let code = diag.get("code").and_then(|c| match c {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    });

    Ok(Some(LspDiagnosticEvent {
        path: path.to_path_buf(),
        message,
        range: (start_line, start_char, end_line, end_char),
        severity,
        code,
    }))
}

/// Convert LSP file:// URI to PathBuf
fn uri_to_path(uri: &str) -> Result<PathBuf> {
    let path_str = uri.strip_prefix("file://").context("URI does not start with file://")?;

    Ok(PathBuf::from(path_str))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_roundtrip() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
        let framed = frame_jsonrpc_message(json);
        let parsed = parse_jsonrpc_message(&framed).unwrap();
        assert_eq!(parsed, json);
    }

    #[test]
    fn test_parse_publish_diagnostics() {
        let json = r#"{
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///tmp/test.rs",
                "diagnostics": [
                    {
                        "range": {
                            "start": {"line": 5, "character": 10},
                            "end": {"line": 5, "character": 20}
                        },
                        "severity": 1,
                        "code": "E0308",
                        "message": "mismatched types"
                    }
                ]
            }
        }"#;

        let events = parse_publish_diagnostics(json).unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert!(event.path.to_str().unwrap().ends_with("test.rs"));
        assert_eq!(event.message, "mismatched types");
        assert_eq!(event.range, (5, 10, 5, 20));
        assert_eq!(event.severity, Some(1));
        assert_eq!(event.code.as_deref(), Some("E0308"));
    }

    #[test]
    fn test_uri_to_path() {
        let uri = "file:///home/user/project/src/main.rs";
        let path = uri_to_path(uri).unwrap();
        assert_eq!(path.to_str().unwrap(), "/home/user/project/src/main.rs");
    }
}
