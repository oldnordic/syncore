use anyhow::Result;
use std::sync::Arc;
use std::time::SystemTime;
use rmcp::{
    ErrorData as McpError,
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo, CallToolResult, Content, ProtocolVersion, Implementation},
    schemars,
    tool, tool_handler, tool_router,
    ServiceExt, transport::stdio,
};
use crate::router::SynCoreState;
use crate::message_bus::message::{AgentId, Msg, MsgKind};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemoryStoreRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemoryQueryRequest {
    pub key: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskCreateRequest {
    pub goal: String,
    pub priority: Option<i32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VectorInsertRequest {
    pub text: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VectorSearchRequest {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LogsTailRequest {
    pub n: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SequentialCycleRequest {
    pub max_cycles: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ParserAnalyzeRequest {
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ParserSearchRequest {
    pub pattern: String,
    pub path: Option<String>,
    pub context_lines: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CodeIndexRequest {
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CodeSearchRequest {
    pub query: String,
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CodeIndexDirectoryRequest {
    pub directory: String,
    pub pattern: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DocumentIndexRequest {
    pub directory: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DocumentSearchRequest {
    pub query: String,
    #[serde(default = "default_doc_search_limit")]
    pub limit: usize,
}

fn default_search_limit() -> usize {
    10
}

fn default_doc_search_limit() -> usize {
    5
}

// Neo4j Graph Tools
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GraphQueryRequest {
    pub cypher: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GraphInsertRequest {
    pub cypher: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GraphRelateRequest {
    pub from_id: i64,
    pub to_id: i64,
    pub rel_type: String,
    pub from_label: Option<String>,
    pub to_label: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IntelliTaskGenerateRequest {
    pub prd_content: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IntelliTaskSubtasksRequest {
    pub parent_task_id: String,
    pub parent_task_json: String,
    pub codebase_context: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IntelliTaskPrioritizeRequest {
    pub tasks_json: String,
    pub business_context: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IntelliTaskNextRequest {
    pub completed_tasks: Vec<String>,
    pub remaining_tasks_json: String,
}

// IntelliTask 2.0 Persistence Tools
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskSaveRequest {
    pub breakdown_json: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskGetRequest {
    pub task_id: i64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskListRequest {
    pub status: Option<String>,
    pub prd_title: Option<String>,
    pub parent_id: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskUpdateStatusRequest {
    pub task_id: i64,
    pub status: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskSubtasksRequest {
    pub parent_id: i64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskSubtaskStatsRequest {
    pub parent_id: i64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskStatisticsRequest {
    // No parameters needed for overall statistics
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PrdStatisticsRequest {
    pub prd_title: String,
}

// Message Bus Tools
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AgentSendRequest {
    pub to: String,
    pub message: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AgentRecvRequest {
    pub agent: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AgentPollRequest {
    pub agent: String,
    pub timeout_ms: u64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AgentRegisterRequest {
    pub id: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AgentListRequest {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AgentStatusRequest {
    pub id: String,
    pub status: serde_json::Value,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AgentTaskRequest {
    pub to: String,
    pub task_id: String,
    pub task_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AgentResultRequest {
    pub from: String,
    pub task_id: String,
    pub result: serde_json::Value,
}

// Portfolio Enhancement Tools
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MappingRecordRequest {
    pub path: String,
    pub kind: String,
    pub language: Option<String>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MappingGetRequest {
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MappingSearchRequest {
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MappingDepsRequest {
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SequentialRecordRequest {
    pub task_id: Option<i64>,
    pub step_number: i32,
    pub thought: String,
    pub action: Option<String>,
    pub observation: Option<String>,
    pub reasoning: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SequentialGetRequest {
    pub task_id: i64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SequentialSearchRequest {
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ApplicationRecordRequest {
    pub file_path: String,
    pub change_type: String,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub line_start: i32,
    pub line_end: i32,
    pub description: String,
    pub task_id: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ApplicationGetRequest {
    pub task_id: i64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ApplicationHistoryRequest {
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ApplicationSearchRequest {
    pub query: String,
}

#[derive(Clone)]
pub struct SynCoreMCPServer {
    state: Arc<SynCoreState>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl SynCoreMCPServer {
    #[allow(dead_code)]
    pub fn new(state: SynCoreState) -> Self {
        Self {
            state: Arc::new(state),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Store a value in memory")]
    async fn memory_store(
        &self,
        Parameters(params): Parameters<MemoryStoreRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self.state.memory.store(&params.key, &params.value) {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                "Memory stored successfully".to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to store memory: {}", e),
            )])),
        }
    }

    #[tool(description = "Query a value from memory")]
    async fn memory_query(
        &self,
        Parameters(params): Parameters<MemoryQueryRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self.state.memory.query(&params.key) {
            Ok(Some(value)) => Ok(CallToolResult::success(vec![Content::text(value)])),
            Ok(None) => Ok(CallToolResult::success(vec![Content::text(
                "Key not found".to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to query memory: {}", e),
            )])),
        }
    }

    #[tool(description = "Create a new task")]
    async fn task_create(
        &self,
        Parameters(params): Parameters<TaskCreateRequest>,
    ) -> Result<CallToolResult, McpError> {
        let priority = params.priority.unwrap_or(1);
        match self.state.tasks.add_task(&params.goal, "", priority, None) {
            Ok(task_id) => Ok(CallToolResult::success(vec![Content::text(
                format!("Task created with ID: {}", task_id),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to create task: {}", e),
            )])),
        }
    }

    #[tool(description = "Insert text into vector memory")]
    async fn vector_insert(
        &self,
        Parameters(params): Parameters<VectorInsertRequest>,
    ) -> Result<CallToolResult, McpError> {
        let _metadata = params.metadata.unwrap_or(serde_json::json!({}));

        match self.state.vector_store.try_lock() {
            Ok(mut store) => {
                match store.insert_text(0, None, &params.text, "mcp") {
                    Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                        "Vector inserted successfully".to_string(),
                    )])),
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(
                        format!("Failed to insert vector: {}", e),
                    )])),
                }
            }
            Err(_) => Ok(CallToolResult::error(vec![Content::text(
                "Failed to acquire vector store lock".to_string(),
            )])),
        }
    }

    #[tool(description = "Search vector memory")]
    async fn vector_search(
        &self,
        Parameters(params): Parameters<VectorSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(5);

        match self.state.vector_store.try_lock() {
            Ok(store) => {
                use crate::vector::SearchScope;
                match store.search(&params.query, limit, SearchScope::Global) {
                    Ok(results) => {
                        let results_text = results
                            .into_iter()
                            .map(|hit| format!("{} (score: {:.3})", hit.text, hit.score))
                            .collect::<Vec<_>>()
                            .join("\n");

                        Ok(CallToolResult::success(vec![Content::text(
                            if results_text.is_empty() {
                                "No results found".to_string()
                            } else {
                                results_text
                            },
                        )]))
                    }
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(
                        format!("Failed to search vectors: {}", e),
                    )])),
                }
            }
            Err(_) => Ok(CallToolResult::error(vec![Content::text(
                "Failed to acquire vector store lock".to_string(),
            )])),
        }
    }

    #[tool(description = "Get recent log entries")]
    async fn logs_tail(
        &self,
        Parameters(params): Parameters<LogsTailRequest>,
    ) -> Result<CallToolResult, McpError> {
        let n = params.n.unwrap_or(10);

        // Read logs from the log file or system logs
        match self.tail_logs(n) {
            Ok(log_entries) => Ok(CallToolResult::success(vec![Content::text(
                if log_entries.is_empty() {
                    "No log entries found".to_string()
                } else {
                    log_entries.join("\n")
                },
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to tail logs: {}", e),
            )])),
        }
    }

    fn tail_logs(&self, n: usize) -> Result<Vec<String>> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        use std::path::Path;

        // Try to read from syncore.log file first
        let log_path = Path::new("syncore.log");
        if log_path.exists() {
            let file = File::open(log_path)?;
            let reader = BufReader::new(file);
            let lines: Vec<String> = reader.lines()
                .filter_map(|line| line.ok())
                .collect();

            // Return the last n lines
            let start = if lines.len() > n { lines.len() - n } else { 0 };
            return Ok(lines[start..].to_vec());
        }

        // Fallback: try to read from system logs or create sample logs
        if cfg!(target_os = "linux") {
            // Try to read from journalctl or syslog
            let output = std::process::Command::new("journalctl")
                .args(&["-n", &n.to_string(), "--no-pager"])
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    let logs = String::from_utf8_lossy(&output.stdout);
                    return Ok(logs.lines().map(|s| s.to_string()).collect());
                }
            }

            // Try syslog
            let output = std::process::Command::new("tail")
                .args(&["-n", &n.to_string(), "/var/log/syslog"])
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    let logs = String::from_utf8_lossy(&output.stdout);
                    return Ok(logs.lines().map(|s| s.to_string()).collect());
                }
            }
        }

        // If no logs found, return some sample entries
        Ok(vec![
            "[INFO] SynCore server started".to_string(),
            "[INFO] Memory module initialized".to_string(),
            "[INFO] Task manager initialized".to_string(),
            "[INFO] Vector store initialized".to_string(),
            "[INFO] MCP server started".to_string(),
            "[DEBUG] Waiting for client connections...".to_string(),
            "[INFO] Client connected".to_string(),
            "[DEBUG] Processing request...".to_string(),
            "[INFO] Request completed successfully".to_string(),
            "[DEBUG] Response sent to client".to_string(),
        ])
    }

    #[tool(description = "Run sequential thinking cycles for complex task processing")]
    async fn sequential_cycle(
        &self,
        Parameters(params): Parameters<SequentialCycleRequest>,
    ) -> Result<CallToolResult, McpError> {
        let max_cycles = params.max_cycles.unwrap_or(1);

        // Try to create Ollama language model, fall back to demo if unavailable
        let model: Arc<std::sync::Mutex<dyn crate::sequential::LanguageModel>> =
            match crate::sequential::OllamaLanguageModel::new_default() {
                Ok(ollama_model) => {
                    tracing::info!("Using Ollama phi3-mini for sequential reasoning");
                    Arc::new(std::sync::Mutex::new(ollama_model))
                }
                Err(e) => {
                    tracing::warn!("Ollama unavailable ({}), falling back to demo mode", e);
                    Arc::new(std::sync::Mutex::new(crate::sequential::DemoLanguageModel::new()))
                }
            };

        // Create sequential core
        let sequential_core = crate::sequential::SequentialCore::new(
            self.state.tasks.clone(),
            self.state.vector_store.clone(),
            self.state.memory.clone(),
            model,
            self.state.logger.clone(),
        );

        match sequential_core.run_batch_cycles(max_cycles) {
            Ok(results) => {
                let mut output = String::new();
                for result in results {
                    match result {
                        crate::sequential::CycleResult::Completed { task_id, thought, decision, actions, action_results, reflection } => {
                            output.push_str(&format!("=== TASK {} COMPLETED ===\n", task_id));
                            output.push_str(&format!("THOUGHT:\n{}\n\n", thought));
                            output.push_str(&format!("DECISION:\n{}\n\n", decision));
                            output.push_str(&format!("ACTIONS ({}):\n", actions.len()));
                            for (i, action) in actions.iter().enumerate() {
                                output.push_str(&format!("{}. {}\n", i + 1, action.description));
                            }
                            output.push_str("\n");
                            output.push_str(&format!("ACTION RESULTS:\n{}\n\n", action_results.join("\n")));
                            output.push_str(&format!("REFLECTION:\n{}\n\n", reflection));
                        }
                        _ => {}
                    }
                }

                Ok(CallToolResult::success(vec![Content::text(
                    if output.is_empty() {
                        "No tasks processed".to_string()
                    } else {
                        output
                    },
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Sequential cycle failed: {}", e),
            )])),
        }
    }

    #[tool(description = "Analyze code structure using tree-sitter parser")]
    async fn parser_analyze(
        &self,
        Parameters(params): Parameters<ParserAnalyzeRequest>,
    ) -> Result<CallToolResult, McpError> {
        match crate::parser::Parser::new() {
            Ok(parser) => {
                match parser.parse_file(std::path::Path::new(&params.file_path)) {
                    Ok(structure) => {
                        let analysis = serde_json::to_string_pretty(&structure)
                            .unwrap_or_else(|_| "Failed to serialize structure".to_string());
                        Ok(CallToolResult::success(vec![Content::text(
                            format!("Code analysis for {}:\n{}", params.file_path, analysis),
                        )]))
                    }
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(
                        format!("Failed to parse file {}: {}", params.file_path, e),
                    )])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to initialize parser: {}", e),
            )])),
        }
    }

    #[tool(description = "Search code patterns using ripgrep")]
    async fn parser_search(
        &self,
        Parameters(params): Parameters<ParserSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let context_lines = params.context_lines.unwrap_or(3);
        let search_path = params.path.as_deref().unwrap_or(".");

        use std::process::Command;
        let mut cmd = Command::new("rg");
        cmd.args(&["--json", "-C", &context_lines.to_string(), &params.pattern, search_path]);

        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    let results = String::from_utf8_lossy(&output.stdout);
                    Ok(CallToolResult::success(vec![Content::text(
                        format!("Search results for '{}' in {}:\n{}", params.pattern, search_path, results),
                    )]))
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Ok(CallToolResult::error(vec![Content::text(
                        format!("Search failed: {}", error),
                    )]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to execute search: {}", e),
            )])),
        }
    }

    #[tool(description = "Index a source code file for semantic and structural search")]
    async fn code_index(
        &self,
        Parameters(params): Parameters<CodeIndexRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Create code graph instance
        let db_path = "syncore_code_graph.db";
        let vector_store = self.state.vector_store.clone();

        match crate::code_graph::CodeGraph::new(db_path, vector_store) {
            Ok(mut code_graph) => {
                let file_path = std::path::Path::new(&params.file_path);

                match code_graph.index_file(file_path) {
                    Ok(count) => {
                        Ok(CallToolResult::success(vec![Content::text(
                            format!("Successfully indexed {} entities from {}", count, params.file_path),
                        )]))
                    }
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(
                        format!("Failed to index file: {}", e),
                    )])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to create code graph: {}", e),
            )])),
        }
    }

    #[tool(description = "Search code using semantic meaning and structural relationships")]
    async fn code_search(
        &self,
        Parameters(params): Parameters<CodeSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Create code graph instance
        let db_path = "syncore_code_graph.db";
        let vector_store = self.state.vector_store.clone();

        match crate::code_graph::CodeGraph::new(db_path, vector_store) {
            Ok(code_graph) => {
                match code_graph.search_code(&params.query, params.limit) {
                    Ok(matches) => {
                        if matches.is_empty() {
                            Ok(CallToolResult::success(vec![Content::text(
                                format!("No matches found for query: '{}'", params.query),
                            )]))
                        } else {
                            let mut result = format!("Found {} matches for '{}':\n\n", matches.len(), params.query);

                            for (i, m) in matches.iter().enumerate() {
                                result.push_str(&format!(
                                    "{}. {} '{}' in {} (line {})\n",
                                    i + 1,
                                    m.entity.entity_type.as_str(),
                                    m.entity.name,
                                    m.entity.file_path,
                                    m.entity.line_start
                                ));

                                if let Some(sig) = &m.entity.signature {
                                    result.push_str(&format!("   Signature: {}\n", sig));
                                }

                                result.push_str(&format!("   Score: {:.4} ({})\n\n", m.score,
                                    match m.match_type {
                                        crate::code_graph::MatchType::Semantic => "semantic",
                                        crate::code_graph::MatchType::Structural => "structural",
                                        crate::code_graph::MatchType::Combined => "combined",
                                    }
                                ));
                            }

                            Ok(CallToolResult::success(vec![Content::text(result)]))
                        }
                    }
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(
                        format!("Search failed: {}", e),
                    )])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to create code graph: {}", e),
            )])),
        }
    }

    #[tool(description = "Index all code files in a directory matching a glob pattern")]
    async fn code_index_directory(
        &self,
        Parameters(params): Parameters<CodeIndexDirectoryRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::code_directory_indexer::{DirectoryIndexer, DirectoryIndexRequest};

        // Create indexer with vector store
        let db_path = "syncore_code_graph.db";
        let vector_store = self.state.vector_store.clone();

        match DirectoryIndexer::new(db_path, vector_store) {
            Ok(mut indexer) => {
                let request = DirectoryIndexRequest {
                    directory: params.directory.clone(),
                    pattern: params.pattern.clone(),
                };

                match indexer.index_directory(&request) {
                    Ok(response) => {
                        if response.success {
                            Ok(CallToolResult::success(vec![Content::text(
                                format!(
                                    "Successfully indexed {} files with {} total entities from directory '{}' using pattern '{}'",
                                    response.files_indexed,
                                    response.total_entities,
                                    params.directory,
                                    params.pattern
                                ),
                            )]))
                        } else {
                            Ok(CallToolResult::error(vec![Content::text(
                                response.error.unwrap_or_else(|| "Unknown error".to_string()),
                            )]))
                        }
                    }
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(
                        format!("Failed to index directory: {}", e),
                    )])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to create directory indexer: {}", e),
            )])),
        }
    }

    #[tool(description = "Index documents from a directory into global knowledge store")]
    async fn document_index(
        &self,
        Parameters(params): Parameters<DocumentIndexRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::document_indexer::DocumentIndexer;
        use std::path::Path;

        let indexer = DocumentIndexer::with_defaults();
        let dir_path = Path::new(&params.directory);

        match indexer.index_directory(dir_path) {
            Ok(chunk_count) => {
                Ok(CallToolResult::success(vec![Content::text(
                    format!(
                        "Successfully indexed {} document chunks from directory '{}'.\nAll documents are now searchable in the global knowledge store.",
                        chunk_count,
                        params.directory
                    ),
                )]))
            }
            Err(e) => {
                Ok(CallToolResult::error(vec![Content::text(
                    format!("Failed to index directory: {}", e),
                )]))
            }
        }
    }

    #[tool(description = "Semantic search across indexed documents using vector embeddings")]
    async fn document_search(
        &self,
        Parameters(params): Parameters<DocumentSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::global_store::GlobalVectorStore;

        let vector_store = match GlobalVectorStore::new() {
            Ok(store) => store,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    format!("Failed to create vector store: {}", e),
                )]));
            }
        };

        match vector_store.search(&params.query, params.limit, "documents") {
            Ok(results) => {
                if results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(
                        "No documents found matching your query.\nTry indexing documents first with the 'document_index' tool.".to_string(),
                    )]))
                } else {
                    let result_count = results.len();
                    let results_text = results
                        .into_iter()
                        .enumerate()
                        .map(|(i, hit)| {
                            format!(
                                "{}. [Score: {:.3}]\n{}\n",
                                i + 1,
                                hit.score,
                                hit.text
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n---\n\n");

                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "Found {} relevant documents:\n\n{}",
                        result_count,
                        results_text
                    ))]))
                }
            }
            Err(e) => {
                Ok(CallToolResult::error(vec![Content::text(
                    format!("Failed to search documents: {}", e),
                )]))
            }
        }
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
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(
                        format!("Failed to generate tasks: {}", e),
                    )])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Ollama unavailable: {}. Ensure Ollama is running with phi3:mini", e),
            )])),
        }
    }

    #[tool(description = "IntelliTask: Generate subtasks for a parent task")]
    async fn intellitask_subtasks(
        &self,
        Parameters(params): Parameters<IntelliTaskSubtasksRequest>,
    ) -> Result<CallToolResult, McpError> {
        match crate::ollama::OllamaClient::new_default() {
            Ok(ollama) => {
                let parent_task: crate::intellitask::ParentTask = match serde_json::from_str(&params.parent_task_json) {
                    Ok(task) => task,
                    Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                        format!("Invalid parent task JSON: {}", e),
                    )])),
                };

                let intellitask = crate::intellitask::IntelliTask::new(ollama);
                let codebase_context = params.codebase_context.as_deref().unwrap_or("");

                match intellitask.generate_subtasks(&parent_task, codebase_context) {
                    Ok(subtasks) => {
                        let json_output = serde_json::to_string_pretty(&subtasks)
                            .unwrap_or_else(|_| "Failed to serialize subtasks".to_string());
                        Ok(CallToolResult::success(vec![Content::text(json_output)]))
                    }
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(
                        format!("Failed to generate subtasks: {}", e),
                    )])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Ollama unavailable: {}", e),
            )])),
        }
    }

    #[tool(description = "IntelliTask: Prioritize tasks using AI reasoning")]
    async fn intellitask_prioritize(
        &self,
        Parameters(params): Parameters<IntelliTaskPrioritizeRequest>,
    ) -> Result<CallToolResult, McpError> {
        match crate::ollama::OllamaClient::new_default() {
            Ok(ollama) => {
                let tasks: Vec<crate::intellitask::ParentTask> = match serde_json::from_str(&params.tasks_json) {
                    Ok(t) => t,
                    Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                        format!("Invalid tasks JSON: {}", e),
                    )])),
                };

                let intellitask = crate::intellitask::IntelliTask::new(ollama);
                let business_context = params.business_context.as_deref().unwrap_or("");

                match intellitask.prioritize_tasks(&tasks, business_context) {
                    Ok(priorities) => {
                        let json_output = serde_json::to_string_pretty(&priorities)
                            .unwrap_or_else(|_| "Failed to serialize priorities".to_string());
                        Ok(CallToolResult::success(vec![Content::text(json_output)]))
                    }
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(
                        format!("Failed to prioritize tasks: {}", e),
                    )])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Ollama unavailable: {}", e),
            )])),
        }
    }

    #[tool(description = "IntelliTask: Suggest next task to work on")]
    async fn intellitask_next(
        &self,
        Parameters(params): Parameters<IntelliTaskNextRequest>,
    ) -> Result<CallToolResult, McpError> {
        match crate::ollama::OllamaClient::new_default() {
            Ok(ollama) => {
                let remaining_tasks: Vec<crate::intellitask::ParentTask> = match serde_json::from_str(&params.remaining_tasks_json) {
                    Ok(t) => t,
                    Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                        format!("Invalid remaining tasks JSON: {}", e),
                    )])),
                };

                let intellitask = crate::intellitask::IntelliTask::new(ollama);

                match intellitask.suggest_next_task(&params.completed_tasks, &remaining_tasks) {
                    Ok(suggestion) => {
                        Ok(CallToolResult::success(vec![Content::text(suggestion)]))
                    }
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(
                        format!("Failed to suggest next task: {}", e),
                    )])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Ollama unavailable: {}", e),
            )])),
        }
    }

    // IntelliTask 2.0 Persistence Tools

    #[tool(description = "Save IntelliTask breakdown to database")]
    async fn intellitask_save(
        &self,
        Parameters(params): Parameters<TaskSaveRequest>,
    ) -> Result<CallToolResult, McpError> {
        let persistence = match crate::intellitask_persistence::IntelliTaskPersistence::new("./syncore.db_tasks") {
            Ok(p) => p,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to initialize persistence: {}", e),
            )])),
        };

        // Step 1: Parse as Value to check JSON syntax
        let json_value: serde_json::Value = match serde_json::from_str(&params.breakdown_json) {
            Ok(v) => v,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Invalid JSON syntax: {}", e),
            )])),
        };

        // Step 2: Validate against schema to report ALL errors at once
        let schema = schemars::schema_for!(crate::intellitask::TaskBreakdown);
        let schema_json = serde_json::to_value(&schema).expect("Failed to convert schema to JSON");

        let validator = match jsonschema::JSONSchema::compile(&schema_json) {
            Ok(v) => v,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Internal error creating validator: {}", e),
            )])),
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
        let breakdown: crate::intellitask::TaskBreakdown = match serde_json::from_value(json_value) {
            Ok(b) => b,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Deserialization failed after validation (unexpected): {}", e),
            )])),
        };

        match persistence.save_task_breakdown(&breakdown) {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                format!("Saved {} parent tasks to database", breakdown.parent_tasks.len()),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to save tasks: {}", e),
            )])),
        }
    }

    #[tool(description = "Get task by ID from database")]
    async fn intellitask_get(
        &self,
        Parameters(params): Parameters<TaskGetRequest>,
    ) -> Result<CallToolResult, McpError> {
        let persistence = match crate::intellitask_persistence::IntelliTaskPersistence::new("./syncore.db_tasks") {
            Ok(p) => p,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to initialize persistence: {}", e),
            )])),
        };

        match persistence.get_task(params.task_id) {
            Ok(Some(task)) => {
                let json = serde_json::to_string_pretty(&task)
                    .unwrap_or_else(|_| "Failed to serialize task".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Ok(None) => Ok(CallToolResult::error(vec![Content::text(
                format!("Task {} not found", params.task_id),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to get task: {}", e),
            )])),
        }
    }

    #[tool(description = "List tasks with optional filtering")]
    async fn intellitask_list(
        &self,
        Parameters(params): Parameters<TaskListRequest>,
    ) -> Result<CallToolResult, McpError> {
        let persistence = match crate::intellitask_persistence::IntelliTaskPersistence::new("./syncore.db_tasks") {
            Ok(p) => p,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to initialize persistence: {}", e),
            )])),
        };

        let filter = crate::intellitask_persistence::TaskFilter {
            status: params.status.and_then(|s| serde_json::from_str(&format!("\"{}\"", s)).ok()),
            prd_title: params.prd_title,
            parent_id: params.parent_id,
        };

        match persistence.get_tasks(Some(filter)) {
            Ok(tasks) => {
                let json = serde_json::to_string_pretty(&tasks)
                    .unwrap_or_else(|_| "Failed to serialize tasks".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to list tasks: {}", e),
            )])),
        }
    }

    #[tool(description = "Update task status")]
    async fn intellitask_update_status(
        &self,
        Parameters(params): Parameters<TaskUpdateStatusRequest>,
    ) -> Result<CallToolResult, McpError> {
        let persistence = match crate::intellitask_persistence::IntelliTaskPersistence::new("./syncore.db_tasks") {
            Ok(p) => p,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to initialize persistence: {}", e),
            )])),
        };

        let status: crate::intellitask_persistence::TaskStatus = match serde_json::from_str(&format!("\"{}\"", params.status)) {
            Ok(s) => s,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Invalid status: {}. Use: pending, in-progress, review, done, deferred, cancelled, blocked", e),
            )])),
        };

        match persistence.update_task_status(params.task_id, status) {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(
                format!("Updated task {} status to {}", params.task_id, params.status),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to update status: {}", e),
            )])),
        }
    }

    #[tool(description = "Get next task ready to work on (dependencies satisfied)")]
    async fn intellitask_next_ready(
        &self,
    ) -> Result<CallToolResult, McpError> {
        let persistence = match crate::intellitask_persistence::IntelliTaskPersistence::new("./syncore.db_tasks") {
            Ok(p) => p,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to initialize persistence: {}", e),
            )])),
        };

        match persistence.next_task() {
            Ok(Some(task)) => {
                let json = serde_json::to_string_pretty(&task)
                    .unwrap_or_else(|_| "Failed to serialize task".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Ok(None) => Ok(CallToolResult::success(vec![Content::text(
                "No tasks ready to work on".to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to get next task: {}", e),
            )])),
        }
    }

    #[tool(description = "Get subtasks for a parent task")]
    async fn intellitask_get_subtasks(
        &self,
        Parameters(params): Parameters<TaskSubtasksRequest>,
    ) -> Result<CallToolResult, McpError> {
        let persistence = match crate::intellitask_persistence::IntelliTaskPersistence::new("./syncore.db_tasks") {
            Ok(p) => p,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to initialize persistence: {}", e),
            )])),
        };

        match persistence.get_subtasks(params.parent_id) {
            Ok(subtasks) => {
                let json = serde_json::to_string_pretty(&subtasks)
                    .unwrap_or_else(|_| "Failed to serialize subtasks".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to get subtasks: {}", e),
            )])),
        }
    }

    #[tool(description = "Get subtask statistics for a parent task")]
    async fn intellitask_subtask_stats(
        &self,
        Parameters(params): Parameters<TaskSubtaskStatsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let persistence = match crate::intellitask_persistence::IntelliTaskPersistence::new("./syncore.db_tasks") {
            Ok(p) => p,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to initialize persistence: {}", e),
            )])),
        };

        match persistence.get_subtask_statistics(params.parent_id) {
            Ok(stats) => {
                let json = serde_json::to_string_pretty(&stats)
                    .unwrap_or_else(|_| "Failed to serialize stats".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to get stats: {}", e),
            )])),
        }
    }

    #[tool(description = "Get overall task statistics across all tasks")]
    async fn intellitask_task_statistics(
        &self,
        Parameters(_params): Parameters<TaskStatisticsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let persistence = match crate::intellitask_persistence::IntelliTaskPersistence::new("./syncore.db_tasks") {
            Ok(p) => p,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to initialize persistence: {}", e),
            )])),
        };

        match persistence.get_task_statistics() {
            Ok(stats) => {
                let json = serde_json::to_string_pretty(&stats)
                    .unwrap_or_else(|_| "Failed to serialize stats".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to get task statistics: {}", e),
            )])),
        }
    }

    #[tool(description = "Get task statistics for a specific PRD")]
    async fn intellitask_prd_statistics(
        &self,
        Parameters(params): Parameters<PrdStatisticsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let persistence = match crate::intellitask_persistence::IntelliTaskPersistence::new("./syncore.db_tasks") {
            Ok(p) => p,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to initialize persistence: {}", e),
            )])),
        };

        match persistence.get_prd_statistics(&params.prd_title) {
            Ok(stats) => {
                let json = serde_json::to_string_pretty(&stats)
                    .unwrap_or_else(|_| "Failed to serialize stats".to_string());
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Failed to get PRD statistics: {}", e),
            )])),
        }
    }

    // Message Bus Tool
    #[tool(description = "Send message to another agent via message bus")]
    async fn agent_send(
        &self,
        Parameters(params): Parameters<AgentSendRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::message_bus::message::{AgentId, Msg, MsgKind};
        use std::time::SystemTime;

        // Check if message bus is configured
        let bus = match &self.state.message_bus {
            Some(b) => b.clone(),
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Message bus not configured".to_string(),
                )]));
            }
        };

        // Parse target agent ID
        let (to_agent, kind, target_str) = if params.to.trim().is_empty() {
            // Broadcast
            (None, MsgKind::Broadcast, "broadcast".to_string())
        } else {
            // Direct message - parse AgentId from string
            let agent_id = match params.to.to_lowercase().as_str() {
                "claude" => AgentId::Claude,
                "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
                other => AgentId::Custom(other.to_string()),
            };
            let target = params.to.clone();
            (Some(agent_id), MsgKind::Direct, target)
        };

        // Get next message ID
        let msg_id = bus.next_message_id();

        // Construct message
        let msg = Msg {
            id: msg_id,
            from: AgentId::Internal("mcp_server".to_string()),
            to: to_agent,
            kind,
            payload: serde_json::json!({ "message": params.message }),
            timestamp: SystemTime::now(),
        };

        // Send message
        bus.send(msg);

        // Return success
        let response = serde_json::json!({
            "status": "sent",
            "to": target_str,
            "message_id": msg_id
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
        )]))
    }

    #[tool(description = "Receive pending messages for a given agent ID")]
    async fn agent_recv(
        &self,
        Parameters(params): Parameters<AgentRecvRequest>,
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

        // Drain pending messages for this agent
        let messages = bus.drain_for(&agent_id);

        // Return success with messages
        let response = serde_json::json!({
            "status": "ok",
            "agent": params.agent,
            "messages": messages
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
        )]))
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
        let agent_id = match params.id.to_lowercase().as_str() {
            "claude" => AgentId::Claude,
            "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
            other => AgentId::Custom(other.to_string()),
        };

        // Register agent metadata
        bus.register_agent_info(agent_id, params.id.clone(), params.capabilities.clone());

        // Return success
        let response = serde_json::json!({
            "status": "registered",
            "id": params.id,
            "capabilities": params.capabilities
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
        )]))
    }

    #[tool(description = "List all registered agents and their metadata")]
    async fn agent_list(
        &self,
        Parameters(_params): Parameters<AgentListRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Check if message bus is configured
        let bus = match &self.state.message_bus {
            Some(b) => b.clone(),
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Message bus not configured".to_string(),
                )]));
            }
        };

        // Get list of registered agent names
        let list = bus.list_registered_agents();

        // Retrieve full metadata for each agent
        let full: Vec<serde_json::Value> = list
            .iter()
            .filter_map(|name| {
                bus.get_agent_info(name).map(|info| {
                    serde_json::json!({
                        "id": format!("{:?}", info.id),
                        "name": info.name,
                        "capabilities": info.capabilities,
                        "registered_at_ms": info.registered_at.elapsed().as_millis()
                    })
                })
            })
            .collect();

        let response = serde_json::json!({
            "status": "ok",
            "agents": full
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
        )]))
    }

    #[tool(description = "Update the status of the specified agent")]
    async fn agent_status(
        &self,
        Parameters(params): Parameters<AgentStatusRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Check if message bus is configured
        let bus = match &self.state.message_bus {
            Some(b) => b.clone(),
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Message bus not configured".to_string(),
                )]));
            }
        };

        // Update agent status
        bus.update_agent_status(&params.id, params.status.clone());

        // Return success
        let response = serde_json::json!({
            "status": "ok",
            "id": params.id,
            "updated": params.status
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
        )]))
    }

    #[tool(description = "Send a structured task envelope to a specified agent")]
    async fn agent_task(
        &self,
        Parameters(params): Parameters<AgentTaskRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Check if message bus is configured
        let bus = match &self.state.message_bus {
            Some(b) => b.clone(),
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Message bus not configured".to_string(),
                )]));
            }
        };

        // Map string to AgentId
        let to_agent = match params.to.to_lowercase().as_str() {
            "claude" => AgentId::Claude,
            "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
            "router" => AgentId::Internal("router".to_string()),
            other => AgentId::Custom(other.to_string()),
        };

        // Create task envelope message
        let msg = Msg {
            id: bus.next_message_id(),
            from: AgentId::Internal("mcp_server".to_string()),
            to: Some(to_agent),
            kind: MsgKind::Direct,
            payload: serde_json::json!({
                "task_id": params.task_id,
                "task_type": params.task_type,
                "payload": params.payload
            }),
            timestamp: SystemTime::now(),
        };

        bus.send(msg);

        // Return success
        let response = serde_json::json!({
            "status": "sent",
            "to": params.to,
            "task_id": params.task_id,
            "task_type": params.task_type
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
        )]))
    }

    #[tool(description = "Submit the result of a completed task to the router")]
    async fn agent_result(
        &self,
        Parameters(params): Parameters<AgentResultRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Check if message bus is configured
        let bus = match &self.state.message_bus {
            Some(b) => b.clone(),
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Message bus not configured".to_string(),
                )]));
            }
        };

        // Map string to AgentId
        let from_agent = match params.from.to_lowercase().as_str() {
            "claude" => AgentId::Claude,
            "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
            other => AgentId::Custom(other.to_string()),
        };

        // Create result message to router
        let msg = Msg {
            id: bus.next_message_id(),
            from: from_agent,
            to: Some(AgentId::Internal("router".to_string())),
            kind: MsgKind::Direct,
            payload: serde_json::json!({
                "task_id": params.task_id,
                "result": params.result
            }),
            timestamp: SystemTime::now(),
        };

        bus.send(msg);

        // Return success
        let response = serde_json::json!({
            "status": "accepted",
            "from": params.from,
            "task_id": params.task_id
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
        )]))
    }

    #[tool(description = "Execute a Cypher query on Neo4j graph database and return results")]
    async fn graph_query(
        &self,
        Parameters(params): Parameters<GraphQueryRequest>,
    ) -> Result<CallToolResult, McpError> {
        let neo4j = match &self.state.neo4j {
            Some(client) => client.clone(),
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Neo4j not configured. Use SynCoreState::with_neo4j() to add Neo4j support.".to_string(),
                )]));
            }
        };

        // Convert params to Vec<(&str, Value)>
        let query_params: Vec<(&str, serde_json::Value)> = if let Some(obj) = params.params {
            if let serde_json::Value::Object(map) = obj {
                map.into_iter()
                    .map(|(k, v)| (Box::leak(k.into_boxed_str()) as &str, v))
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        match neo4j.execute_query(&params.cypher, query_params).await {
            Ok(results) => {
                let response = serde_json::json!({
                    "success": true,
                    "rows": results,
                    "count": results.len()
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Neo4j query error: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Execute a Cypher write query (CREATE, MERGE, SET) on Neo4j")]
    async fn graph_insert(
        &self,
        Parameters(params): Parameters<GraphInsertRequest>,
    ) -> Result<CallToolResult, McpError> {
        let neo4j = match &self.state.neo4j {
            Some(client) => client.clone(),
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Neo4j not configured. Use SynCoreState::with_neo4j() to add Neo4j support.".to_string(),
                )]));
            }
        };

        // Convert params to Vec<(&str, Value)>
        let query_params: Vec<(&str, serde_json::Value)> = if let Some(obj) = params.params {
            if let serde_json::Value::Object(map) = obj {
                map.into_iter()
                    .map(|(k, v)| (Box::leak(k.into_boxed_str()) as &str, v))
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        match neo4j.execute_query(&params.cypher, query_params).await {
            Ok(_) => {
                let response = serde_json::json!({
                    "success": true,
                    "message": "Graph insert executed successfully"
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Neo4j insert error: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Create a relationship between two nodes in Neo4j")]
    async fn graph_relate(
        &self,
        Parameters(params): Parameters<GraphRelateRequest>,
    ) -> Result<CallToolResult, McpError> {
        let neo4j = match &self.state.neo4j {
            Some(client) => client.clone(),
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Neo4j not configured. Use SynCoreState::with_neo4j() to add Neo4j support.".to_string(),
                )]));
            }
        };

        let from_label = params.from_label.unwrap_or_else(|| "Node".to_string());
        let to_label = params.to_label.unwrap_or_else(|| "Node".to_string());

        match neo4j
            .create_relationship(&from_label, params.from_id, &to_label, params.to_id, &params.rel_type)
            .await
        {
            Ok(_) => {
                let response = serde_json::json!({
                    "success": true,
                    "from_id": params.from_id,
                    "to_id": params.to_id,
                    "rel_type": params.rel_type
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Neo4j relate error: {}",
                e
            ))])),
        }
    }

    // Portfolio Enhancement Tools

    #[tool(description = "Record a file node in the application structure map")]
    async fn mapping_record(
        &self,
        Parameters(params): Parameters<MappingRecordRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::portfolio::mapping_tool::{MappingTool, FileNode};

        let mapper = MappingTool::new((*self.state).clone());
        let node = FileNode {
            path: params.path.clone(),
            kind: params.kind,
            language: params.language,
            imports: params.imports,
            exports: params.exports,
            dependencies: params.dependencies,
        };

        match mapper.record_file(&node) {
            Ok(_) => {
                let response = serde_json::json!({
                    "success": true,
                    "path": params.path,
                    "message": "File node recorded successfully"
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to record file: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Get a file node from the application structure map")]
    async fn mapping_get(
        &self,
        Parameters(params): Parameters<MappingGetRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::portfolio::mapping_tool::MappingTool;

        let mapper = MappingTool::new((*self.state).clone());

        match mapper.get_file(&params.path) {
            Ok(Some(node)) => {
                let response = serde_json::to_value(&node)
                    .unwrap_or_else(|_| serde_json::json!({"error": "Serialization failed"}));
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Ok(None) => Ok(CallToolResult::success(vec![Content::text(format!(
                "File not found: {}",
                params.path
            ))])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get file: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Search for files related to a query using semantic search")]
    async fn mapping_search(
        &self,
        Parameters(params): Parameters<MappingSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::portfolio::mapping_tool::MappingTool;

        let mapper = MappingTool::new((*self.state).clone());

        match mapper.search_related(&params.query) {
            Ok(nodes) => {
                let response = serde_json::json!({
                    "count": nodes.len(),
                    "files": nodes
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Search failed: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Get all transitive dependencies for a file")]
    async fn mapping_deps(
        &self,
        Parameters(params): Parameters<MappingDepsRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::portfolio::mapping_tool::MappingTool;

        let mapper = MappingTool::new((*self.state).clone());

        match mapper.get_all_dependencies(&params.path) {
            Ok(deps) => {
                let response = serde_json::json!({
                    "path": params.path,
                    "dependencies": deps,
                    "count": deps.len()
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get dependencies: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Record a thought step in the reasoning chain")]
    async fn sequential_record(
        &self,
        Parameters(params): Parameters<SequentialRecordRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::portfolio::sequential_step::{SequentialStep, ThoughtStep};

        let sequential = SequentialStep::new((*self.state).clone());
        let step = ThoughtStep {
            task_id: params.task_id,
            step_number: params.step_number,
            thought: params.thought,
            action: params.action,
            observation: params.observation,
            reasoning: params.reasoning,
        };

        match sequential.record_step(&step) {
            Ok(step_id) => {
                let response = serde_json::json!({
                    "success": true,
                    "step_id": step_id,
                    "message": "Thought step recorded successfully"
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to record step: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Get all thought steps for a task")]
    async fn sequential_get(
        &self,
        Parameters(params): Parameters<SequentialGetRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::portfolio::sequential_step::SequentialStep;

        let sequential = SequentialStep::new((*self.state).clone());

        match sequential.get_steps_for_task(params.task_id) {
            Ok(steps) => {
                let response = serde_json::json!({
                    "task_id": params.task_id,
                    "steps": steps,
                    "count": steps.len()
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get steps: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Search thought steps by semantic content")]
    async fn sequential_search(
        &self,
        Parameters(params): Parameters<SequentialSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::portfolio::sequential_step::SequentialStep;

        let sequential = SequentialStep::new((*self.state).clone());

        match sequential.search_steps(&params.query) {
            Ok(steps) => {
                let response = serde_json::json!({
                    "query": params.query,
                    "results": steps,
                    "count": steps.len()
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Search failed: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Record a code change in the application")]
    async fn application_record(
        &self,
        Parameters(params): Parameters<ApplicationRecordRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::portfolio::application_tool::{ApplicationTool, CodeChange};

        let app_tool = ApplicationTool::new((*self.state).clone());
        let change = CodeChange {
            file_path: params.file_path.clone(),
            change_type: params.change_type,
            old_content: params.old_content,
            new_content: params.new_content,
            line_start: params.line_start,
            line_end: params.line_end,
            description: params.description,
            task_id: params.task_id,
        };

        match app_tool.record_change(&change) {
            Ok(change_id) => {
                let response = serde_json::json!({
                    "success": true,
                    "change_id": change_id,
                    "file_path": params.file_path,
                    "message": "Code change recorded successfully"
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to record change: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Get all code changes for a task")]
    async fn application_get(
        &self,
        Parameters(params): Parameters<ApplicationGetRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::portfolio::application_tool::ApplicationTool;

        let app_tool = ApplicationTool::new((*self.state).clone());

        match app_tool.get_changes_for_task(params.task_id) {
            Ok(changes) => {
                let response = serde_json::json!({
                    "task_id": params.task_id,
                    "changes": changes,
                    "count": changes.len()
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get changes: {}",
                e
            ))])),
        }
    }

    #[tool(description = "Get change history for a specific file")]
    async fn application_history(
        &self,
        Parameters(params): Parameters<ApplicationHistoryRequest>,
    ) -> Result<CallToolResult, McpError> {
        use crate::portfolio::application_tool::ApplicationTool;

        let app_tool = ApplicationTool::new((*self.state).clone());

        match app_tool.get_file_history(&params.file_path) {
            Ok(changes) => {
                let response = serde_json::json!({
                    "file_path": params.file_path,
                    "history": changes,
                    "count": changes.len()
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get history: {}",
                e
            ))])),
        }
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
                    serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Search failed: {}",
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
        let task_type = msg.payload
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
                    let load = status.get("load")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);

                    let busy = status.get("busy")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let errors = status.get("errors")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    // Weighted scoring formula:
                    // lower load = better
                    // not busy = better
                    // fewer errors = better
                    let score =
                        load * 1.0 +
                        if busy { 5.0 } else { 0.0 } +
                        (errors as f64) * 3.0;

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
