//! IntelliTask Commands Module
//!
//! Handles execution of AI-powered task management operations.
//! Extracted from memory_suite.rs (lines 196-866).
//!
//! Commands:
//! - intellitask_list: List all tasks
//! - intellitask_get: Get specific task by ID
//! - intellitask_update_status: Update task status
//! - intellitask_next_ready: Get next ready task (dependencies satisfied)
//! - intellitask_get_subtasks: Get subtasks for parent task
//! - intellitask_subtask_stats: Get statistics for subtasks
//! - intellitask_task_statistics: Get overall task statistics
//! - intellitask_prd_statistics: Get PRD-specific statistics
//! - intellitask_generate: Generate tasks from PRD using AI
//! - intellitask_subtasks: Generate subtasks using AI
//! - intellitask_prioritize: Prioritize tasks using AI
//! - intellitask_next: Suggest next task using AI
//! - intellitask_save: Save task breakdown to database

use super::{MemorySuite, MemorySuiteArgs};
use crate::mcp_tools::SuiteResult;

pub fn cmd_intellitask_list(suite: &MemorySuite, _args: MemorySuiteArgs) -> SuiteResult {
    use crate::tasks::Task;

    let tasks_result: Result<Vec<Task>, rusqlite::Error> = {
        let db_guard = suite.state.tasks.db.lock().unwrap();
        let query = "SELECT id, goal, description, status, priority, parent_id, created_at, updated_at FROM tasks ORDER BY priority ASC, created_at ASC";

        db_guard.prepare(query).and_then(|mut stmt| {
            stmt.query_map([], |row| {
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
            })
            .and_then(|rows| rows.collect())
        })
    };

    match tasks_result {
        Ok(tasks) => SuiteResult::ok(
            "intellitask_list",
            serde_json::json!({
                "tasks": tasks,
                "count": tasks.len()
            }),
        ),
        Err(e) => SuiteResult::err("intellitask_list", format!("Database error: {}", e)),
    }
}

pub fn cmd_intellitask_get(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let task_id = match args.task_id {
        Some(tid) => tid,
        None => return SuiteResult::err("intellitask_get", "Missing required parameter: task_id"),
    };

    match suite.state.tasks.get_task(task_id) {
        Ok(Some(task)) => match serde_json::to_value(&task) {
            Ok(v) => SuiteResult::ok("intellitask_get", v),
            Err(e) => SuiteResult::err(
                "intellitask_get",
                format!("Failed to serialize task: {}", e),
            ),
        },
        Ok(None) => SuiteResult::err("intellitask_get", format!("Task {} not found", task_id)),
        Err(e) => SuiteResult::err("intellitask_get", format!("Database error: {}", e)),
    }
}

pub fn cmd_intellitask_update_status(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let task_id = match args.task_id {
        Some(tid) => tid,
        None => {
            return SuiteResult::err(
                "intellitask_update_status",
                "Missing required parameter: task_id",
            )
        }
    };

    let status = match args.status {
        Some(ref s) => s.as_str().unwrap_or("unknown"),
        None => {
            return SuiteResult::err(
                "intellitask_update_status",
                "Missing required parameter: status",
            )
        }
    };

    let db_guard = suite.state.tasks.db.lock().unwrap();
    match crate::tasks::update_task(&db_guard, task_id, Some(status), None, None) {
        Ok(_) => SuiteResult::ok(
            "intellitask_update_status",
            serde_json::json!({
                "updated": true,
                "task_id": task_id,
                "status": status
            }),
        ),
        Err(e) => SuiteResult::err(
            "intellitask_update_status",
            format!("Failed to update status: {}", e),
        ),
    }
}

