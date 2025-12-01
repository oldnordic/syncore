//! Logs Tools Executor
//!
//! Handles execution of log file operations.
//! Extracted from executor_real.rs giant match statement (lines 364-429).
//!
//! Tools:
//! - logs_tail: Retrieve the last N lines from a log file

use crate::mcp::types::ErrorType;
use crate::router::SynCoreState;
use serde_json::{json, Value};
use std::sync::Arc;

/// Helper: Wrap error response
fn wrap_error_static(tool: &str, msg: &str) -> Value {
    let error_type = ErrorType::from_message(msg);
    json!({
        "ok": false,
        "error": {
            "type": error_type.to_string(),
            "message": msg,
            "tool": tool,
            "executor": "real"
        }
    })
}

/// Helper: Wrap success response
fn wrap_success(tool: &str, data: Value) -> Value {
    json!({
        "ok": true,
        "tool": tool,
        "executor": "real",
        "data": data
    })
}

/// Helper: Wrap error with state access
fn wrap_error(tool: &str, error: &str) -> Value {
    wrap_error_static(tool, error)
}

/// Execute logs_tail tool
pub async fn execute_logs_tail(
    _state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // PARAMETER VALIDATION - MUST BE FIRST (before any imports or I/O)
    let file_path = match params.get("file_path").and_then(|v| v.as_str()) {
        Some(path) if !path.is_empty() => path,
        _ => return Ok(wrap_error_static("logs_tail", "Missing 'file_path' parameter")),
    };

    let n = params.get("n").and_then(|n| n.as_u64()).map(|n| n as usize).unwrap_or(50); // Default to 50 lines

    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::Path;

    if dry_run {
        let result = wrap_success(
            "logs_tail",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would tail {} lines from: {}", n, file_path),
                "file_path": file_path,
                "n": n,
                "lines": [],
                "count": 0
            }),
        );
        return Ok(result);
    }

    // Read log file
    let path = Path::new(file_path);
    if !path.exists() {
        return Ok(wrap_error("logs_tail", &format!("IoError: Log file not found: {}", file_path)));
    }

    let file = File::open(path).map_err(|e| anyhow::anyhow!("Failed to open log file: {}", e))?;
    let reader = BufReader::new(file);

    // Read all lines
    let all_lines: Vec<String> = reader.lines().filter_map(|line| line.ok()).collect();

    // Get last n lines
    let start = if all_lines.len() > n {
        all_lines.len() - n
    } else {
        0
    };
    let tail_lines: Vec<String> = all_lines[start..].to_vec();

    Ok(wrap_success(
        "logs_tail",
        json!({
            "lines": tail_lines,
            "count": tail_lines.len()
        }),
    ))
}
