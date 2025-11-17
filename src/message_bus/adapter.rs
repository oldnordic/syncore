//! Agent adapters for different transport mechanisms
//!
//! Provides abstraction over stdio, HTTP, and internal communication.

use anyhow::Result;
use crate::message_bus::message::{AgentId, Msg};
use tokio::sync::mpsc;

/// Trait for agent communication adapters
pub trait AgentAdapter: Send + Sync {
    /// Get the agent's identifier
    fn agent_id(&self) -> AgentId;

    /// Send a message to this agent (non-blocking)
    fn send(&self, msg: Msg) -> Result<()>;

    /// Check if agent is connected
    fn is_connected(&self) -> bool;
}

/// Adapter for stdio-based agents (e.g., Claude Code)
pub struct StdioAdapter {
    /// Agent identifier
    pub id: AgentId,
    /// Channel sender for outgoing messages
    pub tx: mpsc::Sender<Msg>,
    /// Connection status flag
    pub connected: bool,
}

impl StdioAdapter {
    /// Create a new stdio adapter (skeleton)
    pub fn new(_id: AgentId, _tx: mpsc::Sender<Msg>) -> Self {
        // TODO: Initialize stdio adapter
        todo!("Implement StdioAdapter::new()")
    }
}

impl AgentAdapter for StdioAdapter {
    fn agent_id(&self) -> AgentId {
        self.id.clone()
    }

    fn send(&self, _msg: Msg) -> Result<()> {
        // TODO: Send message via stdio channel
        todo!("Implement StdioAdapter::send()")
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Adapter for HTTP-based agents (e.g., GLM-4.6)
pub struct HttpAdapter {
    /// Agent identifier
    pub id: AgentId,
    /// Endpoint URL for message delivery
    pub endpoint: String,
    /// Connection status flag
    pub connected: bool,
}

impl HttpAdapter {
    /// Create a new HTTP adapter (skeleton)
    pub fn new(_id: AgentId, _endpoint: String) -> Self {
        // TODO: Initialize HTTP adapter
        todo!("Implement HttpAdapter::new()")
    }
}

impl AgentAdapter for HttpAdapter {
    fn agent_id(&self) -> AgentId {
        self.id.clone()
    }

    fn send(&self, _msg: Msg) -> Result<()> {
        // TODO: Send message via HTTP POST
        todo!("Implement HttpAdapter::send()")
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Adapter for internal Syncore modules
pub struct InternalAdapter {
    /// Agent identifier
    pub id: AgentId,
    /// Direct callback for message handling
    pub callback: Box<dyn Fn(Msg) -> Result<()> + Send + Sync>,
}

impl AgentAdapter for InternalAdapter {
    fn agent_id(&self) -> AgentId {
        self.id.clone()
    }

    fn send(&self, msg: Msg) -> Result<()> {
        (self.callback)(msg)
    }

    fn is_connected(&self) -> bool {
        true // Internal adapters are always connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_trait_object_safety() {
        // TODO: Ensure AgentAdapter can be used as trait object
    }
}