pub fn cmd_intellitask_next_ready(suite: &MemorySuite, _args: MemorySuiteArgs) -> SuiteResult {
    // Find next task ready to work on (no pending dependencies)
    // Simple heuristic: tasks with status='open' and no parent_id, or whose parent is 'done'

    use crate::tasks::Task;

    let ready_tasks_result: Result<Vec<Task>, rusqlite::Error> = {
        let db_guard = suite.state.tasks.db.lock().unwrap();

        // Query: tasks that are 'open' AND either:
        // 1. Have no parent (top-level tasks)
        // 2. Have a parent that is 'done'
        let query = "
                SELECT t.id, t.goal, t.description, t.status, t.priority, t.parent_id, t.created_at, t.updated_at
                FROM tasks t
                WHERE t.status = 'open'
                  AND (
                    t.parent_id IS NULL
                    OR EXISTS (
                      SELECT 1 FROM tasks p
                      WHERE p.id = t.parent_id AND p.status = 'done'
                    )
                  )
                ORDER BY t.priority ASC, t.created_at ASC
                LIMIT 10
            ";

        db_guard.prepare(query).and_then(|mut stmt| {
            stmt.query_map([], |row| {
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
            })
            .and_then(|rows| rows.collect())
        })
    };

    match ready_tasks_result {
        Ok(tasks) => {
            if tasks.is_empty() {
                SuiteResult::ok(
                    "intellitask_next_ready",
                    serde_json::json!({
                        "ready_tasks": [],
                        "count": 0,
                        "message": "No tasks ready to work on. All tasks either completed or have pending dependencies."
                    }),
                )
            } else {
                SuiteResult::ok(
                    "intellitask_next_ready",
                    serde_json::json!({
                        "ready_tasks": tasks,
                        "count": tasks.len(),
                        "next_task": tasks.first()
                    }),
                )
            }
        }
        Err(e) => SuiteResult::err("intellitask_next_ready", format!("Database error: {}", e)),
    }
}

pub fn cmd_intellitask_get_subtasks(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let parent_id = match args.parent_id {
        Some(pid) => pid,
        None => {
            return SuiteResult::err(
                "intellitask_get_subtasks",
                "Missing required parameter: parent_id",
            )
        }
    };

    use crate::tasks::Task;

    let subtasks_result: Result<Vec<Task>, rusqlite::Error> = {
        let db_guard = suite.state.tasks.db.lock().unwrap();
        let query = "SELECT id, goal, description, status, priority, parent_id, created_at, updated_at FROM tasks WHERE parent_id = ? ORDER BY priority ASC";

        db_guard.prepare(query).and_then(|mut stmt| {
            stmt.query_map([parent_id], |row| {
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
            })
            .and_then(|rows| rows.collect())
        })
    };

    match subtasks_result {
        Ok(subtasks) => SuiteResult::ok(
            "intellitask_get_subtasks",
            serde_json::json!({
                "parent_id": parent_id,
                "subtasks": subtasks,
                "count": subtasks.len()
            }),
        ),
        Err(e) => SuiteResult::err("intellitask_get_subtasks", format!("Database error: {}", e)),
    }
}

