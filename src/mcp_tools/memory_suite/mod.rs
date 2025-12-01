//! Memory Suite - Unified memory and vector operations
//!
//! Commands:
//! - `store`: Store key-value pair in memory
//! - `query`: Query value by key
//! - `vector_insert`: Insert text into vector store
//! - `vector_search`: Semantic search in vector store
//! - `task_create`: Create new task with goal and priority
//! - `sequential_record`: Record a thought step in reasoning chain
//! - `sequential_get`: Get all thought steps for a task
//! - `sequential_search`: Search thought steps by semantic content
//! - `sequential_cycle`: Run sequential thinking cycles for complex task processing
//! - `help`: Show available commands

pub mod agent_commands;
pub mod intellitask_commands;
pub mod memory_commands;
pub mod sequential_commands;
pub mod task_commands;
pub mod vector_commands;

use crate::mcp_tools::{SuiteDispatcher, SuiteResult};
use crate::router::SynCoreState;
use serde::{Deserialize, Serialize};

/// Memory suite arguments
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemorySuiteArgs {
    pub command: String,
    // Memory operations
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    // Vector operations
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub namespace: Option<String>,
    // Task operations
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default)]
    pub priority: Option<i32>,
    // Sequential operations
    #[serde(default)]
    pub task_id: Option<i64>,
    #[serde(default)]
    pub step_number: Option<i32>,
    #[serde(default)]
    pub thought: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub observation: Option<String>,
    #[serde(default)]
    pub max_cycles: Option<usize>,
    // Agent operations
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
    #[serde(default)]
    pub status: Option<serde_json::Value>,
    #[serde(default)]
    pub task_type: Option<String>,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    // IntelliTask operations
    #[serde(default)]
    pub prd_content: Option<String>,
    #[serde(default)]
    pub parent_task_id: Option<String>,
    #[serde(default)]
    pub parent_task_json: Option<String>,
    #[serde(default)]
    pub tasks_json: Option<String>,
    #[serde(default)]
    pub business_context: Option<String>,
    #[serde(default)]
    pub completed_tasks: Option<Vec<String>>,
    #[serde(default)]
    pub remaining_tasks_json: Option<String>,
    #[serde(default)]
    pub breakdown_json: Option<String>,
    #[serde(default)]
    pub parent_id: Option<i64>,
    #[serde(default)]
    pub prd_title: Option<String>,
    // Advanced memory operations
    #[serde(default)]
    pub keywords: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub min_importance: Option<f32>,
    #[serde(default)]
    pub unix_timestamp: Option<u64>,
    #[serde(default)]
    pub seconds: Option<u64>,
    #[serde(default)]
    pub threshold: Option<f32>,
    // Common
    #[serde(default)]
    pub dry_run: Option<bool>,
}

/// Memory suite implementation
pub struct MemorySuite {
    state: SynCoreState,
}

impl MemorySuite {
    pub fn new(state: SynCoreState) -> Self {
        Self {
            state,
        }
    }

