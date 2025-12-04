//! Task Tools Executor
//!
//! Handles execution of task management and intellitask tools.
//! Extracted from executor_real.rs giant match statement (lines 161-440).
//!
//! Tools:
//! - task_create: Create a new task
//! - intellitask_list: List all tasks
//! - intellitask_get: Get a specific task
//! - intellitask_update_status: Update task status
//! - intellitask_next_ready: Find next ready task
//! - intellitask_get_subtasks: Get subtasks for a parent
//! - intellitask_subtask_stats: Get subtask statistics
//! - intellitask_task_statistics: Get overall task statistics
//! - intellitask_prd_statistics: Get PRD-specific statistics

use crate::intellitask_persistence::IntelliTaskPersistence;
use crate::mcp::types::ErrorType;
use crate::router::SynCoreState;
use crate::tasks::{Task, Tasks};
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

/// Execute task_create tool
pub async fn execute_task_create(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let goal = match param_str("task_create", params, "goal") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let priority = params.get("priority").and_then(|v| v.as_i64()).unwrap_or(1) as i32;

    if dry_run {
        let result = wrap_success(
            "task_create",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would create task with goal='{}' and priority={}", goal, priority)
            }),
        );
        return Ok(result);
    }

    // Use Tasks.add_task() for real execution
    let task_id = state.tasks.add_task(goal, "", priority, None)?;
    Ok(wrap_success(
        "task_create",
        json!({
            "created": true,
            "task_id": task_id,
            "message": format!("Task created with ID: {}", task_id)
        }),
    ))
}

/// Execute intellitask_list tool
pub async fn execute_intellitask_list(
    state: &Arc<SynCoreState>,
    _params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        let result = wrap_success(
            "intellitask_list",
            json!({
                "dry_run": true,
                "message": "[DRY RUN] Would list all tasks"
            }),
        );
        return Ok(result);
    }

    // Use Tasks directly to list all tasks (no filtering for now - simpler)
    let db = state.tasks.db.lock().unwrap();

    let query = "SELECT id, goal, description, status, priority, parent_id, created_at, updated_at FROM tasks ORDER BY priority ASC, created_at ASC";
    let mut stmt = db.prepare(query)?;

    let tasks: Vec<Task> = stmt
        .query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                goal: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                parent_id: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(wrap_success(
        "intellitask_list",
        json!({
            "tasks": tasks,
            "count": tasks.len()
        }),
    ))
}

/// Execute intellitask_get tool
pub async fn execute_intellitask_get(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let task_id = match params.get("task_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => return Ok(wrap_error_static("intellitask_get", "Missing 'task_id' parameter")),
    };

    if dry_run {
        let result = wrap_success(
            "intellitask_get",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would get task with id={}", task_id)
            }),
        );
        return Ok(result);
    }

    // Use Tasks.get_task() for real execution
    match state.tasks.get_task(task_id) {
        Ok(Some(task)) => match serde_json::to_value(&task) {
            Ok(v) => Ok(v),
            Err(e) => {
                Ok(wrap_error("intellitask_get", &format!("Failed to serialize task: {}", e)))
            }
        },
        Ok(None) => Ok(wrap_error("intellitask_get", &format!("Task {} not found", task_id))),
        Err(e) => Ok(wrap_error("intellitask_get", &format!("Database error: {}", e))),
    }
}

/// Execute intellitask_update_status tool
pub async fn execute_intellitask_update_status(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let task_id = match params.get("task_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => {
            return Ok(wrap_error_static(
                "intellitask_update_status",
                "Missing 'task_id' parameter",
            ))
        }
    };
    let status = match param_str("intellitask_update_status", params, "status") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "intellitask_update_status",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would update task {} to status '{}'", task_id, status)
            }),
        );
        return Ok(result);
    }

    // Use Tasks.update_task() for real execution
    let db = state.tasks.db.lock().unwrap();
    Tasks::update_task(&db, task_id, Some(status), None, None)?;

    Ok(wrap_success(
        "intellitask_update_status",
        json!({
            "updated": true,
            "task_id": task_id,
            "status": status
        }),
    ))
}

/// Execute intellitask_next_ready tool
pub async fn execute_intellitask_next_ready(
    _state: &Arc<SynCoreState>,
    _params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        let result = wrap_success(
            "intellitask_next_ready",
            json!({
                "dry_run": true,
                "message": "[DRY RUN] Would find next ready task"
            }),
        );
        return Ok(result);
    }

    // Use IntelliTaskPersistence.next_task()
    let persistence = IntelliTaskPersistence::new(":memory:")?;

    match persistence.next_task()? {
        Some(task) => Ok(serde_json::to_value(&task)?),
        None => Ok(wrap_success(
            "intellitask_next_ready",
            json!({
                "next_task": null,
                "message": "No ready tasks available"
            }),
        )),
    }
}

