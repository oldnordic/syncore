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
        Self { state }
    }

    /// Execute the suite command
    pub fn execute(&self, args: MemorySuiteArgs) -> SuiteResult {
        match args.command.as_str() {
            "store" => self.cmd_store(args),
            "query" => self.cmd_query(args),
            "delete" => self.cmd_delete(args),
            "list_keys" => self.cmd_list_keys(args),
            "memory_stats" => self.cmd_memory_stats(args),
            "search_semantic" => self.cmd_search_semantic(args),
            "search_hybrid" => self.cmd_search_hybrid(args),
            "query_by_tags" => self.cmd_query_by_tags(args),
            "query_by_importance" => self.cmd_query_by_importance(args),
            "query_recent" => self.cmd_query_recent(args),
            "query_since" => self.cmd_query_since(args),
            "consolidate_similar" => self.cmd_consolidate_similar(args),
            "get_related_memories" => self.cmd_get_related_memories(args),
            "vector_insert" => self.cmd_vector_insert(args),
            "vector_search" => self.cmd_vector_search(args),
            "task_create" => self.cmd_task_create(args),
            "sequential_record" => self.cmd_sequential_record(args),
            "sequential_get" => self.cmd_sequential_get(args),
            "sequential_search" => self.cmd_sequential_search(args),
            "sequential_cycle" => self.cmd_sequential_cycle(args),
            "agent_send" => self.cmd_agent_send(args),
            "agent_recv" => self.cmd_agent_recv(args),
            "agent_poll" => self.cmd_agent_poll(args),
            "agent_register" => self.cmd_agent_register(args),
            "agent_list" => self.cmd_agent_list(args),
            "agent_status" => self.cmd_agent_status(args),
            "agent_task" => self.cmd_agent_task(args),
            "agent_result" => self.cmd_agent_result(args),
            "intellitask_generate" => self.cmd_intellitask_generate(args),
            "intellitask_subtasks" => self.cmd_intellitask_subtasks(args),
            "intellitask_prioritize" => self.cmd_intellitask_prioritize(args),
            "intellitask_next" => self.cmd_intellitask_next(args),
            "intellitask_save" => self.cmd_intellitask_save(args),
            "intellitask_get" => self.cmd_intellitask_get(args),
            "intellitask_list" => self.cmd_intellitask_list(args),
            "intellitask_update_status" => self.cmd_intellitask_update_status(args),
            "intellitask_next_ready" => self.cmd_intellitask_next_ready(args),
            "intellitask_get_subtasks" => self.cmd_intellitask_get_subtasks(args),
            "intellitask_subtask_stats" => self.cmd_intellitask_subtask_stats(args),
            "intellitask_task_statistics" => self.cmd_intellitask_task_statistics(args),
            "intellitask_prd_statistics" => self.cmd_intellitask_prd_statistics(args),
            "help" => self.cmd_help(),
            _ => SuiteResult::err(
                &args.command,
                format!(
                    "Unknown command '{}'. Use 'help' for available commands.",
                    args.command
                ),
            ),
        }
    }

    fn cmd_store(&self, args: MemorySuiteArgs) -> SuiteResult {
        let key = match args.key {
            Some(k) => k,
            None => return SuiteResult::err("store", "Missing required parameter: key"),
        };
        let value = match args.value {
            Some(v) => v,
            None => return SuiteResult::err("store", "Missing required parameter: value"),
        };

        // APEX 2.0-M-FIX: Extract namespace parameter
        let namespace = args.namespace.as_deref();

        if args.dry_run.unwrap_or(false) {
            return SuiteResult::ok(
                "store",
                serde_json::json!({
                    "dry_run": true,
                    "would_store": { "key": key, "value": value, "namespace": namespace }
                }),
            );
        }

        // APEX 2.0-M-FIX: Use store_with_metadata() with namespace instead of store()
        let result = if let Some(ns) = namespace {
            // Explicit namespace provided
            self.state.memory.store_with_metadata(&key, &value, ns, &[], 0.5)
        } else {
            // No namespace - use configured default via store()
            self.state.memory.store(&key, &value).map(|_| 0)
        };

        match result {
            Ok(_) => SuiteResult::ok(
                "store",
                serde_json::json!({
                    "stored": true,
                    "key": key,
                    "namespace": namespace.unwrap_or("default")
                }),
            ),
            Err(e) => SuiteResult::err("store", e.to_string()),
        }
    }

    fn cmd_query(&self, args: MemorySuiteArgs) -> SuiteResult {
        let key = match args.key {
            Some(k) => k,
            None => return SuiteResult::err("query", "Missing required parameter: key"),
        };

        match self.state.memory.query(&key) {
            Ok(Some(value)) => SuiteResult::ok(
                "query",
                serde_json::json!({
                    "found": true,
                    "key": key,
                    "value": value
                }),
            ),
            Ok(None) => SuiteResult::ok(
                "query",
                serde_json::json!({
                    "found": false,
                    "key": key
                }),
            ),
            Err(e) => SuiteResult::err("query", e.to_string()),
        }
    }

    fn cmd_delete(&self, args: MemorySuiteArgs) -> SuiteResult {
        let key = match args.key {
            Some(k) => k,
            None => return SuiteResult::err("delete", "Missing required parameter: key"),
        };

        match self.state.memory.delete(&key) {
            Ok(_) => SuiteResult::ok(
                "delete",
                serde_json::json!({
                    "success": true,
                    "key": key
                }),
            ),
            Err(e) => SuiteResult::err("delete", e.to_string()),
        }
    }

    fn cmd_list_keys(&self, args: MemorySuiteArgs) -> SuiteResult {
        let limit = args.limit.map(|l| l as i64);

        match self.state.memory.list_keys(limit) {
            Ok(keys) => SuiteResult::ok(
                "list_keys",
                serde_json::json!({
                    "keys": keys,
                    "count": keys.len()
                }),
            ),
            Err(e) => SuiteResult::err("list_keys", e.to_string()),
        }
    }

    fn cmd_memory_stats(&self, _args: MemorySuiteArgs) -> SuiteResult {
        match self.state.memory.get_stats() {
            Ok((count, namespaces)) => SuiteResult::ok(
                "memory_stats",
                serde_json::json!({
                    "count": count,
                    "namespaces": namespaces,
                    "size_bytes": count * 1024 // Rough estimate
                }),
            ),
            Err(e) => SuiteResult::err("memory_stats", e.to_string()),
        }
    }

    fn cmd_search_semantic(&self, args: MemorySuiteArgs) -> SuiteResult {
        let query = match args.query {
            Some(q) => q,
            None => return SuiteResult::err("search_semantic", "Missing required parameter: query"),
        };

        let limit = args.limit.unwrap_or(10);
        let namespace = args.namespace.as_deref();

        match self.state.memory.search_semantic(&query, namespace, limit) {
            Ok(results) => SuiteResult::ok(
                "search_semantic",
                serde_json::json!({
                    "results": results,
                    "count": results.len()
                }),
            ),
            Err(e) => SuiteResult::err("search_semantic", e.to_string()),
        }
    }

    fn cmd_search_hybrid(&self, args: MemorySuiteArgs) -> SuiteResult {
        let query = match args.query {
            Some(q) => q,
            None => return SuiteResult::err("search_hybrid", "Missing required parameter: query"),
        };

        let keywords: Vec<&str> = args.keywords
            .as_ref()
            .map(|kws| kws.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();

        let limit = args.limit.unwrap_or(10);
        let namespace = args.namespace.as_deref();

        match self.state.memory.search_hybrid(&query, &keywords, namespace, limit) {
            Ok(results) => SuiteResult::ok(
                "search_hybrid",
                serde_json::json!({
                    "results": results,
                    "count": results.len()
                }),
            ),
            Err(e) => SuiteResult::err("search_hybrid", e.to_string()),
        }
    }

    fn cmd_query_by_tags(&self, args: MemorySuiteArgs) -> SuiteResult {
        let tags: Vec<&str> = match args.tags {
            Some(ref t) => t.iter().map(|s| s.as_str()).collect(),
            None => return SuiteResult::err("query_by_tags", "Missing required parameter: tags"),
        };

        let namespace = args.namespace.as_deref();

        match self.state.memory.query_by_tags(&tags, namespace) {
            Ok(entries) => SuiteResult::ok(
                "query_by_tags",
                serde_json::json!({
                    "entries": entries,
                    "count": entries.len()
                }),
            ),
            Err(e) => SuiteResult::err("query_by_tags", e.to_string()),
        }
    }

    fn cmd_query_by_importance(&self, args: MemorySuiteArgs) -> SuiteResult {
        let min_importance = match args.min_importance {
            Some(imp) => imp,
            None => return SuiteResult::err("query_by_importance", "Missing required parameter: min_importance"),
        };

        let limit = args.limit.unwrap_or(10);

        match self.state.memory.query_by_importance(min_importance, limit) {
            Ok(entries) => SuiteResult::ok(
                "query_by_importance",
                serde_json::json!({
                    "entries": entries,
                    "count": entries.len()
                }),
            ),
            Err(e) => SuiteResult::err("query_by_importance", e.to_string()),
        }
    }

    fn cmd_query_recent(&self, args: MemorySuiteArgs) -> SuiteResult {
        let limit = args.limit.unwrap_or(10);
        let namespace = args.namespace.as_deref();

        match self.state.memory.query_recent(limit, namespace) {
            Ok(entries) => SuiteResult::ok(
                "query_recent",
                serde_json::json!({
                    "entries": entries,
                    "count": entries.len()
                }),
            ),
            Err(e) => SuiteResult::err("query_recent", e.to_string()),
        }
    }

    fn cmd_query_since(&self, args: MemorySuiteArgs) -> SuiteResult {
        let timestamp = match args.unix_timestamp {
            Some(ts) => ts as i64,
            None => return SuiteResult::err("query_since", "Missing required parameter: unix_timestamp"),
        };

        let namespace = args.namespace.as_deref();

        match self.state.memory.query_since(timestamp, namespace) {
            Ok(entries) => SuiteResult::ok(
                "query_since",
                serde_json::json!({
                    "entries": entries,
                    "count": entries.len()
                }),
            ),
            Err(e) => SuiteResult::err("query_since", e.to_string()),
        }
    }

    fn cmd_consolidate_similar(&self, args: MemorySuiteArgs) -> SuiteResult {
        let threshold = match args.threshold {
            Some(t) => t,
            None => return SuiteResult::err("consolidate_similar", "Missing required parameter: threshold"),
        };

        match self.state.memory.consolidate_similar(threshold) {
            Ok(removed_ids) => SuiteResult::ok(
                "consolidate_similar",
                serde_json::json!({
                    "merged": removed_ids.len(),
                    "removed": removed_ids.len(),
                    "removed_ids": removed_ids
                }),
            ),
            Err(e) => SuiteResult::err("consolidate_similar", e.to_string()),
        }
    }

    fn cmd_get_related_memories(&self, args: MemorySuiteArgs) -> SuiteResult {
        let key = match args.key {
            Some(k) => k,
            None => return SuiteResult::err("get_related_memories", "Missing required parameter: key"),
        };

        let limit = args.limit.unwrap_or(10);

        match self.state.memory.get_related_memories(&key, limit) {
            Ok(entries) => SuiteResult::ok(
                "get_related_memories",
                serde_json::json!({
                    "entries": entries,
                    "count": entries.len()
                }),
            ),
            Err(e) => SuiteResult::err("get_related_memories", e.to_string()),
        }
    }

    fn cmd_vector_insert(&self, args: MemorySuiteArgs) -> SuiteResult {
        let text = match args.text {
            Some(t) => t,
            None => return SuiteResult::err("vector_insert", "Missing required parameter: text"),
        };

        if args.dry_run.unwrap_or(false) {
            return SuiteResult::ok(
                "vector_insert",
                serde_json::json!({
                    "dry_run": true,
                    "text_length": text.len()
                }),
            );
        }

        let namespace = args.namespace.as_deref().unwrap_or("default");

        // Use GENERAL domain store for memory/document operations
        match self.state.general_store.lock() {
            Ok(mut store) => {
                // Generate a unique ID based on text hash
                let id = {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    text.hash(&mut hasher);
                    (hasher.finish() & 0x7FFFFFFFFFFFFFFF) as i64
                };

                match store.insert_text(id, None, &text, namespace) {
                    Ok(_) => SuiteResult::ok(
                        "vector_insert",
                        serde_json::json!({
                            "inserted": true,
                            "id": id,
                            "namespace": namespace
                        }),
                    ),
                    Err(e) => SuiteResult::err("vector_insert", e.to_string()),
                }
            }
            Err(e) => SuiteResult::err("vector_insert", format!("Lock error: {}", e)),
        }
    }

    fn cmd_vector_search(&self, args: MemorySuiteArgs) -> SuiteResult {
        let query = match args.query {
            Some(q) => q,
            None => return SuiteResult::err("vector_search", "Missing required parameter: query"),
        };

        let limit = args.limit.unwrap_or(10);

        // Use GENERAL domain store for memory/document operations
        match self.state.general_store.lock() {
            Ok(store) => {
                use crate::vector::SearchScope;
                match store.search(&query, limit, SearchScope::Global) {
                    Ok(results) => {
                        let hits: Vec<serde_json::Value> = results
                            .iter()
                            .map(|hit| {
                                serde_json::json!({
                                    "id": hit.id,
                                    "score": hit.score,
                                    "text": hit.text
                                })
                            })
                            .collect();

                        SuiteResult::ok(
                            "vector_search",
                            serde_json::json!({
                                "query": query,
                                "count": hits.len(),
                                "results": hits
                            }),
                        )
                    }
                    Err(e) => SuiteResult::err("vector_search", e.to_string()),
                }
            }
            Err(e) => SuiteResult::err("vector_search", format!("Lock error: {}", e)),
        }
    }

    fn cmd_task_create(&self, args: MemorySuiteArgs) -> SuiteResult {
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

        match self.state.tasks.add_task(&goal, "", priority, None) {
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

    fn cmd_sequential_record(&self, args: MemorySuiteArgs) -> SuiteResult {
        let step_number = match args.step_number {
            Some(n) => n,
            None => {
                return SuiteResult::err(
                    "sequential_record",
                    "Missing required parameter: step_number",
                )
            }
        };

        let thought = match args.thought {
            Some(ref t) => t,
            None => {
                return SuiteResult::err("sequential_record", "Missing required parameter: thought")
            }
        };

        let reasoning = match args.reasoning {
            Some(ref r) => r,
            None => {
                return SuiteResult::err(
                    "sequential_record",
                    "Missing required parameter: reasoning",
                )
            }
        };

        use crate::portfolio::sequential_step::{SequentialStep, ThoughtStep};

        let sequential = SequentialStep::new(self.state.clone());

        let step = ThoughtStep {
            task_id: args.task_id,
            step_number,
            thought: thought.clone(),
            action: args.action.clone(),
            observation: args.observation.clone(),
            reasoning: reasoning.clone(),
        };

        match sequential.record_step(&step) {
            Ok(step_id) => SuiteResult::ok(
                "sequential_record",
                serde_json::json!({
                    "success": true,
                    "step_id": step_id,
                    "message": "Thought step recorded successfully"
                }),
            ),
            Err(e) => {
                SuiteResult::err("sequential_record", format!("Failed to record step: {}", e))
            }
        }
    }

    fn cmd_sequential_get(&self, args: MemorySuiteArgs) -> SuiteResult {
        let task_id = match args.task_id {
            Some(id) => id,
            None => {
                return SuiteResult::err("sequential_get", "Missing required parameter: task_id")
            }
        };

        use crate::portfolio::sequential_step::SequentialStep;

        let sequential = SequentialStep::new(self.state.clone());

        match sequential.get_steps_for_task(task_id) {
            Ok(steps) => SuiteResult::ok(
                "sequential_get",
                serde_json::json!({
                    "task_id": task_id,
                    "steps": steps,
                    "count": steps.len()
                }),
            ),
            Err(e) => SuiteResult::err("sequential_get", format!("Failed to get steps: {}", e)),
        }
    }

    fn cmd_sequential_search(&self, args: MemorySuiteArgs) -> SuiteResult {
        let query = match args.query {
            Some(ref q) => q,
            None => {
                return SuiteResult::err("sequential_search", "Missing required parameter: query")
            }
        };

        use crate::portfolio::sequential_step::SequentialStep;

        let sequential = SequentialStep::new(self.state.clone());
        let limit = args.limit.unwrap_or(10);

        match sequential.search_steps(query) {
            Ok(mut steps) => {
                // Apply limit manually since API doesn't support it
                steps.truncate(limit);

                SuiteResult::ok(
                    "sequential_search",
                    serde_json::json!({
                        "query": query,
                        "steps": steps,
                        "count": steps.len()
                    }),
                )
            }
            Err(e) => SuiteResult::err(
                "sequential_search",
                format!("Failed to search steps: {}", e),
            ),
        }
    }

    fn cmd_sequential_cycle(&self, args: MemorySuiteArgs) -> SuiteResult {
        let max_cycles = args.max_cycles.unwrap_or(1);

        // Create Ollama configuration
        use crate::ollama::OllamaConfig;
        use crate::sequential::{CycleResult, OllamaLanguageModel, SequentialCore};
        use std::sync::{Arc, Mutex};

        let model_name =
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:3B".to_string());

        let config = OllamaConfig {
            model: model_name.clone(),
            temperature: 0.7,
            max_tokens: 2000,
            timeout_secs: 60,
        };

        let llm = match OllamaLanguageModel::new(config) {
            Ok(llm) => {
                Arc::new(Mutex::new(llm)) as Arc<Mutex<dyn crate::sequential::LanguageModel>>
            }
            Err(e) => {
                return SuiteResult::err(
                    "sequential_cycle",
                    format!(
                        "Failed to initialize Ollama language model with model '{}': {}. \
                        Ensure Ollama is installed and the model is available (ollama pull {}).",
                        model_name, e, model_name
                    ),
                );
            }
        };

        // Create sequential reasoning engine (uses GENERAL domain for reasoning steps)
        let reasoning = SequentialCore::new(
            Arc::clone(&self.state.tasks),
            Arc::clone(&self.state.general_store),
            Arc::clone(&self.state.memory),
            llm,
            self.state.logger.clone(),
        );

        // Execute the reasoning cycle(s)
        let mut results = Vec::new();
        for _ in 0..max_cycles {
            match reasoning.run_cycle() {
                Ok(result) => match result {
                    CycleResult::Completed {
                        task_id,
                        thought,
                        decision,
                        actions,
                        action_results,
                        reflection,
                    } => {
                        results.push(serde_json::json!({
                            "success": true,
                            "task_id": task_id,
                            "thought": thought,
                            "decision": decision,
                            "actions": actions,
                            "action_results": action_results,
                            "reflection": reflection,
                        }));
                    }
                    CycleResult::NoTasks => {
                        break; // No more tasks to process
                    }
                },
                Err(e) => {
                    return SuiteResult::err(
                        "sequential_cycle",
                        format!("Sequential reasoning cycle failed: {}", e),
                    );
                }
            }
        }

        SuiteResult::ok(
            "sequential_cycle",
            serde_json::json!({
                "cycles_completed": results.len(),
                "max_cycles": max_cycles,
                "results": results
            }),
        )
    }

    // Agent commands

    fn cmd_agent_send(&self, args: MemorySuiteArgs) -> SuiteResult {
        // Check if message_bus is available
        if self.state.message_bus.is_none() {
            return SuiteResult::err(
                "agent_send",
                "NotAvailable: Agent system unavailable - MessageBus not configured",
            );
        }

        let to = match args.to {
            Some(ref t) => t,
            None => return SuiteResult::err("agent_send", "Missing required parameter: to"),
        };

        let message = match args.message {
            Some(ref m) => m,
            None => return SuiteResult::err("agent_send", "Missing required parameter: message"),
        };

        use crate::message_bus::message::{AgentId, Msg, MsgKind};
        use std::time::SystemTime;

        let bus = self.state.message_bus.as_ref().unwrap();

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
            payload: serde_json::json!({"message": message}),
            timestamp: SystemTime::now(),
        };

        bus.send(msg);

        SuiteResult::ok(
            "agent_send",
            serde_json::json!({
                "sent": true,
                "to": to
            }),
        )
    }

    fn cmd_agent_recv(&self, args: MemorySuiteArgs) -> SuiteResult {
        // Check if message_bus is available
        if self.state.message_bus.is_none() {
            return SuiteResult::err(
                "agent_recv",
                "NotAvailable: Agent system unavailable - MessageBus not configured",
            );
        }

        let _agent = match args.agent {
            Some(ref a) => a,
            None => return SuiteResult::err("agent_recv", "Missing required parameter: agent"),
        };

        // HONEST ERROR - MessageBus API does not support message polling/receiving
        SuiteResult::err(
            "agent_recv",
            "NotImplemented: MessageBus does not support message polling. \
            The current API only supports push-based message delivery via register_agent(). \
            To fix: Add get_messages() or poll_messages() method to MessageBus.",
        )
    }

    fn cmd_agent_poll(&self, args: MemorySuiteArgs) -> SuiteResult {
        // Check if message_bus is available
        if self.state.message_bus.is_none() {
            return SuiteResult::err(
                "agent_poll",
                "NotAvailable: Agent system unavailable - MessageBus not configured",
            );
        }

        let _agent = match args.agent {
            Some(ref a) => a,
            None => return SuiteResult::err("agent_poll", "Missing required parameter: agent"),
        };

        let _timeout_ms = args.timeout_ms.unwrap_or(5000);

        // HONEST ERROR - MessageBus API does not support message polling
        SuiteResult::err(
            "agent_poll",
            "NotImplemented: MessageBus does not support message polling with timeout. \
            The current API only supports push-based message delivery via register_agent().",
        )
    }

    fn cmd_agent_register(&self, args: MemorySuiteArgs) -> SuiteResult {
        // Check if message_bus is available
        if self.state.message_bus.is_none() {
            return SuiteResult::err(
                "agent_register",
                "NotAvailable: Agent system unavailable - MessageBus not configured",
            );
        }

        let id = match args.id {
            Some(ref i) => i,
            None => return SuiteResult::err("agent_register", "Missing required parameter: id"),
        };

        let capabilities = match args.capabilities {
            Some(ref c) => c.clone(),
            None => {
                return SuiteResult::err(
                    "agent_register",
                    "Missing required parameter: capabilities",
                )
            }
        };

        use crate::message_bus::message::AgentId;

        let bus = self.state.message_bus.as_ref().unwrap();

        // Parse agent ID
        let agent_id = match id.to_lowercase().as_str() {
            "claude" => AgentId::Claude,
            "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
            other => AgentId::Custom(other.to_string()),
        };

        bus.register_agent_info(agent_id, id.clone(), capabilities);

        SuiteResult::ok(
            "agent_register",
            serde_json::json!({
                "registered": true,
                "id": id
            }),
        )
    }

    fn cmd_agent_list(&self, _args: MemorySuiteArgs) -> SuiteResult {
        // Check if message_bus is available
        if self.state.message_bus.is_none() {
            return SuiteResult::err(
                "agent_list",
                "NotAvailable: Agent system unavailable - MessageBus not configured",
            );
        }

        let bus = self.state.message_bus.as_ref().unwrap();

        // Get list of registered agents
        let agents = bus.list_agents();
        let agent_names: Vec<String> = agents.iter().map(|a| format!("{:?}", a)).collect();

        SuiteResult::ok(
            "agent_list",
            serde_json::json!({
                "agents": agent_names
            }),
        )
    }

    fn cmd_agent_status(&self, args: MemorySuiteArgs) -> SuiteResult {
        // Check if message_bus is available
        if self.state.message_bus.is_none() {
            return SuiteResult::err(
                "agent_status",
                "NotAvailable: Agent system unavailable - MessageBus not configured",
            );
        }

        let id = match args.id {
            Some(ref i) => i,
            None => return SuiteResult::err("agent_status", "Missing required parameter: id"),
        };

        let status = match args.status {
            Some(ref s) => s,
            None => return SuiteResult::err("agent_status", "Missing required parameter: status"),
        };

        let bus = self.state.message_bus.as_ref().unwrap();

        // Update agent status (uses agent name, not AgentId)
        bus.update_agent_status(id.as_str(), status.clone());

        SuiteResult::ok(
            "agent_status",
            serde_json::json!({
                "updated": true,
                "id": id
            }),
        )
    }

    fn cmd_agent_task(&self, args: MemorySuiteArgs) -> SuiteResult {
        // Check if message_bus is available
        if self.state.message_bus.is_none() {
            return SuiteResult::err(
                "agent_task",
                "NotAvailable: Agent system unavailable - MessageBus not configured",
            );
        }

        let to = match args.to {
            Some(ref t) => t,
            None => return SuiteResult::err("agent_task", "Missing required parameter: to"),
        };

        let task_id = match args.task_id {
            Some(tid) => tid.to_string(),
            None => return SuiteResult::err("agent_task", "Missing required parameter: task_id"),
        };

        let task_type = match args.task_type {
            Some(ref tt) => tt,
            None => return SuiteResult::err("agent_task", "Missing required parameter: task_type"),
        };

        let payload = match args.payload {
            Some(ref p) => p,
            None => return SuiteResult::err("agent_task", "Missing required parameter: payload"),
        };

        use crate::message_bus::message::{AgentId, Msg, MsgKind};
        use std::time::SystemTime;

        let bus = self.state.message_bus.as_ref().unwrap();

        // Parse target agent ID
        let to_agent = match to.to_lowercase().as_str() {
            "claude" => AgentId::Claude,
            "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
            other => AgentId::Custom(other.to_string()),
        };

        let task_payload = serde_json::json!({
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

        SuiteResult::ok(
            "agent_task",
            serde_json::json!({
                "sent": true,
                "task_id": task_id
            }),
        )
    }

    fn cmd_agent_result(&self, args: MemorySuiteArgs) -> SuiteResult {
        // Check if message_bus is available
        if self.state.message_bus.is_none() {
            return SuiteResult::err(
                "agent_result",
                "NotAvailable: Agent system unavailable - MessageBus not configured",
            );
        }

        let from = match args.from {
            Some(ref f) => f,
            None => return SuiteResult::err("agent_result", "Missing required parameter: from"),
        };

        let task_id = match args.task_id {
            Some(tid) => tid.to_string(),
            None => return SuiteResult::err("agent_result", "Missing required parameter: task_id"),
        };

        let result = match args.result {
            Some(ref r) => r,
            None => return SuiteResult::err("agent_result", "Missing required parameter: result"),
        };

        use crate::message_bus::message::{AgentId, Msg, MsgKind};
        use std::time::SystemTime;

        let bus = self.state.message_bus.as_ref().unwrap();

        // Parse source agent ID
        let from_agent = match from.to_lowercase().as_str() {
            "claude" => AgentId::Claude,
            "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
            other => AgentId::Custom(other.to_string()),
        };

        let result_payload = serde_json::json!({
            "task_id": task_id,
            "result": result
        });

        let msg_id = bus.next_message_id();
        let msg = Msg {
            id: msg_id,
            from: from_agent,
            to: None, // Broadcast result
            kind: MsgKind::Response,
            payload: result_payload,
            timestamp: SystemTime::now(),
        };

        bus.send(msg);

        SuiteResult::ok(
            "agent_result",
            serde_json::json!({
                "recorded": true,
                "task_id": task_id
            }),
        )
    }

    // IntelliTask commands

    fn cmd_intellitask_list(&self, _args: MemorySuiteArgs) -> SuiteResult {
        use crate::tasks::Task;

        let tasks_result: Result<Vec<Task>, rusqlite::Error> = {
            let db_guard = self.state.tasks.db.lock().unwrap();
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

    fn cmd_intellitask_get(&self, args: MemorySuiteArgs) -> SuiteResult {
        let task_id = match args.task_id {
            Some(tid) => tid,
            None => {
                return SuiteResult::err("intellitask_get", "Missing required parameter: task_id")
            }
        };

        match self.state.tasks.get_task(task_id) {
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

    fn cmd_intellitask_update_status(&self, args: MemorySuiteArgs) -> SuiteResult {
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

        let db_guard = self.state.tasks.db.lock().unwrap();
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

    fn cmd_intellitask_next_ready(&self, _args: MemorySuiteArgs) -> SuiteResult {
        // Find next task ready to work on (no pending dependencies)
        // Simple heuristic: tasks with status='open' and no parent_id, or whose parent is 'done'

        use crate::tasks::Task;

        let ready_tasks_result: Result<Vec<Task>, rusqlite::Error> = {
            let db_guard = self.state.tasks.db.lock().unwrap();

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
            Err(e) => {
                SuiteResult::err("intellitask_next_ready", format!("Database error: {}", e))
            }
        }
    }

    fn cmd_intellitask_get_subtasks(&self, args: MemorySuiteArgs) -> SuiteResult {
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
            let db_guard = self.state.tasks.db.lock().unwrap();
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
            Err(e) => {
                SuiteResult::err("intellitask_get_subtasks", format!("Database error: {}", e))
            }
        }
    }

    fn cmd_intellitask_subtask_stats(&self, args: MemorySuiteArgs) -> SuiteResult {
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
            let db_guard = self.state.tasks.db.lock().unwrap();
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

    fn cmd_intellitask_task_statistics(&self, _args: MemorySuiteArgs) -> SuiteResult {
        let stats_result = {
            let db_guard = self.state.tasks.db.lock().unwrap();
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

    fn cmd_intellitask_prd_statistics(&self, args: MemorySuiteArgs) -> SuiteResult {
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
            let db_guard = self.state.tasks.db.lock().unwrap();

            let query = "
                SELECT status, COUNT(*) as count
                FROM tasks
                WHERE goal LIKE ? OR description LIKE ?
                GROUP BY status
            ";

            let search_pattern = format!("%{}%", prd_title);

            db_guard
                .prepare(query)
                .and_then(|mut stmt| {
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

    fn cmd_intellitask_generate(&self, args: MemorySuiteArgs) -> SuiteResult {
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
        let intellitask = match &self.state.intellitask {
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

    fn cmd_intellitask_subtasks(&self, args: MemorySuiteArgs) -> SuiteResult {
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
        let intellitask = match &self.state.intellitask {
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
                format!("IntelliTask subtask generation failed: {}. Check LLM backend connectivity.", e),
            ),
        }
    }

    fn cmd_intellitask_prioritize(&self, args: MemorySuiteArgs) -> SuiteResult {
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
        let intellitask = match &self.state.intellitask {
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

                SuiteResult::ok("intellitask_prioritize", serde_json::json!({ "priorities": priorities_json }))
            }
            Err(e) => SuiteResult::err(
                "intellitask_prioritize",
                format!("IntelliTask prioritization failed: {}. Check LLM backend connectivity.", e),
            ),
        }
    }

    fn cmd_intellitask_next(&self, args: MemorySuiteArgs) -> SuiteResult {
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
        let intellitask = match &self.state.intellitask {
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
                format!("IntelliTask next task suggestion failed: {}. Check LLM backend connectivity.", e),
            ),
        }
    }

    fn cmd_intellitask_save(&self, args: MemorySuiteArgs) -> SuiteResult {
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
        let breakdown: crate::intellitask::TaskBreakdown =
            match serde_json::from_str(breakdown_json) {
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
            let parent_id = match self.state.tasks.add_task(
                &parent_task.title,
                &parent_task.description,
                3, // Default priority
                None,
            ) {
                Ok(id) => id,
                Err(e) => {
                    return SuiteResult::err(
                        "intellitask_save",
                        format!("Failed to insert parent task '{}': {}", parent_task.title, e),
                    )
                }
            };
            parent_task_ids.push(parent_id);

            // Insert subtasks
            for subtask in &parent_task.subtasks {
                match self.state.tasks.add_task(
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
