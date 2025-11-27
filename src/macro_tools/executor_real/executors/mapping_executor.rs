//! Mapping/Application Structure Tools Executor
//!
//! Handles execution of application structure mapping and file node management tools.
//! Extracted from executor_real.rs giant match statement (lines 646-825).
//!
//! Tools:
//! - mapping_record: Record a file node in the application structure map
//! - mapping_get: Get a file node from the application structure map
//! - mapping_search: Search for files related to a query using semantic search
//! - mapping_deps: Get all transitive dependencies for a file

use crate::mcp::types::ErrorType;
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

/// Execute mapping_record tool
pub async fn execute_mapping_record(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let path = match param_str("mapping_record", params, "path") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let kind = match param_str("mapping_record", params, "kind") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let language = params.get("language").and_then(|l| l.as_str());
    let imports = params["imports"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing 'imports' parameter"))?;
    let exports = params["exports"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing 'exports' parameter"))?;
    let dependencies = params["dependencies"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing 'dependencies' parameter"))?;

    if dry_run {
        let result = wrap_success(
            "mapping_record",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would record file node: {}", path),
                "recorded": true
            }),
        );
        return Ok(result);
    }

    // Record file node using MappingTool
    use crate::portfolio::mapping_tool::{FileNode, MappingTool};
    let mapper = MappingTool::new((**state).clone());

    let imports_vec: Vec<String> = imports
        .iter()
        .filter_map(|i| i.as_str().map(|s| s.to_string()))
        .collect();
    let exports_vec: Vec<String> = exports
        .iter()
        .filter_map(|e| e.as_str().map(|s| s.to_string()))
        .collect();
    let dependencies_vec: Vec<String> = dependencies
        .iter()
        .filter_map(|d| d.as_str().map(|s| s.to_string()))
        .collect();

    let node = FileNode {
        path: path.to_string(),
        kind: kind.to_string(),
        language: language.map(|l| l.to_string()),
        imports: imports_vec,
        exports: exports_vec,
        dependencies: dependencies_vec,
    };

    mapper
        .record_file(&node)
        .map_err(|e| anyhow::anyhow!("Failed to record file: {}", e))?;

    Ok(wrap_success(
        "mapping_record",
        json!({
            "recorded": true,
            "path": path
        }),
    ))
}

/// Execute mapping_get tool
pub async fn execute_mapping_get(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let path = match param_str("mapping_get", params, "path") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "mapping_get",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would get file node: {}", path),
                "path": path
            }),
        );
        return Ok(result);
    }

    // Get file node using MappingTool
    use crate::portfolio::mapping_tool::MappingTool;
    let mapper = MappingTool::new((**state).clone());

    match mapper.get_file(path)? {
        Some(node) => {
            let node_data = serde_json::to_value(&node)
                .unwrap_or_else(|_| json!({"error": "Serialization failed"}));
            Ok(wrap_success("mapping_get", node_data))
        }
        None => Ok(wrap_success(
            "mapping_get",
            json!({
                "path": path,
                "found": false,
                "message": format!("File not found: {}", path)
            }),
        )),
    }
}

/// Execute mapping_search tool
pub async fn execute_mapping_search(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // PARAMETER VALIDATION - MUST BE FIRST
    let query = params
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;

    if dry_run {
        let result = wrap_success(
            "mapping_search",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would search for: {}", query),
                "files": [],
                "count": 0
            }),
        );
        return Ok(result);
    }

    // Search files using MappingTool
    use crate::portfolio::mapping_tool::MappingTool;
    let mapper = MappingTool::new((**state).clone());

    let nodes = mapper
        .search_related(query)
        .map_err(|e| anyhow::anyhow!("Failed to search: {}", e))?;

    Ok(wrap_success(
        "mapping_search",
        json!({
            "count": nodes.len(),
            "files": nodes
        }),
    ))
}

/// Execute mapping_deps tool
pub async fn execute_mapping_deps(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // PARAMETER VALIDATION - MUST BE FIRST
    let path = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

    if dry_run {
        let result = wrap_success(
            "mapping_deps",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would get dependencies for: {}", path),
                "dependencies": []
            }),
        );
        return Ok(result);
    }

    // Get transitive dependencies using MappingTool
    use crate::portfolio::mapping_tool::MappingTool;
    let mapper = MappingTool::new((**state).clone());

    let deps = mapper
        .get_all_dependencies(path)
        .map_err(|e| anyhow::anyhow!("Failed to get dependencies: {}", e))?;

    Ok(wrap_success(
        "mapping_deps",
        json!({
            "path": path,
            "dependencies": deps,
            "count": deps.len()
        }),
    ))
}
