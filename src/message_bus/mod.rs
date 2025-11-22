//! Message bus for inter-agent communication in Syncore
//!
//! Provides routing and delivery of messages between:
//! - Claude Code (stdio)
//! - GLM-4.6 (HTTP)
//! - Internal Syncore modules
//! - Future agents

pub mod adapter;
pub mod message;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::mpsc;

use message::{AgentId, Msg};

/// Metadata about a registered agent
#[derive(Clone, Debug)]
pub struct AgentInfo {
    pub id: AgentId,
    pub name: String,
    pub capabilities: Vec<String>,
    pub registered_at: Instant,
}

/// Central message bus for routing inter-agent communication
#[derive(Clone)]
pub struct MessageBus {
    /// Registry of connected agents: AgentId -> Sender<Msg>
    agents: Arc<RwLock<HashMap<AgentId, mpsc::Sender<Msg>>>>,
    /// Agent metadata registry: name -> AgentInfo
    agent_registry: Arc<RwLock<HashMap<String, AgentInfo>>>,
    /// Capability index: capability -> list of agent names
    capability_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Agent runtime status: name -> JSON status
    agent_status: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// Message history for debugging (append-only)
    history: Arc<RwLock<Vec<Msg>>>,
    /// Next message ID counter
    next_id: Arc<RwLock<u64>>,
}

impl MessageBus {
    /// Create a new message bus instance
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            agent_registry: Arc::new(RwLock::new(HashMap::new())),
            capability_index: Arc::new(RwLock::new(HashMap::new())),
            agent_status: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Register an agent with the message bus
    ///
    /// Returns a receiver channel for incoming messages to the agent.
    /// If an agent with the same ID already exists, it is replaced.
    pub fn register_agent(&self, id: AgentId) -> mpsc::Receiver<Msg> {
        let (tx, rx) = mpsc::channel(100); // Buffer size of 100 messages

        let mut agents = self.agents.write().unwrap();
        // Replace existing agent if present (old sender is dropped)
        agents.insert(id, tx);

        rx
    }

    /// Unregister an agent from the message bus
    ///
    /// If the agent is not found, does nothing (no panic).
    pub fn unregister_agent(&self, id: &AgentId) {
        let mut agents = self.agents.write().unwrap();
        agents.remove(id);
    }

    /// Send a message through the bus
    ///
    /// - If `msg.to` is Some(agent_id): sends only to that agent
    /// - If `msg.to` is None: broadcasts to all registered agents
    ///
    /// Returns Ok if message was queued for delivery.
    /// Silently ignores agents with dropped receivers.
    pub fn send(&self, msg: Msg) {
        // Record in history for debugging
        {
            let mut history = self.history.write().unwrap();
            history.push(msg.clone());
        }

        let agents = self.agents.read().unwrap();

        match &msg.to {
            Some(target_id) => {
                // Direct message to specific agent
                if let Some(sender) = agents.get(target_id) {
                    // try_send is non-blocking; ignores if channel full or closed
                    let _ = sender.try_send(msg);
                }
            }
            None => {
                // Broadcast to all agents
                for sender in agents.values() {
                    let _ = sender.try_send(msg.clone());
                }
            }
        }
    }

    /// Get list of registered agents
    pub fn list_agents(&self) -> Vec<AgentId> {
        let agents = self.agents.read().unwrap();
        agents.keys().cloned().collect()
    }

    /// Get next unique message ID (monotonically increasing)
    pub fn next_message_id(&self) -> u64 {
        let mut next_id = self.next_id.write().unwrap();
        let id = *next_id;
        *next_id += 1;
        id
    }

    /// Get message history (for debugging)
    pub fn message_history(&self) -> Vec<Msg> {
        let history = self.history.read().unwrap();
        history.clone()
    }

    /// Clear message history
    pub fn clear_history(&self) {
        let mut history = self.history.write().unwrap();
        history.clear();
    }

