//! Task Commands Module
//!
//! Handles execution of task creation operations.
//! Extracted from memory_suite.rs (lines 191-221).
//!
//! Commands:
//! - task_create: Create new task with goal and priority
//! - task_list: List all tasks
//! - task_get: Get specific task by ID
//! - task_update: Update task status
//! - task_next: Get next task ready to work on

use super::{MemorySuite, MemorySuiteArgs};
use crate::mcp_tools::SuiteResult;

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

/// Execute task_list command
pub fn cmd_task_list(_suite: &MemorySuite, _args: MemorySuiteArgs) -> SuiteResult {
    // For now, return empty list since list_tasks method doesn't exist
    // This would need to be implemented in Tasks struct
    SuiteResult::ok(
        "task_list",
        serde_json::json!({"tasks": [], "note": "list_tasks method not implemented yet"}),
    )
}

/// Execute task_get command
pub fn cmd_task_get(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let task_id = match args.task_id {
        Some(id) => id,
        None => return SuiteResult::err("task_get", "Missing required parameter: task_id"),
    };

    match suite.state.tasks.get_task(task_id) {
        Ok(Some(task)) => SuiteResult::ok("task_get", serde_json::to_value(task).unwrap()),
        Ok(None) => SuiteResult::err("task_get", format!("Task {} not found", task_id)),
        Err(e) => SuiteResult::err("task_get", format!("Database error: {}", e)),
    }
}

/// Execute task_update command
pub fn cmd_task_update(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let task_id = match args.task_id {
        Some(id) => id,
        None => return SuiteResult::err("task_update", "Missing required parameter: task_id"),
    };

    let status = match args.status {
        Some(s) => s,
        None => return SuiteResult::err("task_update", "Missing required parameter: status"),
    };

    // Convert status to string if it's a Value, otherwise use as-is
    let status_str = match status {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    };

    // Use the exported function with database connection
    let db = suite.state.tasks.get_db();
    let conn = db.lock().unwrap();
    match crate::tasks::update_task(&conn, task_id, Some(&status_str), None, None) {
        Ok(()) => SuiteResult::ok(
            "task_update",
            serde_json::json!({
                "updated": true,
                "task_id": task_id,
                "status": status_str
            }),
        ),
        Err(e) => SuiteResult::err("task_update", format!("Failed to update status: {}", e)),
    }
}

/// Execute task_next command
pub fn cmd_task_next(suite: &MemorySuite, _args: MemorySuiteArgs) -> SuiteResult {
    // Use the exported function with database connection
    let db = suite.state.tasks.get_db();
    let conn = db.lock().unwrap();
    match crate::tasks::next_task(&conn, None, None) {
        Ok(Some(task)) => SuiteResult::ok("task_next", serde_json::to_value(task).unwrap()),
        Ok(None) => {
            SuiteResult::ok("task_next", serde_json::json!({"message": "No ready tasks found"}))
        }
        Err(e) => SuiteResult::err("task_next", format!("Database error: {}", e)),
    }
}

/// Execute task_create_dependency command
pub fn cmd_task_create_dependency(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let task_id = match args.task_id {
        Some(id) => id,
        None => return SuiteResult::err("task_create_dependency", "Missing required parameter: task_id"),
    };

    let depends_on_task_id = match args.depends_on_task_id {
        Some(id) => id,
        None => return SuiteResult::err("task_create_dependency", "Missing required parameter: depends_on_task_id"),
    };

    // Validate that both tasks exist
    let db = suite.state.tasks.get_db();
    let conn = db.lock().unwrap();

    // Check source task exists
    let mut src_check = conn.prepare("SELECT id FROM tasks WHERE id = ?1").unwrap();
    if src_check.query_row([task_id], |_| Ok(())).is_err() {
        return SuiteResult::err("task_create_dependency", format!("Source task {} not found", task_id));
    }

    // Check destination task exists
    let mut dst_check = conn.prepare("SELECT id FROM tasks WHERE id = ?1").unwrap();
    if dst_check.query_row([depends_on_task_id], |_| Ok(())).is_err() {
        return SuiteResult::err("task_create_dependency", format!("Dependency task {} not found", depends_on_task_id));
    }

    // Create dependency relationship
    match conn.execute(
        "INSERT OR REPLACE INTO task_links (src_id, dst_id, kind) VALUES (?1, ?2, 'depends_on')",
        (task_id, depends_on_task_id),
    ) {
        Ok(_) => SuiteResult::ok(
            "task_create_dependency",
            serde_json::json!({
                "created": true,
                "task_id": task_id,
                "depends_on": depends_on_task_id,
                "relationship": "depends_on"
            }),
        ),
        Err(e) => SuiteResult::err("task_create_dependency", format!("Failed to create dependency: {}", e)),
    }
}

/// Execute task_get_graph command
pub fn cmd_task_get_graph(suite: &MemorySuite, _args: MemorySuiteArgs) -> SuiteResult {
    let db = suite.state.tasks.get_db();
    let conn = db.lock().unwrap();

    // Get all tasks with their dependencies
    let mut tasks_stmt = conn.prepare("
        SELECT t.id, t.goal, t.status, t.priority, t.parent_id
        FROM tasks t
        ORDER BY t.id
    ").unwrap();

    let task_rows = tasks_stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "goal": row.get::<_, String>(1)?,
            "status": row.get::<_, String>(2)?,
            "priority": row.get::<_, i32>(3)?,
            "parent_id": row.get::<_, Option<i64>>(4)?
        }))
    }).unwrap();

    let mut tasks = Vec::new();
    for task in task_rows {
        tasks.push(task.unwrap());
    }

    // Get all dependencies
    let mut deps_stmt = conn.prepare("
        SELECT tl.src_id, tl.dst_id, tl.kind
        FROM task_links tl
        WHERE tl.kind = 'depends_on'
        ORDER BY tl.src_id, tl.dst_id
    ").unwrap();

    let dep_rows = deps_stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "source_id": row.get::<_, i64>(0)?,
            "target_id": row.get::<_, i64>(1)?,
            "relationship": row.get::<_, String>(2)?
        }))
    }).unwrap();

    let mut dependencies = Vec::new();
    for dep in dep_rows {
        dependencies.push(dep.unwrap());
    }

    SuiteResult::ok(
        "task_get_graph",
        serde_json::json!({
            "tasks": tasks,
            "dependencies": dependencies,
            "total_tasks": tasks.len(),
            "total_dependencies": dependencies.len()
        }),
    )
}
