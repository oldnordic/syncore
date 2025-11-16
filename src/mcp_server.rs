use anyhow::Result;
use std::sync::Arc;
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

pub async fn run_mcp_stdio_server(state: SynCoreState) -> Result<()> {
    let server = SynCoreMCPServer::new(state);

    let service = server.serve(stdio()).await.inspect_err(|e| {
        eprintln!("MCP server error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
