//! Memory Tools Executor
//!
//! Handles execution of memory_store and memory_query tools.
//! Extracted from executor_real.rs giant match statement (lines 137-192).

use crate::mcp::types::ErrorType;
use crate::router::SynCoreState;
use serde_json::{json, Value};
use std::sync::Arc;

/// Helper: Extract string parameter from Value params
fn param_str<'a>(tool: &str, params: &'a Value, key: &str) -> Result<&'a str, Value> {
    match params.get(key).and_then(|v| v.as_str()) {
        Some(v) if !v.is_empty() => Ok(v),
        _ => Err(wrap_error_static(tool, &format!("Missing '{}' parameter", key))),
    }
}

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

/// Execute memory_store tool
pub async fn execute_memory_store(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let key = match param_str("memory_store", params, "key") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let value = match param_str("memory_store", params, "value") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "memory_store",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would store key='{}' with value (length: {} bytes)", key, value.len())
            }),
        );
        return Ok(result);
    }

    state.memory.store(key, value)?;
    Ok(wrap_success("memory_store", json!({"stored": true, "key": key})))
}

/// Execute memory_query tool
pub async fn execute_memory_query(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let key = match param_str("memory_query", params, "key") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "memory_query",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would query key='{}'", key)
            }),
        );
        return Ok(result);
    }

    match state.memory.query(key)? {
        Some(value) => Ok(wrap_success(
            "memory_query",
            json!({
                "value": value,
                "found": true
            }),
        )),
        None => Ok(wrap_success(
            "memory_query",
            json!({
                "value": null,
                "found": false
            }),
        )),
    }
}
