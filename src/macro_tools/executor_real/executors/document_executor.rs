//! Document Tools Executor
//!
//! Handles execution of document indexing and search tools.
//! Extracted from executor_real.rs giant match statement (lines 223-307).
//!
//! Tools:
//! - document_index: Index documents from a directory into vector store
//! - document_search: Semantic search across indexed documents

use crate::document_indexer::DocumentIndexer;
use crate::mcp::types::ErrorType;
use crate::router::SynCoreState;
use crate::vector::SearchScope;
use serde_json::{json, Value};
use std::path::Path;
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

/// Execute document_index tool
pub async fn execute_document_index(
    _state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let directory = match param_str("document_index", params, "directory") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "document_index",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would index documents in '{}'", directory)
            }),
        );
        return Ok(result);
    }

    // Use DocumentIndexer to index directory
    let indexer = DocumentIndexer::with_defaults();
    let dir_path = Path::new(directory);

    match indexer.index_directory(dir_path) {
        Ok(chunk_count) => Ok(wrap_success(
            "document_index",
            json!({
                "indexed": true,
                "chunk_count": chunk_count,
                "directory": directory,
                "message": format!("Successfully indexed {} document chunks", chunk_count)
            }),
        )),
        Err(e) => Err(anyhow::anyhow!("Failed to index directory: {}", e)),
    }
}

/// Execute document_search tool
pub async fn execute_document_search(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let query = match param_str("document_search", params, "query") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(5) as usize;

    if dry_run {
        let result = wrap_success(
            "document_search",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would search for '{}' with limit {}", query, limit),
                "results": []
            }),
        );
        return Ok(result);
    }

    // Use VectorStore to search documents (spawn_blocking to avoid blocking async runtime)
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
        "document_search",
        json!({
            "results": results,
            "count": results.len()
        }),
    ))
}
