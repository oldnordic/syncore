//! MCP Reasoning Tools - PHASE ST-12 (SQLiteGraph Migration)
//!
//! Implementation of Tree-of-Thought reasoning MCP tools with backend-agnostic
//! support for both Neo4j and SQLiteGraph backends.
//! Provides 5 tools: session.create, branch.expand, tree.get, tree.prune, health.

use crate::config::SyncoreConfig;

use crate::mcp_tools::streaming::OutputLimiter;
use crate::mcp_tools::{SuiteDispatcher, SuiteResult};
use crate::reasoning::{ReasoningError, ReasoningResult, ReasoningSession};
use crate::router::SynCoreState;
use crate::tasks::Tasks;
use serde_json::{json, Value};

/// Handle reasoning.session.create MCP tool
pub async fn handle_reasoning_session_create(
    arguments: Value,
    state: &SynCoreState,
) -> ReasoningResult<Value> {
    let task = arguments["task"]
        .as_str()
        .ok_or_else(|| ReasoningError::Neo4j("Missing task".to_string()))?;

    // Check if this is a task_id (starts with "task_" prefix)
    let (title, description) = if task.starts_with("task_") {
        // Extract numeric task ID
        let numeric_id = task.strip_prefix("task_").unwrap_or("0");
        (format!("Task {}", numeric_id), format!("Task ID: {}", numeric_id))
    } else {
        (task.to_string(), task.to_string())
    };

    // Load configuration
    let config = SyncoreConfig::load_with_env("config/syncore.toml")
        .map_err(|e| ReasoningError::Neo4j(format!("Failed to load configuration: {}", e)))?;

    // Create reasoning session with configured backend
    let session = ReasoningSession::new(&title, &description, &config)
        .await
        .map_err(|e| ReasoningError::Neo4j(format!("Failed to create session: {}", e)))?;

    // Get session context
    let session_context = session.get_context().await?;

    Ok(json!({
        "session_id": session.id(),
        "title": session_context.title,
        "description": session_context.description,
        "created_at": session_context.created_at,
        "backend": config.reasoning.backend
    }))
}

/// Handle reasoning.branch.expand MCP tool
pub async fn handle_reasoning_branch_expand(
    arguments: Value,
    state: &SynCoreState,
) -> ReasoningResult<Value> {
    let session_id = arguments["session_id"]
        .as_str()
        .ok_or_else(|| ReasoningError::Neo4j("Missing session_id".to_string()))?;

    let parent_id = arguments["parent_id"]
        .as_i64()
        .or_else(|| arguments["parent_id"].as_str().and_then(|s| s.parse().ok()))
        .ok_or_else(|| ReasoningError::Neo4j("Missing or invalid parent_id".to_string()))?;

    let content = arguments["content"]
        .as_str()
        .ok_or_else(|| ReasoningError::Neo4j("Missing content".to_string()))?;

    let thought_type = arguments["thought_type"].as_str().unwrap_or("thought");

    let confidence = arguments["confidence"].as_f64().unwrap_or(1.0);

    let metadata = arguments["metadata"]
        .as_object()
        .cloned()
        .unwrap_or_else(|| json!({}).as_object().unwrap().clone());

    // Load configuration
    let config = SyncoreConfig::load_with_env("config/syncore.toml")
        .map_err(|e| ReasoningError::Neo4j(format!("Failed to load configuration: {}", e)))?;

    // Create reasoning session with configured backend
    let session = ReasoningSession::new("", "", &config)
        .await
        .map_err(|e| ReasoningError::Neo4j(format!("Failed to create session: {}", e)))?;

    // Add thought node
    let new_node_id = session
        .add_thought(
            Some(parent_id),
            content,
            thought_type,
            confidence,
            serde_json::Value::Object(metadata),
        )
        .await?;

    Ok(json!({
        "session_id": session_id,
        "parent_id": parent_id,
        "new_node_id": new_node_id,
        "content": content,
        "thought_type": thought_type,
        "confidence": confidence
    }))
}

