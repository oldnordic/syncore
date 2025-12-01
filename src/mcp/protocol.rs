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
        ToolInfo {
            name: "memory.store".into(),
            description: "Stores a key-value pair into SynCore's memory".into(),
            input_schema: "schemas/memory_store.json".into(),
            output_schema: "schemas/ok.json".into(),
        },
        ToolInfo {
            name: "memory.query".into(),
            description: "Queries a value from SynCore's memory by key".into(),
            input_schema: "schemas/memory_query.json".into(),
            output_schema: "schemas/memory_value.json".into(),
        },
        ToolInfo {
            name: "task.create".into(),
            description: "Creates a new task with a goal".into(),
            input_schema: "schemas/task_create.json".into(),
            output_schema: "schemas/task_id.json".into(),
        },
        ToolInfo {
            name: "vector.insert".into(),
            description: "Inserts text into vector memory for semantic search".into(),
            input_schema: "schemas/vector_insert.json".into(),
            output_schema: "schemas/vector_insert_response.json".into(),
        },
        ToolInfo {
            name: "vector.search".into(),
            description: "Searches vector memory for semantically similar content".into(),
            input_schema: "schemas/vector_search.json".into(),
            output_schema: "schemas/vector_results.json".into(),
        },
        ToolInfo {
            name: "logs.tail".into(),
            description: "Returns the last N entries from markdown logs".into(),
            input_schema: "schemas/logs_tail.json".into(),
            output_schema: "schemas/log_entries.json".into(),
        },
        ToolInfo {
            name: "parser.analyze".into(),
            description: "Analyzes code structure and extracts functions, classes, imports, and variables".into(),
            input_schema: "schemas/parse_file.json".into(),
            output_schema: "schemas/parse_file_output.json".into(),
        },
        ToolInfo {
            name: "parser.search".into(),
            description: "Searches for patterns in code files using ripgrep".into(),
            input_schema: "schemas/search_code.json".into(),
            output_schema: "schemas/search_code_output.json".into(),
        },
        ToolInfo {
            name: "code.explain".into(),
            description: "Explains code using local Ollama LLM. Can explain specific functions or entire files.".into(),
            input_schema: "schemas/code_explain.json".into(),
            output_schema: "schemas/code_explain_output.json".into(),
        },
        ToolInfo {
            name: "code.index_directory".into(),
            description: "Index all code files in a directory matching a glob pattern. Extracts functions, classes, and relationships for semantic search.".into(),
            input_schema: "schemas/code_index_directory.json".into(),
            output_schema: "schemas/code_index_directory_output.json".into(),
        },
        // Code Graph Tools (PHASE 5)
        ToolInfo {
            name: "code_graph.index".into(),
            description: "Index a Rust source file into the Code Intelligence Graph. Extracts imports, functions, calls, structs, traits, and implementations with semantic embeddings.".into(),
            input_schema: "schemas/code_graph_index.json".into(),
            output_schema: "schemas/code_graph_index_output.json".into(),
        },
        ToolInfo {
            name: "code_graph.query".into(),
            description: "Query the Code Intelligence Graph for function relationships including callers, callees, and semantically similar functions.".into(),
            input_schema: "schemas/code_graph_query.json".into(),
            output_schema: "schemas/code_graph_query_output.json".into(),
        },
        ToolInfo {
            name: "code_graph.explain".into(),
            description: "Get a comprehensive explanation of a function including its signature, callers, callees, and implementation context.".into(),
            input_schema: "schemas/code_graph_explain.json".into(),
            output_schema: "schemas/code_graph_explain_output.json".into(),
        },
        ToolInfo {
            name: "code_graph.impact".into(),
            description: "Analyze the impact of modifying a function by finding all callers and dependent code paths.".into(),
            input_schema: "schemas/code_graph_impact.json".into(),
            output_schema: "schemas/code_graph_impact_output.json".into(),
        },
        // Refactoring Tools (PHASE 7)
        ToolInfo {
            name: "code_graph.refactor_check".into(),
            description: "Run comprehensive refactoring analysis detecting long functions, dead code, and duplicate functions.".into(),
            input_schema: "schemas/code_graph_refactor_check.json".into(),
            output_schema: "schemas/code_graph_refactor_check_output.json".into(),
        },
        ToolInfo {
            name: "code_graph.refactor_symbol".into(),
            description: "Generate a detailed refactoring plan for a specific symbol (function, struct, or trait).".into(),
            input_schema: "schemas/code_graph_refactor_symbol.json".into(),
            output_schema: "schemas/code_graph_refactor_symbol_output.json".into(),
        },
        // SMEL - Static Macro Expansion Layer
        ToolInfo {
            name: "project.macro_expand".into(),
            description: "Get macro expansions for a Rust file from the Static Macro Expansion Layer (SMEL).".into(),
            input_schema: "schemas/project_macro_expand.json".into(),
            output_schema: "schemas/project_macro_expand_output.json".into(),
        },
    ]
}

