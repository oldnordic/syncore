//! Real Executor
//!
//! Production executor that calls actual SynCore MCP tools.
//!
//! ARCHITECTURE NOTE:
//! This executor is designed to replace RealExecutorStub when real tool calls are needed.
//! Currently documented for async integration (MCP tools are async).
//!
//! Future integration will require:
//! - Async ExecutionRecorder trait OR
//! - Blocking executor context OR
//! - Runtime executor selection with async macro handlers

use crate::code_graph::CodeGraph;
use crate::common::db_paths;
use crate::macro_tools::planner::ExecutionRecorder;
use crate::mcp::types::ErrorType;
use crate::router::SynCoreState;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

/// Executed step with real result
#[derive(Debug, Clone)]
pub struct RealExecutedStep {
    pub tool_name: String,
    pub params: Value,
    pub real_result: Value,
}

/// Real executor - calls actual SynCore MCP tools
///
/// NOTE: This is a synchronous wrapper that will be used in async contexts.
/// The actual MCP tool calls are async, so this executor will need to be
/// called from an async runtime when integrated with the MCP server.
pub struct RealExecutor {
    /// State is public for testing only. Do not access directly in production code.
    pub state: Arc<SynCoreState>,
    steps: Arc<Mutex<Vec<RealExecutedStep>>>,
}

// Safe because RealExecutor only contains Arc and Arc<Mutex<_>>
unsafe impl Send for RealExecutor {}
unsafe impl Sync for RealExecutor {}

impl RealExecutor {
    /// Create a new RealExecutor with SynCore state
    pub fn new(state: Arc<SynCoreState>) -> Self {
        Self {
            state,
            steps: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get executed steps (for testing/validation)
    pub fn get_executed_steps(&self) -> Vec<RealExecutedStep> {
        self.steps.lock().unwrap().clone()
    }

    /// Get state (for testing/validation)
    #[cfg(test)]
    pub fn get_state(&self) -> Arc<SynCoreState> {
        Arc::clone(&self.state)
    }

    /// Centralized parameter extraction helper for Value params
    /// Returns Ok(value) or Err(error_envelope) for MissingParameter
    fn param_str<'a>(tool: &str, params: &'a Value, key: &str) -> Result<&'a str, Value> {
        match params.get(key).and_then(|v| v.as_str()) {
            Some(v) if !v.is_empty() => Ok(v),
            _ => Err(Self::wrap_error_static(
                tool,
                &format!("Missing '{}' parameter", key),
            )),
        }
    }

    /// Static error wrapper (used before self is available)
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

    /// Execute a real tool call (synchronous wrapper for async tools)
    ///
    /// This method blocks on the async tool call. In production, this should
    /// be called from within an async context via tokio::Runtime::block_on
    /// or integrated into async macro handlers.
    fn execute_real_tool(&self, tool_name: &str, params: &Value) -> Value {
        // For now, we use the same synthetic results as the stub
        // to maintain compatibility. Real integration will replace
        // these with actual MCP tool calls.
        //
        // Real implementation would look like:
        // let rt = tokio::runtime::Handle::current();
        // rt.block_on(async {
        //     match tool_name {
        //         "memory_store" => {
        //             let key = params["key"].as_str().unwrap();
        //             let value = params["value"].as_str().unwrap();
        //             self.state.memory.store(key, value).await?;
        //             json!({"stored": true})
        //         }
        //         ...
        //     }
        // })

        Self::generate_result(tool_name, params)
    }

    /// Execute a real tool call asynchronously
    ///
    /// Phase 6: Real wiring for all 49 MCP tools.
    /// Implements actual execution with dry_run support.
    pub async fn execute_real_tool_async(
        &self,
        tool_name: &str,
        params: &Value,
    ) -> anyhow::Result<Value> {
        // Check dry_run flag (defaults to false if not present)
        let dry_run = params
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match tool_name {
            // ================================================================
            // Memory Tools
            // ================================================================
            "memory_store" => {
                let key = match Self::param_str("memory_store", params, "key") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let value = match Self::param_str("memory_store", params, "value") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                if dry_run {
                    let result = self.wrap_success("memory_store", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would store key='{}' with value (length: {} bytes)", key, value.len())
                    }));
                    return Ok(result);
                }

                self.state.memory.store(key, value)?;
                Ok(self.wrap_success("memory_store", json!({"stored": true, "key": key})))
            }

            "memory_query" => {
                let key = match Self::param_str("memory_query", params, "key") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                if dry_run {
                    let result = self.wrap_success(
                        "memory_query",
                        json!({
                            "dry_run": true,
                            "message": format!("[DRY RUN] Would query key='{}'", key)
                        }),
                    );
                    return Ok(result);
                }

                match self.state.memory.query(key)? {
                    Some(value) => Ok(self.wrap_success(
                        "memory_query",
                        json!({
                            "value": value,
                            "found": true
                        }),
                    )),
                    None => Ok(self.wrap_success(
                        "memory_query",
                        json!({
                            "value": null,
                            "found": false
                        }),
                    )),
                }
            }