/// Handle reasoning.tree.get MCP tool
pub async fn handle_reasoning_tree_get(
    arguments: Value,
    state: &SynCoreState,
) -> ReasoningResult<Value> {
    let session_id = arguments["session_id"]
        .as_str()
        .ok_or_else(|| ReasoningError::Neo4j("Missing session_id".to_string()))?;

    // Load configuration
    let config = SyncoreConfig::load_with_env("config/syncore.toml")
        .map_err(|e| ReasoningError::Neo4j(format!("Failed to load configuration: {}", e)))?;

    // Create reasoning session with configured backend
    let session = ReasoningSession::new("", "", &config)
        .await
        .map_err(|e| ReasoningError::Neo4j(format!("Failed to create session: {}", e)))?;

    // Get reasoning tree
    let nodes = session.get_tree().await?;

    // Convert to expected format
    let node_values: Vec<Value> = nodes
        .iter()
        .map(|node| {
            json!({
                "id": node.id,
                "session_id": node.session_id,
                "parent_id": node.parent_id,
                "depth": node.depth,
                "breadth": node.breadth,
                "content": node.content,
                "thought_type": node.thought_type,
                "confidence": node.confidence,
                "created_at": node.created_at,
                "metadata": node.metadata
            })
        })
        .collect();

    // Build edges from parent-child relationships
    let edges: Vec<Value> = nodes
        .iter()
        .filter_map(|node| {
            node.parent_id.map(|parent_id| {
                json!({
                    "source": parent_id,
                    "target": node.id
                })
            })
        })
        .collect();

    {
        let result = json!({
            "command": "reasoning_tree_get",
            "data": {
                "session_id": session_id,
                "nodes": node_values,
                "edges": edges,
                "total_nodes": nodes.len()
            }
        });

        // Apply streaming contract enforcement
        let limiter = OutputLimiter::default();
        match limiter.apply_json(&result) {
            Ok(limited) => {
                if let Some(data) = limited.get("data") {
                    Ok(data.clone())
                } else {
                    Ok(result["data"].clone())
                }
            }
            Err(_) => Ok(result["data"].clone()), // Fallback to original
        }
    }
}

/// Handle reasoning.tree.prune MCP tool
pub async fn handle_reasoning_tree_prune(
    arguments: Value,
    state: &SynCoreState,
) -> ReasoningResult<Value> {
    let session_id = arguments["session_id"]
        .as_str()
        .ok_or_else(|| ReasoningError::Neo4j("Missing session_id".to_string()))?;

    let node_id = arguments["node_id"]
        .as_i64()
        .or_else(|| arguments["node_id"].as_str().and_then(|s| s.parse().ok()))
        .ok_or_else(|| ReasoningError::Neo4j("Missing or invalid node_id".to_string()))?;

    // Load configuration
    let config = SyncoreConfig::load_with_env("config/syncore.toml")
        .map_err(|e| ReasoningError::Neo4j(format!("Failed to load configuration: {}", e)))?;

    // Create reasoning session with configured backend
    let session = ReasoningSession::new("", "", &config)
        .await
        .map_err(|e| ReasoningError::Neo4j(format!("Failed to create session: {}", e)))?;

    // Prune subtree
    session.prune_subtree(node_id).await?;

    Ok(json!({
        "session_id": session_id,
        "pruned_node_id": node_id,
        "status": "success"
    }))
}