pub async fn describe_server() -> serde_json::Value {
    serde_json::json!({
        "name": "SynCore",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Cognitive micro-kernel with sequential thinking, memory, and task management",
        "encodings": ["json", "msgpack"],
        "tools_count": 17,
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
        schemas.insert("memory.store".to_string(), include_str!("../../schemas/memory_store.json").to_string());
        schemas.insert("memory.query".to_string(), include_str!("../../schemas/memory_query.json").to_string());
        schemas.insert("task.create".to_string(), include_str!("../../schemas/task_create.json").to_string());
        schemas.insert("vector.insert".to_string(), include_str!("../../schemas/vector_insert.json").to_string());
        schemas.insert("vector.search".to_string(), include_str!("../../schemas/vector_search.json").to_string());
        schemas.insert("logs.tail".to_string(), include_str!("../../schemas/logs_tail.json").to_string());
        schemas.insert("parser.analyze".to_string(), include_str!("../../schemas/parse_file.json").to_string());
        schemas.insert("parser.search".to_string(), include_str!("../../schemas/search_code.json").to_string());
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
    // Convert JSON arguments to MessagePack format for router
    let args_vec = match name {
        "memory.store" => {
            let key = arguments["key"].as_str().ok_or("Missing key")?;
            let value = arguments["value"].as_str().ok_or("Missing value")?;
            rmp_serde::to_vec(&(key.to_string(), value.to_string()))?
        }
        "memory.query" => {
            let key = arguments["key"].as_str().ok_or("Missing key")?;
            rmp_serde::to_vec(&key.to_string())?
        }
        "task.create" => {
            let goal = arguments["goal"].as_str().ok_or("Missing goal")?;
            rmp_serde::to_vec(&goal.to_string())?
        }
        "vector.insert" => {
            let id = arguments["id"].as_i64().ok_or("Missing id")?;
            let task_id = arguments.get("task_id").and_then(|v| v.as_i64());
            let text = arguments["text"].as_str().ok_or("Missing text")?;
            let kind = arguments["kind"].as_str().unwrap_or("note");
            rmp_serde::to_vec(&(id, task_id, text.to_string(), kind.to_string()))?
        }
        "vector.search" => {
            let query = arguments["query"].as_str().ok_or("Missing query")?;
            let k = arguments["k"].as_u64().unwrap_or(5) as usize;
            let scope = if let Some(scope_obj) = arguments.get("scope").and_then(|v| v.as_object())
            {
                if let Some(task_obj) = scope_obj.get("task").and_then(|v| v.as_object()) {
                    let task_id =
                        task_obj["task_id"].as_u64().ok_or("Missing task_id in task scope")?;
                    crate::vector::SearchScope::Task(task_id.try_into().unwrap())
                } else {
                    return Err("Invalid scope format".into());
                }
            } else {
                crate::vector::SearchScope::Global
            };
            rmp_serde::to_vec(&(query.to_string(), k, scope))?
        }
        "graph.link" => {
            let src_id = arguments["src_id"].as_i64().ok_or("Missing src_id")?;
            let dst_id = arguments["dst_id"].as_i64().ok_or("Missing dst_id")?;
            let kind = arguments["kind"].as_str().ok_or("Missing kind")?;
            rmp_serde::to_vec(&(src_id, dst_id, kind.to_string()))?
        }
        "graph.query" => {
            let task_id = arguments["task_id"].as_i64().ok_or("Missing task_id")?;
            let direction = arguments["direction"].as_str().unwrap_or("both");
            rmp_serde::to_vec(&(task_id, direction.to_string()))?
        }
        "logs.tail" => {
            let n = arguments["n"].as_u64().unwrap_or(10) as usize;
            rmp_serde::to_vec(&n)?
        }
        "parser.analyze" => {
            let file_path = arguments["file_path"].as_str().ok_or("Missing file_path")?;
            rmp_serde::to_vec(&file_path.to_string())?
        }
        "parser.search" => {
            let pattern = arguments["pattern"].as_str().ok_or("Missing pattern")?;
            let directory =
                arguments.get("directory").and_then(|v| v.as_str()).map(|s| s.to_string());
            let context_lines =
                arguments.get("context_lines").and_then(|v| v.as_u64()).map(|n| n as usize);
            rmp_serde::to_vec(&(pattern.to_string(), directory, context_lines))?
        }
        "code.explain" => {
            use crate::code_explainer::ExplainRequest;
            let request = ExplainRequest {
                file_path: arguments["file_path"].as_str().ok_or("Missing file_path")?.to_string(),
                function_name: arguments
                    .get("function_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                model: arguments.get("model").and_then(|v| v.as_str()).map(|s| s.to_string()),
            };
            rmp_serde::to_vec(&request)?
        }
        "code.index_directory" => {
            use crate::code_directory_indexer::DirectoryIndexRequest;
            let request = DirectoryIndexRequest {
                directory: arguments["directory"].as_str().ok_or("Missing directory")?.to_string(),
                pattern: arguments["pattern"].as_str().ok_or("Missing pattern")?.to_string(),
            };
            rmp_serde::to_vec(&request)?
        }
        "project.macro_expand" => {
            use crate::mcp::code_graph_tools::handle_project_macro_expand;
            let result = handle_project_macro_expand(arguments.clone()).await?;
            rmp_serde::to_vec(&result)?
        }
        _ => return Err(format!("Unknown tool: {}", name).into()),
    };

    // Use router to handle the tool call
    let response_bytes = crate::router::route_tool(name, &args_vec, state)?;

    // Convert MessagePack response back to JSON
    let response_value: Value = rmp_serde::from_slice(&response_bytes)?;
    Ok(response_value)
}
