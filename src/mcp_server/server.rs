use crate::macro_tools::planner::ExecutionRecorder;
use crate::message_bus::message::{AgentId, Msg, MsgKind};
use crate::router::SynCoreState;
use crate::runtime::{create_executor, ExecutorKind};
use anyhow::Result;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

// Request type definitions
use super::types::*;


#[derive(Clone)]
pub struct SynCoreMCPServer {
    state: Arc<SynCoreState>,
    tool_router: ToolRouter<Self>,
    pub executor: Arc<dyn ExecutionRecorder + Send + Sync>,
}

#[tool_router]
impl SynCoreMCPServer {
    #[allow(dead_code)]
    pub fn new(state: SynCoreState) -> Self {
        let state = Arc::new(state);

        // Select executor at runtime via environment variable
        let kind = ExecutorKind::from_env();
        let executor = create_executor(kind, state.clone());

        Self {
            state,
            tool_router: Self::tool_router(),
            executor,
        }
    }

    /// Delegate MCP tool call to RealExecutor and convert envelope to MCP response
    ///
    /// This helper:
    /// 1. Converts MCP request to serde_json::Value params
    /// 2. Calls RealExecutor.execute_real_tool_async(tool_name, params)
    /// 3. Unwraps the envelope {"ok": bool, "data": ..., "error": ...}
    /// 4. Returns CallToolResult::success or CallToolResult::error
    ///
    /// # Arguments
    /// - tool_name: The macro tool name (e.g., "memory_store")
    /// - params: JSON params matching the tool's schema
    ///
    /// # Returns
    /// MCP CallToolResult with unwrapped data or error
    async fn mcp_delegate(
        &self,
        tool_name: &str,
        params: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        use crate::macro_tools::executor_real::RealExecutor;

        // Create a new RealExecutor instance to call execute_real_tool_async
        // (The self.executor is trait object, we need concrete type)
        let real_executor = RealExecutor::new(self.state.clone());

        // Call unified executor
        let envelope = real_executor
            .execute_real_tool_async(tool_name, &params)
            .await
            .map_err(|e| McpError::internal_error(format!("Executor error: {}", e), None))?;

        // Unwrap envelope {"ok": bool, "data": ..., "error": {...}}
        match envelope.get("ok") {
            Some(serde_json::Value::Bool(true)) => {
                // Success case: extract data field
                let empty_obj = serde_json::json!({});
                let data = envelope.get("data").unwrap_or(&empty_obj);

                // Convert data to text for MCP response
                let text = if data.is_string() {
                    data.as_str().unwrap().to_string()
                } else {
                    serde_json::to_string_pretty(data).unwrap_or_else(|_| data.to_string())
                };

                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Some(serde_json::Value::Bool(false)) | _ => {
                // Error case: extract error field
                let error = envelope
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");

                Ok(CallToolResult::error(vec![Content::text(
                    error.to_string(),
                )]))
            }
        }
    }

    /// Build unified router for ALL MCP transports (STDIO, HTTP SSE, HTTP Streaming)
    /// This ensures all 49 tools are available on all transport modes
    pub fn build_unified_router() -> ToolRouter<Self> {
        Self::tool_router()
    }

    // ==================== APEX v1.3 UNIFIED SUITE TOOLS ====================

    #[tool(
        description = "Unified memory and vector operations. Commands: store, query, vector_insert, vector_search, task_create, sequential_record, sequential_get, sequential_search, sequential_cycle, agent_send, agent_recv, agent_poll, agent_register, agent_list, agent_status, agent_task, agent_result, intellitask_generate, intellitask_subtasks, intellitask_prioritize, intellitask_next, intellitask_save, intellitask_get, intellitask_list, intellitask_update_status, intellitask_next_ready, intellitask_get_subtasks, intellitask_subtask_stats, intellitask_task_statistics, intellitask_prd_statistics, help"
    )]
    async fn memory_suite(
        &self,
        Parameters(params): Parameters<MemorySuiteRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::mcp_tools::memory_suite::{MemorySuite, MemorySuiteArgs};

        let args = MemorySuiteArgs {
            command: params.command,
            key: params.key,
            value: params.value,
            text: params.text,
            query: params.query,
            limit: params.limit,
            namespace: params.namespace,
            goal: params.goal,
            priority: params.priority,
            task_id: params.task_id,
            step_number: params.step_number,
            thought: params.thought,
            reasoning: params.reasoning,
            action: params.action,
            observation: params.observation,
            max_cycles: params.max_cycles,
            to: params.to,
            from: params.from,
            agent: params.agent,
            id: params.id,
            message: params.message,
            capabilities: params.capabilities,
            status: params.status,
            task_type: params.task_type,
            payload: params.payload,
            result: params.result,
            timeout_ms: params.timeout_ms,
            prd_content: params.prd_content,
            parent_task_id: params.parent_task_id,
            parent_task_json: params.parent_task_json,
            tasks_json: params.tasks_json,
            business_context: params.business_context,
            completed_tasks: params.completed_tasks,
            remaining_tasks_json: params.remaining_tasks_json,
            breakdown_json: params.breakdown_json,
            parent_id: params.parent_id,
            prd_title: params.prd_title,
            keywords: None,
            tags: None,
            min_importance: None,
            unix_timestamp: None,
            seconds: None,
            threshold: None,
            dry_run: params.dry_run,
        };

        let suite = MemorySuite::new((*self.state).clone());
        let result = suite.execute(args);

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{:?}", result)),
        )]))
    }

    #[tool(
        description = "Unified code indexing, search, and parsing. Commands: index, search, parse, help"
    )]
    async fn code_suite(
        &self,
        Parameters(params): Parameters<CodeSuiteRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::mcp_tools::code_suite::{CodeSuite, CodeSuiteArgs};

        let args = CodeSuiteArgs {
            command: params.command,
            file_path: params.file_path,
            query: params.query,
            pattern: params.pattern,
            limit: params.limit,
            directory: params.directory,
            context_lines: params.context_lines,
            function_name: params.function_name,
            namespace: None,
            mode_hint: None,
            top_k: None,
            scope: None,
            project_label: None,
            local_root: None,
            only_missing: None,
        };

        let suite = CodeSuite::new((*self.state).clone());
        let result = suite.execute(args);

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{:?}", result)),
        )]))
    }

    #[tool(description = "Unified Neo4j graph operations. Commands: query, insert, relate, help")]
    async fn graph_suite(
        &self,
        Parameters(params): Parameters<GraphSuiteRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::mcp_tools::graph_suite::{GraphSuite, GraphSuiteArgs};

        let args = GraphSuiteArgs {
            command: params.command,
            cypher: params.cypher,
            params: params.params,
            from_id: params.from_id,
            to_id: params.to_id,
            rel_type: params.rel_type,
            from_label: params.from_label,
            to_label: params.to_label,
            query_text: None,
            seed_nodes: None,
        };

        let suite = GraphSuite::new((*self.state).clone());
        let result = suite.execute(args);

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{:?}", result)),
        )]))
    }

    #[tool(
        description = "Unified application structure mapping. Commands: record, get, search, deps, help"
    )]
    async fn mapping_suite(
        &self,
        Parameters(params): Parameters<MappingSuiteRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::mcp_tools::mapping_suite::{MappingSuite, MappingSuiteArgs};

        let args = MappingSuiteArgs {
            command: params.command,
            path: params.path,
            kind: params.kind,
            language: params.language,
            imports: params.imports,
            exports: params.exports,
            dependencies: params.dependencies,
            query: params.query,
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
        let result = suite.execute(args);

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{:?}", result)),
        )]))
    }

    #[tool(
        description = "Unified debugging, logs, and project analysis. Commands: logs_tail, tool_metadata_list, project_file_report, project_module_map, project_hotspots, project_cycles, project_dead_code, project_unused_imports, project_refactor_suggestions, help"
    )]
    async fn debug_suite(
        &self,
        Parameters(params): Parameters<DebugSuiteRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::mcp_tools::debug_suite::{DebugSuite, DebugSuiteArgs};

        let args = DebugSuiteArgs {
            command: params.command,
            file_path: params.file_path,
            n: params.n,
            limit: params.limit,
            root: params.root,
            max_modules: params.max_modules,
            max_cycles: params.max_cycles,
            max_depth: params.max_depth,
            min_loc: params.min_loc,
            min_fan_in: params.min_fan_in,
            min_fan_out: params.min_fan_out,
            min_entity_count: params.min_entity_count,
            exclude_public: params.exclude_public,
            loc_threshold: params.loc_threshold,
            fan_in_threshold: params.fan_in_threshold,
            fan_out_threshold: params.fan_out_threshold,
            entity_threshold: params.entity_threshold,
        };

        let suite = DebugSuite::new((*self.state).clone());
        let result = suite.execute(args);

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{:?}", result)),
        )]))
    }

    #[tool(
        description = "APEX 1.8 REFRAG - Selective expansion pipeline. Commands: query, configure, help"
    )]
    async fn refrag_suite(
        &self,
        Parameters(params): Parameters<RefragSuiteRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::mcp_tools::refrag_suite::RefragSuite;
        use crate::mcp_tools::SuiteDispatcher;

        let suite = RefragSuite::new((*self.state).clone());

        let args = serde_json::json!({
            "query": params.query,
            "top_k_raw": params.top_k_raw,
            "max_tokens": params.max_tokens,
            "policy": params.policy,
        });

        let result = suite.dispatch(&params.command, args);

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{:?}", result)),
        )]))
    }

    // ==================== DEPRECATED TOOLS ====================

    #[tool(
        description = "[DEPRECATED] Store a value in memory. Use memory_suite with command='store' instead."
    )]
    async fn memory_store(
        &self,
        Parameters(params): Parameters<MemoryStoreRequest>,
    ) -> Result<CallToolResult, McpError> {
        crate::mcp_tools::emit_deprecation_warning("memory_store", "memory_suite", "store");
        self.mcp_delegate(
            "memory_store",
            serde_json::json!({
                "key": params.key,
                "value": params.value,
                "dry_run": params.dry_run
            }),
        )
        .await
    }

    #[tool(
        description = "[DEPRECATED] Query a value from memory. Use memory_suite with command='query' instead."
    )]
    async fn memory_query(
        &self,
        Parameters(params): Parameters<MemoryQueryRequest>,
    ) -> Result<CallToolResult, McpError> {
        crate::mcp_tools::emit_deprecation_warning("memory_query", "memory_suite", "query");
        self.mcp_delegate(
            "memory_query",
            serde_json::json!({
                "key": params.key
            }),
        )
        .await
    }

    #[tool(description = "Create a new task")]
    async fn task_create(
        &self,
        Parameters(params): Parameters<TaskCreateRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "task_create",
            serde_json::json!({
                "goal": params.goal,
                "priority": params.priority
            }),
        )
        .await
    }

    #[tool(
        description = "[DEPRECATED] Insert text into vector memory. Use memory_suite with command='vector_insert' instead."
    )]
    async fn vector_insert(
        &self,
        Parameters(params): Parameters<VectorInsertRequest>,
    ) -> Result<CallToolResult, McpError> {
        crate::mcp_tools::emit_deprecation_warning(
            "vector_insert",
            "memory_suite",
            "vector_insert",
        );
        self.mcp_delegate(
            "vector_insert",
            serde_json::json!({
                "text": params.text,
                "metadata": params.metadata,
                "dry_run": params.dry_run
            }),
        )
        .await
    }

    #[tool(
        description = "[DEPRECATED] Search vector memory. Use memory_suite with command='vector_search' instead."
    )]
    async fn vector_search(
        &self,
        Parameters(params): Parameters<VectorSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        crate::mcp_tools::emit_deprecation_warning(
            "vector_search",
            "memory_suite",
            "vector_search",
        );
        self.mcp_delegate(
            "vector_search",
            serde_json::json!({
                "query": params.query,
                "limit": params.limit
            }),
        )
        .await
    }

    #[tool(
        description = "[DEPRECATED] Get recent log entries. Use debug_suite with command='logs_tail' instead."
    )]
    async fn logs_tail(
        &self,
        Parameters(params): Parameters<LogsTailRequest>,
    ) -> Result<CallToolResult, McpError> {
        crate::mcp_tools::emit_deprecation_warning("logs_tail", "debug_suite", "logs_tail");
        self.mcp_delegate(
            "logs_tail",
            serde_json::json!({
                "n": params.n
            }),
        )
        .await
    }

    #[tool(description = "List metadata for all MCP tools (category, cost, side effects)")]
    async fn tool_metadata_list(&self) -> Result<CallToolResult, McpError> {
        use crate::mcp::tool_metadata;

        let metadata = tool_metadata::list_all_metadata();
        let metadata_json: Vec<serde_json::Value> = metadata
            .iter()
            .map(|m| {
                serde_json::json!({
                    "name": m.name,
                    "version": m.version,
                    "category": m.category,
                    "cost": m.cost,
                    "side_effects": {
                        "modifies_database": m.side_effects.modifies_database,
                        "modifies_filesystem": m.side_effects.modifies_filesystem,
                        "modifies_vector_store": m.side_effects.modifies_vector_store,
                        "modifies_graph": m.side_effects.modifies_graph,
                        "network_call": m.side_effects.network_call,
                    },
                    "description": m.description,
                })
            })
            .collect();

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&serde_json::json!({
                "tools": metadata_json,
                "count": metadata.len(),
            }))
            .unwrap_or_else(|_| "Failed to serialize metadata".to_string()),
        )]))
    }

    #[tool(description = "Run sequential thinking cycles for complex task processing")]
    async fn sequential_cycle(
        &self,
        Parameters(params): Parameters<SequentialCycleRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "sequential_cycle",
            serde_json::json!({
                "max_cycles": params.max_cycles
            }),
        )
        .await
    }

    #[tool(
        description = "Analyze code structure using tree-sitter parser. Set persist=true to also index entities to SQLite, update HNSW, and sync to Neo4j."
    )]
    async fn parser_analyze(
        &self,
        Parameters(params): Parameters<ParserAnalyzeRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "parser_analyze",
            serde_json::json!({
                "file_path": params.file_path,
                "persist": params.persist
            }),
        )
        .await
    }

    #[tool(description = "Search code patterns using ripgrep")]
    async fn parser_search(
        &self,
        Parameters(params): Parameters<ParserSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "parser_search",
            serde_json::json!({
                "pattern": params.pattern,
                "path": params.path,
                "context_lines": params.context_lines
            }),
        )
        .await
    }

    #[tool(
        description = "[DEPRECATED] Index a source code file. Use code_suite with command='index' instead."
    )]
    async fn code_index(
        &self,
        Parameters(params): Parameters<CodeIndexRequest>,
    ) -> Result<CallToolResult, McpError> {
        crate::mcp_tools::emit_deprecation_warning("code_index", "code_suite", "index");
        self.mcp_delegate(
            "code_index",
            serde_json::json!({
                "file_path": params.file_path
            }),
        )
        .await
    }

    #[tool(
        description = "[DEPRECATED] Search code using semantic meaning. Use code_suite with command='search' instead."
    )]
    async fn code_search(
        &self,
        Parameters(params): Parameters<CodeSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        crate::mcp_tools::emit_deprecation_warning("code_search", "code_suite", "search");
        self.mcp_delegate(
            "code_search",
            serde_json::json!({
                "query": params.query,
                "limit": params.limit
            }),
        )
        .await
    }

    #[tool(description = "Index all code files in a directory matching a glob pattern")]
    async fn code_index_directory(
        &self,
        Parameters(params): Parameters<CodeIndexDirectoryRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "code_index_directory",
            serde_json::json!({
                "directory": params.directory,
                "pattern": params.pattern
            }),
        )
        .await
    }

    // STEP D: Explain Function Tool - returns signature, docstring, callers, callees, complexity
    #[tool(
        description = "Explain a function with signature, docstring, callers, callees, and complexity metrics"
    )]
    async fn explain_function(
        &self,
        Parameters(params): Parameters<ExplainFunctionRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::code_graph::explain::FunctionExplainer;

        // Read the source file
        let code = match std::fs::read_to_string(&params.file_path) {
            Ok(c) => c,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to read file '{}': {}",
                    params.file_path, e
                ))]));
            }
        };

        // Get callers and callees from code graph via mcp_delegate
        let callers = self.get_callers(&params.function_name).await;
        let callees = self.get_callees(&params.function_name).await;

        // Use FunctionExplainer to analyze the function
        let explainer = FunctionExplainer::new();
        match explainer.explain(
            &params.function_name,
            &params.file_path,
            &code,
            callers,
            callees,
        ) {
            Some(response) => {
                let json_output = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "Failed to serialize response".to_string());
                Ok(CallToolResult::success(vec![Content::text(json_output)]))
            }
            None => Ok(CallToolResult::error(vec![Content::text(format!(
                "Function '{}' not found in '{}'",
                params.function_name, params.file_path
            ))])),
        }
    }

    /// Helper to get callers of a function from Neo4j or SQLite code_edges
    async fn get_callers(&self, function_name: &str) -> Vec<String> {
        // Try Neo4j first
        if self.state.neo4j.is_some() {
            let query = format!(
                "MATCH (caller:CodeEntity)-[:CALLS]->(f:CodeEntity {{name: '{}'}}) RETURN caller.name AS name LIMIT 20",
                function_name
            );

            if let Ok(result) = self
                .mcp_delegate("graph_query", serde_json::json!({"cypher": query}))
                .await
            {
                let names = Self::extract_names_from_result(result);
                if !names.is_empty() {
                    return names;
                }
            }
        }

        // Fall back to SQLite code_edges (CALLS edges)
        self.get_callers_from_sqlite(function_name)
    }

    /// Helper to get callees of a function from Neo4j or SQLite code_edges
    async fn get_callees(&self, function_name: &str) -> Vec<String> {
        // Try Neo4j first
        if self.state.neo4j.is_some() {
            let query = format!(
                "MATCH (f:CodeEntity {{name: '{}'}})-[:CALLS]->(callee:CodeEntity) RETURN callee.name AS name LIMIT 20",
                function_name
            );

            if let Ok(result) = self
                .mcp_delegate("graph_query", serde_json::json!({"cypher": query}))
                .await
            {
                let names = Self::extract_names_from_result(result);
                if !names.is_empty() {
                    return names;
                }
            }
        }

        // Fall back to SQLite code_edges (CALLS edges)
        self.get_callees_from_sqlite(function_name)
    }

    /// Get callers from SQLite code_edges table
    fn get_callers_from_sqlite(&self, function_name: &str) -> Vec<String> {
        let conn = self.state.db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let query = r#"
            SELECT DISTINCT src.name
            FROM code_edges e
            JOIN code_entities src ON e.src_entity_id = src.id
            JOIN code_entities dst ON e.dst_entity_id = dst.id
            WHERE dst.name = ?1 AND e.edge_type = 'calls'
            LIMIT 20
        "#;

        let mut stmt = match conn_guard.prepare(query) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let names: Vec<String> = stmt
            .query_map([function_name], |row| row.get::<_, String>(0))
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();

        names
    }

    /// Get callees from SQLite code_edges table
    fn get_callees_from_sqlite(&self, function_name: &str) -> Vec<String> {
        let conn = self.state.db_manager.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        let query = r#"
            SELECT DISTINCT dst.name
            FROM code_edges e
            JOIN code_entities src ON e.src_entity_id = src.id
            JOIN code_entities dst ON e.dst_entity_id = dst.id
            WHERE src.name = ?1 AND e.edge_type = 'calls'
            LIMIT 20
        "#;

        let mut stmt = match conn_guard.prepare(query) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let names: Vec<String> = stmt
            .query_map([function_name], |row| row.get::<_, String>(0))
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();

        names
    }

    /// Extract names from a graph query result
    fn extract_names_from_result(result: CallToolResult) -> Vec<String> {
        // Get text content from result
        for content in &result.content {
            // Content is an enum - check if it's a text variant
            if let Some(text) = Self::get_text_from_content(content) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(rows) = json.get("rows").and_then(|r| r.as_array()) {
                        return rows
                            .iter()
                            .filter_map(|row| row.get("name").and_then(|n| n.as_str()))
                            .map(|s| s.to_string())
                            .collect();
                    }
                }
            }
        }
        vec![]
    }

    /// Extract text from a Content item if it's a text variant
    fn get_text_from_content(content: &Content) -> Option<String> {
        // Use serde to extract text - Content is serializable
        let json = serde_json::to_value(content).ok()?;
        json.get("text")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
    }

    #[tool(description = "Index documents from a directory into global knowledge store")]
    async fn document_index(
        &self,
        Parameters(params): Parameters<DocumentIndexRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "document_index",
            serde_json::json!({
                "directory": params.directory
            }),
        )
        .await
    }

    #[tool(description = "Semantic search across indexed documents using vector embeddings")]
    async fn document_search(
        &self,
        Parameters(params): Parameters<DocumentSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "document_search",
            serde_json::json!({
                "query": params.query,
                "limit": params.limit
            }),
        )
        .await
    }

    #[tool(description = "IntelliTask: Generate intelligent task breakdown from PRD using AI")]
    async fn intellitask_generate(
        &self,
        Parameters(params): Parameters<IntelliTaskGenerateRequest>,
    ) -> Result<CallToolResult, McpError> {
        match crate::ollama::OllamaClient::new_default() {
            Ok(ollama) => {
                let intellitask = crate::intellitask::IntelliTask::new(ollama);
                match intellitask.generate_tasks_from_prd(&params.prd_content) {
                    Ok(breakdown) => {
                        let json_output = serde_json::to_string_pretty(&breakdown)
                            .unwrap_or_else(|_| "Failed to serialize breakdown".to_string());
                        Ok(CallToolResult::success(vec![Content::text(json_output)]))
                    }
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                        "Failed to generate tasks: {}",
                        e
                    ))])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Ollama unavailable: {}. Ensure Ollama is running with phi3:mini",
                e
            ))])),
        }
    }

    #[tool(description = "IntelliTask: Generate subtasks for a parent task")]
    async fn intellitask_subtasks(
        &self,
        Parameters(params): Parameters<IntelliTaskSubtasksRequest>,
    ) -> Result<CallToolResult, McpError> {
        match crate::ollama::OllamaClient::new_default() {
            Ok(ollama) => {
                let parent_task: crate::intellitask::ParentTask =
                    match serde_json::from_str(&params.parent_task_json) {
                        Ok(task) => task,
                        Err(e) => {
                            return Ok(CallToolResult::error(vec![Content::text(format!(
                                "Invalid parent task JSON: {}",
                                e
                            ))]))
                        }
                    };

                let intellitask = crate::intellitask::IntelliTask::new(ollama);
                let codebase_context = params.codebase_context.as_deref().unwrap_or("");

                match intellitask.generate_subtasks(&parent_task, codebase_context) {
                    Ok(subtasks) => {
                        let json_output = serde_json::to_string_pretty(&subtasks)
                            .unwrap_or_else(|_| "Failed to serialize subtasks".to_string());
                        Ok(CallToolResult::success(vec![Content::text(json_output)]))
                    }
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                        "Failed to generate subtasks: {}",
                        e
                    ))])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Ollama unavailable: {}",
                e
            ))])),
        }
    }

    #[tool(description = "IntelliTask: Prioritize tasks using AI reasoning")]
    async fn intellitask_prioritize(
        &self,
        Parameters(params): Parameters<IntelliTaskPrioritizeRequest>,
    ) -> Result<CallToolResult, McpError> {
        match crate::ollama::OllamaClient::new_default() {
            Ok(ollama) => {
                let tasks: Vec<crate::intellitask::ParentTask> =
                    match serde_json::from_str(&params.tasks_json) {
                        Ok(t) => t,
                        Err(e) => {
                            return Ok(CallToolResult::error(vec![Content::text(format!(
                                "Invalid tasks JSON: {}",
                                e
                            ))]))
                        }
                    };

                let intellitask = crate::intellitask::IntelliTask::new(ollama);
                let business_context = params.business_context.as_deref().unwrap_or("");

                match intellitask.prioritize_tasks(&tasks, business_context) {
                    Ok(priorities) => {
                        let json_output = serde_json::to_string_pretty(&priorities)
                            .unwrap_or_else(|_| "Failed to serialize priorities".to_string());
                        Ok(CallToolResult::success(vec![Content::text(json_output)]))
                    }
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                        "Failed to prioritize tasks: {}",
                        e
                    ))])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Ollama unavailable: {}",
                e
            ))])),
        }
    }

    #[tool(description = "IntelliTask: Suggest next task to work on")]
    async fn intellitask_next(
        &self,
        Parameters(params): Parameters<IntelliTaskNextRequest>,
    ) -> Result<CallToolResult, McpError> {
        match crate::ollama::OllamaClient::new_default() {
            Ok(ollama) => {
                let remaining_tasks: Vec<crate::intellitask::ParentTask> =
                    match serde_json::from_str(&params.remaining_tasks_json) {
                        Ok(t) => t,
                        Err(e) => {
                            return Ok(CallToolResult::error(vec![Content::text(format!(
                                "Invalid remaining tasks JSON: {}",
                                e
                            ))]))
                        }
                    };

                let intellitask = crate::intellitask::IntelliTask::new(ollama);

                match intellitask.suggest_next_task(&params.completed_tasks, &remaining_tasks) {
                    Ok(suggestion) => Ok(CallToolResult::success(vec![Content::text(suggestion)])),
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                        "Failed to suggest next task: {}",
                        e
                    ))])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Ollama unavailable: {}",
                e
            ))])),
        }
    }

    // IntelliTask 2.0 Persistence Tools

    #[tool(description = "Save IntelliTask breakdown to database")]
    async fn intellitask_save(
        &self,
        Parameters(params): Parameters<TaskSaveRequest>,
    ) -> Result<CallToolResult, McpError> {
        let persistence =
            match crate::intellitask_persistence::IntelliTaskPersistence::new(crate::common::db_paths::intellitask_db_path().as_str())
            {
                Ok(p) => p,
                Err(e) => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Failed to initialize persistence: {}",
                        e
                    ))]))
                }
            };

        // Step 1: Parse as Value to check JSON syntax
        let json_value: serde_json::Value = match serde_json::from_str(&params.breakdown_json) {
            Ok(v) => v,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid JSON syntax: {}",
                    e
                ))]))
            }
        };

        // Step 2: Validate against schema to report ALL errors at once
        let schema = schemars::schema_for!(crate::intellitask::TaskBreakdown);
        let schema_json = serde_json::to_value(&schema).expect("Failed to convert schema to JSON");

        let validator = match jsonschema::JSONSchema::compile(&schema_json) {
            Ok(v) => v,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Internal error creating validator: {}",
                    e
                ))]))
            }
        };

        let errors: Vec<String> = validator
            .validate(&json_value)
            .err()
            .map(|iter| iter.collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .map(|error| {
                let path_str = error.instance_path.to_string();
                let path = if path_str.is_empty() {
                    "root".to_string()
                } else {
                    path_str
                };
                format!("- {}: {}", path, error)
            })
            .collect();

        if !errors.is_empty() {
            let error_msg = format!(
                "Schema validation failed with {} error(s):\n\n{}\n\nRequired structure:\n- prd_title: string\n- parent_tasks: array of ParentTask\n  - id, title, description: strings\n  - subtasks: array with id, description, acceptance_criteria[], dependencies[], files_to_modify[], complexity, estimated_hours\n  - dependencies: string array\n  - complexity: Trivial|Simple|Moderate|Complex|VeryComplex\n  - estimated_hours: number\n- relevant_files: array of {{ path, purpose, action: Create|Modify|Review }}\n- estimated_complexity: Trivial|Simple|Moderate|Complex|VeryComplex",
                errors.len(),
                errors.join("\n")
            );
            return Ok(CallToolResult::error(vec![Content::text(error_msg)]));
        }

        // Step 3: Now deserialize (should succeed since schema validated)
        let breakdown: crate::intellitask::TaskBreakdown = match serde_json::from_value(json_value)
        {
            Ok(b) => b,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Deserialization failed after validation (unexpected): {}",
                    e
                ))]))
            }
        };

        match persistence.save_task_breakdown(&breakdown) {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Saved {} parent tasks to database",
                breakdown.parent_tasks.len()
            ))])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to save tasks: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Get task by ID from database")]
    async fn intellitask_get(
        &self,
        Parameters(params): Parameters<TaskGetRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "intellitask_get",
            serde_json::json!({
                "task_id": params.task_id
            }),
        )
        .await
    }

    #[tool(description = "List tasks with optional filtering")]
    async fn intellitask_list(
        &self,
        Parameters(params): Parameters<TaskListRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "intellitask_list",
            serde_json::json!({
                "status": params.status,
                "prd_title": params.prd_title,
                "parent_id": params.parent_id
            }),
        )
        .await
    }

    #[tool(description = "Update task status")]
    async fn intellitask_update_status(
        &self,
        Parameters(params): Parameters<TaskUpdateStatusRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "intellitask_update_status",
            serde_json::json!({
                "task_id": params.task_id,
                "status": params.status
            }),
        )
        .await
    }

    #[tool(description = "Get next task ready to work on (dependencies satisfied)")]
    async fn intellitask_next_ready(&self) -> Result<CallToolResult, McpError> {
        self.mcp_delegate("intellitask_next_ready", serde_json::json!({}))
            .await
    }

    #[tool(description = "Get subtasks for a parent task")]
    async fn intellitask_get_subtasks(
        &self,
        Parameters(params): Parameters<TaskSubtasksRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "intellitask_get_subtasks",
            serde_json::json!({
                "parent_id": params.parent_id
            }),
        )
        .await
    }

    #[tool(description = "Get subtask statistics for a parent task")]
    async fn intellitask_subtask_stats(
        &self,
        Parameters(params): Parameters<TaskSubtaskStatsRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "intellitask_subtask_stats",
            serde_json::json!({
                "parent_id": params.parent_id
            }),
        )
        .await
    }

    #[tool(description = "Get overall task statistics across all tasks")]
    async fn intellitask_task_statistics(
        &self,
        Parameters(_params): Parameters<TaskStatisticsRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate("intellitask_task_statistics", serde_json::json!({}))
            .await
    }

    #[tool(description = "Get task statistics for a specific PRD")]
    async fn intellitask_prd_statistics(
        &self,
        Parameters(params): Parameters<PrdStatisticsRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "intellitask_prd_statistics",
            serde_json::json!({
                "prd_title": params.prd_title
            }),
        )
        .await
    }

    // Message Bus Tool
    #[tool(description = "Send message to another agent via message bus")]
    async fn agent_send(
        &self,
        Parameters(params): Parameters<AgentSendRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "agent_send",
            serde_json::json!({
                "to": params.to,
                "message": params.message
            }),
        )
        .await
    }

    #[tool(description = "Receive pending messages for a given agent ID")]
    async fn agent_recv(
        &self,
        Parameters(params): Parameters<AgentRecvRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "agent_recv",
            serde_json::json!({
                "agent": params.agent
            }),
        )
        .await
    }

    #[tool(description = "Wait for the next message addressed to the specified agent")]
    async fn agent_poll(
        &self,
        Parameters(params): Parameters<AgentPollRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::message_bus::message::AgentId;

        // Check if message bus is configured
        let bus = match &self.state.message_bus {
            Some(b) => b.clone(),
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Message bus not configured".to_string(),
                )]));
            }
        };

        // Parse agent ID from string
        let agent_id = match params.agent.to_lowercase().as_str() {
            "claude" => AgentId::Claude,
            "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
            "" => AgentId::Internal("mcp_server".to_string()),
            other => AgentId::Custom(other.to_string()),
        };

        // Try to receive immediately first
        if let Some(msg) = bus.try_recv_for(&agent_id) {
            let response = serde_json::json!({
                "status": "ok",
                "agent": params.agent,
                "message": msg
            });
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
            )]));
        }

        // Otherwise wait with timeout
        let msg = bus.wait_for(&agent_id, params.timeout_ms);

        let response = serde_json::json!({
            "status": "ok",
            "agent": params.agent,
            "message": msg
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
        )]))
    }

    #[tool(description = "Register an agent ID and its capabilities")]
    async fn agent_register(
        &self,
        Parameters(params): Parameters<AgentRegisterRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "agent_register",
            serde_json::json!({
                "id": params.id,
                "capabilities": params.capabilities
            }),
        )
        .await
    }

    #[tool(description = "List all registered agents and their metadata")]
    async fn agent_list(
        &self,
        Parameters(_params): Parameters<AgentListRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate("agent_list", serde_json::json!({})).await
    }

    #[tool(description = "Update the status of the specified agent")]
    async fn agent_status(
        &self,
        Parameters(params): Parameters<AgentStatusRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "agent_status",
            serde_json::json!({
                "id": params.id,
                "status": params.status
            }),
        )
        .await
    }

    #[tool(description = "Send a structured task envelope to a specified agent")]
    async fn agent_task(
        &self,
        Parameters(params): Parameters<AgentTaskRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "agent_task",
            serde_json::json!({
                "to": params.to,
                "task_id": params.task_id,
                "task_type": params.task_type,
                "payload": params.payload
            }),
        )
        .await
    }

    #[tool(description = "Submit the result of a completed task to the router")]
    async fn agent_result(
        &self,
        Parameters(params): Parameters<AgentResultRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "agent_result",
            serde_json::json!({
                "from": params.from,
                "task_id": params.task_id,
                "result": params.result
            }),
        )
        .await
    }

    #[tool(
        description = "[DEPRECATED] Execute a Cypher query. Use graph_suite with command='query' instead."
    )]
    async fn graph_query(
        &self,
        Parameters(params): Parameters<GraphQueryRequest>,
    ) -> Result<CallToolResult, McpError> {
        crate::mcp_tools::emit_deprecation_warning("graph_query", "graph_suite", "query");
        self.mcp_delegate(
            "graph_query",
            serde_json::json!({
                "cypher": params.cypher,
                "params": params.params
            }),
        )
        .await
    }

    #[tool(
        description = "[DEPRECATED] Execute a Cypher write query. Use graph_suite with command='insert' instead."
    )]
    async fn graph_insert(
        &self,
        Parameters(params): Parameters<GraphInsertRequest>,
    ) -> Result<CallToolResult, McpError> {
        crate::mcp_tools::emit_deprecation_warning("graph_insert", "graph_suite", "insert");
        self.mcp_delegate(
            "graph_insert",
            serde_json::json!({
                "cypher": params.cypher,
                "params": params.params
            }),
        )
        .await
    }

    #[tool(
        description = "[DEPRECATED] Create a relationship in Neo4j. Use graph_suite with command='relate' instead."
    )]
    async fn graph_relate(
        &self,
        Parameters(params): Parameters<GraphRelateRequest>,
    ) -> Result<CallToolResult, McpError> {
        crate::mcp_tools::emit_deprecation_warning("graph_relate", "graph_suite", "relate");
        self.mcp_delegate(
            "graph_relate",
            serde_json::json!({
                "from_id": params.from_id,
                "to_id": params.to_id,
                "rel_type": params.rel_type,
                "from_label": params.from_label,
                "to_label": params.to_label
            }),
        )
        .await
    }

    // Portfolio Enhancement Tools

    #[tool(description = "Record a file node in the application structure map")]
    async fn mapping_record(
        &self,
        Parameters(params): Parameters<MappingRecordRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "mapping_record",
            serde_json::json!({
                "path": params.path,
                "kind": params.kind,
                "language": params.language,
                "imports": params.imports,
                "exports": params.exports,
                "dependencies": params.dependencies
            }),
        )
        .await
    }

    #[tool(description = "Get a file node from the application structure map")]
    async fn mapping_get(
        &self,
        Parameters(params): Parameters<MappingGetRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "mapping_get",
            serde_json::json!({
                "path": params.path
            }),
        )
        .await
    }

    #[tool(description = "Search for files related to a query using semantic search")]
    async fn mapping_search(
        &self,
        Parameters(params): Parameters<MappingSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "mapping_search",
            serde_json::json!({
                "query": params.query
            }),
        )
        .await
    }

    #[tool(description = "Get all transitive dependencies for a file")]
    async fn mapping_deps(
        &self,
        Parameters(params): Parameters<MappingDepsRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "mapping_deps",
            serde_json::json!({
                "path": params.path
            }),
        )
        .await
    }

    #[tool(description = "Record a thought step in the reasoning chain")]
    async fn sequential_record(
        &self,
        Parameters(params): Parameters<SequentialRecordRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "sequential_record",
            serde_json::json!({
                "task_id": params.task_id,
                "step_number": params.step_number,
                "thought": params.thought,
                "reasoning": params.reasoning,
                "action": params.action,
                "observation": params.observation
            }),
        )
        .await
    }

    #[tool(description = "Get all thought steps for a task")]
    async fn sequential_get(
        &self,
        Parameters(params): Parameters<SequentialGetRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "sequential_get",
            serde_json::json!({
                "task_id": params.task_id
            }),
        )
        .await
    }

    #[tool(description = "Search thought steps by semantic content")]
    async fn sequential_search(
        &self,
        Parameters(params): Parameters<SequentialSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "sequential_search",
            serde_json::json!({
                "query": params.query
            }),
        )
        .await
    }

    #[tool(description = "Record a code change in the application")]
    async fn application_record(
        &self,
        Parameters(params): Parameters<ApplicationRecordRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "application_record",
            serde_json::json!({
                "file_path": params.file_path,
                "change_type": params.change_type,
                "line_start": params.line_start,
                "line_end": params.line_end,
                "old_content": params.old_content,
                "new_content": params.new_content,
                "description": params.description,
                "task_id": params.task_id
            }),
        )
        .await
    }

    #[tool(description = "Get all code changes for a task")]
    async fn application_get(
        &self,
        Parameters(params): Parameters<ApplicationGetRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "application_get",
            serde_json::json!({
                "task_id": params.task_id
            }),
        )
        .await
    }

    #[tool(description = "Get change history for a specific file")]
    async fn application_history(
        &self,
        Parameters(params): Parameters<ApplicationHistoryRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.mcp_delegate(
            "application_history",
            serde_json::json!({
                "file_path": params.file_path
            }),
        )
        .await
    }

    #[tool(description = "Search code changes by semantic content")]
    async fn application_search(
        &self,
        Parameters(params): Parameters<ApplicationSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::portfolio::application_tool::ApplicationTool;

        let app_tool = ApplicationTool::new((*self.state).clone());

        match app_tool.search_changes(&params.query) {
            Ok(changes) => {
                let response = serde_json::json!({
                    "query": params.query,
                    "results": changes,
                    "count": changes.len()
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response)
                        .unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Search failed: {}",
                e
            ))])),
        }
    }

    // ========================================
    // RagGraph Tools
    // ========================================

    #[tool(description = "Execute RAG query with multi-hop graph reasoning")]
    async fn raggraph_query(
        &self,
        Parameters(params): Parameters<RagGraphQueryRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::raggraph::{
            validate_real_backend, RagGraphConfig, RagQuery, RaggraphBackendMode,
            RealStorageAdapter,
        };

        // Read config from environment (SYNCORE_RAGGRAPH_BACKEND)
        let config = RagGraphConfig::from_env();

        // Log query initiation
        eprintln!(
            "[RAGGraph] query: backend={:?}, query_len={}, num_hops={}, top_k={}",
            config.backend_mode,
            params.query_text.len(),
            config.num_hops,
            config.top_k
        );

        let query_engine = if config.backend_mode == RaggraphBackendMode::Real {
            // Real mode: create storage adapter with VectorStore + Neo4j
            if let Some(ref neo4j) = self.state.neo4j {
                // Cast VectorStore to VectorIndex trait object (CODE domain for graph operations)
                let vector_index =
                    self.state.code_store.clone() as Arc<Mutex<dyn crate::vector::VectorIndex>>;

                // Get dimension from VectorStore (via VectorIndex trait)
                let dimension = {
                    use crate::vector::VectorIndex;
                    let store = self.state.code_store.lock().unwrap();
                    VectorIndex::dimension(&*store).unwrap_or(384)
                };

                // Validate real backend before executing query
                if let Err(e) = validate_real_backend(
                    config.backend_mode.clone(),
                    Some(&**neo4j),
                    Some(&vector_index),
                    dimension,
                )
                .await
                {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "RAGGraph real mode validation failed: {}",
                        e
                    ))]));
                }

                // Create Real storage adapter
                let storage = Arc::new(RealStorageAdapter::new(
                    vector_index,
                    (**neo4j).clone(),
                    dimension,
                ));

                RagQuery::with_storage(config.clone(), storage)
            } else {
                RagQuery::new() // Neo4j not available, use mock
            }
        } else {
            // Mock mode (default)
            RagQuery::new()
        };

        match query_engine.query(&params.query_text) {
            Ok(result) => {
                eprintln!(
                    "[RAGGraph] query completed: top_nodes={}, reasoning_steps={}",
                    result.top_nodes.len(),
                    result.reasoning_path.len()
                );
                let response = serde_json::json!({
                    "top_nodes": result.top_nodes,
                    "context_embedding_dim": result.context_embedding.len(),
                    "reasoning_path": result.reasoning_path
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response)
                        .unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "RAG query failed: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Execute multi-hop graph diffusion from seed nodes")]
    async fn raggraph_multihop(
        &self,
        Parameters(params): Parameters<RagGraphMultihopRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::raggraph::{
            validate_real_backend, HopGraphTransformer, RagGraphConfig, RaggraphBackendMode,
            RealStorageAdapter,
        };

        // Read config from environment (SYNCORE_RAGGRAPH_BACKEND)
        let config = RagGraphConfig::from_env();

        // Log multihop initiation
        eprintln!(
            "[RAGGraph] multihop: backend={:?}, seed_nodes={}, num_hops={}, alpha={}",
            config.backend_mode,
            params.seed_nodes.len(),
            config.num_hops,
            config.alpha
        );

        let transformer = if config.backend_mode == RaggraphBackendMode::Real {
            // Real mode: create storage adapter with VectorStore + Neo4j
            if let Some(ref neo4j) = self.state.neo4j {
                // Cast VectorStore to VectorIndex trait object (CODE domain for graph operations)
                let vector_index =
                    self.state.code_store.clone() as Arc<Mutex<dyn crate::vector::VectorIndex>>;

                // Get dimension from VectorStore (via VectorIndex trait)
                let dimension = {
                    use crate::vector::VectorIndex;
                    let store = self.state.code_store.lock().unwrap();
                    VectorIndex::dimension(&*store).unwrap_or(384)
                };

                // Validate real backend before executing multihop reasoning
                if let Err(e) = validate_real_backend(
                    config.backend_mode.clone(),
                    Some(&**neo4j),
                    Some(&vector_index),
                    dimension,
                )
                .await
                {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "RAGGraph real mode validation failed: {}",
                        e
                    ))]));
                }

                // Create Real storage adapter
                let storage = Arc::new(RealStorageAdapter::new(
                    vector_index,
                    (**neo4j).clone(),
                    dimension,
                ));

                HopGraphTransformer::with_storage(config.clone(), storage)
            } else {
                HopGraphTransformer::new(config) // Neo4j not available, use mock
            }
        } else {
            // Mock mode (default)
            HopGraphTransformer::new(config)
        };

        match transformer.multi_hop_reasoning(&params.seed_nodes) {
            Ok(result) => {
                eprintln!(
                    "[RAGGraph] multihop completed: top_nodes={}, reasoning_steps={}",
                    result.top_nodes.len(),
                    result.reasoning_path.len()
                );
                let response = serde_json::json!({
                    "top_nodes": result.top_nodes,
                    "context_embedding_dim": result.context_embedding.len(),
                    "reasoning_path": result.reasoning_path
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response)
                        .unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Multi-hop reasoning failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Execute CodeGraph query with tri-mode fusion (Simple/Attention/Reasoning)"
    )]
    async fn code_graph_fusion_query(
        &self,
        Parameters(params): Parameters<crate::code_graph::RagGraphQueryRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::code_graph::{QueryScope, RagGraphAPI};

        // Parse scope from string if provided
        let scope = params
            .scope
            .as_ref()
            .map(|s| QueryScope::parse(s))
            .unwrap_or(QueryScope::Global);

        eprintln!(
            "[CodeGraph] fusion_query: query_len={}, mode_hint={:?}, top_k={:?}, scope={:?}, project={:?}",
            params.query.len(),
            params.mode_hint,
            params.top_k,
            scope,
            params.project_label
        );

        // Check if we have Neo4j available
        if let Some(ref neo4j) = self.state.neo4j {
            // Create CodeGraph instance
            let code_graph_conn = self.state.db_manager.code_graph_conn();
            let code_graph = match crate::code_graph::CodeGraph::with_connection(
                code_graph_conn,
                self.state.code_store.clone(),
            ) {
                Ok(cg) => cg,
                Err(e) => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Failed to create CodeGraph: {}",
                        e
                    ))]));
                }
            };

            // Create RAGGraph API
            let api = RagGraphAPI::new(code_graph, (**neo4j).clone());

            // Execute query with scope control
            match api
                .query_with_scope(
                    &params.query,
                    params.namespace.as_deref(),
                    params.mode_hint.as_deref(),
                    params.top_k,
                    scope,
                    params.project_label.as_deref(),
                    params.local_root.as_deref(),
                )
                .await
            {
                Ok(response) => {
                    eprintln!(
                        "[CodeGraph] fusion_query completed: entities={}, mode={}, scope={}",
                        response.entities.len(),
                        response.selected_mode,
                        response.applied_scope
                    );

                    let json_response = serde_json::to_string_pretty(&response)
                        .unwrap_or_else(|_| serde_json::to_string(&response).unwrap());

                    Ok(CallToolResult::success(vec![Content::text(json_response)]))
                }
                Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "CodeGraph fusion query failed: {}",
                    e
                ))])),
            }
        } else {
            Ok(CallToolResult::error(vec![Content::text(
                "CodeGraph fusion query requires Neo4j connection".to_string(),
            )]))
        }
    }

    #[tool(
        description = "Sync code entities and relationships from SQLite to Neo4j (post-index rebuild)"
    )]
    async fn code_graph_sync_neo4j(
        &self,
        Parameters(params): Parameters<CodeGraphSyncNeo4jRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::code_graph::neo4j_sync;

        eprintln!(
            "[CodeGraphSync] neo4j: namespace={:?}, limit={:?}",
            params.namespace, params.limit
        );

        // Check if we have Neo4j available
        if let Some(ref neo4j) = self.state.neo4j {
            // Get SQLite connection
            let code_graph_conn = self.state.db_manager.code_graph_conn();

            // STEP 1: Sync entities FIRST (required for edges to reference)
            let entity_summary = match neo4j_sync::sync_entities_to_neo4j(
                &code_graph_conn,
                &**neo4j,
                params.namespace.as_deref(),
                params.limit,
            )
            .await
            {
                Ok(summary) => {
                    eprintln!(
                        "[CodeGraphSync] entities synced: processed={}, created={}, skipped={}",
                        summary.entities_processed,
                        summary.entities_created,
                        summary.entities_skipped
                    );
                    summary
                }
                Err(e) => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "CodeGraph Neo4j entity sync failed: {}",
                        e
                    ))]));
                }
            };

            // STEP 2: Sync edges (relationships between entities)
            match neo4j_sync::sync_relationships_to_neo4j(
                &code_graph_conn,
                &**neo4j,
                params.namespace.as_deref(),
                params.limit,
            )
            .await
            {
                Ok(mut edge_summary) => {
                    // Combine entity summary into edge summary
                    edge_summary.entities_processed = entity_summary.entities_processed;
                    edge_summary.entities_created = entity_summary.entities_created;
                    edge_summary.entities_skipped = entity_summary.entities_skipped;

                    eprintln!(
                        "[CodeGraphSync] edges synced: processed={}, created={}, skipped={}",
                        edge_summary.edges_processed,
                        edge_summary.edges_created,
                        edge_summary.edges_skipped
                    );

                    let json_response = serde_json::to_string_pretty(&edge_summary)
                        .unwrap_or_else(|_| serde_json::to_string(&edge_summary).unwrap());

                    Ok(CallToolResult::success(vec![Content::text(json_response)]))
                }
                Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "CodeGraph Neo4j relationship sync failed: {}",
                    e
                ))])),
            }
        } else {
            Ok(CallToolResult::error(vec![Content::text(
                "CodeGraph Neo4j sync requires Neo4j connection".to_string(),
            )]))
        }
    }

    #[tool(description = "Enrich code entities with temporal metadata (git history + filesystem)")]
    async fn code_graph_enrich_temporal(
        &self,
        Parameters(params): Parameters<CodeGraphEnrichTemporalRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::code_graph::temporal_extractor::extract_temporal_metadata;

        eprintln!(
            "[CodeGraphEnrich] temporal: limit={:?}, only_missing={}",
            params.limit, params.only_missing
        );

        // Get SQLite connection
        let conn = self.state.db_manager.code_graph_conn();

        // Build query to get entities
        let query = if params.only_missing {
            "SELECT id, file_path FROM code_entities WHERE created_at IS NULL LIMIT ?1"
        } else {
            "SELECT id, file_path FROM code_entities LIMIT ?1"
        };

        let limit = params.limit.unwrap_or(u64::MAX);

        // Get entities to enrich
        let entities: Vec<(i64, String)> = match conn.lock() {
            Ok(conn) => {
                let mut stmt = conn.prepare(query).map_err(|e| {
                    McpError::internal_error(format!("Failed to prepare query: {}", e), None)
                })?;

                let rows = stmt
                    .query_map([limit as i64], |row| {
                        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|e| {
                        McpError::internal_error(format!("Failed to query entities: {}", e), None)
                    })?;

                rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
                    McpError::internal_error(format!("Failed to collect entities: {}", e), None)
                })?
            }
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to lock database connection: {}",
                    e
                ))]));
            }
        };

        let total_entities = entities.len();
        let mut enriched_count = 0;
        let mut failed_count = 0;

        // Enrich each entity
        for (entity_id, file_path) in entities {
            match extract_temporal_metadata(&file_path) {
                Ok(metadata) => {
                    // Update entity with temporal metadata
                    if let Ok(conn) = conn.lock() {
                        match conn.execute(
                            "UPDATE code_entities SET created_at = ?1, last_modified_at = ?2, change_count = ?3, author_count = ?4 WHERE id = ?5",
                            rusqlite::params![
                                metadata.created_at,
                                metadata.last_modified_at,
                                metadata.change_count,
                                metadata.author_count,
                                entity_id
                            ],
                        ) {
                            Ok(_) => enriched_count += 1,
                            Err(e) => {
                                eprintln!("[CodeGraphEnrich] Failed to update entity {}: {}", entity_id, e);
                                failed_count += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[CodeGraphEnrich] Failed to extract temporal metadata for {}: {}",
                        file_path, e
                    );
                    failed_count += 1;
                }
            }
        }

        let response = serde_json::json!({
            "total_entities": total_entities,
            "enriched": enriched_count,
            "failed": failed_count,
            "only_missing": params.only_missing
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap(),
        )]))
    }

    // ========================================
    // Project Analysis Engine (PAE) Tools
    // ========================================

    #[tool(description = "Generate detailed report for a single source file")]
    async fn project_file_report(
        &self,
        Parameters(params): Parameters<ProjectFileReportRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::project_analysis::{file_report::FileReportRequest, ProjectAnalysisEngine};

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = FileReportRequest {
            file_path: params.file_path,
        };

        match engine.file_report(request).await {
            Ok(response) => {
                let json = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "Failed to serialize response".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "File report failed: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Generate module-level dependency map of the project")]
    async fn project_module_map(
        &self,
        Parameters(params): Parameters<ProjectModuleMapRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::project_analysis::{deps::ModuleMapRequest, ProjectAnalysisEngine};

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = ModuleMapRequest {
            root: params.root,
            max_modules: params.max_modules,
        };

        match engine.module_map(request).await {
            Ok(response) => {
                let json = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "Failed to serialize response".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Module map failed: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Identify code hotspots based on complexity metrics")]
    async fn project_hotspots(
        &self,
        Parameters(params): Parameters<ProjectHotspotsRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::project_analysis::{hotspots::HotspotsRequest, ProjectAnalysisEngine};

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = HotspotsRequest {
            limit: params.limit,
            min_fan_in: params.min_fan_in,
            min_fan_out: params.min_fan_out,
            min_entity_count: params.min_entity_count,
            min_loc: params.min_loc,
        };

        match engine.hotspots(request).await {
            Ok(response) => {
                let json = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "Failed to serialize response".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Hotspots analysis failed: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Detect circular dependencies between files and modules")]
    async fn project_cycles(
        &self,
        Parameters(params): Parameters<ProjectCyclesRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::project_analysis::{cycles::CyclesRequest, ProjectAnalysisEngine};

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = CyclesRequest {
            max_cycles: params.max_cycles,
            max_depth: params.max_depth,
        };

        match engine.cycles(request).await {
            Ok(response) => {
                let json = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "Failed to serialize response".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Cycle detection failed: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Identify potentially dead code entities")]
    async fn project_dead_code(
        &self,
        Parameters(params): Parameters<ProjectDeadCodeRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::project_analysis::{dead_code::DeadCodeRequest, ProjectAnalysisEngine};

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = DeadCodeRequest {
            exclude_public: params.exclude_public,
            limit: params.limit,
        };

        match engine.dead_code(request).await {
            Ok(response) => {
                let json = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "Failed to serialize response".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Dead code detection failed: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Identify unused imports in source files")]
    async fn project_unused_imports(
        &self,
        Parameters(params): Parameters<ProjectUnusedImportsRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::project_analysis::{
            unused_imports::UnusedImportsRequest, ProjectAnalysisEngine,
        };

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = UnusedImportsRequest {
            file_path: params.file_path,
            limit: params.limit,
        };

        match engine.unused_imports(request).await {
            Ok(response) => {
                let json = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "Failed to serialize response".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Unused imports detection failed: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Detect code smells and anti-patterns in project")]
    async fn project_code_smells(
        &self,
        Parameters(params): Parameters<ProjectCodeSmellsRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::project_analysis::ProjectAnalysisEngine;

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());

        let limit = params.limit.unwrap_or(50) as usize;
        let include_entities = params.include_entities.unwrap_or(false);

        match engine.detect_file_smells(limit) {
            Ok(file_smells) => {
                let entity_smells = if include_entities {
                    engine.detect_entity_smells(limit).unwrap_or_default()
                } else {
                    vec![]
                };

                let response = serde_json::json!({
                    "ok": true,
                    "data": {
                        "file_smells": file_smells,
                        "entity_smells": entity_smells
                    }
                });

                let json = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "Failed to serialize response".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Code smell detection failed: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Generate heuristic refactor suggestions based on project analysis")]
    async fn project_refactor_suggestions(
        &self,
        Parameters(params): Parameters<ProjectRefactorSuggestionsRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::project_analysis::{
            refactor::RefactorSuggestionsRequest, ProjectAnalysisEngine,
        };

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = RefactorSuggestionsRequest {
            limit: params.limit,
            loc_threshold: params.loc_threshold,
            entity_threshold: params.entity_threshold,
            fan_in_threshold: params.fan_in_threshold,
            fan_out_threshold: params.fan_out_threshold,
        };

        match engine.refactor_suggestions(request).await {
            Ok(response) => {
                let json = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "Failed to serialize response".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Refactor suggestions failed: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Clean up indexed data for excluded directories (target/, node_modules/, etc.)"
    )]
    async fn project_cleanup_excluded(
        &self,
        Parameters(params): Parameters<ProjectCleanupExcludedRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::project_analysis::{cleanup::CleanupExcludedRequest, ProjectAnalysisEngine};

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = CleanupExcludedRequest {
            dry_run: params.dry_run,
            excluded_dirs: params.excluded_dirs,
        };

        match engine.cleanup_excluded(request).await {
            Ok(response) => {
                let json = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "Failed to serialize response".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Cleanup failed: {}",
                e
            ))])),
        }
    }
}

#[tool_handler]
impl ServerHandler for SynCoreMCPServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "SynCore".to_string(),
                version: env!("CARGO_PKG_VERSION", "unknown").to_string(),
                title: Some("SynCore MCP Server".to_string()),
                website_url: None,
                icons: None,
            },
            instructions: Some("SynCore MCP server providing memory, task, vector, sequential, and parser capabilities".to_string()),
        }
    }
}

/// Background router loop that forwards messages based on agent capabilities
async fn router_loop(state: SynCoreState) {
    // Router registers itself
    if let Some(bus) = &state.message_bus {
        bus.register_agent_info(
            AgentId::Internal("router".to_string()),
            "router".to_string(),
            vec!["routing".to_string()],
        );
    }

    loop {
        // Block until a message arrives for router (2 second timeout)
        let msg = {
            let bus = match &state.message_bus {
                Some(b) => b.clone(),
                None => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }
            };
            bus.wait_for(&AgentId::Internal("router".to_string()), 2000)
        };

        if msg.is_none() {
            continue;
        }

        let msg = msg.unwrap();

        // Extract task_type from envelope payload
        let task_type = msg
            .payload
            .get("task_type")
            .and_then(|v| v.as_str())
            .unwrap_or("nlp");

        // Map task_type to capability
        let cap = match task_type {
            "code" => "coding",
            "analysis" => "analysis",
            _ => "nlp",
        };

        // Determine which agent can handle this message
        let chosen = {
            let bus = match &state.message_bus {
                Some(b) => b,
                None => continue,
            };
            let mut target_agents = bus.agents_with_capability(cap);

            // Fallback to any registered agent if none match capability
            if target_agents.is_empty() {
                target_agents = bus.list_registered_agents();
            }

            // Score candidate agents
            let mut best: Option<(String, f64)> = None;

            for name in &target_agents {
                if let Some(status) = bus.get_agent_status(name) {
                    // Extract simple metrics
                    let load = status.get("load").and_then(|v| v.as_f64()).unwrap_or(0.0);

                    let busy = status
                        .get("busy")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let errors = status.get("errors").and_then(|v| v.as_u64()).unwrap_or(0);

                    // Weighted scoring formula:
                    // lower load = better
                    // not busy = better
                    // fewer errors = better
                    let score = load * 1.0 + if busy { 5.0 } else { 0.0 } + (errors as f64) * 3.0;

                    match &mut best {
                        None => best = Some((name.clone(), score)),
                        Some((_, best_score)) => {
                            if score < *best_score {
                                best = Some((name.clone(), score));
                            }
                        }
                    }
                } else {
                    // Agent has no status yet - treat as available with score 0
                    if best.is_none() {
                        best = Some((name.clone(), 0.0));
                    }
                }
            }

            best.map(|(name, _)| name)
        };

        if let Some(agent_name) = chosen {
            if let Some(bus) = &state.message_bus {
                bus.send(Msg {
                    id: bus.next_message_id(),
                    from: AgentId::Internal("router".to_string()),
                    to: Some(AgentId::Custom(agent_name.clone())),
                    kind: MsgKind::Direct,
                    payload: msg.payload.clone(),
                    timestamp: SystemTime::now(),
                });
            }
        }
    }
}

pub async fn run_mcp_stdio_server(state: SynCoreState) -> Result<()> {
    // Spawn background router task
    {
        let state_clone = state.clone();
        tokio::task::spawn(async move {
            router_loop(state_clone).await;
        });
    }

    let server = SynCoreMCPServer::new(state);

    let service = server.serve(stdio()).await.inspect_err(|e| {
        eprintln!("MCP server error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
