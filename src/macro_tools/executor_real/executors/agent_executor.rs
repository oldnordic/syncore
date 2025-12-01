//! Agent/MessageBus Tools Executor
//!
//! Handles execution of agent communication and message bus tools.
//! Extracted from executor_real.rs giant match statement (lines 272-641).
//!
//! Tools:
//! - agent_send: Send message to another agent via message bus
//! - agent_recv: Receive pending messages for a given agent ID (NOT IMPLEMENTED)
//! - agent_register: Register an agent ID and its capabilities
//! - agent_list: List all registered agents
//! - agent_status: Update the status of the specified agent
//! - agent_task: Send a structured task envelope to a specified agent
//! - agent_result: Submit the result of a completed task to the router

use crate::mcp::types::ErrorType;
use crate::router::SynCoreState;
use serde_json::{json, Value};
use std::sync::Arc;

/// Helper: Extract string parameter from Value params
fn param_str<'a>(tool: &str, params: &'a Value, key: &str) -> Result<&'a str, Value> {
    match params.get(key).and_then(|v| v.as_str()) {
        Some(v) if !v.is_empty() => Ok(v),
        _ => Err(wrap_error_static(tool, &format!("Missing '{}' parameter", key))),
    }
}

/// Helper: Wrap error response
fn wrap_error_static(tool: &str, msg: &str) -> Value {
    let error_type = ErrorType::from_message(msg);
    json!({
        "ok": false,
        "error": {
            "type": error_type.to_string(),
            "message": msg,
            "tool": tool,
            "executor": "real"
        }
    })
}

/// Helper: Wrap success response
fn wrap_success(tool: &str, data: Value) -> Value {
    json!({
        "ok": true,
        "tool": tool,
        "executor": "real",
        "data": data
    })
}

/// Helper: Wrap error with state access
fn wrap_error(tool: &str, error: &str) -> Value {
    wrap_error_static(tool, error)
}

/// Execute agent_send tool
pub async fn execute_agent_send(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // Check if message_bus is available FIRST (before any parameter parsing)
    if state.message_bus.is_none() {
        return Ok(wrap_error(
            "agent_send",
            "NotAvailable: Agent system unavailable - MessageBus not configured",
        ));
    }

    let to = match param_str("agent_send", params, "to") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let message = match param_str("agent_send", params, "message") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "agent_send",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would send message to '{}': {}", to, message),
                "sent": true
            }),
        );
        return Ok(result);
    }

    let bus = state.message_bus.as_ref().unwrap();

    // Send message via bus
    use crate::message_bus::message::{AgentId, Msg, MsgKind};
    use std::time::SystemTime;

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
        payload: json!({"message": message}),
        timestamp: SystemTime::now(),
    };

    bus.send(msg);

    Ok(wrap_success(
        "agent_send",
        json!({
            "sent": true,
            "to": to
        }),
    ))
}

/// Execute agent_recv tool
pub async fn execute_agent_recv(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // Check if message_bus is available FIRST (before any parameter parsing)
    if state.message_bus.is_none() {
        return Ok(wrap_error(
            "agent_recv",
            "NotAvailable: Agent system unavailable - MessageBus not configured",
        ));
    }

    let agent = match param_str("agent_recv", params, "agent") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };

    if dry_run {
        let result = wrap_success(
            "agent_recv",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would receive messages for agent '{}'", agent),
                "messages": []
            }),
        );
        return Ok(result);
    }

    // HONEST ERROR - MessageBus API does not support message polling/receiving
    // The current MessageBus design uses register_agent() which returns a Receiver<Msg>,
    // but there is no API to poll messages for an already-registered agent.
    //
    // To implement this properly, MessageBus needs one of:
    // 1. pub fn get_messages(&self, agent_id: &AgentId) -> Vec<Msg>
    // 2. pub fn poll_messages(&self, agent_id: &AgentId, limit: usize) -> Vec<Msg>
    // 3. A persistent message queue that can be queried
    //
    // Returning an error instead of fake empty messages.
    Ok(wrap_error(
        "agent_recv",
        "NotImplemented: MessageBus does not support message polling. \
        The current API only supports push-based message delivery via register_agent(). \
        To fix: Add get_messages() or poll_messages() method to MessageBus, \
        or implement a persistent message queue that can be queried.",
    ))
}

/// Execute agent_register tool
pub async fn execute_agent_register(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // Check if message_bus is available FIRST (before any parameter parsing)
    if state.message_bus.is_none() {
        return Ok(wrap_error(
            "agent_register",
            "NotAvailable: Agent system unavailable - MessageBus not configured",
        ));
    }

    let id = match param_str("agent_register", params, "id") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let capabilities = params["capabilities"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing 'capabilities' parameter"))?;

    if dry_run {
        let result = wrap_success(
            "agent_register",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would register agent '{}' with {} capabilities", id, capabilities.len()),
                "registered": true
            }),
        );
        return Ok(result);
    }

    let bus = state.message_bus.as_ref().unwrap();

    // Register agent
    use crate::message_bus::message::AgentId;
    let agent_id = match id.to_lowercase().as_str() {
        "claude" => AgentId::Claude,
        "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
        other => AgentId::Custom(other.to_string()),
    };
    let caps: Vec<String> =
        capabilities.iter().filter_map(|c| c.as_str().map(|s| s.to_string())).collect();

    bus.register_agent_info(agent_id.clone(), id.to_string(), caps);

    Ok(wrap_success(
        "agent_register",
        json!({
            "registered": true,
            "id": id
        }),
    ))
}

