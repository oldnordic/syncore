//! Graph/RAG Tools Executor
//!
//! Handles execution of Neo4j graph database and RAG graph tools.
//! Extracted from executor_real.rs giant match statement (lines 234-551).
//!
//! Tools:
//! - graph_query: Execute Cypher read query on Neo4j
//! - graph_insert: Execute Cypher write query on Neo4j
//! - graph_relate: Create relationship between two nodes
//! - raggraph_query: RAG graph query via GraphSuite
//! - raggraph_multihop: Multi-hop graph traversal via GraphSuite
//! - code_graph_sync_neo4j: Sync code entities to Neo4j via CodeSuite
//! - code_graph_enrich_temporal: Enrich code entities with temporal data via CodeSuite
//! - code_graph_fusion_query: Code graph fusion query via CodeSuite

use crate::mcp::types::ErrorType;
use crate::mcp_tools::code_suite::{CodeSuite, CodeSuiteArgs};
use crate::mcp_tools::graph_suite::{GraphSuite, GraphSuiteArgs};
use crate::router::SynCoreState;
use serde_json::{json, Value};
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

/// Helper: Route through suite (mimics RealExecutor::route_through_suite)
fn route_through_suite(suite_result: crate::mcp_tools::SuiteResult) -> Value {
    if suite_result.success {
        json!({
            "ok": true,
            "data": suite_result.data
        })
    } else {
        json!({
            "ok": false,
            "error": {
                "type": "ExecutionError",
                "message": suite_result.error.unwrap_or_else(|| "Unknown error".to_string()),
                "tool": suite_result.command,
                "executor": "real"
            }
        })
    }
}

/// Execute graph_query tool
pub async fn execute_graph_query(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
    tool_name: &str,
) -> anyhow::Result<Value> {
    // PARAMETER VALIDATION - MUST BE FIRST
    let cypher = match param_str("graph_query", params, "cypher") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    // Check if neo4j client is available BEFORE dry_run
    if state.neo4j.is_none() {
        return Ok(wrap_error(
            tool_name,
            "NotAvailable: Graph database unavailable (neo4j disabled)",
        ));
    }

    if dry_run {
        let result = wrap_success(
            "graph_query",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would execute Cypher query: {}", cypher),
                "results": []
            }),
        );
        return Ok(result);
    }

    // Execute cypher query via neo4j client
    let neo4j = state.neo4j.as_ref().unwrap();
    let params_json = params.get("params").cloned().unwrap_or(json!({}));
    let params_vec: Vec<(&str, serde_json::Value)> = params_json
        .as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.as_str(), v.clone())).collect())
        .unwrap_or_default();

    match neo4j.execute_query(cypher, params_vec).await {
        Ok(results) => Ok(wrap_success(
            "graph_query",
            json!({
                "results": results
            }),
        )),
        Err(e) => Ok(wrap_error(
            "graph_query",
            &format!("Neo4j query failed: {}", e),
        )),
    }
}

/// Execute graph_insert tool
pub async fn execute_graph_insert(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
    tool_name: &str,
) -> anyhow::Result<Value> {
    // PARAMETER VALIDATION - MUST BE FIRST
    let cypher = match param_str("graph_query", params, "cypher") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "graph_insert",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would execute Cypher insert: {}", cypher),
                "created": true
            }),
        );
        return Ok(result);
    }

    // Check if neo4j client is available
    if state.neo4j.is_none() {
        return Ok(wrap_error(
            tool_name,
            "NotAvailable: Graph database unavailable (neo4j disabled)",
        ));
    }

    // Execute cypher write via neo4j client
    let neo4j = state.neo4j.as_ref().unwrap();
    let params_json = params.get("params").cloned().unwrap_or(json!({}));
    let params_vec: Vec<(&str, serde_json::Value)> = params_json
        .as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.as_str(), v.clone())).collect())
        .unwrap_or_default();

    match neo4j.execute_query(cypher, params_vec).await {
        Ok(_) => Ok(wrap_success(
            "graph_insert",
            json!({
                "created": true
            }),
        )),
        Err(e) => Ok(wrap_error(
            "graph_insert",
            &format!("Neo4j insert failed: {}", e),
        )),
    }
}

