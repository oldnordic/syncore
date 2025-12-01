//! Macro Tools Router
//!
//! Provides action validation and routing logic for macro tools.
//! This module contains the core validation functions used by all macro tools.

use anyhow::Result;

/// Validate memory action
pub fn validate_memory_action(params: &serde_json::Value) -> Result<String> {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    match action {
        "store" | "query" => Ok(action.to_string()),
        _ => Err(anyhow::anyhow!("Invalid action for syncore.memory: {}", action)),
    }
}

/// Validate task action
pub fn validate_task_action(params: &serde_json::Value) -> Result<String> {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    match action {
        "create" | "list" | "get" | "update_status" | "next_ready" | "get_subtasks"
        | "subtask_stats" | "statistics" | "prd_statistics" => Ok(action.to_string()),
        _ => Err(anyhow::anyhow!("Invalid action for syncore.task: {}", action)),
    }
}

/// Validate vector action
pub fn validate_vector_action(params: &serde_json::Value) -> Result<String> {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    match action {
        "insert" | "search" => Ok(action.to_string()),
        _ => Err(anyhow::anyhow!("Invalid action for syncore.vector: {}", action)),
    }
}

/// Validate code action
pub fn validate_code_action(params: &serde_json::Value) -> Result<String> {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    match action {
        "analyze" | "search" | "index" | "semantic_search" | "index_directory" => {
            Ok(action.to_string())
        }
        _ => Err(anyhow::anyhow!("Invalid action for syncore.code: {}", action)),
    }
}

/// Validate document action
pub fn validate_document_action(params: &serde_json::Value) -> Result<String> {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    match action {
        "index" | "search" => Ok(action.to_string()),
        _ => Err(anyhow::anyhow!("Invalid action for syncore.document: {}", action)),
    }
}

/// Validate graph action
pub fn validate_graph_action(params: &serde_json::Value) -> Result<String> {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    match action {
        "query" | "insert" | "relate" => Ok(action.to_string()),
        _ => Err(anyhow::anyhow!("Invalid action for syncore.graph: {}", action)),
    }
}

/// Validate agent action
pub fn validate_agent_action(params: &serde_json::Value) -> Result<String> {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    match action {
        "send" | "recv" | "poll" | "register" | "list" | "status" | "task" | "result" => {
            Ok(action.to_string())
        }
        _ => Err(anyhow::anyhow!("Invalid action for syncore.agent: {}", action)),
    }
}

/// Validate mapping action
pub fn validate_mapping_action(params: &serde_json::Value) -> Result<String> {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    match action {
        "record" | "get" | "search" | "deps" => Ok(action.to_string()),
        _ => Err(anyhow::anyhow!("Invalid action for syncore.mapping: {}", action)),
    }
}

/// Validate reasoning action
pub fn validate_reasoning_action(params: &serde_json::Value) -> Result<String> {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    match action {
        "cycle" | "record" | "get" | "search" => Ok(action.to_string()),
        _ => Err(anyhow::anyhow!("Invalid action for syncore.reasoning: {}", action)),
    }
}

/// Validate logs action
pub fn validate_logs_action(params: &serde_json::Value) -> Result<String> {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

    match action {
        "tail" => Ok(action.to_string()),
        _ => Err(anyhow::anyhow!("Invalid action for syncore.logs: {}", action)),
    }
}
