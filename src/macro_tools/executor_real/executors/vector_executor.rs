//! Vector Tools Executor
//!
//! Handles execution of vector_insert and vector_search tools.
//! Extracted from executor_real.rs giant match statement (lines 150-233).

use crate::mcp::types::ErrorType;
use crate::router::SynCoreState;
use crate::vector::SearchScope;
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

/// Execute vector_insert tool
pub async fn execute_vector_insert(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let text = match param_str("vector_insert", params, "text") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "vector_insert",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would insert text into vector store (length: {} chars)", text.len())
            }),
        );
        return Ok(result);
    }

    // Insert into vector store (spawn_blocking to avoid blocking async runtime)
    let vector_store = Arc::clone(&state.general_store);
    let text_owned = text.to_string();

    let vector_id = tokio::task::spawn_blocking(move || {
        let mut store = vector_store.lock().unwrap();
        let id = store.len() as i64 + 1; // Simple ID generation
        store.insert_text(id, None, &text_owned, "executor")?;
        Ok::<i64, anyhow::Error>(id)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Spawn blocking error: {}", e))??;

    Ok(wrap_success(
        "vector_insert",
        json!({
            "inserted": true,
            "vector_id": vector_id
        }),
    ))
}

/// Execute vector_search tool
pub async fn execute_vector_search(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let query = match param_str("vector_search", params, "query") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    if dry_run {
        let result = wrap_success(
            "vector_search",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would search for '{}' with limit {}", query, limit)
            }),
        );
        return Ok(result);
    }

    // Search vector store (spawn_blocking to avoid blocking async runtime)
    let vector_store = Arc::clone(&state.general_store);
    let query_owned = query.to_string();

    let results = tokio::task::spawn_blocking(move || {
        let store = vector_store.lock().unwrap();
        let hits = store.search(&query_owned, limit, SearchScope::Global)?;

        let results: Vec<serde_json::Value> = hits
            .iter()
            .map(|hit| {
                json!({
                    "id": hit.id,
                    "text": hit.text,
                    "score": hit.score
                })
            })
            .collect();

        Ok::<Vec<serde_json::Value>, anyhow::Error>(results)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Spawn blocking error: {}", e))??;

    Ok(wrap_success(
        "vector_search",
        json!({
            "results": results,
            "count": results.len()
        }),
    ))
}
