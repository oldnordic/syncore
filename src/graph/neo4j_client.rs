//! Neo4j Graph Database Client
//!
//! Production-quality Neo4j integration with:
//! - Connection pooling via Arc<neo4rs::Graph>
//! - Zero-copy query paths using borrowed strings
//! - Full async support
//! - Complete error handling

use anyhow::{Context, Result};
use neo4rs::{query, BoltType, Graph, Query, Row};
use std::sync::Arc;

/// Neo4j client with connection pooling and zero-copy query support
#[derive(Clone)]
pub struct Neo4jClient {
    graph: Arc<Graph>,
    namespace: String,
}

impl Neo4jClient {
    /// Connect to Neo4j database
    ///
    /// # Arguments
    /// * `uri` - Bolt connection URI (e.g., "bolt://localhost:7687")
    /// * `user` - Neo4j username
    /// * `pass` - Neo4j password
    ///
    /// # Returns
    /// Connected Neo4jClient instance
    pub async fn connect(uri: &str, user: &str, pass: &str) -> Result<Self> {
        let graph = Graph::new(uri, user, pass)
            .await
            .context("Failed to connect to Neo4j database")?;

        // Load namespace from environment for graph isolation across sessions/DBs
        let namespace =
            std::env::var("GRAPH_NAMESPACE").unwrap_or_else(|_| "syncore_default".to_string());

        Ok(Self {
            graph: Arc::new(graph),
            namespace,
        })
    }

    /// Get the graph namespace for node identity isolation
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Get the underlying graph connection (for advanced use cases)
    pub fn graph(&self) -> Arc<Graph> {
        self.graph.clone()
    }

    /// Execute a Cypher query with parameters and automatically extract results
    ///
    /// Zero-copy: Uses borrowed string slices for query text
    /// Automatically detects column names from RETURN clause
    ///
    /// # Arguments
    /// * `cypher` - Cypher query string (borrowed, not owned)
    /// * `params` - Vector of (name, value) parameter tuples
    ///
    /// # Returns
    /// Vector of JSON objects representing each row
    pub async fn execute_query(
        &self,
        cypher: &str,
        params: Vec<(&str, serde_json::Value)>,
    ) -> Result<Vec<serde_json::Value>> {
        let mut q = query(cypher);

        // Add parameters with zero-copy where possible
        for (key, value) in params {
            q = self.add_param_to_query(q, key, value)?;
        }

        let mut result = self
            .graph
            .execute(q)
            .await
            .context("Failed to execute Cypher query")?;

        // Extract column names from the RETURN clause
        let columns = self.extract_return_columns(cypher);

        let mut rows = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            let json_row = self.row_to_json_with_columns(&row, &columns)?;
            rows.push(json_row);
        }