    /// Drain messages addressed to a specific agent from history
    ///
    /// Returns all messages where `msg.to` matches the given agent ID,
    /// and removes them from the history.
    pub fn drain_for(&self, agent_id: &AgentId) -> Vec<Msg> {
        let mut history = self.history.write().unwrap();
        let (matching, remaining): (Vec<Msg>, Vec<Msg>) = history
            .drain(..)
            .partition(|msg| msg.to.as_ref() == Some(agent_id));
        *history = remaining;
        matching
    }

    /// Try to receive the next message for an agent (non-blocking)
    ///
    /// Returns the first message addressed to the agent if available,
    /// removing it from history.
    pub fn try_recv_for(&self, agent_id: &AgentId) -> Option<Msg> {
        let mut history = self.history.write().unwrap();
        if let Some(pos) = history
            .iter()
            .position(|msg| msg.to.as_ref() == Some(agent_id))
        {
            Some(history.remove(pos))
        } else {
            None
        }
    }

    /// Wait for a message addressed to the agent with timeout
    ///
    /// Polls the history every 10ms until a message is found or timeout expires.
    /// Returns None if timeout is reached without receiving a message.
    pub fn wait_for(&self, agent_id: &AgentId, timeout_ms: u64) -> Option<Msg> {
        use std::thread;
        use std::time::Duration;

        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        let poll_interval = Duration::from_millis(10);

        loop {
            // Check for message
            if let Some(msg) = self.try_recv_for(agent_id) {
                return Some(msg);
            }

            // Check timeout
            if start.elapsed() >= timeout {
                return None;
            }

            // Sleep briefly before next poll
            thread::sleep(poll_interval);
        }
    }

    /// Register agent metadata (ID, name, capabilities)
    ///
    /// Stores agent info for routing and capability discovery.
    pub fn register_agent_info(&self, id: AgentId, name: String, capabilities: Vec<String>) {
        // Update agent registry
        {
            let mut registry = self.agent_registry.write().unwrap();
            registry.insert(
                name.clone(),
                AgentInfo {
                    id,
                    name: name.clone(),
                    capabilities: capabilities.clone(),
                    registered_at: Instant::now(),
                },
            );
        }

        // Update capability index
        {
            let mut index = self.capability_index.write().unwrap();
            for cap in &capabilities {
                index
                    .entry(cap.clone())
                    .or_insert_with(Vec::new)
                    .push(name.clone());
            }
        }
    }

    /// Get list of registered agent names
    pub fn list_registered_agents(&self) -> Vec<String> {
        let registry = self.agent_registry.read().unwrap();
        registry.keys().cloned().collect()
    }

    /// Get agent info by name
    pub fn get_agent_info(&self, name: &str) -> Option<AgentInfo> {
        let registry = self.agent_registry.read().unwrap();
        registry.get(name).cloned()
    }

    /// Get list of agent names that have a specific capability
    pub fn agents_with_capability(&self, cap: &str) -> Vec<String> {
        let index = self.capability_index.read().unwrap();
        index.get(cap).cloned().unwrap_or_default()
    }

    /// Update the runtime status of an agent
    pub fn update_agent_status(&self, name: &str, status: serde_json::Value) {
        let mut map = self.agent_status.write().unwrap();
        map.insert(name.to_string(), status);
    }

