//! Code/Parser Tools Executor
//!
//! Handles execution of parser and code indexing tools.
//! Extracted from executor_real.rs giant match statement (lines 200-542).
//!
//! Tools:
//! - parser_analyze: Tree-sitter code analysis with optional persistence
//! - parser_search: Ripgrep-based code search
//! - code_index: Index single code file with persistent storage
//! - code_index_directory: Batch index directory with glob pattern
//! - code_search: Semantic code search using vector store

use crate::code_graph::CodeGraph;
use crate::common::db_paths;
use crate::mcp::types::ErrorType;
use crate::parser::{Parser, RipgrepSearcher};
use crate::router::SynCoreState;
use crate::vector::SearchScope;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

/// Helper: Extract string parameter from Value params
fn param_str<'a>(tool: &str, params: &'a Value, key: &str) -> Result<&'a str, Value> {
    match params.get(key).and_then(|v| v.as_str()) {
        Some(v) if !v.is_empty() => Ok(v),
        _ => Err(wrap_error_static(
            tool,
            &format!("Missing '{}' parameter", key),
        )),
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

/// Helper: Wrap error with state access
fn wrap_error(tool: &str, error: &str) -> Value {
    wrap_error_static(tool, error)
}

/// Execute parser_analyze tool
pub async fn execute_parser_analyze(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let file_path = match param_str("parser_analyze", params, "file_path") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let persist = params
        .get("persist")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if dry_run {
        let result = wrap_success(
            "parser_analyze",
            json!({
                "dry_run": true,
                "persist": persist,
                "message": format!("[DRY RUN] Would analyze file '{}' (persist={})", file_path, persist)
            }),
        );
        return Ok(result);
    }

    // Use Parser to analyze the file
    let parser = match Parser::new() {
        Ok(p) => p,
        Err(e) => {
            return Ok(wrap_error(
                "parser_analyze",
                &format!("Failed to initialize parser: {}", e),
            ))
        }
    };

    let analysis = match parser.parse_file(Path::new(file_path)) {
        Ok(a) => a,
        Err(e) => {
            return Ok(wrap_error(
                "parser_analyze",
                &format!("Failed to parse file '{}': {}", file_path, e),
            ))
        }
    };

    // If persist=true, also index the file using CodeGraph (same as code_index)
    let persisted_count = if persist {
        let code_graph_conn = state.db_manager.code_graph_conn();
        let mut code_graph =
            match CodeGraph::with_connection(code_graph_conn, Arc::clone(&state.general_store)) {
                Ok(cg) => cg,
                Err(e) => {
                    return Ok(wrap_error(
                        "parser_analyze",
                        &format!("Failed to initialize code graph for persistence: {}", e),
                    ));
                }
            };

        match code_graph.index_file(Path::new(file_path)) {
            Ok(count) => Some(count),
            Err(e) => {
                return Ok(wrap_error(
                    "parser_analyze",
                    &format!("Failed to persist entities: {}", e),
                ));
            }
        }
    } else {
        None
    };

    match serde_json::to_value(&analysis) {
        Ok(mut v) => {
            // Add persistence info to result
            if let Some(count) = persisted_count {
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("persisted".to_string(), json!(true));
                    obj.insert("persisted_entity_count".to_string(), json!(count));
                }
            }
            Ok(wrap_success("parser_analyze", v))
        }
        Err(e) => Ok(wrap_error(
            "parser_analyze",
            &format!("Failed to serialize analysis: {}", e),
        )),
    }
}

/// Execute parser_search tool
pub async fn execute_parser_search(
    _state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let pattern = match param_str("parser_search", params, "pattern") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let path = params.get("path").and_then(|p| p.as_str());
    let context_lines = params
        .get("context_lines")
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as usize;

    if dry_run {
        let result = wrap_success(
            "parser_search",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would search for pattern '{}' in {:?}", pattern, path)
            }),
        );
        return Ok(result);
    }

    // Use RipgrepSearcher for pattern search
    let search_path = path.unwrap_or(".");

    let results = match RipgrepSearcher::search(pattern, Path::new(search_path), context_lines) {
        Ok(r) => r,
        Err(e) => {
            return Ok(wrap_error(
                "parser_search",
                &format!(
                    "Search failed for pattern '{}' in '{}': {}",
                    pattern, search_path, e
                ),
            ))
        }
    };

    Ok(wrap_success(
        "parser_search",
        json!({
            "matches": results,
            "count": results.len()
        }),
    ))
}