    /// Execute the suite command
    pub fn execute(&self, args: MemorySuiteArgs) -> SuiteResult {
        match args.command.as_str() {
            // Memory commands (delegated to memory_commands module)
            "store" => memory_commands::cmd_store(self, args),
            "query" => memory_commands::cmd_query(self, args),
            "delete" => memory_commands::cmd_delete(self, args),
            "list_keys" => memory_commands::cmd_list_keys(self, args),
            "memory_stats" => memory_commands::cmd_memory_stats(self, args),
            "search_semantic" => memory_commands::cmd_search_semantic(self, args),
            "search_hybrid" => memory_commands::cmd_search_hybrid(self, args),
            "query_by_tags" => memory_commands::cmd_query_by_tags(self, args),
            "query_by_importance" => memory_commands::cmd_query_by_importance(self, args),
            "query_recent" => memory_commands::cmd_query_recent(self, args),
            "query_since" => memory_commands::cmd_query_since(self, args),
            "consolidate_similar" => memory_commands::cmd_consolidate_similar(self, args),
            "get_related_memories" => memory_commands::cmd_get_related_memories(self, args),
            // Vector commands (delegated to vector_commands module)
            "vector_insert" => vector_commands::cmd_vector_insert(self, args),
            "vector_search" => vector_commands::cmd_vector_search(self, args),
            // Task commands (delegated to task_commands module)
            "task_create" => task_commands::cmd_task_create(self, args),
            // Sequential commands (delegated to sequential_commands module)
            "sequential_record" => sequential_commands::cmd_sequential_record(self, args),
            "sequential_get" => sequential_commands::cmd_sequential_get(self, args),
            "sequential_search" => sequential_commands::cmd_sequential_search(self, args),
            "sequential_cycle" => sequential_commands::cmd_sequential_cycle(self, args),
            // Agent commands (delegated to agent_commands module)
            "agent_send" => agent_commands::cmd_agent_send(self, args),
            "agent_recv" => agent_commands::cmd_agent_recv(self, args),
            "agent_poll" => agent_commands::cmd_agent_poll(self, args),
            "agent_register" => agent_commands::cmd_agent_register(self, args),
            "agent_list" => agent_commands::cmd_agent_list(self, args),
            "agent_status" => agent_commands::cmd_agent_status(self, args),
            "agent_task" => agent_commands::cmd_agent_task(self, args),
            "agent_result" => agent_commands::cmd_agent_result(self, args),
            // IntelliTask commands (delegated to intellitask_commands module)
            "intellitask_list" => intellitask_commands::cmd_intellitask_list(self, args),
            "intellitask_get" => intellitask_commands::cmd_intellitask_get(self, args),
            "intellitask_update_status" => {
                intellitask_commands::cmd_intellitask_update_status(self, args)
            }
            "intellitask_next_ready" => {
                intellitask_commands::cmd_intellitask_next_ready(self, args)
            }
            "intellitask_get_subtasks" => {
                intellitask_commands::cmd_intellitask_get_subtasks(self, args)
            }
            "intellitask_subtask_stats" => {
                intellitask_commands::cmd_intellitask_subtask_stats(self, args)
            }
            "intellitask_task_statistics" => {
                intellitask_commands::cmd_intellitask_task_statistics(self, args)
            }
            "intellitask_prd_statistics" => {
                intellitask_commands::cmd_intellitask_prd_statistics(self, args)
            }
            "intellitask_generate" => intellitask_commands::cmd_intellitask_generate(self, args),
            "intellitask_subtasks" => intellitask_commands::cmd_intellitask_subtasks(self, args),
            "intellitask_prioritize" => {
                intellitask_commands::cmd_intellitask_prioritize(self, args)
            }
            "intellitask_next" => intellitask_commands::cmd_intellitask_next(self, args),
            "intellitask_save" => intellitask_commands::cmd_intellitask_save(self, args),
            "help" => self.cmd_help(),
            _ => SuiteResult::err(
                &args.command,
                format!("Unknown command '{}'. Use 'help' for available commands.", args.command),
            ),
        }
    }

    fn cmd_help(&self) -> SuiteResult {
        let commands = self.list_commands();
        SuiteResult::ok(
            "help",
            serde_json::json!({
                "suite": "memory_suite",
                "description": "Memory, vector, and task operations",
                "total_commands": commands.len(),
                "commands": commands,
                "categories": {
                    "memory": ["store", "query", "delete", "list_keys", "memory_stats"],
                    "semantic": ["search_semantic", "search_hybrid", "query_by_tags", "query_by_importance", "query_recent", "query_since", "consolidate_similar", "get_related_memories"],
                    "vector": ["vector_insert", "vector_search"],
                    "tasks": ["task_create", "intellitask_list", "intellitask_get", "intellitask_update_status", "intellitask_get_subtasks", "intellitask_subtask_stats", "intellitask_task_statistics", "intellitask_next_ready", "intellitask_prd_statistics", "intellitask_generate", "intellitask_subtasks", "intellitask_prioritize", "intellitask_next", "intellitask_save"],
                    "sequential": ["sequential_record", "sequential_get", "sequential_search", "sequential_cycle"],
                    "agent": ["agent_send", "agent_recv", "agent_poll", "agent_register", "agent_list", "agent_status", "agent_task", "agent_result"]
                },
                "usage": "Use help(<command>) trait method for detailed parameters"
            }),
        )
    }
}