    /// Get the runtime status of an agent
    pub fn get_agent_status(&self, name: &str) -> Option<serde_json::Value> {
        let map = self.agent_status.read().unwrap();
        map.get(name).cloned()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_bus::message::MsgKind;
    use std::time::SystemTime;

    fn make_test_msg(from: AgentId, to: Option<AgentId>, id: u64) -> Msg {
        Msg {
            id,
            from,
            to,
            kind: MsgKind::Direct,
            payload: serde_json::json!({"test": "data"}),
            timestamp: SystemTime::now(),
        }
    }

    #[tokio::test]
    async fn test_message_bus_creation() {
        let bus = MessageBus::new();
        assert!(bus.list_agents().is_empty());
        assert_eq!(bus.next_message_id(), 1);
        assert_eq!(bus.next_message_id(), 2);
    }

    #[tokio::test]
    async fn test_agent_registration() {
        let bus = MessageBus::new();

        let _rx1 = bus.register_agent(AgentId::Claude);
        let _rx2 = bus.register_agent(AgentId::Glm46);

        let agents = bus.list_agents();
        assert_eq!(agents.len(), 2);
        assert!(agents.contains(&AgentId::Claude));
        assert!(agents.contains(&AgentId::Glm46));
    }

    #[tokio::test]
    async fn test_direct_message_delivery() {
        let bus = MessageBus::new();
        let mut rx = bus.register_agent(AgentId::Claude);

        let msg = make_test_msg(AgentId::Glm46, Some(AgentId::Claude), 1);
        bus.send(msg.clone());

        let received = rx.try_recv().unwrap();
        assert_eq!(received.id, 1);
        assert_eq!(received.from, AgentId::Glm46);
        assert_eq!(received.to, Some(AgentId::Claude));
    }

    #[tokio::test]
    async fn test_broadcast_message() {
        let bus = MessageBus::new();
        let mut rx1 = bus.register_agent(AgentId::Claude);
        let mut rx2 = bus.register_agent(AgentId::Glm46);

        let msg = make_test_msg(AgentId::Internal("system".into()), None, 42);
        bus.send(msg);

        let received1 = rx1.try_recv().unwrap();
        let received2 = rx2.try_recv().unwrap();

        assert_eq!(received1.id, 42);
        assert_eq!(received2.id, 42);
        assert_eq!(received1.to, None);
        assert_eq!(received2.to, None);
    }

    #[tokio::test]
    async fn test_unregister_agent() {
        let bus = MessageBus::new();
        let _rx = bus.register_agent(AgentId::Claude);

        assert_eq!(bus.list_agents().len(), 1);

        bus.unregister_agent(&AgentId::Claude);

        assert!(bus.list_agents().is_empty());
    }

    #[tokio::test]
    async fn test_unregister_prevents_delivery() {
        let bus = MessageBus::new();
        let mut rx = bus.register_agent(AgentId::Claude);

        bus.unregister_agent(&AgentId::Claude);

        let msg = make_test_msg(AgentId::Glm46, Some(AgentId::Claude), 1);
        bus.send(msg);

        // Channel still exists but no sender in registry, so no message
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_next_message_id_monotonic() {
        let bus = MessageBus::new();

        let id1 = bus.next_message_id();
        let id2 = bus.next_message_id();
        let id3 = bus.next_message_id();

        assert!(id1 < id2);
        assert!(id2 < id3);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[tokio::test]
    async fn test_message_history() {
        let bus = MessageBus::new();
        let _rx = bus.register_agent(AgentId::Claude);

        let msg1 = make_test_msg(AgentId::Glm46, Some(AgentId::Claude), 1);
        let msg2 = make_test_msg(AgentId::Glm46, Some(AgentId::Claude), 2);

        bus.send(msg1);
        bus.send(msg2);

        let history = bus.message_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].id, 1);
        assert_eq!(history[1].id, 2);
    }

    #[tokio::test]
    async fn test_replace_agent_on_reregister() {
        let bus = MessageBus::new();

        let _rx1 = bus.register_agent(AgentId::Claude);
        let mut rx2 = bus.register_agent(AgentId::Claude); // Replaces rx1

        let msg = make_test_msg(AgentId::Glm46, Some(AgentId::Claude), 1);
        bus.send(msg);

        // Only rx2 should receive (rx1's sender was replaced)
        assert!(rx2.try_recv().is_ok());
        assert_eq!(bus.list_agents().len(), 1);
    }

    #[tokio::test]
    async fn test_send_to_nonexistent_agent() {
        let bus = MessageBus::new();

        // Send to agent that doesn't exist - should not panic
        let msg = make_test_msg(AgentId::Claude, Some(AgentId::Glm46), 1);
        bus.send(msg);

        // Should be recorded in history even if no delivery
        assert_eq!(bus.message_history().len(), 1);
    }
}
