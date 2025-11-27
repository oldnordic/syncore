//! Agent Commands Module
//!
//! Handles execution of agent communication and coordination operations.
//! Extracted from memory_suite.rs (lines 199-528).
//!
//! Commands:
//! - agent_send: Send message to another agent via message bus
//! - agent_recv: Receive pending messages for agent (NOT IMPLEMENTED - API limitation)
//! - agent_poll: Wait for messages with timeout (NOT IMPLEMENTED - API limitation)
//! - agent_register: Register agent ID and capabilities
//! - agent_list: List all registered agents
//! - agent_status: Update agent status
//! - agent_task: Send task envelope to agent
//! - agent_result: Submit task result from agent

use crate::mcp_tools::SuiteResult;
use super::{MemorySuite, MemorySuiteArgs};

/// Execute agent_send command
pub fn cmd_agent_send(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    // Check if message_bus is available
    if suite.state.message_bus.is_none() {
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

    let bus = suite.state.message_bus.as_ref().unwrap();

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

/// Execute agent_recv command
pub fn cmd_agent_recv(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    // Check if message_bus is available
    if suite.state.message_bus.is_none() {
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

/// Execute agent_poll command
pub fn cmd_agent_poll(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    // Check if message_bus is available
    if suite.state.message_bus.is_none() {
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

/// Execute agent_register command
pub fn cmd_agent_register(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    // Check if message_bus is available
    if suite.state.message_bus.is_none() {
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

    let bus = suite.state.message_bus.as_ref().unwrap();

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

/// Execute agent_list command
pub fn cmd_agent_list(suite: &MemorySuite, _args: MemorySuiteArgs) -> SuiteResult {
    // Check if message_bus is available
    if suite.state.message_bus.is_none() {
        return SuiteResult::err(
            "agent_list",
            "NotAvailable: Agent system unavailable - MessageBus not configured",
        );
    }

    let bus = suite.state.message_bus.as_ref().unwrap();

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

/// Execute agent_status command
pub fn cmd_agent_status(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    // Check if message_bus is available
    if suite.state.message_bus.is_none() {
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

    let bus = suite.state.message_bus.as_ref().unwrap();

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

/// Execute agent_task command
pub fn cmd_agent_task(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    // Check if message_bus is available
    if suite.state.message_bus.is_none() {
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

    let bus = suite.state.message_bus.as_ref().unwrap();

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

/// Execute agent_result command
pub fn cmd_agent_result(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    // Check if message_bus is available
    if suite.state.message_bus.is_none() {
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

    let bus = suite.state.message_bus.as_ref().unwrap();

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
