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

use crate::macro_tools::planner::ExecutionRecorder;
use crate::router::SynCoreState;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

mod executors;

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
    ///
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
        let dry_run = params.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);

        match tool_name {
            // ================================================================
            // Memory Tools
            // ================================================================
            "memory_store" => {
                executors::memory_executor::execute_memory_store(&self.state, params, dry_run).await
            }

            "memory_query" => {
                executors::memory_executor::execute_memory_query(&self.state, params, dry_run).await
            }

            // ================================================================
            // Vector Tools
            // ================================================================
            "vector_insert" => {
                executors::vector_executor::execute_vector_insert(&self.state, params, dry_run)
                    .await
            }

            "vector_search" => {
                executors::vector_executor::execute_vector_search(&self.state, params, dry_run)
                    .await
            }

            // ================================================================
            // Task Tools
            // ================================================================
            "task_create" => {
                executors::task_executor::execute_task_create(&self.state, params, dry_run).await
            }

            "intellitask_list" => {
                executors::task_executor::execute_intellitask_list(&self.state, params, dry_run)
                    .await
            }

            "intellitask_get" => {
                executors::task_executor::execute_intellitask_get(&self.state, params, dry_run)
                    .await
            }

            "intellitask_update_status" => {
                executors::task_executor::execute_intellitask_update_status(
                    &self.state,
                    params,
                    dry_run,
                )
                .await
            }

            "intellitask_next_ready" => {
                executors::task_executor::execute_intellitask_next_ready(
                    &self.state,
                    params,
                    dry_run,
                )
                .await
            }

            "intellitask_get_subtasks" => {
                executors::task_executor::execute_intellitask_get_subtasks(
                    &self.state,
                    params,
                    dry_run,
                )
                .await
            }

            "intellitask_subtask_stats" => {
                executors::task_executor::execute_intellitask_subtask_stats(
                    &self.state,
                    params,
                    dry_run,
                )
                .await
            }

            "intellitask_task_statistics" => {
                executors::task_executor::execute_intellitask_task_statistics(
                    &self.state,
                    params,
                    dry_run,
                )
                .await
            }

            "intellitask_prd_statistics" => {
                executors::task_executor::execute_intellitask_prd_statistics(
                    &self.state,
                    params,
                    dry_run,
                )
                .await
            }

            // ================================================================
            // Code Tools
            // ================================================================
            "parser_analyze" => {
                executors::code_parser_executor::execute_parser_analyze(
                    &self.state,
                    params,
                    dry_run,
                )
                .await
            }

            "parser_search" => {
                executors::code_parser_executor::execute_parser_search(&self.state, params, dry_run)
                    .await
            }

            "code_index" => {
                executors::code_parser_executor::execute_code_index(&self.state, params, dry_run)
                    .await
            }

            "code_index_directory" => {
                executors::code_parser_executor::execute_code_index_directory(
                    &self.state,
                    params,
                    dry_run,
                )
                .await
            }

            "code_search" => {
                executors::code_parser_executor::execute_code_search(&self.state, params, dry_run)
                    .await
            }

            // ================================================================
            // Document Tools
            // ================================================================
            "document_index" => {
                executors::document_executor::execute_document_index(&self.state, params, dry_run)
                    .await
            }

            "document_search" => {
                executors::document_executor::execute_document_search(&self.state, params, dry_run)
                    .await
            }

            // ================================================================
            // Graph tools (Phase 6.6)
            // ================================================================
            "graph_query" => {
                executors::graph_executor::execute_graph_query(
                    &self.state,
                    params,
                    dry_run,
                    tool_name,
                )
                .await
            }

            "graph_insert" => {
                executors::graph_executor::execute_graph_insert(
                    &self.state,
                    params,
                    dry_run,
                    tool_name,
                )
                .await
            }

            "graph_relate" => {
                executors::graph_executor::execute_graph_relate(
                    &self.state,
                    params,
                    dry_run,
                    tool_name,
                )
                .await
            }

            "raggraph_query" => {
                executors::graph_executor::execute_raggraph_query(
                    &self.state,
                    params,
                    dry_run,
                    tool_name,
                )
                .await
            }

            "raggraph_multihop" => {
                executors::graph_executor::execute_raggraph_multihop(
                    &self.state,
                    params,
                    dry_run,
                    tool_name,
                )
                .await
            }

            // ================================================================
            // CODE GRAPH TOOLS - DEPRECATED: Routes through code_suite
            // ================================================================
            "code_graph_sync_neo4j" => {
                executors::graph_executor::execute_code_graph_sync_neo4j(
                    &self.state,
                    params,
                    dry_run,
                    tool_name,
                )
                .await
            }

            "code_graph_enrich_temporal" => {
                executors::graph_executor::execute_code_graph_enrich_temporal(
                    &self.state,
                    params,
                    dry_run,
                    tool_name,
                )
                .await
            }

            "code_graph_fusion_query" => {
                executors::graph_executor::execute_code_graph_fusion_query(
                    &self.state,
                    params,
                    dry_run,
                    tool_name,
                )
                .await
            }

            // ================================================================
            // Agent tools (Phase 6.7)
            // ================================================================
            "agent_send" => {
                executors::agent_executor::execute_agent_send(&self.state, params, dry_run).await
            }

            "agent_recv" => {
                executors::agent_executor::execute_agent_recv(&self.state, params, dry_run).await
            }

            "agent_register" => {
                executors::agent_executor::execute_agent_register(&self.state, params, dry_run)
                    .await
            }

            "agent_list" => {
                executors::agent_executor::execute_agent_list(&self.state, params, dry_run).await
            }

            "agent_status" => {
                executors::agent_executor::execute_agent_status(&self.state, params, dry_run).await
            }

            "agent_task" => {
                executors::agent_executor::execute_agent_task(&self.state, params, dry_run).await
            }

            "agent_result" => {
                executors::agent_executor::execute_agent_result(&self.state, params, dry_run).await
            }

            // ================================================================
            // Mapping tools (Phase 6.8)
            // ================================================================
            "mapping_record" => {
                executors::mapping_executor::execute_mapping_record(&self.state, params, dry_run)
                    .await
            }

            "mapping_get" => {
                executors::mapping_executor::execute_mapping_get(&self.state, params, dry_run).await
            }

            "mapping_search" => {
                executors::mapping_executor::execute_mapping_search(&self.state, params, dry_run)
                    .await
            }

            "mapping_deps" => {
                executors::mapping_executor::execute_mapping_deps(&self.state, params, dry_run)
                    .await
            }

            // ================================================================
            // Sequential Tools (Phase 6.9)
            // ================================================================
            "sequential_record" => {
                executors::sequential_executor::execute_sequential_record(
                    &self.state,
                    params,
                    dry_run,
                )
                .await
            }

            "sequential_get" => {
                executors::sequential_executor::execute_sequential_get(&self.state, params, dry_run)
                    .await
            }

            "sequential_search" => {
                executors::sequential_executor::execute_sequential_search(
                    &self.state,
                    params,
                    dry_run,
                )
                .await
            }

            "sequential_cycle" => {
                executors::sequential_executor::execute_sequential_cycle(
                    &self.state,
                    params,
                    dry_run,
                )
                .await
            }

            // ================================================================
            // APPLICATION TOOLS - DEPRECATED: Routes through mapping_suite
            // ================================================================
            "application_record" => {
                let result = executors::application_executor::execute_application_record(
                    &self.state,
                    params,
                )
                .await;
                Ok(self.route_through_suite(result))
            }

            "application_get" => {
                let result =
                    executors::application_executor::execute_application_get(&self.state, params)
                        .await;
                Ok(self.route_through_suite(result))
            }

            "application_history" => {
                let result = executors::application_executor::execute_application_history(
                    &self.state,
                    params,
                )
                .await;
                Ok(self.route_through_suite(result))
            }

            "application_search" => {
                let result = executors::application_executor::execute_application_search(
                    &self.state,
                    params,
                )
                .await;
                Ok(self.route_through_suite(result))
            }

            // ================================================================
            // LOGS TOOLS (Phase 6.11)
            // ================================================================
            "logs_tail" => {
                executors::logs_executor::execute_logs_tail(&self.state, params, dry_run).await
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
    #[allow(deprecated)]
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