/// Execute graph_relate tool
pub async fn execute_graph_relate(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
    tool_name: &str,
) -> anyhow::Result<Value> {
    let from_id = match params.get("from_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => {
            return Ok(wrap_error_static(
                "graph_relate",
                "Missing 'from_id' parameter",
            ))
        }
    };
    let to_id = match params.get("to_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => {
            return Ok(wrap_error_static(
                "graph_relate",
                "Missing 'to_id' parameter",
            ))
        }
    };
    let rel_type = match param_str("graph_relate", params, "rel_type") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "graph_relate",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would create relationship {} -[{}]-> {}", from_id, rel_type, to_id),
                "success": true
            }),
        );
        return Ok(result);
    }

    // Check if neo4j client is available
    if state.neo4j.is_none() {
        return Ok(wrap_error(
            tool_name,
            "NotAvailable: Graph database unavailable (neo4j disabled)",
        ));
    }

    // Create relationship via neo4j client
    let neo4j = state.neo4j.as_ref().unwrap();
    let from_label = params
        .get("from_label")
        .and_then(|v| v.as_str())
        .unwrap_or("Node");
    let to_label = params
        .get("to_label")
        .and_then(|v| v.as_str())
        .unwrap_or("Node");

    match neo4j
        .create_relationship(from_label, from_id, to_label, to_id, rel_type)
        .await
    {
        Ok(_) => Ok(wrap_success(
            "graph_relate",
            json!({
                "success": true
            }),
        )),
        Err(e) => Ok(wrap_error(
            "graph_relate",
            &format!("Neo4j relationship creation failed: {}", e),
        )),
    }
}

/// Execute raggraph_query tool
pub async fn execute_raggraph_query(
    state: &Arc<SynCoreState>,
    params: &Value,
    _dry_run: bool,
    _tool_name: &str,
) -> anyhow::Result<Value> {
    let suite_args = GraphSuiteArgs {
        command: "rag_query".to_string(),
        cypher: None,
        params: None,
        from_id: None,
        to_id: None,
        rel_type: None,
        from_label: None,
        to_label: None,
        query_text: params
            .get("query_text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        seed_nodes: None,
    };

    let suite = GraphSuite::new((**state).clone());
    Ok(route_through_suite(suite.execute(suite_args)))
}

/// Execute raggraph_multihop tool
pub async fn execute_raggraph_multihop(
    state: &Arc<SynCoreState>,
    params: &Value,
    _dry_run: bool,
    _tool_name: &str,
) -> anyhow::Result<Value> {
    let suite_args = GraphSuiteArgs {
        command: "rag_multihop".to_string(),
        cypher: None,
        params: None,
        from_id: None,
        to_id: None,
        rel_type: None,
        from_label: None,
        to_label: None,
        query_text: None,
        seed_nodes: params
            .get("seed_nodes")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|n| n.as_i64()).collect()),
    };

    let suite = GraphSuite::new((**state).clone());
    Ok(route_through_suite(suite.execute(suite_args)))
}

/// Execute code_graph_sync_neo4j tool
pub async fn execute_code_graph_sync_neo4j(
    state: &Arc<SynCoreState>,
    params: &Value,
    _dry_run: bool,
    _tool_name: &str,
) -> anyhow::Result<Value> {
    let suite_args = CodeSuiteArgs {
        command: "sync_neo4j".to_string(),
        file_path: None,
        query: None,
        pattern: None,
        limit: params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|l| l as usize),
        directory: None,
        context_lines: None,
        function_name: None,
        namespace: params
            .get("namespace")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        mode_hint: None,
        top_k: None,
        scope: None,
        project_label: None,
        local_root: None,
        only_missing: None,
    };

    let suite = CodeSuite::new((**state).clone());
    Ok(route_through_suite(suite.execute(suite_args)))
}

/// Execute code_graph_enrich_temporal tool
pub async fn execute_code_graph_enrich_temporal(
    state: &Arc<SynCoreState>,
    params: &Value,
    _dry_run: bool,
    _tool_name: &str,
) -> anyhow::Result<Value> {
    let suite_args = CodeSuiteArgs {
        command: "enrich_temporal".to_string(),
        file_path: None,
        query: None,
        pattern: None,
        limit: params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|l| l as usize),
        directory: None,
        context_lines: None,
        function_name: None,
        namespace: None,
        mode_hint: None,
        top_k: None,
        scope: None,
        project_label: None,
        local_root: None,
        only_missing: params.get("only_missing").and_then(|v| v.as_bool()),
    };

    let suite = CodeSuite::new((**state).clone());
    Ok(route_through_suite(suite.execute(suite_args)))
}

/// Execute code_graph_fusion_query tool
pub async fn execute_code_graph_fusion_query(
    state: &Arc<SynCoreState>,
    params: &Value,
    _dry_run: bool,
    _tool_name: &str,
) -> anyhow::Result<Value> {
    let suite_args = CodeSuiteArgs {
        command: "fusion_query".to_string(),
        file_path: None,
        query: params
            .get("query")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        pattern: None,
        limit: None,
        directory: None,
        context_lines: None,
        function_name: None,
        namespace: params
            .get("namespace")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        mode_hint: params
            .get("mode_hint")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        top_k: params
            .get("top_k")
            .and_then(|v| v.as_u64())
            .map(|k| k as usize),
        scope: params
            .get("scope")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        project_label: params
            .get("project_label")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        local_root: params
            .get("local_root")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        only_missing: None,
    };

    let suite = CodeSuite::new((**state).clone());
    Ok(route_through_suite(suite.execute(suite_args)))
}
