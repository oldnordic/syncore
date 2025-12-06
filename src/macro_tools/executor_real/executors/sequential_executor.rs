//! Sequential Tools Executor
//!
//! Handles execution of all sequential tools by routing through memory_suite.
//! Extracted from executor_real.rs giant match statement (Phase 6.12).

use crate::mcp::types::ErrorType;
use crate::mcp_tools::memory_suite::{MemorySuite, MemorySuiteArgs};
use crate::mcp_tools::SuiteDispatcher;
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

/// Helper: Extract optional string parameter
fn param_str_opt(params: &Value, key: &str) -> Option<String> {
    params.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Helper: Extract optional i64 parameter
fn param_i64_opt(params: &Value, key: &str) -> Option<i64> {
    params.get(key).and_then(|v| v.as_i64())
}

/// Helper: Extract optional i32 parameter
fn param_i32_opt(params: &Value, key: &str) -> Option<i32> {
    params.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
}

/// Helper: Extract optional usize parameter
fn param_usize_opt(params: &Value, key: &str) -> Option<usize> {
    params.get(key).and_then(|v| v.as_u64()).map(|v| v as usize)
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

/// Route sequential tool through memory suite
async fn route_through_memory_suite(
    tool: &str,
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // Create memory suite
    let memory_suite = MemorySuite::new((**state).clone());

    // Convert tool name to memory suite command
    let command = tool.strip_prefix("sequential_").unwrap_or(tool);

    // Build MemorySuiteArgs from params
    let mut suite_args = MemorySuiteArgs {
        command: command.to_string(),
        key: param_str_opt(params, "key"),
        value: param_str_opt(params, "value"),
        text: param_str_opt(params, "text"),
        query: param_str_opt(params, "query"),
        limit: param_usize_opt(params, "limit"),
        namespace: param_str_opt(params, "namespace"),
        goal: param_str_opt(params, "goal"),
        priority: param_i32_opt(params, "priority").map(|p| p as i32),
        task_id: param_i64_opt(params, "task_id"),
        depends_on_task_id: param_i64_opt(params, "depends_on_task_id"),
        step_number: param_i32_opt(params, "step_number"),
        thought: param_str_opt(params, "thought"),
        reasoning: param_str_opt(params, "reasoning"),
        action: param_str_opt(params, "action"),
        observation: param_str_opt(params, "observation"),
        max_cycles: param_usize_opt(params, "max_cycles"),
        sequence_id: param_str_opt(params, "sequence_id"),
        context: param_str_opt(params, "context"),
        depth: param_i32_opt(params, "depth"),
        max_steps: param_usize_opt(params, "max_steps"),
        to: param_str_opt(params, "to"),
        from: param_str_opt(params, "from"),
        agent: param_str_opt(params, "agent"),
        id: param_str_opt(params, "id"),
        message: param_str_opt(params, "message"),
        capabilities: params
            .get("capabilities")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
        status: params.get("status").cloned(),
        task_type: param_str_opt(params, "task_type"),
        payload: params.get("payload").cloned(),
        result: params.get("result").cloned(),
        timeout_ms: params.get("timeout_ms").and_then(|v| v.as_u64()),
        prd_content: param_str_opt(params, "prd_content"),
        parent_task_id: param_str_opt(params, "parent_task_id"),
        parent_task_json: param_str_opt(params, "parent_task_json"),
        tasks_json: param_str_opt(params, "tasks_json"),
        business_context: param_str_opt(params, "business_context"),
        completed_tasks: params
            .get("completed_tasks")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
        remaining_tasks_json: param_str_opt(params, "remaining_tasks_json"),
        breakdown_json: param_str_opt(params, "breakdown_json"),
        parent_id: param_i64_opt(params, "parent_id"),
        prd_title: param_str_opt(params, "prd_title"),
        keywords: params
            .get("keywords")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
        tags: params
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
        min_importance: params.get("min_importance").and_then(|v| v.as_f64()).map(|v| v as f32),
        unix_timestamp: params.get("unix_timestamp").and_then(|v| v.as_u64()),
        seconds: params.get("seconds").and_then(|v| v.as_u64()),
        threshold: params.get("threshold").and_then(|v| v.as_f64()).map(|v| v as f32),
        dry_run: Some(dry_run),
    };

    // Execute through memory suite
    let result = memory_suite.execute(suite_args);

    // Convert SuiteResult to executor response
    if result.success {
        Ok(wrap_success(tool, result.data))
    } else {
        let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
        Ok(wrap_error_static(tool, &error_msg))
    }
}

/// Execute sequential_next tool
pub async fn execute_sequential_next(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    route_through_memory_suite("sequential_next", state, params, dry_run).await
}

/// Execute sequential_run tool
pub async fn execute_sequential_run(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    route_through_memory_suite("sequential_run", state, params, dry_run).await
}

/// Execute sequential_reason tool
pub async fn execute_sequential_reason(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    route_through_memory_suite("sequential_reason", state, params, dry_run).await
}

/// Execute sequential_status tool
pub async fn execute_sequential_status(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    route_through_memory_suite("sequential_status", state, params, dry_run).await
}

/// Execute sequential_reset tool
pub async fn execute_sequential_reset(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    route_through_memory_suite("sequential_reset", state, params, dry_run).await
}

/// Execute sequential_record tool
pub async fn execute_sequential_record(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    route_through_memory_suite("sequential_record", state, params, dry_run).await
}

/// Execute sequential_get tool
pub async fn execute_sequential_get(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    route_through_memory_suite("sequential_get", state, params, dry_run).await
}

/// Execute sequential_search tool
pub async fn execute_sequential_search(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    route_through_memory_suite("sequential_search", state, params, dry_run).await
}

/// Execute sequential_cycle tool
pub async fn execute_sequential_cycle(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    route_through_memory_suite("sequential_cycle", state, params, dry_run).await
}