/// Execute agent_list tool
pub async fn execute_agent_list(
    state: &Arc<SynCoreState>,
    _params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // Check if message_bus is available FIRST (before any parameter parsing)
    if state.message_bus.is_none() {
        return Ok(wrap_error(
            "agent_list",
            "NotAvailable: Agent system unavailable - MessageBus not configured",
        ));
    }

    if dry_run {
        let result = wrap_success(
            "agent_list",
            json!({
                "dry_run": true,
                "message": "[DRY RUN] Would list all registered agents",
                "agents": []
            }),
        );
        return Ok(result);
    }

    let bus = state.message_bus.as_ref().unwrap();

    // Get list of registered agents
    let agents = bus.list_agents();
    let agent_names: Vec<String> = agents.iter().map(|a| format!("{:?}", a)).collect();

    Ok(wrap_success(
        "agent_list",
        json!({
            "agents": agent_names
        }),
    ))
}

/// Execute agent_status tool
pub async fn execute_agent_status(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // Check if message_bus is available FIRST (before any parameter parsing)
    if state.message_bus.is_none() {
        return Ok(wrap_error(
            "agent_status",
            "NotAvailable: Agent system unavailable - MessageBus not configured",
        ));
    }

    let id = match param_str("agent_status", params, "id") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let status =
        params.get("status").ok_or_else(|| anyhow::anyhow!("Missing 'status' parameter"))?;

    if dry_run {
        let result = wrap_success(
            "agent_status",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would update status for agent '{}'", id),
                "updated": true
            }),
        );
        return Ok(result);
    }

    let bus = state.message_bus.as_ref().unwrap();

    // Update agent status (uses agent name, not AgentId)
    bus.update_agent_status(id, status.clone());

    Ok(wrap_success(
        "agent_status",
        json!({
            "updated": true,
            "id": id
        }),
    ))
}

/// Execute agent_task tool
pub async fn execute_agent_task(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // Check if message_bus is available FIRST (before any parameter parsing)
    if state.message_bus.is_none() {
        return Ok(wrap_error(
            "agent_task",
            "NotAvailable: Agent system unavailable - MessageBus not configured",
        ));
    }

    let to = match param_str("agent_task", params, "to") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let task_id = match param_str("agent_task", params, "task_id") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let task_type = match param_str("agent_task", params, "task_type") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let payload =
        params.get("payload").ok_or_else(|| anyhow::anyhow!("Missing 'payload' parameter"))?;

    if dry_run {
        let result = wrap_success(
            "agent_task",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would send task '{}' to agent '{}'", task_id, to),
                "sent": true
            }),
        );
        return Ok(result);
    }

    let bus = state.message_bus.as_ref().unwrap();

    // Send task via bus
    use crate::message_bus::message::{AgentId, Msg, MsgKind};
    use std::time::SystemTime;

    let to_agent = match to.to_lowercase().as_str() {
        "claude" => AgentId::Claude,
        "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
        other => AgentId::Custom(other.to_string()),
    };

    let task_payload = json!({
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

    Ok(wrap_success(
        "agent_task",
        json!({
            "sent": true,
            "task_id": task_id
        }),
    ))
}

/// Execute agent_result tool
pub async fn execute_agent_result(
    state: &Arc<SynCoreState>,
    params: &Value,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // Check if message_bus is available FIRST (before any parameter parsing)
    if state.message_bus.is_none() {
        return Ok(wrap_error(
            "agent_result",
            "NotAvailable: Agent system unavailable - MessageBus not configured",
        ));
    }

    let from = match param_str("agent_result", params, "from") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let task_id = match param_str("agent_result", params, "task_id") {
        Ok(v) => v,
        Err(e) => return Ok(e),
    };
    let result =
        params.get("result").ok_or_else(|| anyhow::anyhow!("Missing 'result' parameter"))?;

    if dry_run {
        let result = wrap_success(
            "agent_result",
            json!({
                "dry_run": true,
                "message": format!("[DRY RUN] Would record result from '{}' for task '{}'", from, task_id),
                "recorded": true
            }),
        );
        return Ok(result);
    }

    let bus = state.message_bus.as_ref().unwrap();

    // Send result via bus
    use crate::message_bus::message::{AgentId, Msg, MsgKind};
    use std::time::SystemTime;

    let from_agent = match from.to_lowercase().as_str() {
        "claude" => AgentId::Claude,
        "glm46" | "glm-46" | "glm4.6" => AgentId::Glm46,
        other => AgentId::Custom(other.to_string()),
    };

    let result_payload = json!({
        "task_id": task_id,
        "result": result
    });

    let msg_id = bus.next_message_id();
    let msg = Msg {
        id: msg_id,
        from: from_agent,
        to: Some(AgentId::Internal("router".to_string())), // Send to router by default
        kind: MsgKind::Response,
        payload: result_payload,
        timestamp: SystemTime::now(),
    };

    bus.send(msg);

    Ok(wrap_success(
        "agent_result",
        json!({
            "recorded": true,
            "task_id": task_id
        }),
    ))
}
