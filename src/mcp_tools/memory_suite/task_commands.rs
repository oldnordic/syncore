//! Task Commands Module
//!
//! Handles execution of task creation operations.
//! Extracted from memory_suite.rs (lines 191-221).
//!
//! Commands:
//! - task_create: Create new task with goal and priority

use crate::mcp_tools::SuiteResult;
use super::{MemorySuite, MemorySuiteArgs};

/// Execute task_create command
pub fn cmd_task_create(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let goal = match args.goal {
        Some(g) => g,
        None => return SuiteResult::err("task_create", "Missing required parameter: goal"),
    };

    let priority = args.priority.unwrap_or(3);

    if args.dry_run.unwrap_or(false) {
        return SuiteResult::ok(
            "task_create",
            serde_json::json!({
                "dry_run": true,
                "would_create": { "goal": goal, "priority": priority }
            }),
        );
    }

    match suite.state.tasks.add_task(&goal, "", priority, None) {
        Ok(task_id) => SuiteResult::ok(
            "task_create",
            serde_json::json!({
                "created": true,
                "task_id": task_id,
                "goal": goal,
                "priority": priority
            }),
        ),
        Err(e) => SuiteResult::err("task_create", e.to_string()),
    }
}