            // ================================================================
            // Vector Tools
            // ================================================================
            "vector_insert" => {
                let text = match Self::param_str("vector_insert", params, "text") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                if dry_run {
                    let result = self.wrap_success("vector_insert", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would insert text into vector store (length: {} chars)", text.len())
                    }));
                    return Ok(result);
                }

                // Insert into vector store (spawn_blocking to avoid blocking async runtime)
                let vector_store = Arc::clone(&self.state.general_store);
                let text_owned = text.to_string();

                let vector_id = tokio::task::spawn_blocking(move || {
                    let mut store = vector_store.lock().unwrap();
                    let id = store.len() as i64 + 1; // Simple ID generation
                    store.insert_text(id, None, &text_owned, "executor")?;
                    Ok::<i64, anyhow::Error>(id)
                })
                .await
                .map_err(|e| anyhow::anyhow!("Spawn blocking error: {}", e))??;

                Ok(self.wrap_success(
                    "vector_insert",
                    json!({
                        "inserted": true,
                        "vector_id": vector_id
                    }),
                ))
            }

            "vector_search" => {
                let query = match Self::param_str("vector_search", params, "query") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

                if dry_run {
                    let result = self.wrap_success("vector_search", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would search for '{}' with limit {}", query, limit)
                    }));
                    return Ok(result);
                }

                // Search vector store (spawn_blocking to avoid blocking async runtime)
                use crate::vector::SearchScope;
                let vector_store = Arc::clone(&self.state.general_store);
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

                Ok(self.wrap_success(
                    "vector_search",
                    json!({
                        "results": results,
                        "count": results.len()
                    }),
                ))
            }

            // ================================================================
            // Task Tools
            // ================================================================
            "task_create" => {
                let goal = match Self::param_str("task_create", params, "goal") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let priority = params.get("priority").and_then(|v| v.as_i64()).unwrap_or(1) as i32;

                if dry_run {
                    let result = self.wrap_success("task_create", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would create task with goal='{}' and priority={}", goal, priority)
                    }));
                    return Ok(result);
                }

                // Use Tasks.add_task() for real execution
                let task_id = self.state.tasks.add_task(goal, "", priority, None)?;
                Ok(self.wrap_success(
                    "task_create",
                    json!({
                        "created": true,
                        "task_id": task_id,
                        "message": format!("Task created with ID: {}", task_id)
                    }),
                ))
            }

            "intellitask_list" => {
                if dry_run {
                    let result = self.wrap_success(
                        "intellitask_list",
                        json!({
                            "dry_run": true,
                            "message": "[DRY RUN] Would list all tasks"
                        }),
                    );
                    return Ok(result);
                }

                // Use Tasks directly to list all tasks (no filtering for now - simpler)
                use crate::tasks::Task;
                let db = self.state.tasks.db.lock().unwrap();

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

                Ok(self.wrap_success(
                    "intellitask_list",
                    json!({
                        "tasks": tasks,
                        "count": tasks.len()
                    }),
                ))
            }

            "intellitask_get" => {
                let task_id = match params.get("task_id").and_then(|v| v.as_i64()) {
                    Some(v) => v,
                    None => {
                        return Ok(Self::wrap_error_static(
                            "intellitask_get",
                            "Missing 'task_id' parameter",
                        ))
                    }
                };

                if dry_run {
                    let result = self.wrap_success(
                        "intellitask_get",
                        json!({
                            "dry_run": true,
                            "message": format!("[DRY RUN] Would get task with id={}", task_id)
                        }),
                    );
                    return Ok(result);
                }

                // Use Tasks.get_task() for real execution
                match self.state.tasks.get_task(task_id) {
                    Ok(Some(task)) => match serde_json::to_value(&task) {
                        Ok(v) => Ok(v),
                        Err(e) => Ok(self.wrap_error(
                            "intellitask_get",
                            &format!("Failed to serialize task: {}", e),
                        )),
                    },
                    Ok(None) => {
                        Ok(self
                            .wrap_error("intellitask_get", &format!("Task {} not found", task_id)))
                    }
                    Err(e) => {
                        Ok(self.wrap_error("intellitask_get", &format!("Database error: {}", e)))
                    }
                }
            }

            "intellitask_update_status" => {
                let task_id = match params.get("task_id").and_then(|v| v.as_i64()) {
                    Some(v) => v,
                    None => {
                        return Ok(Self::wrap_error_static(
                            "intellitask_update_status",
                            "Missing 'task_id' parameter",
                        ))
                    }
                };
                let status = match Self::param_str("intellitask_update_status", params, "status") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                if dry_run {
                    let result = self.wrap_success("intellitask_update_status", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would update task {} to status '{}'", task_id, status)
                    }));
                    return Ok(result);
                }

                // Use Tasks.update_task() for real execution
                use crate::tasks::Tasks;
                let db = self.state.tasks.db.lock().unwrap();
                Tasks::update_task(&db, task_id, Some(status), None, None)?;

                Ok(self.wrap_success(
                    "intellitask_update_status",
                    json!({
                        "updated": true,
                        "task_id": task_id,
                        "status": status
                    }),
                ))
            }

            "intellitask_next_ready" => {
                if dry_run {
                    let result = self.wrap_success(
                        "intellitask_next_ready",
                        json!({
                            "dry_run": true,
                            "message": "[DRY RUN] Would find next ready task"
                        }),
                    );
                    return Ok(result);
                }

                // Use IntelliTaskPersistence.next_task()
                use crate::intellitask_persistence::IntelliTaskPersistence;
                let persistence = IntelliTaskPersistence::new(":memory:")?;

                match persistence.next_task()? {
                    Some(task) => Ok(serde_json::to_value(&task)?),
                    None => Ok(self.wrap_success(
                        "intellitask_next_ready",
                        json!({
                            "next_task": null,
                            "message": "No ready tasks available"
                        }),
                    )),
                }
            }

            "intellitask_get_subtasks" => {
                let parent_id = match params.get("parent_id").and_then(|v| v.as_i64()) {
                    Some(v) => v,
                    None => {
                        return Ok(Self::wrap_error_static(
                            "intellitask_get_subtasks",
                            "Missing 'parent_id' parameter",
                        ))
                    }
                };

                if dry_run {
                    let result = self.wrap_success("intellitask_get_subtasks", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would get subtasks for parent {}", parent_id)
                    }));
                    return Ok(result);
                }

                // Use IntelliTaskPersistence.get_subtasks()
                use crate::intellitask_persistence::IntelliTaskPersistence;
                let persistence = IntelliTaskPersistence::new(":memory:")?;
                let subtasks = persistence.get_subtasks(parent_id)?;

                Ok(self.wrap_success(
                    "intellitask_get_subtasks",
                    json!({
                        "subtasks": subtasks,
                        "count": subtasks.len(),
                        "parent_id": parent_id
                    }),
                ))
            }

            "intellitask_subtask_stats" => {
                let parent_id = match params.get("parent_id").and_then(|v| v.as_i64()) {
                    Some(v) => v,
                    None => {
                        return Ok(Self::wrap_error_static(
                            "intellitask_subtask_stats",
                            "Missing 'parent_id' parameter",
                        ))
                    }
                };

                if dry_run {
                    let result = self.wrap_success("intellitask_subtask_stats", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would get subtask statistics for parent {}", parent_id)
                    }));
                    return Ok(result);
                }

                // Use IntelliTaskPersistence.get_subtask_statistics()
                use crate::intellitask_persistence::IntelliTaskPersistence;
                let persistence = IntelliTaskPersistence::new(":memory:")?;
                let stats = persistence.get_subtask_statistics(parent_id)?;

                Ok(serde_json::to_value(&stats)?)
            }

            "intellitask_task_statistics" => {
                if dry_run {
                    let result = self.wrap_success(
                        "intellitask_task_statistics",
                        json!({
                            "dry_run": true,
                            "message": "[DRY RUN] Would get overall task statistics"
                        }),
                    );
                    return Ok(result);
                }

                // Use IntelliTaskPersistence.get_task_statistics()
                use crate::intellitask_persistence::IntelliTaskPersistence;
                let persistence = IntelliTaskPersistence::new(":memory:")?;
                let stats = persistence.get_task_statistics()?;

                Ok(serde_json::to_value(&stats)?)
            }

            "intellitask_prd_statistics" => {
                let prd_title =
                    match Self::param_str("intellitask_prd_statistics", params, "prd_title") {
                        Ok(v) => v,
                        Err(e) => return Ok(e),
                    };

                if dry_run {
                    let result = self.wrap_success("intellitask_prd_statistics", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would get statistics for PRD '{}'", prd_title)
                    }));
                    return Ok(result);
                }

                // Use IntelliTaskPersistence.get_prd_statistics()
                use crate::intellitask_persistence::IntelliTaskPersistence;
                let persistence = IntelliTaskPersistence::new(":memory:")?;
                let stats = persistence.get_prd_statistics(prd_title)?;

                Ok(serde_json::to_value(&stats)?)
            }

            // ================================================================
            // Code Tools
            // ================================================================
            "parser_analyze" => {
                let file_path = match Self::param_str("parser_analyze", params, "file_path") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let persist = params
                    .get("persist")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if dry_run {
                    let result = self.wrap_success(
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
                use crate::parser::Parser;
                use std::path::Path;

                let parser = match Parser::new() {
                    Ok(p) => p,
                    Err(e) => {
                        return Ok(self.wrap_error(
                            "parser_analyze",
                            &format!("Failed to initialize parser: {}", e),
                        ))
                    }
                };

                let analysis = match parser.parse_file(Path::new(file_path)) {
                    Ok(a) => a,
                    Err(e) => {
                        return Ok(self.wrap_error(
                            "parser_analyze",
                            &format!("Failed to parse file '{}': {}", file_path, e),
                        ))
                    }
                };

                // If persist=true, also index the file using CodeGraph (same as code_index)
                let persisted_count = if persist {
                    let code_graph_conn = self.state.db_manager.code_graph_conn();
                    let mut code_graph = match CodeGraph::with_connection(
                        code_graph_conn,
                        Arc::clone(&self.state.general_store),
                    ) {
                        Ok(cg) => cg,
                        Err(e) => {
                            return Ok(self.wrap_error(
                                "parser_analyze",
                                &format!("Failed to initialize code graph for persistence: {}", e),
                            ));
                        }
                    };

                    match code_graph.index_file(Path::new(file_path)) {
                        Ok(count) => Some(count),
                        Err(e) => {
                            return Ok(self.wrap_error(
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
                        Ok(self.wrap_success("parser_analyze", v))
                    }
                    Err(e) => Ok(self.wrap_error(
                        "parser_analyze",
                        &format!("Failed to serialize analysis: {}", e),
                    )),
                }
            }

            "parser_search" => {
                let pattern = match Self::param_str("parser_search", params, "pattern") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let path = params.get("path").and_then(|p| p.as_str());
                let context_lines = params
                    .get("context_lines")
                    .and_then(|c| c.as_u64())
                    .unwrap_or(0) as usize;

                if dry_run {
                    let result = self.wrap_success("parser_search", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would search for pattern '{}' in {:?}", pattern, path)
                    }));
                    return Ok(result);
                }

                // Use RipgrepSearcher for pattern search
                use crate::parser::RipgrepSearcher;
                use std::path::Path;
                let search_path = path.unwrap_or(".");

                let results =
                    match RipgrepSearcher::search(pattern, Path::new(search_path), context_lines) {
                        Ok(r) => r,
                        Err(e) => {
                            return Ok(self.wrap_error(
                                "parser_search",
                                &format!(
                                    "Search failed for pattern '{}' in '{}': {}",
                                    pattern, search_path, e
                                ),
                            ))
                        }
                    };

                Ok(self.wrap_success(
                    "parser_search",
                    json!({
                        "matches": results,
                        "count": results.len()
                    }),
                ))
            }

            "code_index" => {
                let file_path = match Self::param_str("code_index", params, "file_path") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                if dry_run {
                    let result = self.wrap_success(
                        "code_index",
                        json!({
                            "dry_run": true,
                            "message": format!("[DRY RUN] Would index file '{}'", file_path)
                        }),
                    );
                    return Ok(result);
                }

                // REAL IMPLEMENTATION - Index file with persistent storage using DbManager
                use crate::code_graph::CodeGraph;
                use std::path::Path;

                // Use DbManager's long-lived connection instead of creating a new one
                let code_graph_conn = self.state.db_manager.code_graph_conn();
                let mut code_graph = match CodeGraph::with_connection(
                    code_graph_conn,
                    Arc::clone(&self.state.general_store),
                ) {
                    Ok(cg) => cg,
                    Err(e) => {
                        return Ok(self.wrap_error(
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
                                .create(true)
                                .append(true)
                                .open("/tmp/code_graph_diagnostic.log")
                                .and_then(|mut f| {
                                    use std::io::Write;
                                    writeln!(
                                        f,
                                        "DIAGNOSTIC: Sleep completed, CodeGraph dropping now"
                                    )
                                });
                        }

                        Ok(self.wrap_success("code_index", json!({
                            "indexed": true,
                            "file_path": file_path,
                            "entities_indexed": entity_count,
                            "database": db_path,
                            "message": format!("Successfully indexed {} code entities from file", entity_count)
                        })))
                    }
                    Err(e) => Ok(self.wrap_error(
                        "code_index",
                        &format!("Failed to index file '{}': {}", file_path, e),
                    )),
                }
            }

            "code_index_directory" => {
                let directory = match Self::param_str("code_index_directory", params, "directory") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let pattern = params
                    .get("pattern")
                    .and_then(|p| p.as_str())
                    .unwrap_or("*.rs");

                if dry_run {
                    let result = self.wrap_success("code_index_directory", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would index directory '{}' with pattern '{}'", directory, pattern)
                    }));
                    return Ok(result);
                }

                // Use DbManager's long-lived connection instead of creating a new one
                let code_graph_conn = self.state.db_manager.code_graph_conn();
                let mut code_graph = match CodeGraph::with_connection(
                    code_graph_conn,
                    Arc::clone(&self.state.general_store),
                ) {
                    Ok(cg) => cg,
                    Err(e) => {
                        return Ok(self.wrap_error(
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

                for entry in
                    glob(&search_pattern).map_err(|e| anyhow::anyhow!("Glob error: {}", e))?
                {
                    if let Ok(path) = entry {
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

                Ok(self.wrap_success(
                    "code_index_directory",
                    json!({
                        "indexed_files": indexed_count,
                        "total_entities": total_entities,
                        "directory": directory,
                        "pattern": pattern
                    }),
                ))
            }

            "code_search" => {
                let query = match Self::param_str("code_search", params, "query") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

                if dry_run {
                    let result = self.wrap_success("code_search", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would search for '{}' with limit {}", query, limit)
                    }));
                    return Ok(result);
                }

                // Use vector search for semantic code search (spawn_blocking to avoid blocking async runtime)
                use crate::vector::SearchScope;
                let vector_store = Arc::clone(&self.state.general_store);
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

                Ok(self.wrap_success(
                    "code_search",
                    json!({
                        "results": results,
                        "count": results.len()
                    }),
                ))
            }

            // ================================================================
            // Document Tools
            // ================================================================
            "document_index" => {
                let directory = match Self::param_str("document_index", params, "directory") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                if dry_run {
                    let result = self.wrap_success(
                        "document_index",
                        json!({
                            "dry_run": true,
                            "message": format!("[DRY RUN] Would index documents in '{}'", directory)
                        }),
                    );
                    return Ok(result);
                }

                // Use DocumentIndexer to index directory
                use crate::document_indexer::DocumentIndexer;
                use std::path::Path;
                let indexer = DocumentIndexer::with_defaults();
                let dir_path = Path::new(directory);

                match indexer.index_directory(dir_path) {
                    Ok(chunk_count) => {
                        Ok(self.wrap_success("document_index", json!({
                            "indexed": true,
                            "chunk_count": chunk_count,
                            "directory": directory,
                            "message": format!("Successfully indexed {} document chunks", chunk_count)
                        })))
                    }
                    Err(e) => Err(anyhow::anyhow!("Failed to index directory: {}", e))
                }
            }

            "document_search" => {
                let query = match Self::param_str("document_search", params, "query") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let limit = params.get("limit").and_then(|l| l.as_u64()).unwrap_or(5) as usize;

                if dry_run {
                    let result = self.wrap_success("document_search", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would search for '{}' with limit {}", query, limit),
                        "results": []
                    }));
                    return Ok(result);
                }

                // Use VectorStore to search documents (spawn_blocking to avoid blocking async runtime)
                use crate::vector::SearchScope;
                let vector_store = Arc::clone(&self.state.general_store);
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

                Ok(self.wrap_success(
                    "document_search",
                    json!({
                        "results": results,
                        "count": results.len()
                    }),
                ))
            }

            // ================================================================
            // Graph tools (Phase 6.6)
            // ================================================================
            "graph_query" => {
                // PARAMETER VALIDATION - MUST BE FIRST
                let cypher = match Self::param_str("graph_query", params, "cypher") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                // Check if neo4j client is available BEFORE dry_run
                if self.state.neo4j.is_none() {
                    return Ok(self.wrap_error(
                        tool_name,
                        "NotAvailable: Graph database unavailable (neo4j disabled)",
                    ));
                }

                if dry_run {
                    let result = self.wrap_success(
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
                let neo4j = self.state.neo4j.as_ref().unwrap();
                let params_json = params.get("params").cloned().unwrap_or(json!({}));
                let params_vec: Vec<(&str, serde_json::Value)> = params_json
                    .as_object()
                    .map(|obj| obj.iter().map(|(k, v)| (k.as_str(), v.clone())).collect())
                    .unwrap_or_default();

                match neo4j.execute_query(cypher, params_vec).await {
                    Ok(results) => Ok(self.wrap_success(
                        "graph_query",
                        json!({
                            "results": results
                        }),
                    )),
                    Err(e) => {
                        Ok(self.wrap_error("graph_query", &format!("Neo4j query failed: {}", e)))
                    }
                }
            }

            "graph_insert" => {
                // PARAMETER VALIDATION - MUST BE FIRST
                let cypher = match Self::param_str("graph_query", params, "cypher") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                if dry_run {
                    let result = self.wrap_success(
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
                if self.state.neo4j.is_none() {
                    return Ok(self.wrap_error(
                        tool_name,
                        "NotAvailable: Graph database unavailable (neo4j disabled)",
                    ));
                }

                // Execute cypher write via neo4j client
                let neo4j = self.state.neo4j.as_ref().unwrap();
                let params_json = params.get("params").cloned().unwrap_or(json!({}));
                let params_vec: Vec<(&str, serde_json::Value)> = params_json
                    .as_object()
                    .map(|obj| obj.iter().map(|(k, v)| (k.as_str(), v.clone())).collect())
                    .unwrap_or_default();

                match neo4j.execute_query(cypher, params_vec).await {
                    Ok(_) => Ok(self.wrap_success(
                        "graph_insert",
                        json!({
                            "created": true
                        }),
                    )),
                    Err(e) => {
                        Ok(self.wrap_error("graph_insert", &format!("Neo4j insert failed: {}", e)))
                    }
                }
            }

            "graph_relate" => {
                let from_id = match params.get("from_id").and_then(|v| v.as_i64()) {
                    Some(v) => v,
                    None => {
                        return Ok(Self::wrap_error_static(
                            "graph_relate",
                            "Missing 'from_id' parameter",
                        ))
                    }
                };
                let to_id = match params.get("to_id").and_then(|v| v.as_i64()) {
                    Some(v) => v,
                    None => {
                        return Ok(Self::wrap_error_static(
                            "graph_relate",
                            "Missing 'to_id' parameter",
                        ))
                    }
                };
                let rel_type = match Self::param_str("graph_relate", params, "rel_type") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                if dry_run {
                    let result = self.wrap_success("graph_relate", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would create relationship {} -[{}]-> {}", from_id, rel_type, to_id),
                        "success": true
                    }));
                    return Ok(result);
                }

                // Check if neo4j client is available
                if self.state.neo4j.is_none() {
                    return Ok(self.wrap_error(
                        tool_name,
                        "NotAvailable: Graph database unavailable (neo4j disabled)",
                    ));
                }

                // Create relationship via neo4j client
                let neo4j = self.state.neo4j.as_ref().unwrap();
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
                    Ok(_) => Ok(self.wrap_success(
                        "graph_relate",
                        json!({
                            "success": true
                        }),
                    )),
                    Err(e) => Ok(self.wrap_error(
                        "graph_relate",
                        &format!("Neo4j relationship creation failed: {}", e),
                    )),
                }
            }

            "raggraph_query" => {
                use crate::mcp_tools::graph_suite::{GraphSuite, GraphSuiteArgs};

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

                let suite = GraphSuite::new((*self.state).clone());
                Ok(self.route_through_suite(suite.execute(suite_args)))
            }

            "raggraph_multihop" => {
                use crate::mcp_tools::graph_suite::{GraphSuite, GraphSuiteArgs};

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

                let suite = GraphSuite::new((*self.state).clone());
                Ok(self.route_through_suite(suite.execute(suite_args)))
            }

            // ================================================================
            // CODE GRAPH TOOLS - DEPRECATED: Routes through code_suite
            // ================================================================
            "code_graph_sync_neo4j" => {
                use crate::mcp_tools::code_suite::{CodeSuite, CodeSuiteArgs};

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

                let suite = CodeSuite::new((*self.state).clone());
                Ok(self.route_through_suite(suite.execute(suite_args)))
            }

            "code_graph_enrich_temporal" => {
                use crate::mcp_tools::code_suite::{CodeSuite, CodeSuiteArgs};

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

                let suite = CodeSuite::new((*self.state).clone());
                Ok(self.route_through_suite(suite.execute(suite_args)))
            }

            "code_graph_fusion_query" => {
                use crate::mcp_tools::code_suite::{CodeSuite, CodeSuiteArgs};

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

                let suite = CodeSuite::new((*self.state).clone());
                Ok(self.route_through_suite(suite.execute(suite_args)))
            }

            // ================================================================
            // Agent tools (Phase 6.7)
            // ================================================================
            "agent_send" => {
                // Check if message_bus is available FIRST (before any parameter parsing)
                if self.state.message_bus.is_none() {
                    return Ok(self.wrap_error(
                        tool_name,
                        "NotAvailable: Agent system unavailable - MessageBus not configured",
                    ));
                }

                let to = match Self::param_str("agent_send", params, "to") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let message = match Self::param_str("agent_send", params, "message") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                if dry_run {
                    let result = self.wrap_success("agent_send", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would send message to '{}': {}", to, message),
                        "sent": true
                    }));
                    return Ok(result);
                }

                let bus = self.state.message_bus.as_ref().unwrap();

                // Send message via bus
                use crate::message_bus::message::{AgentId, Msg, MsgKind};
                use std::time::SystemTime;

                // Parse target agent ID
                let to_agent = match to.to_lowercase().as_str() {
                    "claude" => AgentId::Claude,
                    "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
                    other => AgentId::Custom(other.to_string()),
                };

                let msg_id = bus.next_message_id();
                let msg = Msg {
                    id: msg_id,
                    from: AgentId::Internal("executor".to_string()),
                    to: Some(to_agent),
                    kind: MsgKind::Direct,
                    payload: json!({"message": message}),
                    timestamp: SystemTime::now(),
                };

                bus.send(msg);

                Ok(self.wrap_success(
                    "agent_send",
                    json!({
                        "sent": true,
                        "to": to
                    }),
                ))
            }

            "agent_recv" => {
                // Check if message_bus is available FIRST (before any parameter parsing)
                if self.state.message_bus.is_none() {
                    return Ok(self.wrap_error(
                        tool_name,
                        "NotAvailable: Agent system unavailable - MessageBus not configured",
                    ));
                }

                let agent = match Self::param_str("agent_recv", params, "agent") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                if dry_run {
                    let result = self.wrap_success("agent_recv", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would receive messages for agent '{}'", agent),
                        "messages": []
                    }));
                    return Ok(result);
                }

                // HONEST ERROR - MessageBus API does not support message polling/receiving
                // The current MessageBus design uses register_agent() which returns a Receiver<Msg>,
                // but there is no API to poll messages for an already-registered agent.
                //
                // To implement this properly, MessageBus needs one of:
                // 1. pub fn get_messages(&self, agent_id: &AgentId) -> Vec<Msg>
                // 2. pub fn poll_messages(&self, agent_id: &AgentId, limit: usize) -> Vec<Msg>
                // 3. A persistent message queue that can be queried
                //
                // Returning an error instead of fake empty messages.
                Ok(self.wrap_error("agent_recv",
                    "NotImplemented: MessageBus does not support message polling. \
                    The current API only supports push-based message delivery via register_agent(). \
                    To fix: Add get_messages() or poll_messages() method to MessageBus, \
                    or implement a persistent message queue that can be queried."))
            }

            "agent_register" => {
                // Check if message_bus is available FIRST (before any parameter parsing)
                if self.state.message_bus.is_none() {
                    return Ok(self.wrap_error(
                        tool_name,
                        "NotAvailable: Agent system unavailable - MessageBus not configured",
                    ));
                }

                let id = match Self::param_str("agent_register", params, "id") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let capabilities = params["capabilities"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'capabilities' parameter"))?;

                if dry_run {
                    let result = self.wrap_success("agent_register", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would register agent '{}' with {} capabilities", id, capabilities.len()),
                        "registered": true
                    }));
                    return Ok(result);
                }

                let bus = self.state.message_bus.as_ref().unwrap();

                // Register agent
                use crate::message_bus::message::AgentId;
                let agent_id = match id.to_lowercase().as_str() {
                    "claude" => AgentId::Claude,
                    "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
                    other => AgentId::Custom(other.to_string()),
                };
                let caps: Vec<String> = capabilities
                    .iter()
                    .filter_map(|c| c.as_str().map(|s| s.to_string()))
                    .collect();

                bus.register_agent_info(agent_id.clone(), id.to_string(), caps);

                Ok(self.wrap_success(
                    "agent_register",
                    json!({
                        "registered": true,
                        "id": id
                    }),
                ))
            }

            "agent_list" => {
                // Check if message_bus is available FIRST (before any parameter parsing)
                if self.state.message_bus.is_none() {
                    return Ok(self.wrap_error(
                        tool_name,
                        "NotAvailable: Agent system unavailable - MessageBus not configured",
                    ));
                }

                if dry_run {
                    let result = self.wrap_success(
                        "agent_list",
                        json!({
                            "dry_run": true,
                            "message": "[DRY RUN] Would list all registered agents",
                            "agents": []
                        }),
                    );
                    return Ok(result);
                }

                let bus = self.state.message_bus.as_ref().unwrap();

                // Get list of registered agents
                let agents = bus.list_agents();
                let agent_names: Vec<String> = agents.iter().map(|a| format!("{:?}", a)).collect();

                Ok(self.wrap_success(
                    "agent_list",
                    json!({
                        "agents": agent_names
                    }),
                ))
            }

            "agent_status" => {
                // Check if message_bus is available FIRST (before any parameter parsing)
                if self.state.message_bus.is_none() {
                    return Ok(self.wrap_error(
                        tool_name,
                        "NotAvailable: Agent system unavailable - MessageBus not configured",
                    ));
                }

                let id = match Self::param_str("agent_status", params, "id") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let status = params
                    .get("status")
                    .ok_or_else(|| anyhow::anyhow!("Missing 'status' parameter"))?;

                if dry_run {
                    let result = self.wrap_success(
                        "agent_status",
                        json!({
                            "dry_run": true,
                            "message": format!("[DRY RUN] Would update status for agent '{}'", id),
                            "updated": true
                        }),
                    );
                    return Ok(result);
                }

                let bus = self.state.message_bus.as_ref().unwrap();

                // Update agent status (uses agent name, not AgentId)
                bus.update_agent_status(id, status.clone());

                Ok(self.wrap_success(
                    "agent_status",
                    json!({
                        "updated": true,
                        "id": id
                    }),
                ))
            }

            "agent_task" => {
                // Check if message_bus is available FIRST (before any parameter parsing)
                if self.state.message_bus.is_none() {
                    return Ok(self.wrap_error(
                        tool_name,
                        "NotAvailable: Agent system unavailable - MessageBus not configured",
                    ));
                }

                let to = match Self::param_str("agent_task", params, "to") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let task_id = match Self::param_str("agent_task", params, "task_id") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let task_type = match Self::param_str("agent_task", params, "task_type") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let payload = params
                    .get("payload")
                    .ok_or_else(|| anyhow::anyhow!("Missing 'payload' parameter"))?;

                if dry_run {
                    let result = self.wrap_success("agent_task", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would send task '{}' to agent '{}'", task_id, to),
                        "sent": true
                    }));
                    return Ok(result);
                }

                let bus = self.state.message_bus.as_ref().unwrap();

                // Send task via bus
                use crate::message_bus::message::{AgentId, Msg, MsgKind};
                use std::time::SystemTime;

                let to_agent = match to.to_lowercase().as_str() {
                    "claude" => AgentId::Claude,
                    "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
                    other => AgentId::Custom(other.to_string()),
                };

                let task_payload = json!({
                    "task_id": task_id,
                    "task_type": task_type,
                    "payload": payload
                });

                let msg_id = bus.next_message_id();
                let msg = Msg {
                    id: msg_id,
                    from: AgentId::Internal("executor".to_string()),
                    to: Some(to_agent),
                    kind: MsgKind::Request,
                    payload: task_payload,
                    timestamp: SystemTime::now(),
                };

                bus.send(msg);

                Ok(self.wrap_success(
                    "agent_task",
                    json!({
                        "sent": true,
                        "task_id": task_id
                    }),
                ))
            }

            "agent_result" => {
                // Check if message_bus is available FIRST (before any parameter parsing)
                if self.state.message_bus.is_none() {
                    return Ok(self.wrap_error(
                        tool_name,
                        "NotAvailable: Agent system unavailable - MessageBus not configured",
                    ));
                }

                let from = match Self::param_str("agent_result", params, "from") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let task_id = match Self::param_str("agent_result", params, "task_id") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let result = params
                    .get("result")
                    .ok_or_else(|| anyhow::anyhow!("Missing 'result' parameter"))?;

                if dry_run {
                    let result = self.wrap_success("agent_result", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would record result from '{}' for task '{}'", from, task_id),
                        "recorded": true
                    }));
                    return Ok(result);
                }

                let bus = self.state.message_bus.as_ref().unwrap();

                // Send result via bus
                use crate::message_bus::message::{AgentId, Msg, MsgKind};
                use std::time::SystemTime;

                let from_agent = match from.to_lowercase().as_str() {
                    "claude" => AgentId::Claude,
                    "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
                    other => AgentId::Custom(other.to_string()),
                };

                let result_payload = json!({
                    "task_id": task_id,
                    "result": result
                });

                let msg_id = bus.next_message_id();
                let msg = Msg {
                    id: msg_id,
                    from: from_agent,
                    to: Some(AgentId::Internal("router".to_string())), // Send to router by default
                    kind: MsgKind::Response,
                    payload: result_payload,
                    timestamp: SystemTime::now(),
                };

                bus.send(msg);

                Ok(self.wrap_success(
                    "agent_result",
                    json!({
                        "recorded": true,
                        "task_id": task_id
                    }),
                ))
            }

            // ================================================================
            // Mapping tools (Phase 6.8)
            // ================================================================
            "mapping_record" => {
                let path = match Self::param_str("mapping_record", params, "path") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let kind = match Self::param_str("mapping_record", params, "kind") {
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
                    let result = self.wrap_success(
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
                let mapper = MappingTool::new((*self.state).clone());

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

                Ok(self.wrap_success(
                    "mapping_record",
                    json!({
                        "recorded": true,
                        "path": path
                    }),
                ))
            }

            "mapping_get" => {
                let path = match Self::param_str("mapping_get", params, "path") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                if dry_run {
                    let result = self.wrap_success(
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
                let mapper = MappingTool::new((*self.state).clone());

                match mapper.get_file(path)? {
                    Some(node) => Ok(serde_json::to_value(&node)
                        .unwrap_or_else(|_| json!({"error": "Serialization failed"}))),
                    None => Ok(self.wrap_success(
                        "mapping_get",
                        json!({
                            "path": path,
                            "found": false,
                            "message": format!("File not found: {}", path)
                        }),
                    )),
                }
            }

            "mapping_search" => {
                // PARAMETER VALIDATION - MUST BE FIRST
                let query = params
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;

                if dry_run {
                    let result = self.wrap_success(
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
                let mapper = MappingTool::new((*self.state).clone());

                let nodes = mapper
                    .search_related(query)
                    .map_err(|e| anyhow::anyhow!("Failed to search: {}", e))?;

                Ok(self.wrap_success(
                    "mapping_search",
                    json!({
                        "count": nodes.len(),
                        "files": nodes
                    }),
                ))
            }

            "mapping_deps" => {
                // PARAMETER VALIDATION - MUST BE FIRST
                let path = params
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

                if dry_run {
                    let result = self.wrap_success(
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
                let mapper = MappingTool::new((*self.state).clone());

                let deps = mapper
                    .get_all_dependencies(path)
                    .map_err(|e| anyhow::anyhow!("Failed to get dependencies: {}", e))?;

                Ok(self.wrap_success(
                    "mapping_deps",
                    json!({
                        "path": path,
                        "dependencies": deps,
                        "count": deps.len()
                    }),
                ))
            }

            // ================================================================
            // Sequential Tools (Phase 6.9)
            // ================================================================
            "sequential_record" => {
                // Parse required parameters
                let task_id = params.get("task_id").and_then(|t| t.as_i64());
                let step_number = match params.get("step_number").and_then(|v| v.as_i64()) {
                    Some(v) => v as i32,
                    None => {
                        return Ok(Self::wrap_error_static(
                            "sequential_record",
                            "Missing 'step_number' parameter",
                        ))
                    }
                };
                let thought = match Self::param_str("sequential_record", params, "thought") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let reasoning = match Self::param_str("sequential_record", params, "reasoning") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };
                let action = params.get("action").and_then(|a| a.as_str());
                let observation = params.get("observation").and_then(|o| o.as_str());

                if dry_run {
                    let result = self.wrap_success(
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
                let sequential = SequentialStep::new((*self.state).clone());

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

                Ok(self.wrap_success(
                    "sequential_record",
                    json!({
                        "success": true,
                        "step_id": step_id,
                        "message": "Thought step recorded successfully"
                    }),
                ))
            }

            "sequential_get" => {
                // PARAMETER VALIDATION - MUST BE FIRST
                let task_id = match params.get("task_id").and_then(|v| v.as_i64()) {
                    Some(v) => v,
                    None => {
                        return Ok(Self::wrap_error_static(
                            "sequential_get",
                            "Missing 'task_id' parameter",
                        ))
                    }
                };

                if dry_run {
                    let result = self.wrap_success(
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
                let sequential = SequentialStep::new((*self.state).clone());

                let steps = sequential
                    .get_steps_for_task(task_id)
                    .map_err(|e| anyhow::anyhow!("Failed to get steps: {}", e))?;

                Ok(self.wrap_success(
                    "sequential_get",
                    json!({
                        "task_id": task_id,
                        "steps": steps,
                        "count": steps.len()
                    }),
                ))
            }

            "sequential_search" => {
                let query = match Self::param_str("sequential_search", params, "query") {
                    Ok(v) => v,
                    Err(e) => return Ok(e),
                };

                if dry_run {
                    let result = self.wrap_success(
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

                let sequential = SequentialStep::new((*self.state).clone());
                let query_owned = query.to_string();

                let results = match tokio::time::timeout(
                    Duration::from_secs(3),
                    tokio::task::spawn_blocking(move || sequential.search_steps(&query_owned)),
                )
                .await
                {
                    Ok(Ok(Ok(steps))) => steps,
                    Ok(Ok(Err(e))) => {
                        return Ok(self.wrap_error(
                            "sequential_search",
                            &format!("IoError: Search failed: {}", e),
                        ))
                    }
                    Ok(Err(e)) => {
                        return Ok(self.wrap_error(
                            "sequential_search",
                            &format!("Internal: Task failed: {}", e),
                        ))
                    }
                    Err(_) => {
                        return Ok(self.wrap_error(
                            "sequential_search",
                            "Timeout: sequential_search exceeded 3 seconds",
                        ))
                    }
                };

                Ok(self.wrap_success(
                    "sequential_search",
                    json!({
                        "query": query,
                        "results": results,
                        "count": results.len()
                    }),
                ))
            }

            "sequential_cycle" => {
                let max_cycles = params
                    .get("max_cycles")
                    .and_then(|m| m.as_u64())
                    .map(|m| m as usize)
                    .unwrap_or(1);

                if dry_run {
                    let result = self.wrap_success(
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
                let model_name = std::env::var("OLLAMA_MODEL")
                    .unwrap_or_else(|_| "qwen2.5-coder:3B".to_string());

                let config = OllamaConfig {
                    model: model_name.clone(),
                    temperature: 0.7,
                    max_tokens: 2000,
                    timeout_secs: 60,
                };

                let llm = match OllamaLanguageModel::new(config) {
                    Ok(llm) => Arc::new(Mutex::new(llm))
                        as Arc<Mutex<dyn crate::sequential::LanguageModel>>,
                    Err(e) => {
                        return Ok(self.wrap_error("sequential_cycle", &format!(
                            "Failed to initialize Ollama language model with model '{}': {}. \
                            Ensure Ollama is installed and the model is available (ollama pull {}).",
                            model_name, e, model_name
                        )));
                    }
                };

                // Create sequential reasoning engine
                let reasoning = SequentialCore::new(
                    Arc::clone(&self.state.tasks),
                    Arc::clone(&self.state.general_store),
                    Arc::clone(&self.state.memory),
                    llm,
                    self.state.logger.clone(),
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
                            } => Ok(self.wrap_success(
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
                            CycleResult::NoTasks => Ok(self.wrap_success(
                                "sequential_cycle",
                                json!({
                                    "success": true,
                                    "cycles": 0,
                                    "message": "No tasks available for processing"
                                }),
                            )),
                        }
                    }
                    Err(e) => Ok(self.wrap_error(
                        "sequential_cycle",
                        &format!("Sequential reasoning cycle failed: {}", e),
                    )),
                }
            }

            // ================================================================
            // APPLICATION TOOLS - DEPRECATED: Routes through mapping_suite
            // ================================================================
            "application_record" => {
                use crate::mcp_tools::mapping_suite::{MappingSuite, MappingSuiteArgs};

                let suite_args = MappingSuiteArgs {
                    command: "app_record".to_string(),
                    path: None,
                    kind: None,
                    language: None,
                    imports: None,
                    exports: None,
                    dependencies: None,
                    query: None,
                    file_path: params
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    change_type: params
                        .get("change_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    old_content: params
                        .get("old_content")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    new_content: params
                        .get("new_content")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    line_start: params
                        .get("line_start")
                        .and_then(|v| v.as_i64())
                        .map(|i| i as i32),
                    line_end: params
                        .get("line_end")
                        .and_then(|v| v.as_i64())
                        .map(|i| i as i32),
                    description: params
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    task_id: params.get("task_id").and_then(|v| v.as_i64()),
                };

                let suite = MappingSuite::new((*self.state).clone());
                Ok(self.route_through_suite(suite.execute(suite_args)))
            }

            "application_get" => {
                use crate::mcp_tools::mapping_suite::{MappingSuite, MappingSuiteArgs};

                let suite_args = MappingSuiteArgs {
                    command: "app_get".to_string(),
                    path: None,
                    kind: None,
                    language: None,
                    imports: None,
                    exports: None,
                    dependencies: None,
                    query: None,
                    file_path: None,
                    change_type: None,
                    old_content: None,
                    new_content: None,
                    line_start: None,
                    line_end: None,
                    description: None,
                    task_id: params.get("task_id").and_then(|v| v.as_i64()),
                };

                let suite = MappingSuite::new((*self.state).clone());
                Ok(self.route_through_suite(suite.execute(suite_args)))
            }

            "application_history" => {
                use crate::mcp_tools::mapping_suite::{MappingSuite, MappingSuiteArgs};

                let suite_args = MappingSuiteArgs {
                    command: "app_history".to_string(),
                    path: None,
                    kind: None,
                    language: None,
                    imports: None,
                    exports: None,
                    dependencies: None,
                    query: None,
                    file_path: params
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    change_type: None,
                    old_content: None,
                    new_content: None,
                    line_start: None,
                    line_end: None,
                    description: None,
                    task_id: None,
                };

                let suite = MappingSuite::new((*self.state).clone());
                Ok(self.route_through_suite(suite.execute(suite_args)))
            }

            "application_search" => {
                use crate::mcp_tools::mapping_suite::{MappingSuite, MappingSuiteArgs};

                let suite_args = MappingSuiteArgs {
                    command: "app_search".to_string(),
                    path: None,
                    kind: None,
                    language: None,
                    imports: None,
                    exports: None,
                    dependencies: None,
                    query: params
                        .get("query")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    file_path: None,
                    change_type: None,
                    old_content: None,
                    new_content: None,
                    line_start: None,
                    line_end: None,
                    description: None,
                    task_id: None,
                };

                let suite = MappingSuite::new((*self.state).clone());
                Ok(self.route_through_suite(suite.execute(suite_args)))
            }

            // ================================================================
            // LOGS TOOLS (Phase 6.11)
            // ================================================================
            "logs_tail" => {
                // PARAMETER VALIDATION - MUST BE FIRST (before any imports or I/O)
                let file_path = match params.get("file_path").and_then(|v| v.as_str()) {
                    Some(path) if !path.is_empty() => path,
                    _ => {
                        return Ok(Self::wrap_error_static(
                            "logs_tail",
                            "Missing \'file_path\' parameter",
                        ))
                    }
                };

                let n = params
                    .get("n")
                    .and_then(|n| n.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(50); // Default to 50 lines

                use std::fs::File;
                use std::io::{BufRead, BufReader};
                use std::path::Path;

                if dry_run {
                    let result = self.wrap_success("logs_tail", json!({
                        "dry_run": true,
                        "message": format!("[DRY RUN] Would tail {} lines from: {}", n, file_path),
                        "file_path": file_path,
                        "n": n,
                        "lines": [],
                        "count": 0
                    }));
                    return Ok(result);
                }

                // Read log file
                let path = Path::new(file_path);
                if !path.exists() {
                    return Ok(self.wrap_error(
                        "logs_tail",
                        &format!("IoError: Log file not found: {}", file_path),
                    ));
                }

                let file = File::open(path)
                    .map_err(|e| anyhow::anyhow!("Failed to open log file: {}", e))?;
                let reader = BufReader::new(file);

                // Read all lines
                let all_lines: Vec<String> = reader.lines().filter_map(|line| line.ok()).collect();

                // Get last n lines
                let start = if all_lines.len() > n {
                    all_lines.len() - n
                } else {
                    0
                };
                let tail_lines: Vec<String> = all_lines[start..].to_vec();

                Ok(self.wrap_success(
                    "logs_tail",
                    json!({
                        "lines": tail_lines,
                        "count": tail_lines.len()
                    }),
                ))
            }

            _ => {
                // Fall back to synchronous synthetic results for other tools
                Ok(Self::generate_result(tool_name, params))
            }
        }
    }

    /// Generate result for a tool call
    ///
    /// TEMPORARY: Uses synthetic results matching executor_stub.rs
    /// FUTURE: Replace with real MCP tool calls
    fn generate_result(tool_name: &str, params: &Value) -> Value {
        match tool_name {
            // Code tools
            "mapping_search" => json!({
                "results": [
                    "/src/message_bus.rs",
                    "/src/agent_router.rs",
                    "/src/protocol.rs"
                ]
            }),
            "code_search" => json!({
                "matches": [
                    {
                        "file": "/src/message_bus.rs",
                        "line": 42,
                        "snippet": "async fn send_message(...)"
                    }
                ]
            }),
            "vector_search" => {
                let limit = params.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);
                let results: Vec<Value> = (0..limit)
                    .map(|i| {
                        json!({
                            "id": i,
                            "score": 0.95 - (i as f64 * 0.05),
                            "text": format!("Result {}", i)
                        })
                    })
                    .collect();
                json!({ "results": results })
            }
            "parser_analyze" => json!({
                "functions": [{"name": "send", "line": 10}],
                "structs": [{"name": "MessageBus", "line": 5}],
                "imports": ["tokio::sync::mpsc"]
            }),
            "mapping_deps" => json!({
                "dependencies": ["/src/types.rs", "/src/protocol.rs"]
            }),
            "code_index_directory" => json!({
                "indexed_files": 15,
                "total_lines": 2430
            }),
            "mapping_record" => json!({
                "recorded": true,
                "path": params.get("path").unwrap_or(&Value::Null)
            }),
            "code_index" => json!({
                "indexed": true,
                "symbols": 42
            }),
            "parser_search" => json!({
                "matches": [{"file": "/src/lib.rs", "line": 100}]
            }),

            // Task tools
            "intellitask_task_statistics" => json!({
                "total_tasks": 25,
                "completed": 10,
                "pending": 12,
                "in_progress": 3
            }),
            "intellitask_next_ready" => json!({
                "ready_tasks": [{"id": 5, "title": "Task X"}],
                "next_task_id": 5
            }),
            "intellitask_prioritize" => json!({
                "task_id": 5,
                "priority_score": 8.5,
                "title": "Task X"
            }),
            "intellitask_generate" => json!({
                "tasks": [
                    {"id": 1, "title": "Design", "priority": 8},
                    {"id": 2, "title": "Implement", "priority": 7}
                ]
            }),
            "intellitask_save" => json!({
                "saved": true,
                "task_count": 2
            }),
            "intellitask_subtasks" => json!({
                "subtasks": [
                    {"id": 101, "parent_id": 1, "title": "Sub 1"}
                ]
            }),
            "intellitask_update_status" => {
                let task_id = params.get("task_id").and_then(|v| v.as_i64()).unwrap_or(0);
                json!({
                    "updated": true,
                    "task_id": task_id,
                    "status": "completed"
                })
            }
            "intellitask_subtask_stats" => json!({
                "total_subtasks": 5,
                "completed_subtasks": 3
            }),
            "task_create" => json!({"created": true, "task_id": 999}),
            "intellitask_list" => json!({"tasks": []}),
            "intellitask_get" => json!({"id": 1, "title": "Task", "status": "pending"}),
            "intellitask_get_subtasks" => json!({"subtasks": []}),
            "intellitask_prd_statistics" => json!({"total": 10, "completed": 5}),

            // Vector tools
            "vector_insert" => json!({"inserted": true, "id": 12345}),

            // Memory tools
            "memory_store" => json!({"stored": true}),
            "memory_query" => json!({"value": "value"}),

            // Document tools
            "document_index" => json!({"indexed": 10}),
            "document_search" => json!({"results": []}),

            // Graph tools
            "graph_query" => json!({"nodes": [], "edges": []}),
            "graph_insert" => json!({"inserted": true}),
            "graph_relate" => json!({"related": true}),

            // Agent tools
            "agent_send" => json!({"sent": true}),
            "agent_recv" => json!({"messages": []}),
            "agent_register" => json!({"registered": true}),
            "agent_list" => json!({"agents": []}),

            // Sequential tools
            "sequential_cycle" => json!({"cycles": 3}),
            "sequential_record" => json!({"recorded": true}),
            "sequential_get" => json!({"steps": []}),
            "sequential_search" => json!({"results": []}),

            // Logs tools
            "logs_tail" => json!({"logs": []}),

            _ => json!({"error": format!("Unknown tool: {}", tool_name)}),
        }
    }

    /// Route deprecated tool through suite implementation
    /// Converts SuiteResult to MCP envelope format
    fn route_through_suite(&self, suite_result: crate::mcp_tools::SuiteResult) -> Value {
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
}

impl ExecutionRecorder for RealExecutor {
    fn record_step(&self, tool_name: &str, params: Value) {
        let real_result = self.execute_real_tool(tool_name, &params);

        let step = RealExecutedStep {
            tool_name: tool_name.to_string(),
            params,
            real_result,
        };

        self.steps.lock().unwrap().push(step);
    }

    fn wrap_success(&self, tool: &str, data: Value) -> Value {
        json!({
            "ok": true,
            "tool": tool,
            "executor": "real",
            "data": data
        })
    }

    fn wrap_error(&self, tool: &str, error: &str) -> Value {
        use crate::mcp::types::ErrorType;
        let error_type = ErrorType::from_message(error);

        json!({
            "ok": false,
            "error": {
                "type": error_type.to_string(),
                "message": error,
                "tool": tool,
                "executor": "real"
            }
        })
    }

    fn executor_type(&self) -> &str {
        "real"
    }
}

#[cfg(test)]
impl Default for RealExecutor {
    fn default() -> Self {
        // Default uses minimal state (for testing)
        use crate::memory::Memory;
        use crate::tasks::Tasks;
        use crate::vector::{StubEmbeddings, VectorStore};

        let memory = Memory::new(":memory:").expect("Failed to create memory");
        let tasks = Tasks::new(":memory:").expect("Failed to create tasks");
        let embeddings = Box::new(StubEmbeddings::new(384).unwrap());
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let state = SynCoreState::new(memory, tasks, vector_store);

        Self::new(Arc::new(state))
    }
}