/// Execute intellitask_get_subtasks tool
pub async fn execute_intellitask_get_subtasks(
    _state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let parent_id = match params.get("parent_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => {
            return Ok(wrap_error_static(
                "intellitask_get_subtasks",
                "Missing 'parent_id' parameter",
            ))
        }
    };

    if dry_run {
        let result = wrap_success(
            "intellitask_get_subtasks",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would get subtasks for parent {}", parent_id)
            }),
        );
        return Ok(result);
    }

    // Use IntelliTaskPersistence.get_subtasks()
    let persistence = IntelliTaskPersistence::new(":memory:")?;
    let subtasks = persistence.get_subtasks(parent_id)?;

    Ok(wrap_success(
        "intellitask_get_subtasks",
        json!({
            "subtasks": subtasks,
            "count": subtasks.len(),
            "parent_id": parent_id
        }),
    ))
}

/// Execute intellitask_subtask_stats tool
pub async fn execute_intellitask_subtask_stats(
    _state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let parent_id = match params.get("parent_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => {
            return Ok(wrap_error_static(
                "intellitask_subtask_stats",
                "Missing 'parent_id' parameter",
            ))
        }
    };

    if dry_run {
        let result = wrap_success(
            "intellitask_subtask_stats",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would get subtask statistics for parent {}", parent_id)
            }),
        );
        return Ok(result);
    }

    // Use IntelliTaskPersistence.get_subtask_statistics()
    let persistence = IntelliTaskPersistence::new(":memory:")?;
    let stats = persistence.get_subtask_statistics(parent_id)?;

    Ok(serde_json::to_value(&stats)?)
}

/// Execute intellitask_task_statistics tool
pub async fn execute_intellitask_task_statistics(
    _state: &Arc<SynCoreState>,
    _params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        let result = wrap_success(
            "intellitask_task_statistics",
            json!({
                "dry_run": true,
                "message": "[DRY RUN] Would get overall task statistics"
            }),
        );
        return Ok(result);
    }

    // Use IntelliTaskPersistence.get_task_statistics()
    let persistence = IntelliTaskPersistence::new(":memory:")?;
    let stats = persistence.get_task_statistics()?;

    Ok(serde_json::to_value(&stats)?)
}

/// Execute intellitask_prd_statistics tool
pub async fn execute_intellitask_prd_statistics(
    _state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let prd_title = match param_str("intellitask_prd_statistics", params, "prd_title") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "intellitask_prd_statistics",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would get statistics for PRD '{}'", prd_title)
            }),
        );
        return Ok(result);
    }

    // Use IntelliTaskPersistence.get_prd_statistics()
    let persistence = IntelliTaskPersistence::new(":memory:")?;
    let stats = persistence.get_prd_statistics(prd_title)?;

    Ok(serde_json::to_value(&stats)?)
}

/// Execute intellitask_generate tool
pub async fn execute_intellitask_generate(
    state: &crate::router::SynCoreState,
    params: &serde_json::Value,
    dry_run: bool,
) -> anyhow::Result<serde_json::Value> {
    if dry_run {
        let prd_content = param_str("intellitask_generate", params, "prd_content").unwrap_or("");
        let result = serde_json::json!({
            "success": true,
            "message": format!("[DRY RUN] Would generate tasks from PRD content")
        });
        return Ok(result);
    }

    // Delegate to memory suite for actual implementation
    let suite = crate::mcp_tools::memory_suite::MemorySuite::new(state.clone());
    let args = crate::mcp_tools::memory_suite::MemorySuiteArgs {
        command: "intellitask_generate".to_string(),
        prd_content: params.get("prd_content").and_then(|v| v.as_str()).map(|s| s.to_string()),
        ..Default::default()
    };

    match suite.execute(args) {
        crate::mcp_tools::SuiteResult { success: true, data, .. } => Ok(data),
        crate::mcp_tools::SuiteResult { success: false, error, .. } => {
            Ok(serde_json::json!({
                "success": false,
                "error": error
            }))
        }
    }
}

