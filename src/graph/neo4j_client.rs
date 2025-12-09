//! Neo4j Graph Database Client - FROZEN
//!
//! Neo4j integration is disabled. All methods return errors or stubs.
//! This file exists only for compilation compatibility.

use anyhow::Result;

/// Neo4j client stub - disabled
#[derive(Clone)]
pub struct Neo4jClient {
    namespace: String,
}

impl std::fmt::Debug for Neo4jClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Neo4jClient (frozen)").field("namespace", &self.namespace).finish()
    }
}

impl Neo4jClient {
    /// Connect to Neo4j database - STUBBED
    pub async fn connect(_uri: &str, _user: &str, _pass: &str) -> Result<Self> {
        Err(anyhow::anyhow!("Neo4j backend is disabled"))
    }

    /// Get the graph namespace for node identity isolation
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Create a Neo4j client with frozen namespace
    pub fn with_namespace(namespace: String) -> Self {
        Self { namespace }
    }

    /// Execute a Cypher query with parameters - STUBBED
    pub async fn execute_query(
        &self,
        _cypher: &str,
        _params: Vec<(&str, serde_json::Value)>,
    ) -> Result<Vec<serde_json::Value>> {
        Err(anyhow::anyhow!("Neo4j backend is disabled"))
    }

    /// Create a node - STUBBED
    pub async fn create_node(
        &self,
        _label: &str,
        _props: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<i64> {
        Err(anyhow::anyhow!("Neo4j backend is disabled"))
    }

    /// Create a relationship - STUBBED
    pub async fn create_relationship(
        &self,
        _from_id: i64,
        _to_id: i64,
        _rel_type: &str,
        _props: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        Err(anyhow::anyhow!("Neo4j backend is disabled"))
    }

    /// Create a Task node - STUBBED
    pub async fn create_task_node(&self, _id: i64, _title: &str, _status: &str) -> Result<()> {
        Err(anyhow::anyhow!("Neo4j backend is disabled"))
    }

    /// Create a HAS_SUBTASK relationship - STUBBED
    pub async fn create_subtask_relationship(&self, _parent_id: i64, _child_id: i64) -> Result<()> {
        Err(anyhow::anyhow!("Neo4j backend is disabled"))
    }

    /// Create a Memory node - STUBBED
    pub async fn create_memory_node(&self, _key: &str, _value: &str) -> Result<()> {
        Err(anyhow::anyhow!("Neo4j backend is disabled"))
    }

    /// Create an Embedding node - STUBBED
    pub async fn create_embedding_node(&self, _id: i64, _text: &str, _hash: &str) -> Result<()> {
        Err(anyhow::anyhow!("Neo4j backend is disabled"))
    }

    /// Link an embedding to a task - STUBBED
    pub async fn link_embedding_to_task(&self, _embedding_id: i64, _task_id: i64) -> Result<()> {
        Err(anyhow::anyhow!("Neo4j backend is disabled"))
    }
}

// Unit tests moved to tests/neo4j_tests.rs to use real Neo4j instance
// No mocks or stubs allowed per project requirements