//! Message types for inter-agent communication
//!
//! Defines the core message primitives for the Syncore message bus.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Unique identifier for an agent in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentId {
    /// Claude Code agent (stdio transport)
    Claude,
    /// GLM-4.6 agent (HTTP transport)
    Glm46,
    /// Internal Syncore module
    Internal(String),
    /// Custom agent identifier
    Custom(String),
}

/// Message kind/topic for routing and filtering
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MsgKind {
    /// Direct message to specific agent
    Direct,
    /// Broadcast to all agents
    Broadcast,
    /// Event notification (state change)
    Event(String),
    /// Request expecting response
    Request,
    /// Response to a request
    Response,
}

/// Core message structure for inter-agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Msg {
    /// Unique message identifier
    pub id: u64,
    /// Source agent
    pub from: AgentId,
    /// Target agent (None for broadcast)
    pub to: Option<AgentId>,
    /// Message kind/topic
    pub kind: MsgKind,
    /// Payload as JSON value
    pub payload: serde_json::Value,
    /// Timestamp of message creation
    pub timestamp: SystemTime,
}

impl Msg {
    /// Create a new message (skeleton - no logic yet)
    pub fn new(
        _from: AgentId,
        _to: Option<AgentId>,
        _kind: MsgKind,
        _payload: serde_json::Value,
    ) -> Self {
        // TODO: Implement message creation with auto-generated ID
        todo!("Implement Msg::new()")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_serialization() {
        // TODO: Test AgentId serde round-trip
    }

    #[test]
    fn test_msg_kind_variants() {
        // TODO: Test MsgKind serialization
    }
}