impl SuiteDispatcher for MemorySuite {
    fn dispatch(&self, command: &str, args: serde_json::Value) -> SuiteResult {
        let mut suite_args: MemorySuiteArgs = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return SuiteResult::err(command, format!("Invalid arguments: {}", e)),
        };
        suite_args.command = command.to_string();
        self.execute(suite_args)
    }

    fn list_commands(&self) -> Vec<&'static str> {
        vec![
            "store",
            "query",
            "delete",
            "list_keys",
            "memory_stats",
            "search_semantic",
            "search_hybrid",
            "query_by_tags",
            "query_by_importance",
            "query_recent",
            "query_since",
            "consolidate_similar",
            "get_related_memories",
            "vector_insert",
            "vector_search",
            "task_create",
            "sequential_record",
            "sequential_get",
            "sequential_search",
            "sequential_cycle",
            "agent_send",
            "agent_recv",
            "agent_poll",
            "agent_register",
            "agent_list",
            "agent_status",
            "agent_task",
            "agent_result",
            "intellitask_generate",
            "intellitask_subtasks",
            "intellitask_prioritize",
            "intellitask_next",
            "intellitask_save",
            "intellitask_get",
            "intellitask_list",
            "intellitask_update_status",
            "intellitask_next_ready",
            "intellitask_get_subtasks",
            "intellitask_subtask_stats",
            "intellitask_task_statistics",
            "intellitask_prd_statistics",
            "help",
        ]
    }

    fn help(&self, command: &str) -> Option<&'static str> {
        match command {
            "store" => Some("Store key-value pair. Params: key, value, dry_run"),
            "query" => Some("Query value by key. Params: key"),
            "delete" => Some("Delete a memory by key. Params: key"),
            "list_keys" => Some("List all memory keys. Params: limit (optional)"),
            "memory_stats" => Some("Get memory statistics. Params: none"),
            "search_semantic" => Some("Semantic search in memories. Params: query, limit (optional)"),
            "search_hybrid" => Some("Hybrid semantic + keyword search. Params: query, keywords (optional), limit (optional)"),
            "query_by_tags" => Some("Query memories by tags. Params: tags (array), namespace (optional)"),
            "query_by_importance" => Some("Query by importance threshold. Params: min_importance, limit (optional)"),
            "query_recent" => Some("Get recent memories. Params: limit (optional), namespace (optional)"),
            "query_since" => Some("Get memories since timestamp. Params: unix_timestamp, namespace (optional)"),
            "consolidate_similar" => Some("Merge similar memories. Params: threshold"),
            "get_related_memories" => Some("Find related memories. Params: key, limit (optional)"),
            "vector_insert" => Some("Insert text into vector store. Params: text, namespace, dry_run"),
            "vector_search" => Some("Semantic search. Params: query, limit"),
            "task_create" => Some("Create task. Params: goal, priority, dry_run"),
            "sequential_record" => Some("Record thought step. Params: task_id, step_number, thought, reasoning, action, observation"),
            "sequential_get" => Some("Get thought steps. Params: task_id"),
            "sequential_search" => Some("Search thought steps. Params: query, limit"),
            "sequential_cycle" => Some("Run thinking cycles. Params: max_cycles"),
            "agent_send" => Some("Send message to agent. Params: to, message"),
            "agent_recv" => Some("Receive messages for agent (NotImplemented). Params: agent"),
            "agent_poll" => Some("Poll messages with timeout (NotImplemented). Params: agent, timeout_ms"),
            "agent_register" => Some("Register agent. Params: id, capabilities"),
            "agent_list" => Some("List registered agents. Params: none"),
            "agent_status" => Some("Update agent status. Params: id, status"),
            "agent_task" => Some("Send task to agent. Params: to, task_id, task_type, payload"),
            "agent_result" => Some("Send result from agent. Params: from, task_id, result"),
            "intellitask_generate" => Some("Generate tasks from PRD (NotImplemented). Params: prd_content"),
            "intellitask_subtasks" => Some("Generate subtasks (NotImplemented). Params: parent_task_id, parent_task_json"),
            "intellitask_prioritize" => Some("Prioritize tasks (NotImplemented). Params: tasks_json, business_context"),
            "intellitask_next" => Some("Suggest next task (NotImplemented). Params: completed_tasks, remaining_tasks_json"),
            "intellitask_save" => Some("Save task breakdown (NotImplemented). Params: breakdown_json"),
            "intellitask_get" => Some("Get task by ID. Params: task_id"),
            "intellitask_list" => Some("List all tasks. Params: parent_id, prd_title, status"),
            "intellitask_update_status" => Some("Update task status. Params: task_id, status"),
            "intellitask_next_ready" => Some("Get next ready task (NotImplemented). Params: none"),
            "intellitask_get_subtasks" => Some("Get subtasks for parent. Params: parent_id"),
            "intellitask_subtask_stats" => Some("Get subtask statistics. Params: parent_id"),
            "intellitask_task_statistics" => Some("Get overall task statistics. Params: none"),
            "intellitask_prd_statistics" => Some("Get PRD statistics (NotImplemented). Params: prd_title"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_command() {
        let args = MemorySuiteArgs {
            command: "help".to_string(),
            ..Default::default()
        };
        assert_eq!(args.command, "help");
    }

    #[test]
    fn test_suite_args_deserialization() {
        let json = serde_json::json!({
            "command": "store",
            "key": "test_key",
            "value": "test_value"
        });

        let args: MemorySuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "store");
        assert_eq!(args.key, Some("test_key".to_string()));
        assert_eq!(args.value, Some("test_value".to_string()));
    }

    #[test]
    fn test_suite_result_ok() {
        let result = SuiteResult::ok("test", serde_json::json!({"data": 42}));
        assert!(result.success);
        assert_eq!(result.command, "test");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_suite_result_err() {
        let result = SuiteResult::err("test", "Something went wrong");
        assert!(!result.success);
        assert_eq!(result.command, "test");
        assert_eq!(result.error, Some("Something went wrong".to_string()));
    }
}