/// Execute intellitask_subtasks tool
pub async fn execute_intellitask_subtasks(
    state: &crate::router::SynCoreState,
    params: &serde_json::Value,
    dry_run: bool,
) -> anyhow::Result<serde_json::Value> {
    if dry_run {
        let parent_task_id = param_str("intellitask_subtasks", params, "parent_task_id").unwrap_or("");
        let result = serde_json::json!({
            "success": true,
            "message": format!("[DRY RUN] Would generate subtasks for parent task '{}'", parent_task_id)
        });
        return Ok(result);
    }

    // Delegate to memory suite for actual implementation
    let suite = crate::mcp_tools::memory_suite::MemorySuite::new(state.clone());
    let args = crate::mcp_tools::memory_suite::MemorySuiteArgs {
        command: "intellitask_subtasks".to_string(),
        parent_task_id: params.get("parent_task_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        ..Default::default()
    };

    match suite.execute(args) {
        crate::mcp_tools::SuiteResult { success: true, data, .. } => Ok(data),
        crate::mcp_tools::SuiteResult { success: false, error, .. } => {
            Ok(serde_json::json!({
                "success": false,
                "error": error
            }))
        }
    }
}

/// Execute intellitask_prioritize tool
pub async fn execute_intellitask_prioritize(
    state: &crate::router::SynCoreState,
    params: &serde_json::Value,
    dry_run: bool,
) -> anyhow::Result<serde_json::Value> {
    if dry_run {
        let result = serde_json::json!({
            "success": true,
            "message": "[DRY RUN] Would prioritize tasks"
        });
        return Ok(result);
    }

    // Delegate to memory suite for actual implementation
    let suite = crate::mcp_tools::memory_suite::MemorySuite::new(state.clone());
    let args = crate::mcp_tools::memory_suite::MemorySuiteArgs {
        command: "intellitask_prioritize".to_string(),
        tasks_json: params.get("tasks_json").and_then(|v| v.as_str()).map(|s| s.to_string()),
        business_context: params.get("business_context").and_then(|v| v.as_str()).map(|s| s.to_string()),
        ..Default::default()
    };

    match suite.execute(args) {
        crate::mcp_tools::SuiteResult { success: true, data, .. } => Ok(data),
        crate::mcp_tools::SuiteResult { success: false, error, .. } => {
            Ok(serde_json::json!({
                "success": false,
                "error": error
            }))
        }
    }
}

/// Execute intellitask_next tool
pub async fn execute_intellitask_next(
    state: &crate::router::SynCoreState,
    params: &serde_json::Value,
    dry_run: bool,
) -> anyhow::Result<serde_json::Value> {
    if dry_run {
        let result = serde_json::json!({
            "success": true,
            "message": "[DRY RUN] Would suggest next task"
        });
        return Ok(result);
    }

    // Delegate to memory suite for actual implementation
    let suite = crate::mcp_tools::memory_suite::MemorySuite::new(state.clone());
    let args = crate::mcp_tools::memory_suite::MemorySuiteArgs {
        command: "intellitask_next".to_string(),
        completed_tasks: params.get("completed_tasks").and_then(|v| v.as_array()).map(|arr| {
            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
        }),
        remaining_tasks_json: params.get("remaining_tasks_json").and_then(|v| v.as_str()).map(|s| s.to_string()),
        ..Default::default()
    };

    match suite.execute(args) {
        crate::mcp_tools::SuiteResult { success: true, data, .. } => Ok(data),
        crate::mcp_tools::SuiteResult { success: false, error, .. } => {
            Ok(serde_json::json!({
                "success": false,
                "error": error
            }))
        }
    }
}

/// Execute intellitask_save tool
pub async fn execute_intellitask_save(
    state: &crate::router::SynCoreState,
    params: &serde_json::Value,
    dry_run: bool,
) -> anyhow::Result<serde_json::Value> {
    if dry_run {
        let result = serde_json::json!({
            "success": true,
            "message": "[DRY RUN] Would save task breakdown"
        });
        return Ok(result);
    }

    // Delegate to memory suite for actual implementation
    let suite = crate::mcp_tools::memory_suite::MemorySuite::new(state.clone());
    let args = crate::mcp_tools::memory_suite::MemorySuiteArgs {
        command: "intellitask_save".to_string(),
        breakdown_json: params.get("breakdown_json").and_then(|v| v.as_str()).map(|s| s.to_string()),
        ..Default::default()
    };

    match suite.execute(args) {
        crate::mcp_tools::SuiteResult { success: true, data, .. } => Ok(data),
        crate::mcp_tools::SuiteResult { success: false, error, .. } => {
            Ok(serde_json::json!({
                "success": false,
                "error": error
            }))
        }
    }
}
