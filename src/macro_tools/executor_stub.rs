//! Executor Stub
//!
//! Production-like executor that performs deterministic multi-step orchestration
//! WITHOUT calling real MCP tools or doing any I/O.
//!
//! This validates execution ordering, argument propagation, and synthetic response
//! generation BEFORE connecting to real SynCore tools.

use crate::macro_tools::planner::ExecutionRecorder;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

/// Executed step with synthetic result
#[derive(Debug, Clone)]
pub struct ExecutedStep {
    pub tool_name: String,
    pub params: Value,
    pub synthetic_result: Value,
}

/// Real executor stub - deterministic, no I/O
pub struct RealExecutorStub {
    steps: Arc<Mutex<Vec<ExecutedStep>>>,
}

// Safe because RealExecutorStub only contains Arc<Mutex<_>>
unsafe impl Send for RealExecutorStub {}
unsafe impl Sync for RealExecutorStub {}

impl RealExecutorStub {
    pub fn new() -> Self {
        Self {
            steps: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_executed_steps(&self) -> Vec<ExecutedStep> {
        self.steps.lock().unwrap().clone()
    }

    /// Generate synthetic result for a tool call
    fn generate_synthetic_result(tool_name: &str, params: &Value) -> Value {
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
                    },
                    {
                        "file": "/src/agent_router.rs",
                        "line": 108,
                        "snippet": "pub async fn route(...)"
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
                            "text": format!("Synthetic result {}", i)
                        })
                    })
                    .collect();
                json!({ "results": results })
            }
            "parser_analyze" => json!({
                "functions": [
                    {"name": "send", "line": 10, "visibility": "pub"},
                    {"name": "recv", "line": 20, "visibility": "pub"}
                ],
                "structs": [
                    {"name": "MessageBus", "line": 5}
                ],
                "imports": [
                    "tokio::sync::mpsc",
                    "std::sync::Arc"
                ]
            }),
            "mapping_deps" => json!({
                "dependencies": [
                    "/src/types.rs",
                    "/src/protocol.rs"
                ]
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
                "matches": [
                    {"file": "/src/lib.rs", "line": 100},
                    {"file": "/src/main.rs", "line": 50}
                ]
            }),

            // Task tools
            "intellitask_task_statistics" => json!({
                "total_tasks": 25,
                "completed": 10,
                "pending": 12,
                "in_progress": 3
            }),
            "intellitask_next_ready" => json!({
                "ready_tasks": [
                    {"id": 5, "title": "Implement feature X"},
                    {"id": 8, "title": "Write tests for Y"}
                ],
                "next_task_id": 5
            }),
            "intellitask_prioritize" => json!({
                "task_id": 5,
                "priority_score": 8.5,
                "title": "Implement feature X"
            }),
            "intellitask_generate" => json!({
                "tasks": [
                    {"id": 1, "title": "Design authentication flow", "priority": 8},
                    {"id": 2, "title": "Implement user model", "priority": 7},
                    {"id": 3, "title": "Create login endpoint", "priority": 6}
                ]
            }),
            "intellitask_save" => json!({
                "saved": true,
                "task_count": 3
            }),
            "intellitask_subtasks" => json!({
                "subtasks": [
                    {"id": 101, "parent_id": 1, "title": "Design database schema"},
                    {"id": 102, "parent_id": 1, "title": "Create API endpoints"}
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
                "completed_subtasks": 3,
                "pending_subtasks": 2
            }),
            "task_create" => json!({
                "created": true,
                "task_id": 999
            }),
            "intellitask_list" => json!({
                "tasks": [
                    {"id": 1, "title": "Task 1", "status": "pending"},
                    {"id": 2, "title": "Task 2", "status": "in_progress"}
                ]
            }),
            "intellitask_get" => json!({
                "id": 1,
                "title": "Sample task",
                "status": "pending"
            }),
            "intellitask_get_subtasks" => json!({
                "subtasks": []
            }),
            "intellitask_prd_statistics" => json!({
                "total": 10,
                "completed": 5
            }),

            // Vector tools
            "vector_insert" => json!({
                "inserted": true,
                "id": 12345
            }),

            // Memory tools
            "memory_store" => json!({
                "stored": true
            }),
            "memory_query" => json!({
                "value": "synthetic_value"
            }),

            // Document tools
            "document_index" => json!({
                "indexed": 10
            }),
            "document_search" => json!({
                "results": []
            }),

            // Graph tools
            "graph_query" => json!({
                "nodes": [],
                "edges": []
            }),
            "graph_insert" => json!({
                "inserted": true
            }),
            "graph_relate" => json!({
                "related": true
            }),

            // Agent tools
            "agent_send" => json!({
                "sent": true
            }),
            "agent_recv" => json!({
                "messages": []
            }),
            "agent_register" => json!({
                "registered": true
            }),
            "agent_list" => json!({
                "agents": []
            }),

            // Logs tools
            "logs_tail" => json!({
                "logs": []
            }),

            _ => json!({
                "error": format!("Unknown tool: {}", tool_name)
            }),
        }
    }

    // I/O validation methods (always return false - no I/O allowed)
    pub fn performed_sqlite_io(&self) -> bool {
        false
    }

    pub fn performed_neo4j_io(&self) -> bool {
        false
    }

    pub fn performed_vector_io(&self) -> bool {
        false
    }

    pub fn performed_file_io(&self) -> bool {
        false
    }

    pub fn performed_network_io(&self) -> bool {
        false
    }
}

impl ExecutionRecorder for RealExecutorStub {
    fn record_step(&self, tool_name: &str, params: Value) {
        let synthetic_result = Self::generate_synthetic_result(tool_name, &params);

        let step = ExecutedStep {
            tool_name: tool_name.to_string(),
            params,
            synthetic_result: self.wrap_success(tool_name, synthetic_result),
        };

        self.steps.lock().unwrap().push(step);
    }

    fn wrap_success(&self, tool: &str, data: Value) -> Value {
        json!({
            "ok": true,
            "tool": tool,
            "executor": "stub",
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
                "executor": "stub"
            }
        })
    }

    fn executor_type(&self) -> &str {
        "stub"
    }
}

impl Default for RealExecutorStub {
    fn default() -> Self {
        Self::new()
    }
}
