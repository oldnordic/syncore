//! MCP Protocol Types and JSON-RPC Handling
//!
//! Original protocol implementation moved from src/mcp.rs

pub use crate::router::SynCoreState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: String,
    pub output_schema: String,
}

#[derive(Debug, Deserialize)]
pub struct MCPRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Value,
}

#[derive(Debug, Serialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<MCPError>,
    pub id: Value,
}

#[derive(Debug, Serialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
}

pub async fn list_tools() -> Vec<ToolInfo> {
    vec![
        // Document Suite
        ToolInfo {
            name: "document_search".into(),
            description: "Search documents by semantic similarity".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "document_index".into(),
            description: "Index documents from a directory".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        // Vector Suite
        ToolInfo {
            name: "vector_insert".into(),
            description: "Insert text into vector store with embeddings".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "vector_search".into(),
            description: "Search vector store by semantic similarity".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        // Code Suite
        ToolInfo {
            name: "code_search".into(),
            description: "Search code by semantic meaning".into(),
            input_schema: "schemas/code_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "code_index".into(),
            description: "Index a code file for semantic search".into(),
            input_schema: "schemas/code_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "code_index_directory".into(),
            description: "Index all code files in a directory".into(),
            input_schema: "schemas/code_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        // Graph Suite
        ToolInfo {
            name: "graph_query".into(),
            description: "Execute Cypher read query on Neo4j".into(),
            input_schema: "schemas/graph_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "graph_insert".into(),
            description: "Execute Cypher write query on Neo4j".into(),
            input_schema: "schemas/graph_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "graph_relate".into(),
            description: "Create relationship between nodes".into(),
            input_schema: "schemas/graph_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "graph_suite".into(),
            description: "Unified graph operations suite".into(),
            input_schema: "schemas/graph_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        // Memory Suite
        ToolInfo {
            name: "memory_query".into(),
            description: "Query a value from memory by key".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "memory_store".into(),
            description: "Store a key-value pair in memory".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        // Parser Suite
        ToolInfo {
            name: "parser_search".into(),
            description: "Search code patterns using ripgrep".into(),
            input_schema: "schemas/code_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "parser_analyze".into(),
            description: "Analyze code structure using tree-sitter".into(),
            input_schema: "schemas/code_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        // Task Suite
        ToolInfo {
            name: "task_create".into(),
            description: "Create a new task".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "task_list".into(),
            description: "List all tasks".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "task_get".into(),
            description: "Get specific task by ID".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "task_update".into(),
            description: "Update task status".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "task_next".into(),
            description: "Get next task ready to work on".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        // Debug Suite
        ToolInfo {
            name: "debug_suite".into(),
            description: "Debugging, logs, and project analysis suite".into(),
            input_schema: "schemas/debug_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        // Mapping Suite
        ToolInfo {
            name: "mapping_suite".into(),
            description: "Application structure mapping suite".into(),
            input_schema: "schemas/mapping_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        // Reasoning Suite
        ToolInfo {
            name: "reasoning_session_create".into(),
            description: "Create a new reasoning session".into(),
            input_schema: "schemas/reasoning_session_create.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "reasoning_branch_expand".into(),
            description: "Expand a reasoning branch with new thought".into(),
            input_schema: "schemas/reasoning_branch_expand.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "reasoning_tree_get".into(),
            description: "Get reasoning tree structure".into(),
            input_schema: "schemas/reasoning_tree_get.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "reasoning_tree_prune".into(),
            description: "Prune reasoning tree subtree".into(),
            input_schema: "schemas/reasoning_tree_prune.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        // IntelliTask Suite
        ToolInfo {
            name: "intellitask_generate".into(),
            description: "Generate tasks from PRD using AI".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "intellitask_subtasks".into(),
            description: "Generate subtasks using AI".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "intellitask_prioritize".into(),
            description: "Prioritize tasks using AI".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "intellitask_next".into(),
            description: "Suggest next task using AI".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "intellitask_save".into(),
            description: "Save task breakdown to database".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "intellitask_get".into(),
            description: "Get specific task by ID".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "intellitask_list".into(),
            description: "List all tasks".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
        ToolInfo {
            name: "intellitask_update_status".into(),
            description: "Update task status".into(),
            input_schema: "schemas/memory_suite.json".into(),
            output_schema: "schemas/suite_result.json".into(),
        },
    ]
}

pub async fn describe_server() -> serde_json::Value {
    serde_json::json!({
        "name": "SynCore",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Cognitive micro-kernel with sequential thinking, memory, and task management",
        "encodings": ["json", "msgpack"],
        "tools_count": 34,
        "capabilities": {
            "memory": true,
            "vector_search": true,
            "task_management": true,
            "logging": true,
            "mcp_compliant": true,
            "code_intelligence": true,
            "refactoring_analysis": true
        }
    })
}

// Schema cache
lazy_static::lazy_static! {
    static ref SCHEMAS: HashMap<String, String> = {
        let mut schemas = HashMap::new();
        // Suite-based schemas
        schemas.insert("document_search".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("document_index".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("vector_insert".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("vector_search".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("code_search".to_string(), include_str!("../../schemas/code_suite.json").to_string());
        schemas.insert("code_index".to_string(), include_str!("../../schemas/code_suite.json").to_string());
        schemas.insert("code_index_directory".to_string(), include_str!("../../schemas/code_suite.json").to_string());
        schemas.insert("graph_query".to_string(), include_str!("../../schemas/graph_suite.json").to_string());
        schemas.insert("graph_insert".to_string(), include_str!("../../schemas/graph_suite.json").to_string());
        schemas.insert("graph_relate".to_string(), include_str!("../../schemas/graph_suite.json").to_string());
        schemas.insert("graph_suite".to_string(), include_str!("../../schemas/graph_suite.json").to_string());
        schemas.insert("memory_query".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("memory_store".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("parser_search".to_string(), include_str!("../../schemas/code_suite.json").to_string());
        schemas.insert("parser_analyze".to_string(), include_str!("../../schemas/code_suite.json").to_string());
        schemas.insert("task_create".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("task_list".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("task_get".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("task_update".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("task_next".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("debug_suite".to_string(), include_str!("../../schemas/debug_suite.json").to_string());
        schemas.insert("mapping_suite".to_string(), include_str!("../../schemas/mapping_suite.json").to_string());
        // Reasoning schemas
        schemas.insert("reasoning_session_create".to_string(), include_str!("../../schemas/reasoning_session_create.json").to_string());
        schemas.insert("reasoning_branch_expand".to_string(), include_str!("../../schemas/reasoning_branch_expand.json").to_string());
        schemas.insert("reasoning_tree_get".to_string(), include_str!("../../schemas/reasoning_tree_get.json").to_string());
        schemas.insert("reasoning_tree_prune".to_string(), include_str!("../../schemas/reasoning_tree_prune.json").to_string());
        // IntelliTask schemas (reuse memory_suite schemas for now)
        schemas.insert("intellitask_generate".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("intellitask_subtasks".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("intellitask_prioritize".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("intellitask_next".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("intellitask_save".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("intellitask_get".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("intellitask_list".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas.insert("intellitask_update_status".to_string(), include_str!("../../schemas/memory_suite.json").to_string());
        schemas
    };
}

fn validate_arguments(tool_name: &str, arguments: &Value) -> Result<(), String> {
    if let Some(schema_str) = SCHEMAS.get(tool_name) {
        let schema: Value =
            serde_json::from_str(schema_str).map_err(|e| format!("Invalid schema: {}", e))?;

        // Simple validation - in production, use jsonschema crate
        if let Some(obj) = arguments.as_object() {
            if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
                for req_field in required {
                    if let Some(field) = req_field.as_str() {
                        if !obj.contains_key(field) {
                            return Err(format!("Missing required field: {}", field));
                        }
                    }
                }
            }

            // Check for additional properties if specified
            if schema.get("additionalProperties").and_then(|v| v.as_bool()) == Some(false) {
                if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
                    for key in obj.keys() {
                        if !properties.contains_key(key) {
                            return Err(format!("Unexpected field: {}", key));
                        }
                    }
                }
            }
        }

        Ok(())
    } else {
        Err(format!("No schema found for tool: {}", tool_name))
    }
}

pub async fn handle_mcp_request(request: MCPRequest, state: &SynCoreState) -> MCPResponse {
    let method = request.method.as_str();

    match method {
        "mcp.describe" => {
            let info = describe_server().await;
            MCPResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(info),
                error: None,
                id: request.id,
            }
        }
        "mcp.list_tools" => {
            let tools = list_tools().await;
            MCPResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::to_value(tools).unwrap()),
                error: None,
                id: request.id,
            }
        }
        "mcp.call_tool" => {
            if let Some(params) = request.params {
                if let (Some(name), Some(arguments)) =
                    (params.get("name").and_then(|v| v.as_str()), params.get("arguments"))
                {
                    // Validate arguments against schema
                    if let Err(validation_error) = validate_arguments(name, arguments) {
                        return MCPResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(MCPError {
                                code: -32602,
                                message: format!("Invalid arguments: {}", validation_error),
                            }),
                            id: request.id,
                        };
                    }

                    match invoke_tool(name, arguments, state).await {
                        Ok(result) => MCPResponse {
                            jsonrpc: "2.0".to_string(),
                            result: Some(result),
                            error: None,
                            id: request.id,
                        },
                        Err(e) => MCPResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(MCPError {
                                code: -32603,
                                message: format!("Tool execution failed: {}", e),
                            }),
                            id: request.id,
                        },
                    }
                } else {
                    MCPResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(MCPError {
                            code: -32602,
                            message: "Invalid params: missing 'name' or 'arguments'".to_string(),
                        }),
                        id: request.id,
                    }
                }
            } else {
                MCPResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(MCPError {
                        code: -32602,
                        message: "Missing params".to_string(),
                    }),
                    id: request.id,
                }
            }
        }
        _ => MCPResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(MCPError {
                code: -32601,
                message: format!("Method not found: {}", method),
            }),
            id: request.id,
        },
    }
}

async fn invoke_tool(
    name: &str,
    arguments: &Value,
    state: &SynCoreState,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use crate::mcp_tools::{
        code_suite::CodeSuite, debug_suite::DebugSuite, graph_suite::GraphSuite,
        mapping_suite::MappingSuite, memory_suite::MemorySuite, SuiteDispatcher,
    };

    // Route through suite dispatchers based on tool name
    let result = match name {
        // Document tools (use memory suite)
        "document_search" | "document_index" => {
            let suite = MemorySuite::new(state.clone());
            let command = if name == "document_search" {
                "vector_search"
            } else {
                "vector_insert"
            };
            suite.dispatch(command, arguments.clone())
        }
        // Vector tools (use memory suite)
        "vector_insert" | "vector_search" => {
            let suite = MemorySuite::new(state.clone());
            let command = if name == "vector_insert" {
                "vector_insert"
            } else {
                "vector_search"
            };
            suite.dispatch(command, arguments.clone())
        }
        // Code tools (use code suite)
        "code_search"
        | "code_index"
        | "code_index_directory"
        | "parser_search"
        | "parser_analyze" => {
            let suite = CodeSuite::new(state.clone());
            let command = match name {
                "code_search" => "search",
                "code_index" => "index",
                "code_index_directory" => "index_directory",
                "parser_search" => "grep",
                "parser_analyze" => "parse",
                _ => return Err(format!("Unknown code tool: {}", name).into()),
            };
            suite.dispatch(command, arguments.clone())
        }
        // Graph tools (use graph suite)
        "graph_query" | "graph_insert" | "graph_relate" | "graph_suite" => {
            let suite = GraphSuite::new(state.clone());
            let command = match name {
                "graph_query" => "query",
                "graph_insert" => "insert",
                "graph_relate" => "relate",
                "graph_suite" => "help",
                _ => return Err(format!("Unknown graph tool: {}", name).into()),
            };
            suite.dispatch(command, arguments.clone())
        }
        // Memory tools (use memory suite)
        "memory_query" | "memory_store" => {
            let suite = MemorySuite::new(state.clone());
            let command = if name == "memory_query" {
                "query"
            } else {
                "store"
            };
            suite.dispatch(command, arguments.clone())
        }
        // Task tools (use memory suite)
        "task_create" | "task_list" | "task_get" | "task_update" | "task_next" => {
            let suite = MemorySuite::new(state.clone());
            let command = match name {
                "task_create" => "task_create",
                "task_list" => "task_list",
                "task_get" => "task_get",
                "task_update" => "task_update",
                "task_next" => "task_next",
                _ => return Err(format!("Unknown task tool: {}", name).into()),
            };
            suite.dispatch(command, arguments.clone())
        }
        // Debug Suite (direct suite dispatch)
        "debug_suite" => {
            let suite = DebugSuite::new(state.clone());
            suite.dispatch("debug_suite", arguments.clone())
        }
        // Mapping Suite (direct suite dispatch)
        "mapping_suite" => {
            let suite = MappingSuite::new(state.clone());
            suite.dispatch("mapping_suite", arguments.clone())
        }
        // Reasoning Tools
        "reasoning_session_create" => {
            use crate::mcp_tools::reasoning_suite::handle_reasoning_session_create;
            use crate::mcp_tools::SuiteResult;
            match handle_reasoning_session_create(arguments.clone(), state).await {
                Ok(value) => SuiteResult::ok("reasoning_session_create", value),
                Err(e) => SuiteResult::err(
                    "reasoning_session_create",
                    format!("Reasoning session create failed: {}", e),
                ),
            }
        }
        "reasoning_branch_expand" => {
            use crate::mcp_tools::reasoning_suite::handle_reasoning_branch_expand;
            use crate::mcp_tools::SuiteResult;
            match handle_reasoning_branch_expand(arguments.clone(), state).await {
                Ok(value) => SuiteResult::ok("reasoning_branch_expand", value),
                Err(e) => SuiteResult::err(
                    "reasoning_branch_expand",
                    format!("Reasoning branch expand failed: {}", e),
                ),
            }
        }
        "reasoning_tree_get" => {
            use crate::mcp_tools::reasoning_suite::handle_reasoning_tree_get;
            use crate::mcp_tools::SuiteResult;
            match handle_reasoning_tree_get(arguments.clone(), state).await {
                Ok(value) => SuiteResult::ok("reasoning_tree_get", value),
                Err(e) => SuiteResult::err(
                    "reasoning_tree_get",
                    format!("Reasoning tree get failed: {}", e),
                ),
            }
        }
        "reasoning_tree_prune" => {
            use crate::mcp_tools::reasoning_suite::handle_reasoning_tree_prune;
            use crate::mcp_tools::SuiteResult;
            match handle_reasoning_tree_prune(arguments.clone(), state).await {
                Ok(value) => SuiteResult::ok("reasoning_tree_prune", value),
                Err(e) => SuiteResult::err(
                    "reasoning_tree_prune",
                    format!("Reasoning tree prune failed: {}", e),
                ),
            }
        }
        // IntelliTask Tools (use memory suite)
        "intellitask_generate"
        | "intellitask_subtasks"
        | "intellitask_prioritize"
        | "intellitask_next"
        | "intellitask_save"
        | "intellitask_get"
        | "intellitask_list"
        | "intellitask_update_status" => {
            let suite = MemorySuite::new(state.clone());
            let command = match name {
                "intellitask_generate" => "intellitask_generate",
                "intellitask_subtasks" => "intellitask_subtasks",
                "intellitask_prioritize" => "intellitask_prioritize",
                "intellitask_next" => "intellitask_next",
                "intellitask_save" => "intellitask_save",
                "intellitask_get" => "intellitask_get",
                "intellitask_list" => "intellitask_list",
                "intellitask_update_status" => "intellitask_update_status",
                _ => return Err(format!("Unknown intellitask tool: {}", name).into()),
            };
            suite.dispatch(command, arguments.clone())
        }
        _ => return Err(format!("Unknown tool: {}", name).into()),
    };

    // Convert SuiteResult to JSON
    let json_value = serde_json::to_value(result)?;
    Ok(json_value)
}
