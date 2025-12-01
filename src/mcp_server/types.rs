//! MCP Tool Request Type Definitions
//!
//! This module contains all request parameter structures used by the MCP protocol.
//! Each struct represents the parameters for a specific MCP tool.

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemoryStoreRequest {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub dry_run: bool,
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
    #[serde(default)]
    pub dry_run: bool,
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
    /// If true, persist entities to SQLite, update HNSW index, and sync to Neo4j
    #[serde(default)]
    pub persist: bool,
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

// STEP D: Explain Function Tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExplainFunctionRequest {
    /// Name of the function to explain
    pub function_name: String,
    /// Path to the file containing the function
    pub file_path: String,
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

// RagGraph Tools
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RagGraphQueryRequest {
    pub query_text: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RagGraphMultihopRequest {
    pub seed_nodes: Vec<i64>,
}

// CodeGraph Neo4j Sync Tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CodeGraphSyncNeo4jRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
}

// CodeGraph Temporal Enrichment Tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CodeGraphEnrichTemporalRequest {
    /// Optional limit on number of entities to enrich (default: all)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    /// Whether to only enrich entities that don't have temporal data
    #[serde(default = "default_only_missing")]
    pub only_missing: bool,
}

fn default_only_missing() -> bool {
    true
}

// Project Analysis Engine (PAE) Tools

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectFileReportRequest {
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectModuleMapRequest {
    pub root: Option<String>,
    pub max_modules: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectHotspotsRequest {
    pub limit: u32,
    pub min_fan_in: Option<u32>,
    pub min_fan_out: Option<u32>,
    pub min_entity_count: Option<u32>,
    pub min_loc: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectCyclesRequest {
    pub max_cycles: u32,
    pub max_depth: u32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectDeadCodeRequest {
    pub exclude_public: Option<bool>,
    pub limit: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectUnusedImportsRequest {
    pub file_path: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectCodeSmellsRequest {
    pub limit: Option<u32>,
    pub include_entities: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectRefactorSuggestionsRequest {
    pub limit: u32,
    pub loc_threshold: Option<u32>,
    pub entity_threshold: Option<u32>,
    pub fan_in_threshold: Option<u32>,
    pub fan_out_threshold: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectCleanupExcludedRequest {
    /// If true, only report what would be deleted without actually deleting
    #[serde(default)]
    pub dry_run: bool,
    /// Override excluded directories (uses config if not provided)
    pub excluded_dirs: Option<Vec<String>>,
}

// APEX v1.3 Suite Tools
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemorySuiteRequest {
    /// Command to execute: store, query, vector_insert, vector_search, task_create, sequential_record, sequential_get, sequential_search, sequential_cycle, agent_send, agent_recv, agent_poll, agent_register, agent_list, agent_status, agent_task, agent_result, intellitask_generate, intellitask_subtasks, intellitask_prioritize, intellitask_next, intellitask_save, intellitask_get, intellitask_list, intellitask_update_status, intellitask_next_ready, intellitask_get_subtasks, intellitask_subtask_stats, intellitask_task_statistics, intellitask_prd_statistics, help
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
    // Common
    #[serde(default)]
    pub dry_run: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CodeSuiteRequest {
    /// Command to execute: index, index_directory, search, parse, grep, doc_index, doc_search, explain, help
    pub command: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub directory: Option<String>,
    #[serde(default)]
    pub context_lines: Option<usize>,
    #[serde(default)]
    pub function_name: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GraphSuiteRequest {
    /// Command to execute: query, insert, relate, help
    pub command: String,
    #[serde(default)]
    pub cypher: Option<String>,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    #[serde(default)]
    pub from_id: Option<i64>,
    #[serde(default)]
    pub to_id: Option<i64>,
    #[serde(default)]
    pub rel_type: Option<String>,
    #[serde(default)]
    pub from_label: Option<String>,
    #[serde(default)]
    pub to_label: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MappingSuiteRequest {
    /// Command to execute: record, get, search, deps, help
    pub command: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub imports: Option<Vec<String>>,
    #[serde(default)]
    pub exports: Option<Vec<String>>,
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
    #[serde(default)]
    pub query: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DebugSuiteRequest {
    /// Command to execute: logs_tail, tool_metadata_list, project_file_report, project_module_map, project_hotspots, project_cycles, project_dead_code, project_unused_imports, project_refactor_suggestions, help
    pub command: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub n: Option<usize>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub max_modules: Option<u32>,
    #[serde(default)]
    pub max_cycles: Option<u32>,
    #[serde(default)]
    pub max_depth: Option<u32>,
    #[serde(default)]
    pub min_loc: Option<u32>,
    #[serde(default)]
    pub min_fan_in: Option<u32>,
    #[serde(default)]
    pub min_fan_out: Option<u32>,
    #[serde(default)]
    pub min_entity_count: Option<u32>,
    #[serde(default)]
    pub exclude_public: Option<bool>,
    #[serde(default)]
    pub loc_threshold: Option<u32>,
    #[serde(default)]
    pub fan_in_threshold: Option<u32>,
    #[serde(default)]
    pub fan_out_threshold: Option<u32>,
    #[serde(default)]
    pub entity_threshold: Option<u32>,
}

/// APEX 1.8 REFRAG Suite request
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefragSuiteRequest {
    /// Command to execute: query, configure, help
    pub command: String,
    // Query parameters
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub top_k_raw: Option<usize>,
    #[serde(default)]
    pub max_tokens: Option<usize>,
    #[serde(default)]
    pub policy: Option<String>,
}
