//! Sequential Reasoning Tools Executor
//!
//! Handles execution of sequential reasoning and thought step management tools.
//! Extracted from executor_real.rs giant match statement (lines 830-1097).
//!
//! Tools:
//! - sequential_record: Record a thought step in the reasoning chain
//! - sequential_get: Get all thought steps for a task
//! - sequential_search: Search thought steps by semantic content
//! - sequential_cycle: Run sequential reasoning cycles with LLM integration

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

/// Helper: Wrap error with state access
fn wrap_error(tool: &str, error: &str) -> Value {
    wrap_error_static(tool, error)
}

/// Execute sequential_record tool
pub async fn execute_sequential_record(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // Parse required parameters
    let task_id = params.get("task_id").and_then(|t| t.as_i64());
    let step_number = match params.get("step_number").and_then(|v| v.as_i64()) {
        Some(v) => v as i32,
        None => {
            return Ok(wrap_error_static("sequential_record", "Missing 'step_number' parameter"))
        }
    };
    let thought = match param_str("sequential_record", params, "thought") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let reasoning = match param_str("sequential_record", params, "reasoning") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let action = params.get("action").and_then(|a| a.as_str());
    let observation = params.get("observation").and_then(|o| o.as_str());

    if dry_run {
        let result = wrap_success(
            "sequential_record",
            json!({
                "dry_run": true,
                "message": "[DRY RUN] Would record thought step",
                "step_id": 1
            }),
        );
        return Ok(result);
    }

    // Record step using SequentialStep
    use crate::portfolio::sequential_step::{SequentialStep, ThoughtStep};
    let sequential = SequentialStep::new((**state).clone());

    let step = ThoughtStep {
        task_id,
        step_number,
        thought: thought.to_string(),
        action: action.map(|s| s.to_string()),
        observation: observation.map(|s| s.to_string()),
        reasoning: reasoning.to_string(),
    };

    let step_id = sequential
        .record_step(&step)
        .map_err(|e| anyhow::anyhow!("Failed to record step: {}", e))?;

    Ok(wrap_success(
        "sequential_record",
        json!({
            "success": true,
            "step_id": step_id,
            "message": "Thought step recorded successfully"
        }),
    ))
}

/// Execute sequential_get tool
pub async fn execute_sequential_get(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // PARAMETER VALIDATION - MUST BE FIRST
    let task_id = match params.get("task_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => return Ok(wrap_error_static("sequential_get", "Missing 'task_id' parameter")),
    };

    if dry_run {
        let result = wrap_success(
            "sequential_get",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would get steps for task: {}", task_id),
                "task_id": task_id,
                "steps": [],
                "count": 0
            }),
        );
        return Ok(result);
    }

    // Get steps using SequentialStep
    use crate::portfolio::sequential_step::SequentialStep;
    let sequential = SequentialStep::new((**state).clone());

    let steps = sequential
        .get_steps_for_task(task_id)
        .map_err(|e| anyhow::anyhow!("Failed to get steps: {}", e))?;

    Ok(wrap_success(
        "sequential_get",
        json!({
            "task_id": task_id,
            "steps": steps,
            "count": steps.len()
        }),
    ))
}

/// Execute sequential_search tool
pub async fn execute_sequential_search(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let query = match param_str("sequential_search", params, "query") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "sequential_search",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would search for: {}", query),
                "query": query,
                "results": [],
                "count": 0
            }),
        );
        return Ok(result);
    }

    // Search steps using SequentialStep with 3s timeout
    use crate::portfolio::sequential_step::SequentialStep;
    use std::time::Duration;

    let sequential = SequentialStep::new((**state).clone());
    let query_owned = query.to_string();

    let results = match tokio::time::timeout(
        Duration::from_secs(3),
        tokio::task::spawn_blocking(move || sequential.search_steps(&query_owned)),
    )
    .await
    {
        Ok(Ok(Ok(steps))) => steps,
        Ok(Ok(Err(e))) => {
            return Ok(wrap_error("sequential_search", &format!("IoError: Search failed: {}", e)))
        }
        Ok(Err(e)) => {
            return Ok(wrap_error("sequential_search", &format!("Internal: Task failed: {}", e)))
        }
        Err(_) => {
            return Ok(wrap_error(
                "sequential_search",
                "Timeout: sequential_search exceeded 3 seconds",
            ))
        }
    };

    Ok(wrap_success(
        "sequential_search",
        json!({
            "query": query,
            "results": results,
            "count": results.len()
        }),
    ))
}

/// Execute sequential_cycle tool
pub async fn execute_sequential_cycle(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let max_cycles =
        params.get("max_cycles").and_then(|m| m.as_u64()).map(|m| m as usize).unwrap_or(1);

    if dry_run {
        let result = wrap_success(
            "sequential_cycle",
            json!({
                "dry_run": true,
                "message": "[DRY RUN] Would run sequential reasoning cycle",
                "max_cycles": max_cycles,
                "success": true
            }),
        );
        return Ok(result);
    }

    // REAL IMPLEMENTATION - Run actual sequential reasoning cycle
    use crate::ollama::OllamaConfig;
    use crate::sequential::{OllamaLanguageModel, SequentialCore};
    use std::sync::Mutex;

    // Create Ollama configuration
    let model_name =
        std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:3B".to_string());

    let config = OllamaConfig {
        model: model_name.clone(),
        temperature: 0.7,
        max_tokens: 2000,
        timeout_secs: 60,
    };

    let llm = match OllamaLanguageModel::new(config) {
        Ok(llm) => Arc::new(Mutex::new(llm)) as Arc<Mutex<dyn crate::sequential::LanguageModel>>,
        Err(e) => {
            return Ok(wrap_error(
                "sequential_cycle",
                &format!(
                    "Failed to initialize Ollama language model with model '{}': {}. \
                Ensure Ollama is installed and the model is available (ollama pull {}).",
                    model_name, e, model_name
                ),
            ));
        }
    };

    // Create sequential reasoning engine
    let reasoning = SequentialCore::new(
        Arc::clone(&state.tasks),
        Arc::clone(&state.general_store),
        Arc::clone(&state.memory),
        llm,
        state.logger.clone(),
    );

    // Execute the reasoning cycle
    match reasoning.run_cycle() {
        Ok(result) => {
            use crate::sequential::CycleResult;
            match result {
                CycleResult::Completed {
                    task_id,
                    thought,
                    decision,
                    actions,
                    action_results,
                    reflection,
                } => Ok(wrap_success(
                    "sequential_cycle",
                    json!({
                        "success": true,
                        "task_id": task_id,
                        "thought": thought,
                        "decision": decision,
                        "actions": actions,
                        "action_results": action_results,
                        "reflection": reflection,
                        "message": "Sequential reasoning cycle completed successfully"
                    }),
                )),
                CycleResult::NoTasks => Ok(wrap_success(
                    "sequential_cycle",
                    json!({
                        "success": true,
                        "cycles": 0,
                        "message": "No tasks available for processing"
                    }),
                )),
            }
        }
        Err(e) => {
            Ok(wrap_error("sequential_cycle", &format!("Sequential reasoning cycle failed: {}", e)))
        }
    }
}