pub fn cmd_intellitask_subtask_stats(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let parent_id = match args.parent_id {
        Some(pid) => pid,
        None => {
            return SuiteResult::err(
                "intellitask_subtask_stats",
                "Missing required parameter: parent_id",
            )
        }
    };

    let stats_result = {
        let db_guard = suite.state.tasks.db.lock().unwrap();
        let query = "SELECT status, COUNT(*) FROM tasks WHERE parent_id = ? GROUP BY status";

        db_guard.prepare(query).and_then(|mut stmt| {
            stmt.query_map([parent_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        })
    };

    match stats_result {
        Ok(stats) => {
            let mut stats_map = serde_json::Map::new();
            let mut total = 0i64;
            for (status, count) in stats {
                stats_map.insert(status, serde_json::json!(count));
                total += count;
            }
            stats_map.insert("total".to_string(), serde_json::json!(total));

            SuiteResult::ok(
                "intellitask_subtask_stats",
                serde_json::json!({
                    "parent_id": parent_id,
                    "stats": stats_map
                }),
            )
        }
        Err(e) => SuiteResult::err(
            "intellitask_subtask_stats",
            format!("Database error: {}", e),
        ),
    }
}

pub fn cmd_intellitask_task_statistics(suite: &MemorySuite, _args: MemorySuiteArgs) -> SuiteResult {
    let stats_result = {
        let db_guard = suite.state.tasks.db.lock().unwrap();
        let query = "SELECT status, COUNT(*) FROM tasks GROUP BY status";

        db_guard.prepare(query).and_then(|mut stmt| {
            stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        })
    };

    match stats_result {
        Ok(stats) => {
            let mut stats_map = serde_json::Map::new();
            let mut total = 0i64;
            for (status, count) in stats {
                stats_map.insert(status, serde_json::json!(count));
                total += count;
            }
            stats_map.insert("total".to_string(), serde_json::json!(total));

            SuiteResult::ok("intellitask_task_statistics", serde_json::json!(stats_map))
        }
        Err(e) => SuiteResult::err(
            "intellitask_task_statistics",
            format!("Database error: {}", e),
        ),
    }
}

pub fn cmd_intellitask_prd_statistics(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let prd_title = match args.prd_title {
        Some(ref pt) => pt,
        None => {
            return SuiteResult::err(
                "intellitask_prd_statistics",
                "Missing required parameter: prd_title",
            )
        }
    };

    // Get statistics for tasks related to a specific PRD
    // We match tasks where the goal or description contains the PRD title
    let stats_result: Result<Vec<(String, i64)>, rusqlite::Error> = {
        let db_guard = suite.state.tasks.db.lock().unwrap();

        let query = "
                SELECT status, COUNT(*) as count
                FROM tasks
                WHERE goal LIKE ? OR description LIKE ?
                GROUP BY status
            ";

        let search_pattern = format!("%{}%", prd_title);

        db_guard.prepare(query).and_then(|mut stmt| {
            stmt.query_map([&search_pattern, &search_pattern], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        })
    };

    match stats_result {
        Ok(stats) => {
            if stats.is_empty() {
                SuiteResult::ok(
                    "intellitask_prd_statistics",
                    serde_json::json!({
                        "prd_title": prd_title,
                        "stats": {},
                        "total": 0,
                        "message": "No tasks found for this PRD"
                    }),
                )
            } else {
                let mut stats_map = serde_json::Map::new();
                let mut total = 0i64;
                for (status, count) in stats {
                    stats_map.insert(status, serde_json::json!(count));
                    total += count;
                }
                stats_map.insert("total".to_string(), serde_json::json!(total));

                SuiteResult::ok(
                    "intellitask_prd_statistics",
                    serde_json::json!({
                        "prd_title": prd_title,
                        "stats": stats_map
                    }),
                )
            }
        }
        Err(e) => SuiteResult::err(
            "intellitask_prd_statistics",
            format!("Database error: {}", e),
        ),
    }
}

pub fn cmd_intellitask_generate(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    // Extract PRD content
    let prd_content = match args.prd_content {
        Some(content) => content,
        None => {
            return SuiteResult::err(
                "intellitask_generate",
                "Missing required parameter: prd_content",
            )
        }
    };

    // Check if IntelliTask is available
    let intellitask = match &suite.state.intellitask {
        Some(it) => it,
        None => {
            return SuiteResult::err(
                "intellitask_generate",
                "IntelliTask not available. LLM backend not initialized. \
                Set LLM_BACKEND=test for testing, or ensure Ollama is running for production.",
            )
        }
    };

    // Call IntelliTask to generate task breakdown
    match intellitask.generate_tasks_from_prd(&prd_content) {
        Ok(breakdown) => {
            // Convert to JSON
            match serde_json::to_value(&breakdown) {
                Ok(json) => SuiteResult::ok("intellitask_generate", json),
                Err(e) => SuiteResult::err(
                    "intellitask_generate",
                    format!("Failed to serialize task breakdown: {}", e),
                ),
            }
        }
        Err(e) => SuiteResult::err(
            "intellitask_generate",
            format!(
                "IntelliTask generation failed: {}. Check LLM backend connectivity.",
                e
            ),
        ),
    }
}

pub fn cmd_intellitask_subtasks(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    // Extract parent task JSON
    let parent_task_json = match args.parent_task_json {
        Some(ref json) => json,
        None => {
            return SuiteResult::err(
                "intellitask_subtasks",
                "Missing required parameter: parent_task_json",
            )
        }
    };

    // Check if IntelliTask is available
    let intellitask = match &suite.state.intellitask {
        Some(it) => it,
        None => {
            return SuiteResult::err(
                "intellitask_subtasks",
                "IntelliTask not available. LLM backend not initialized. \
                Set LLM_BACKEND=test for testing, or ensure Ollama is running for production.",
            )
        }
    };

    // Parse parent task
    let parent_task: crate::intellitask::ParentTask = match serde_json::from_str(parent_task_json) {
        Ok(task) => task,
        Err(e) => {
            return SuiteResult::err(
                "intellitask_subtasks",
                format!("Failed to parse parent_task_json: {}", e),
            )
        }
    };

    // Get codebase context (optional)
    let codebase_context = args.query.as_deref().unwrap_or("");

    // Call IntelliTask to generate subtasks
    match intellitask.generate_subtasks(&parent_task, codebase_context) {
        Ok(subtasks) => match serde_json::to_value(&subtasks) {
            Ok(json) => SuiteResult::ok("intellitask_subtasks", json),
            Err(e) => SuiteResult::err(
                "intellitask_subtasks",
                format!("Failed to serialize subtasks: {}", e),
            ),
        },
        Err(e) => SuiteResult::err(
            "intellitask_subtasks",
            format!(
                "IntelliTask subtask generation failed: {}. Check LLM backend connectivity.",
                e
            ),
        ),
    }
}

pub fn cmd_intellitask_prioritize(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    // Extract tasks JSON
    let tasks_json = match args.tasks_json {
        Some(ref json) => json,
        None => {
            return SuiteResult::err(
                "intellitask_prioritize",
                "Missing required parameter: tasks_json",
            )
        }
    };

    // Check if IntelliTask is available
    let intellitask = match &suite.state.intellitask {
        Some(it) => it,
        None => {
            return SuiteResult::err(
                "intellitask_prioritize",
                "IntelliTask not available. LLM backend not initialized. \
                Set LLM_BACKEND=test for testing, or ensure Ollama is running for production.",
            )
        }
    };

    // Parse tasks
    let tasks: Vec<crate::intellitask::ParentTask> = match serde_json::from_str(tasks_json) {
        Ok(tasks) => tasks,
        Err(e) => {
            return SuiteResult::err(
                "intellitask_prioritize",
                format!("Failed to parse tasks_json: {}", e),
            )
        }
    };

    // Get business context (optional)
    let business_context = args.business_context.as_deref().unwrap_or("");

    // Call IntelliTask to prioritize tasks
    match intellitask.prioritize_tasks(&tasks, business_context) {
        Ok(priorities) => {
            // Convert to JSON-friendly format
            let priorities_json: Vec<serde_json::Value> = priorities
                .into_iter()
                .map(|(task_id, priority)| {
                    serde_json::json!({
                        "task_id": task_id,
                        "priority": format!("{:?}", priority)
                    })
                })
                .collect();

            SuiteResult::ok(
                "intellitask_prioritize",
                serde_json::json!({ "priorities": priorities_json }),
            )
        }
        Err(e) => SuiteResult::err(
            "intellitask_prioritize",
            format!(
                "IntelliTask prioritization failed: {}. Check LLM backend connectivity.",
                e
            ),
        ),
    }
}

pub fn cmd_intellitask_next(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    // Extract completed tasks
    let completed_tasks = match args.completed_tasks {
        Some(ref tasks) => tasks.clone(),
        None => {
            return SuiteResult::err(
                "intellitask_next",
                "Missing required parameter: completed_tasks",
            )
        }
    };

    // Extract remaining tasks JSON
    let remaining_tasks_json = match args.remaining_tasks_json {
        Some(ref json) => json,
        None => {
            return SuiteResult::err(
                "intellitask_next",
                "Missing required parameter: remaining_tasks_json",
            )
        }
    };

    // Check if IntelliTask is available
    let intellitask = match &suite.state.intellitask {
        Some(it) => it,
        None => {
            return SuiteResult::err(
                "intellitask_next",
                "IntelliTask not available. LLM backend not initialized. \
                Set LLM_BACKEND=test for testing, or ensure Ollama is running for production.",
            )
        }
    };

    // Parse remaining tasks
    let remaining_tasks: Vec<crate::intellitask::ParentTask> =
        match serde_json::from_str(remaining_tasks_json) {
            Ok(tasks) => tasks,
            Err(e) => {
                return SuiteResult::err(
                    "intellitask_next",
                    format!("Failed to parse remaining_tasks_json: {}", e),
                )
            }
        };

    // Call IntelliTask to suggest next task
    match intellitask.suggest_next_task(&completed_tasks, &remaining_tasks) {
        Ok(next_task_id) => SuiteResult::ok(
            "intellitask_next",
            serde_json::json!({
                "next_task_id": next_task_id,
                "completed_count": completed_tasks.len(),
                "remaining_count": remaining_tasks.len()
            }),
        ),
        Err(e) => SuiteResult::err(
            "intellitask_next",
            format!(
                "IntelliTask next task suggestion failed: {}. Check LLM backend connectivity.",
                e
            ),
        ),
    }
}

pub fn cmd_intellitask_save(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    // Extract breakdown JSON
    let breakdown_json = match args.breakdown_json {
        Some(ref json) => json,
        None => {
            return SuiteResult::err(
                "intellitask_save",
                "Missing required parameter: breakdown_json",
            )
        }
    };

    // Parse task breakdown
    let breakdown: crate::intellitask::TaskBreakdown = match serde_json::from_str(breakdown_json) {
        Ok(b) => b,
        Err(e) => {
            return SuiteResult::err(
                "intellitask_save",
                format!("Failed to parse breakdown_json: {}", e),
            )
        }
    };

    // Batch insert tasks
    let mut parent_task_ids = Vec::new();
    let mut total_subtasks = 0;

    for parent_task in &breakdown.parent_tasks {
        // Insert parent task
        let parent_id = match suite.state.tasks.add_task(
            &parent_task.title,
            &parent_task.description,
            3, // Default priority
            None,
        ) {
            Ok(id) => id,
            Err(e) => {
                return SuiteResult::err(
                    "intellitask_save",
                    format!(
                        "Failed to insert parent task '{}': {}",
                        parent_task.title, e
                    ),
                )
            }
        };
        parent_task_ids.push(parent_id);

        // Insert subtasks
        for subtask in &parent_task.subtasks {
            match suite.state.tasks.add_task(
                &format!("{}: {}", subtask.id, subtask.description),
                &subtask.acceptance_criteria.join("\n"),
                3, // Default priority
                Some(parent_id),
            ) {
                Ok(_) => total_subtasks += 1,
                Err(e) => {
                    return SuiteResult::err(
                        "intellitask_save",
                        format!("Failed to insert subtask '{}': {}", subtask.id, e),
                    )
                }
            }
        }
    }

    SuiteResult::ok(
        "intellitask_save",
        serde_json::json!({
            "prd_title": breakdown.prd_title,
            "parent_tasks_inserted": parent_task_ids.len(),
            "subtasks_inserted": total_subtasks,
            "total_tasks": parent_task_ids.len() + total_subtasks
        }),
    )
}