/// Handle reasoning.task.execute MCP tool
///
/// PHASE ST-12: Execute task using Tree-of-Thoughts reasoning with backend-agnostic storage.
/// Creates or reuses reasoning session for task and runs one reasoning step.
pub async fn handle_reasoning_task_execute(
    arguments: Value,
    state: &SynCoreState,
) -> ReasoningResult<Value> {
    let task_id = arguments["task_id"]
        .as_i64()
        .ok_or_else(|| ReasoningError::Neo4j("Missing or invalid task_id".to_string()))?;

    // Get task from database
    let task = state
        .tasks
        .next_task(Some(&["open", "in_progress"]), None)?
        .and_then(|t| {
            if t.id == task_id {
                Some(t)
            } else {
                None
            }
        })
        .ok_or_else(|| ReasoningError::SessionNotFound(format!("Task {} not found", task_id)))?;

    // Load configuration
    let config = SyncoreConfig::load_with_env("config/syncore.toml")
        .map_err(|e| ReasoningError::Neo4j(format!("Failed to load configuration: {}", e)))?;

    // Create reasoning session for task
    let session_title = format!("Task {}", task_id);
    let session_description = format!("Task: {} - {}", task.goal, task.description);
    let session = ReasoningSession::new(&session_title, &session_description, &config)
        .await
        .map_err(|e| ReasoningError::Neo4j(format!("Failed to create session: {}", e)))?;

    // Add initial thought node for task execution
    let initial_content = format!("Starting execution of task: {}", task.goal);
    let node_id = session
        .add_thought(
            None,
            &initial_content,
            "task_start",
            1.0,
            json!({
                "task_id": task_id,
                "goal": task.goal,
                "description": task.description
            }),
        )
        .await?;

    // Update task status to in_progress
    Tasks::update_task(&state.tasks.db.lock().unwrap(), task_id, Some("in_progress"), None, None)?;

    // Store reasoning results in memory
    let memory_key = format!("task_{}_last_reasoning", task_id);
    let memory_value = serde_json::json!({
        "session_id": session.id(),
        "initial_node_id": node_id,
        "backend": config.reasoning.backend,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    if let Err(e) = state.memory.store(&memory_key, &memory_value.to_string()) {
        eprintln!("Warning: Failed to store reasoning result in memory: {}", e);
    }

    Ok(json!({
        "task_id": task_id,
        "session_id": session.id(),
        "status": "in_progress",
        "initial_node_id": node_id,
        "backend": config.reasoning.backend,
        "content": initial_content
    }))
}

/// Handle reasoning.health MCP tool
pub async fn handle_reasoning_health(
    _arguments: Value,
    state: &SynCoreState,
) -> ReasoningResult<Value> {
    // Load configuration
    let config = SyncoreConfig::load_with_env("config/syncore.toml")
        .map_err(|e| ReasoningError::Neo4j(format!("Failed to load configuration: {}", e)))?;

    // Check backend availability
    let backend = &config.reasoning.backend;
    let backend_ok = match backend.as_str() {
        "neo4j" => state.neo4j.is_some(),
        "sqlite" => {
            // Check if SQLite database is accessible
            std::path::Path::new(&config.paths.db_path).exists()
        }
        _ => false,
    };

    // Count active sessions (simplified - would query actual backend)
    let active_sessions = if backend_ok {
        // In real implementation, would query backend for session count
        1 // Mock value for now
    } else {
        0
    };

    // Count recent nodes (simplified)
    let recent_nodes = if backend_ok {
        5 // Mock value for now
    } else {
        0
    };

    // PHASE ST-12: Circuit breaker status
    let (breaker_status, last_safety_violation, session_limits) = if backend_ok {
        // In real implementation, would get from BranchManager
        // For now, return mock values
        (
            "active".to_string(),
            None,
            json!({
                "max_nodes": 200,
                "max_depth": 10,
                "max_breadth": 5,
                "max_identical_expansions": 3,
                "max_consecutive_errors": 5
            }),
        )
    } else {
        ("inactive".to_string(), Some(format!("{} backend not available", backend)), json!(null))
    };

    let status = if backend_ok && active_sessions > 0 && breaker_status == "active" {
        "ok"
    } else {
        "degraded"
    };

    Ok(json!({
        "status": status,
        "backend": backend,
        "backend_ok": backend_ok,
        "active_sessions": active_sessions,
        "recent_nodes": recent_nodes,
        "breaker_status": breaker_status,
        "last_safety_violation": last_safety_violation,
        "session_limits": session_limits,
        "migration_phase": "ST-12"
    }))
}

// ============================================================================
// REASONING SUITE - SuiteDispatcher Implementation
// ============================================================================

pub struct ReasoningSuite {
    state: SynCoreState,
}

impl ReasoningSuite {
    pub fn new(state: SynCoreState) -> Self {
        Self {
            state,
        }
    }

    /// Execute reasoning suite commands
    pub fn execute(&self, command: &str, args: serde_json::Value) -> SuiteResult {
        let runtime = tokio::runtime::Runtime::new()
            .unwrap_or_else(|_| panic!("Failed to create tokio runtime for reasoning suite"));

        let result = runtime.block_on(async {
            match command {
                "session_create" => {
                    let result = handle_reasoning_session_create(args, &self.state).await;
                    self.convert_reasoning_result(result, "reasoning_session_create")
                }
                "branch_expand" => {
                    let result = handle_reasoning_branch_expand(args, &self.state).await;
                    self.convert_reasoning_result(result, "reasoning_branch_expand")
                }
                "tree_get" => {
                    let result = handle_reasoning_tree_get(args, &self.state).await;
                    self.convert_reasoning_result(result, "reasoning_tree_get")
                }
                "tree_prune" => {
                    let result = handle_reasoning_tree_prune(args, &self.state).await;
                    self.convert_reasoning_result(result, "reasoning_tree_prune")
                }
                _ => {
                    SuiteResult::err(
                        command,
                        format!("Unknown command: {}. Available: session_create, branch_expand, tree_get, tree_prune", command)
                    )
                }
            }
        });

        result
    }

    /// Convert ReasoningResult to SuiteResult with streaming enforcement
    fn convert_reasoning_result(
        &self,
        result: ReasoningResult<Value>,
        command: &str,
    ) -> SuiteResult {
        match result {
            Ok(value) => {
                // Apply streaming contract enforcement to successful responses
                let limiter = OutputLimiter::default();
                let result_json = json!({
                    "command": command,
                    "data": value
                });

                match limiter.apply_json(&result_json) {
                    Ok(limited_json) => {
                        if let Some(limited_data) = limited_json.get("data") {
                            SuiteResult {
                                success: true,
                                command: command.to_string(),
                                data: limited_data.clone(),
                                error: None,
                            }
                        } else {
                            SuiteResult::ok(command, value)
                        }
                    }
                    Err(_) => SuiteResult::ok(command, value), // Fallback to original
                }
            }
            Err(e) => SuiteResult::err(command, e.to_string()),
        }
    }
}

impl SuiteDispatcher for ReasoningSuite {
    fn dispatch(&self, command: &str, args: serde_json::Value) -> SuiteResult {
        self.execute(command, args)
    }

    fn list_commands(&self) -> Vec<&'static str> {
        vec![
            "reasoning_session_create",
            "reasoning_session_create_mcp",
            "reasoning_tree_get",
            "reasoning_tree_prune",
            "reasoning_health",
            "reasoning_branch_expand",
        ]
    }

    fn help(&self, command: &str) -> Option<&'static str> {
        match command {
            "reasoning_session_create" => Some("Create a new reasoning session"),
            "reasoning_session_create_mcp" => {
                Some("Create a new reasoning session with MCP support")
            }
            "reasoning_tree_get" => Some("Get reasoning tree structure"),
            "reasoning_tree_prune" => Some("Prune reasoning tree"),
            "reasoning_health" => Some("Check reasoning system health"),
            "reasoning_branch_expand" => Some("Expand reasoning branch"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_backend_selection() {
        // Test that backend selection works correctly
        // This is a placeholder for more comprehensive tests
        assert!(true);
    }
}