/// Execute code_index tool
pub async fn execute_code_index(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let file_path = match param_str("code_index", params, "file_path") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "code_index",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would index file '{}'", file_path)
            }),
        );
        return Ok(result);
    }

    // REAL IMPLEMENTATION - Index file with persistent storage using DbManager
    // Use DbManager's long-lived connection instead of creating a new one
    let code_graph_conn = state.db_manager.code_graph_conn();
    let mut code_graph =
        match CodeGraph::with_connection(code_graph_conn, Arc::clone(&state.general_store)) {
            Ok(cg) => cg,
            Err(e) => {
                return Ok(wrap_error(
                    "code_index",
                    &format!("Failed to initialize code graph: {}", e),
                ));
            }
        };

    // Index the file with persistent storage
    let path = Path::new(file_path);
    match code_graph.index_file(path) {
        Ok(entity_count) => {
            let db_path = db_paths::code_graph_db_path();

            // Opt-in diagnostic sleep for debugging SQLite persistence
            // Only enabled when SYNCORE_CODE_INDEX_DIAG_SLEEP=1
            if std::env::var("SYNCORE_CODE_INDEX_DIAG_SLEEP")
                .map(|v| v == "1")
                .unwrap_or(false)
            {
                let _ = std::fs::write("/tmp/code_graph_diagnostic.log.append",
                    format!("\n=== DIAGNOSTIC: Sleeping 3s for external validation ===\n\
                             Database: {}\n\
                             File: {}\n\
                             Expected entities: {}\n\
                             Run now: sqlite3 {} \"SELECT COUNT(*) FROM code_entities WHERE file_path='{}'\"\n",
                        db_path, file_path, entity_count, db_path, file_path)
                );

                std::thread::sleep(std::time::Duration::from_secs(3));

                let _ = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open("/tmp/code_graph_diagnostic.log.append")
                    .and_then(|mut f| {
                        use std::io::Write;
                        writeln!(f, "=== DIAGNOSTIC: Sleep complete ===\n")
                    });
            }

            Ok(wrap_success(
                "code_index",
                json!({
                    "indexed": true,
                    "entity_count": entity_count,
                    "file_path": file_path,
                    "message": format!("Indexed {} entities from file", entity_count)
                }),
            ))
        }
        Err(e) => Ok(wrap_error(
            "code_index",
            &format!("Failed to index file '{}': {}", file_path, e),
        )),
    }
}

/// Execute code_index_directory tool
pub async fn execute_code_index_directory(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let directory = match param_str("code_index_directory", params, "directory") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let pattern = params
        .get("pattern")
        .and_then(|p| p.as_str())
        .unwrap_or("*.rs");

    if dry_run {
        let result = wrap_success(
            "code_index_directory",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would index directory '{}' with pattern '{}'", directory, pattern)
            }),
        );
        return Ok(result);
    }

    // Use DbManager's long-lived connection instead of creating a new one
    let code_graph_conn = state.db_manager.code_graph_conn();
    let mut code_graph =
        match CodeGraph::with_connection(code_graph_conn, Arc::clone(&state.general_store)) {
            Ok(cg) => cg,
            Err(e) => {
                return Ok(wrap_error(
                    "code_index_directory",
                    &format!("Failed to initialize code graph: {}", e),
                ));
            }
        };

    // Recursively find files matching pattern
    use glob::glob;
    let search_pattern = format!("{}/**/{}", directory, pattern);
    let mut indexed_count = 0;
    let mut total_entities = 0;

    for path in (glob(&search_pattern).map_err(|e| anyhow::anyhow!("Glob error: {}", e))?).flatten() {
        if path.is_file() {
            if path.is_file() {
                match code_graph.index_file(&path) {
                    Ok(count) => {
                        indexed_count += 1;
                        total_entities += count;
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to index {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    Ok(wrap_success(
        "code_index_directory",
        json!({
            "indexed_files": indexed_count,
            "total_entities": total_entities,
            "directory": directory,
            "pattern": pattern
        }),
    ))
}

/// Execute code_search tool
pub async fn execute_code_search(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let query = match param_str("code_search", params, "query") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

    if dry_run {
        let result = wrap_success(
            "code_search",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would search for '{}' with limit {}", query, limit)
            }),
        );
        return Ok(result);
    }

    // Use vector search for semantic code search (spawn_blocking to avoid blocking async runtime)
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
        "code_search",
        json!({
            "results": results,
            "count": results.len()
        }),
    ))
}