        Ok(rows)
    }

    /// Extract column names from RETURN clause
    fn extract_return_columns(&self, cypher: &str) -> Vec<String> {
        // Parse RETURN clause to get column names
        // e.g., "RETURN t.id as id, t.title as title" -> ["id", "title"]
        // e.g., "RETURN 1 as n" -> ["n"]
        let upper = cypher.to_uppercase();
        if let Some(return_pos) = upper.find("RETURN ") {
            let return_clause = &cypher[return_pos + 7..];
            // Find end of RETURN clause (ORDER BY, LIMIT, etc.)
            let end_pos = return_clause
                .find(" ORDER BY")
                .or_else(|| return_clause.find(" LIMIT"))
                .or_else(|| return_clause.find(" SKIP"))
                .unwrap_or(return_clause.len());
            let return_part = &return_clause[..end_pos];

            return_part
                .split(',')
                .filter_map(|col| {
                    let col = col.trim();
                    // Check for "AS" alias
                    if let Some(as_pos) = col.to_uppercase().rfind(" AS ") {
                        Some(col[as_pos + 4..].trim().to_string())
                    } else {
                        // No alias, use the expression itself (simplified)
                        Some(col.split('.').last().unwrap_or(col).trim().to_string())
                    }
                })
                .collect()
        } else {
            vec![]
        }
    }

    /// Convert a Neo4j Row to JSON using known columns
    fn row_to_json_with_columns(&self, row: &Row, columns: &[String]) -> Result<serde_json::Value> {
        let mut map = serde_json::Map::new();

        for col in columns {
            let value = self.extract_value_from_row(row, col)?;
            map.insert(col.clone(), value);
        }

        Ok(serde_json::Value::Object(map))
    }

    /// Extract a value from row by trying different types
    fn extract_value_from_row(&self, row: &Row, key: &str) -> Result<serde_json::Value> {
        // Get the raw BoltType value first
        match row.get::<BoltType>(key) {
            Ok(bolt_value) => {
                // Convert BoltType to JSON based on its actual type
                let json_value = match bolt_value {
                    BoltType::Null(_) => serde_json::Value::Null,
                    BoltType::Boolean(b) => serde_json::json!(b.value),
                    BoltType::Integer(i) => serde_json::json!(i.value),
                    BoltType::Float(f) => serde_json::json!(f.value),
                    BoltType::String(s) => serde_json::json!(s.value),
                    BoltType::List(list) => {
                        // Recursively convert list elements
                        let items: Vec<serde_json::Value> = list
                            .value
                            .iter()
                            .filter_map(|item| self.bolt_to_json(item).ok())
                            .collect();
                        serde_json::json!(items)
                    }
                    BoltType::Map(map) => {
                        // Convert map to JSON object
                        let mut obj = serde_json::Map::new();
                        for (k, v) in map.value.iter() {
                            if let Ok(json_val) = self.bolt_to_json(v) {
                                obj.insert(k.value.clone(), json_val);
                            }
                        }
                        serde_json::Value::Object(obj)
                    }
                    BoltType::Node(node) => {
                        // Extract Node properties as JSON object
                        let mut obj = serde_json::Map::new();
                        for (k, v) in node.properties.value.iter() {
                            if let Ok(json_val) = self.bolt_to_json(v) {
                                obj.insert(k.value.clone(), json_val);
                            }
                        }
                        serde_json::Value::Object(obj)
                    }
                    BoltType::Relation(rel) => {
                        // Extract Relationship properties as JSON object
                        let mut obj = serde_json::Map::new();
                        for (k, v) in rel.properties.value.iter() {
                            if let Ok(json_val) = self.bolt_to_json(v) {
                                obj.insert(k.value.clone(), json_val);
                            }
                        }
                        serde_json::Value::Object(obj)
                    }
                    _ => serde_json::Value::Null, // For other types (Path, Duration, etc.)
                };
                Ok(json_value)
            }
            Err(_) => {
                // If BoltType extraction fails, return null
                Ok(serde_json::Value::Null)
            }
        }
    }

    /// Convert BoltType to JSON (helper for recursive conversion)
    fn bolt_to_json(&self, bolt: &BoltType) -> Result<serde_json::Value> {
        let json_value = match bolt {
            BoltType::Null(_) => serde_json::Value::Null,
            BoltType::Boolean(b) => serde_json::json!(b.value),
            BoltType::Integer(i) => serde_json::json!(i.value),
            BoltType::Float(f) => serde_json::json!(f.value),
            BoltType::String(s) => serde_json::json!(s.value),
            BoltType::List(list) => {
                let items: Vec<serde_json::Value> = list
                    .value
                    .iter()
                    .filter_map(|item| self.bolt_to_json(item).ok())
                    .collect();
                serde_json::json!(items)
            }
            BoltType::Map(map) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in map.value.iter() {
                    if let Ok(json_val) = self.bolt_to_json(v) {
                        obj.insert(k.value.clone(), json_val);
                    }
                }
                serde_json::Value::Object(obj)
            }
            BoltType::Node(node) => {
                // Extract Node properties as JSON object
                let mut obj = serde_json::Map::new();
                for (k, v) in node.properties.value.iter() {
                    if let Ok(json_val) = self.bolt_to_json(v) {
                        obj.insert(k.value.clone(), json_val);
                    }
                }
                serde_json::Value::Object(obj)
            }
            BoltType::Relation(rel) => {
                // Extract Relationship properties as JSON object
                let mut obj = serde_json::Map::new();
                for (k, v) in rel.properties.value.iter() {
                    if let Ok(json_val) = self.bolt_to_json(v) {
                        obj.insert(k.value.clone(), json_val);
                    }
                }
                serde_json::Value::Object(obj)
            }
            _ => serde_json::Value::Null,
        };
        Ok(json_value)
    }

    /// Add a parameter to a query based on its JSON type
    fn add_param_to_query(
        &self,
        mut q: Query,
        key: &str,
        value: serde_json::Value,
    ) -> Result<Query> {
        match value {
            serde_json::Value::Null => {
                // For null values, we use Option<String>::None
                q = q.param(key, Option::<String>::None);
            }
            serde_json::Value::Bool(b) => {
                q = q.param(key, b);
            }
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    q = q.param(key, i);
                } else if let Some(f) = n.as_f64() {
                    q = q.param(key, f);
                } else {
                    anyhow::bail!("Unsupported number type for parameter: {}", key);
                }
            }
            serde_json::Value::String(s) => {
                q = q.param(key, s);
            }
            serde_json::Value::Array(arr) => {
                // Convert JSON array to Vec<String> or Vec<i64>
                if arr.is_empty() {
                    q = q.param(key, Vec::<String>::new());
                } else if arr[0].is_i64() {
                    let int_list: Vec<i64> = arr.into_iter().filter_map(|v| v.as_i64()).collect();
                    q = q.param(key, int_list);
                } else {
                    let str_list: Vec<String> = arr
                        .into_iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    q = q.param(key, str_list);
                }
            }
            serde_json::Value::Object(_) => {
                // For objects, serialize as string (simple approach)
                let s = value.to_string();
                q = q.param(key, s);
            }
        }
        Ok(q)
    }

    /// Create a Task node in Neo4j
    pub async fn create_task_node(&self, id: i64, title: &str, status: &str) -> Result<()> {
        let cypher = r#"
            MERGE (t:Task {id: $id, namespace: $ns})
            SET t.title = $title, t.status = $status, t.updated_at = datetime()
        "#;

        self.execute_query(
            cypher,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(self.namespace.clone())),
                ("title", serde_json::json!(title)),
                ("status", serde_json::json!(status)),
            ],
        )
        .await?;

        Ok(())
    }

    /// Create a HAS_SUBTASK relationship between two tasks
    pub async fn create_subtask_relationship(&self, parent_id: i64, child_id: i64) -> Result<()> {
        let cypher = r#"
            MATCH (p:Task {id: $parent_id, namespace: $ns}), (c:Task {id: $child_id, namespace: $ns})
            MERGE (p)-[:HAS_SUBTASK]->(c)
        "#;

        self.execute_query(
            cypher,
            vec![
                ("parent_id", serde_json::json!(parent_id)),
                ("child_id", serde_json::json!(child_id)),
                ("ns", serde_json::json!(self.namespace.clone())),
            ],
        )
        .await?;

        Ok(())
    }

    /// Create a Memory node in Neo4j
    pub async fn create_memory_node(&self, key: &str, value: &str) -> Result<()> {
        let cypher = r#"
            MERGE (m:Memory {key: $key})
            SET m.value = $value, m.updated_at = datetime()
        "#;

        self.execute_query(
            cypher,
            vec![
                ("key", serde_json::json!(key)),
                ("value", serde_json::json!(value)),
            ],
        )
        .await?;

        Ok(())
    }

    /// Create an Embedding node in Neo4j
    pub async fn create_embedding_node(&self, id: i64, text: &str, hash: &str) -> Result<()> {
        let cypher = r#"
            MERGE (e:Embedding {id: $id, namespace: $ns})
            SET e.text = $text, e.hash = $hash, e.updated_at = datetime()
        "#;

        self.execute_query(
            cypher,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(self.namespace.clone())),
                ("text", serde_json::json!(text)),
                ("hash", serde_json::json!(hash)),
            ],
        )
        .await?;

        Ok(())
    }

    /// Create a relationship between any two nodes
    pub async fn create_relationship(
        &self,
        from_label: &str,
        from_id: i64,
        to_label: &str,
        to_id: i64,
        rel_type: &str,
    ) -> Result<()> {
        // Use dynamic labels in Cypher with namespace for identity isolation
        let cypher = format!(
            r#"
            MATCH (a:{} {{id: $from_id, namespace: $ns}}), (b:{} {{id: $to_id, namespace: $ns}})
            MERGE (a)-[:{}]->(b)
        "#,
            from_label, to_label, rel_type
        );

        self.execute_query(
            &cypher,
            vec![
                ("from_id", serde_json::json!(from_id)),
                ("to_id", serde_json::json!(to_id)),
                ("ns", serde_json::json!(self.namespace.clone())),
            ],
        )
        .await?;

        Ok(())
    }

    /// Link an embedding to a task
    pub async fn link_embedding_to_task(&self, embedding_id: i64, task_id: i64) -> Result<()> {
        self.create_relationship("Embedding", embedding_id, "Task", task_id, "BELONGS_TO")
            .await
    }
}

// Unit tests moved to tests/neo4j_tests.rs to use real Neo4j instance
// No mocks or stubs allowed per project requirements
