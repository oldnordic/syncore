use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
// SynCoreState stub - will be re-implemented later
struct SynCoreState {
    // Empty placeholder
}

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
    ]
}

pub async fn describe_server() -> serde_json::Value {
    serde_json::json!({
        "name": "SynCore",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Cognitive micro-kernel with sequential thinking, memory, and task management",
        "encodings": ["json", "msgpack"],
        "tools_count": 5,
        "capabilities": {
            "memory": true,
            "vector_search": true,
            "task_management": true,
            "logging": true,
            "mcp_compliant": true
        }
    })
}

// Schema cache
lazy_static::lazy_static! {
    static ref SCHEMAS: HashMap<String, String> = {
        let mut schemas = HashMap::new();
        schemas.insert("memory.store".to_string(), include_str!("../schemas/memory_store.json").to_string());
        schemas.insert("memory.query".to_string(), include_str!("../schemas/memory_query.json").to_string());
        schemas.insert("task.create".to_string(), include_str!("../schemas/task_create.json").to_string());
        schemas.insert("vector.search".to_string(), include_str!("../schemas/vector_search.json").to_string());
        schemas.insert("logs.tail".to_string(), include_str!("../schemas/logs_tail.json").to_string());
        schemas
    };
}

fn validate_arguments(tool_name: &str, arguments: &Value) -> Result<(), String> {
    if let Some(schema_str) = SCHEMAS.get(tool_name) {
        let schema: Value = serde_json::from_str(schema_str)
            .map_err(|e| format!("Invalid schema: {}", e))?;

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
                if let (Some(name), Some(arguments)) = (
                    params.get("name").and_then(|v| v.as_str()),
                    params.get("arguments")
                ) {
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

async fn invoke_tool(name: &str, arguments: &Value, state: &SynCoreState) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use crate::protocol::SynCoreMsg;
    use crate::protocol::SynCoreTool;

    let tool = match name {
        "memory.store" => {
            let key = arguments["key"].as_str().ok_or("Missing key")?;
            let value = arguments["value"].as_str().ok_or("Missing value")?;
            let args = (key.to_string(), value.to_string());
            let args_vec = rmp_serde::to_vec(&args)?;
            let msg = SynCoreMsg { tool: SynCoreTool::MemoryStore, args: args_vec };
            let response = vec![]; // Stub response
            let response_value: Value = rmp_serde::from_slice(&response)?;
            return Ok(response_value);
        }
        "memory.query" => {
            let key = arguments["key"].as_str().ok_or("Missing key")?;
            let args = key.to_string();
            let args_vec = rmp_serde::to_vec(&args)?;
            let msg = SynCoreMsg { tool: SynCoreTool::MemoryQuery, args: args_vec };
            let response = vec![]; // Stub response
            let response_value: Value = rmp_serde::from_slice(&response)?;
            return Ok(response_value);
        }
        "task.create" => {
            let goal = arguments["goal"].as_str().ok_or("Missing goal")?;
            let args = goal.to_string();
            let args_vec = rmp_serde::to_vec(&args)?;
            let msg = SynCoreMsg { tool: SynCoreTool::TaskCreate, args: args_vec };
            let response = vec![]; // Stub response
            let response_value: Value = rmp_serde::from_slice(&response)?;
            return Ok(response_value);
        }
        "vector.search" => {
            let query = arguments["query"].as_str().ok_or("Missing query")?;
            let k = arguments["k"].as_u64().unwrap_or(5) as usize;
            let scope_str = arguments["scope"].as_str().unwrap_or("global");
            let scope = match scope_str {
                "global" => crate::vector::SearchScope::Global,
                "task" => {
                    let task_id = arguments["task_id"].as_u64().unwrap_or(0);
                    crate::vector::SearchScope::Task(task_id.try_into().unwrap())
                },
                _ => crate::vector::SearchScope::Global,
            };
            let args = (query.to_string(), k, scope);
            let args_vec = rmp_serde::to_vec(&args)?;
            let msg = SynCoreMsg { tool: SynCoreTool::VectorSearch, args: args_vec };
            let response = vec![]; // Stub response
            let response_value: Value = rmp_serde::from_slice(&response)?;
            return Ok(response_value);
        }
        "logs.tail" => {
            let n = arguments["n"].as_u64().unwrap_or(10) as usize;
            let args = n;
            let args_vec = rmp_serde::to_vec(&args)?;
            let msg = SynCoreMsg { tool: SynCoreTool::LogsTail, args: args_vec };
            let response = vec![]; // Stub response
            let response_value: Value = rmp_serde::from_slice(&response)?;
            return Ok(response_value);
        }
        _ => return Err(format!("Unknown tool: {}", name).into()),
    };
}
