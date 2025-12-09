//! SQLiteGraph Backend Implementation
//!
//! Provides a SQLite-based implementation of the GraphBackend trait.
//! This backend uses the existing CodeGraph SQLite schema and operations
//! while providing deterministic ordering and Neo4j-compatible behavior.

use super::*;
use crate::code_graph::{CodeEntity, CodeGraph, EntityType};
use crate::vector::VectorStore;
use anyhow::{Context, Result};
use async_trait::async_trait;
use rusqlite::Connection;
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// SQLiteGraph backend implementation
#[derive(Clone, Debug)]
pub struct SQLiteGraphBackend {
    code_graph: Arc<CodeGraph>,
    namespace: String,
}

impl SQLiteGraphBackend {
    /// Create a new SQLiteGraph backend
    ///
    /// # Arguments
    /// * `path` - Path to SQLite database file
    /// * `namespace` - Namespace for multi-tenant isolation
    pub async fn new(path: &str, namespace: &str) -> Result<Self> {
        // Create a dummy vector store (required by CodeGraph but not used for graph operations)
        let embeddings = Box::new(crate::vector::StubEmbeddings::new(384)?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        // Create CodeGraph instance
        let code_graph = Arc::new(CodeGraph::new(path, vector_store)?);

        Ok(Self {
            code_graph,
            namespace: namespace.to_string(),
        })
    }

    /// Get the underlying CodeGraph (for advanced use cases)
    pub fn code_graph(&self) -> &Arc<CodeGraph> {
        &self.code_graph
    }

    /// Convert EntityType to backend NodeLabel
    fn entity_type_to_node_label(entity_type: EntityType) -> NodeLabel {
        match entity_type {
            EntityType::Function => NodeLabel::Function,
            EntityType::Struct => NodeLabel::Struct,
            EntityType::Enum => NodeLabel::Enum,
            EntityType::Trait => NodeLabel::Trait,
            EntityType::Class => NodeLabel::Struct, // Map Class to Struct for compatibility
            EntityType::Method => NodeLabel::Function, // Map Method to Function
            EntityType::Import => NodeLabel::Import,
            EntityType::Constant => NodeLabel::Struct, // Map Constant to Struct for now
        }
    }

    /// Convert backend NodeLabel to EntityType
    fn node_label_to_entity_type(label: NodeLabel) -> EntityType {
        match label {
            NodeLabel::Function => EntityType::Function,
            NodeLabel::Struct => EntityType::Struct,
            NodeLabel::Enum => EntityType::Enum,
            NodeLabel::Trait => EntityType::Trait,
            NodeLabel::Impl => EntityType::Struct, // Map Impl to Struct
            NodeLabel::Module => EntityType::Struct, // Map Module to Struct
            NodeLabel::Import => EntityType::Import,
            NodeLabel::Constant => EntityType::Struct, // Map Constant to Struct
            NodeLabel::TypeAlias => EntityType::Struct, // Map TypeAlias to Struct
            NodeLabel::File => EntityType::Struct, // Map File to Struct (no File EntityType exists)
        }
    }

    /// Convert CodeEntity to backend NodeProperties
    fn code_entity_to_node_properties(entity: &CodeEntity) -> NodeProperties {
        NodeProperties {
            id: entity.id.unwrap_or(0),
            name: entity.name.clone(),
            path: Some(entity.file_path.clone()),
            start_line: Some(entity.line_start as i64),
            end_line: Some(entity.line_end as i64),
            signature: entity.signature.clone(),
            body_snippet: entity.body_snippet.clone(),
            docstring: entity.docstring.clone(),
            hash: None, // Not available in CodeEntity
            language: Some(entity.language.clone()),
            file_sha256: None, // Not available in CodeEntity
            mtime: None,       // Not available in CodeEntity
            created_at: Some(chrono::Utc::now().to_rfc3339()), // Use current time
            last_modified_at: Some(chrono::Utc::now().to_rfc3339()), // Use current time
            change_count: Some(1), // Default to 1 for new entities
            author_count: Some(1), // Default to 1 for new entities
        }
    }

    /// Execute a SQL query with parameters and return JSON results
    async fn execute_sql_query(
        &self,
        query: &str,
        params: Vec<(&str, Value)>,
    ) -> Result<Vec<Value>> {
        let conn = self.code_graph.db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        let mut stmt =
            db.prepare(query).with_context(|| format!("Failed to prepare query: {}", query))?;

        // Convert parameters to rusqlite types using owned values
        let rusqlite_params: Vec<rusqlite::types::Value> = params
            .iter()
            .map(|(_, value)| match value {
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        rusqlite::types::Value::Integer(i)
                    } else if let Some(f) = n.as_f64() {
                        rusqlite::types::Value::Real(f)
                    } else {
                        rusqlite::types::Value::Null
                    }
                }
                Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                Value::Bool(b) => rusqlite::types::Value::Integer(*b as i64),
                Value::Null => rusqlite::types::Value::Null,
                _ => rusqlite::types::Value::Null,
            })
            .collect();

        // Convert to references for query_map
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            rusqlite_params.iter().map(|v| v as &dyn rusqlite::ToSql).collect();

        let rows = stmt.query_map(&param_refs[..], |row| {
            let mut obj = serde_json::Map::new();

            // Get all column names and values
            for i in 0..row.as_ref().column_count() {
                let name = row.as_ref().column_name(i).unwrap_or("col_unknown");
                {
                    let value = match row.get::<_, Option<i64>>(i) {
                        Ok(Some(val)) => Value::Number(val.into()),
                        Ok(None) => Value::Null,
                        Err(_) => {
                            // Try as string
                            match row.get::<_, Option<String>>(i) {
                                Ok(Some(val)) => Value::String(val),
                                Ok(None) => Value::Null,
                                Err(_) => Value::Null,
                            }
                        }
                    };
                    obj.insert(name.to_string(), value);
                }
            }

            Ok(Value::Object(obj))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Get entity by ID from SQLite
    async fn get_entity_from_sqlite(&self, id: i64) -> Result<Option<EntityResult>> {
        let query = r#"
            SELECT id, name, entity_type, file_path, line_start, line_end, 
                   signature, docstring, language, body_snippet
            FROM code_entities 
            WHERE id = ?
            ORDER BY id
        "#;

        let results = self.execute_sql_query(query, vec![("id", Value::Number(id.into()))]).await?;

        if results.is_empty() {
            return Ok(None);
        }

        let entity = &results[0];
        if let Value::Object(obj) = entity {
            let label = obj.get("entity_type").and_then(|v| v.as_str()).unwrap_or("").to_string();

            Ok(Some(EntityResult {
                id: obj.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                name: obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                label,
                path: obj.get("file_path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                start_line: obj.get("line_start").and_then(|v| v.as_i64()),
                end_line: obj.get("line_end").and_then(|v| v.as_i64()),
                signature: obj.get("signature").and_then(|v| v.as_str()).map(|s| s.to_string()),
                body_snippet: obj
                    .get("body_snippet")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                created_at: None,       // Not available in SQLite schema
                last_modified_at: None, // Not available in SQLite schema
                change_count: None,     // Not available in SQLite schema
                author_count: None,     // Not available in SQLite schema
            }))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl GraphBackend for SQLiteGraphBackend {
    async fn connect(uri: &str, user: &str, pass: &str, namespace: &str) -> Result<Self>
    where
        Self: Sized,
    {
        // For SQLite, we ignore user/pass and use uri as database path
        let db_path = if uri.is_empty() {
            "synapse.db"
        } else {
            uri
        };
        Self::new(db_path, namespace).await
    }

    fn namespace(&self) -> &str {
        &self.namespace
    }

    async fn execute_query(&self, query: &str, params: Vec<(&str, Value)>) -> Result<Vec<Value>> {
        // For SQLite, we need to convert Cypher-like queries to SQL
        // This is a simplified implementation - in practice, you'd want a full Cypher-to-SQL translator
        if query.contains("MATCH") && query.contains("RETURN") {
            // Handle count query
            if query.contains("count(n)") {
                let sql_query = "SELECT COUNT(*) as count FROM code_entities";
                return self.execute_sql_query(sql_query, vec![]).await;
            }

            // Simple MATCH query - convert to SELECT
            let sql_query = query
                .replace("MATCH (n)", "SELECT * FROM code_entities n")
                .replace("RETURN n", "RETURN *")
                .replace("WHERE n.id = $id", "WHERE id = ?");

            self.execute_sql_query(&sql_query, params).await
        } else {
            // Try to execute as SQL directly
            self.execute_sql_query(query, params).await
        }
    }

    // === Entity Operations ===

    async fn upsert_entity(&self, label: NodeLabel, props: NodeProperties) -> Result<()> {
        // Insert into SQLite using existing CodeGraph infrastructure
        let conn = self.code_graph.db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        // Handle NULL values for File entities properly
        let line_start_val: Option<usize> = props.start_line.map(|v| v as usize);
        let line_end_val: Option<usize> = props.end_line.map(|v| v as usize);

        db.execute(
            r#"
                INSERT OR REPLACE INTO code_entities 
                (id, file_path, entity_type, name, signature, line_start, line_end, 
                 docstring, language, body_snippet, indexed_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            rusqlite::params![
                props.id,
                props.path.unwrap_or_default(),
                label.as_str(), // Store original NodeLabel string directly
                props.name,
                props.signature,
                line_start_val,
                line_end_val,
                props.docstring,
                props.language.unwrap_or_else(|| "unknown".to_string()),
                props.body_snippet,
                chrono::Utc::now().timestamp()
            ],
        )?;

        Ok(())
    }

    async fn delete_entity(&self, id: i64) -> Result<()> {
        let conn = self.code_graph.db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        // Delete from code_entities (cascades to code_edges)
        db.execute("DELETE FROM code_entities WHERE id = ?", [id])?;

        Ok(())
    }

    async fn delete_file_entities(&self, file_path: &str) -> Result<usize> {
        let conn = self.code_graph.db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        let rows_affected =
            db.execute("DELETE FROM code_entities WHERE file_path = ?", [file_path])?;

        Ok(rows_affected as usize)
    }

    async fn batch_upsert_entities(
        &self,
        label: NodeLabel,
        entities: Vec<NodeProperties>,
        batch_size: usize,
    ) -> Result<usize> {
        let mut processed = 0;

        for chunk in entities.chunks(batch_size) {
            for props in chunk {
                self.upsert_entity(label, props.clone()).await?;
                processed += 1;
            }
        }

        Ok(processed)
    }

    // === Relationship Operations ===

    async fn create_relationship(
        &self,
        src_id: i64,
        dst_id: i64,
        rel_type: RelationType,
    ) -> Result<()> {
        let conn = self.code_graph.db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        db.execute(
            r#"
                INSERT OR REPLACE INTO code_edges (src_entity_id, dst_entity_id, edge_type)
                VALUES (?, ?, ?)
            "#,
            rusqlite::params![src_id, dst_id, rel_type.as_str()],
        )?;

        Ok(())
    }

    async fn batch_create_relationships(
        &self,
        relationships: Vec<(i64, i64, RelationType)>,
        batch_size: usize,
    ) -> Result<usize> {
        let mut processed = 0;

        for chunk in relationships.chunks(batch_size) {
            for (src_id, dst_id, rel_type) in chunk {
                self.create_relationship(*src_id, *dst_id, *rel_type).await?;
                processed += 1;
            }
        }

        Ok(processed)
    }

    async fn create_file_dependency(&self, from_path: &str, to_path: &str) -> Result<()> {
        // Create file nodes if they don't exist
        self.upsert_file_by_path(from_path).await?;
        self.upsert_file_by_path(to_path).await?;

        // Get file IDs
        let from_id = self.get_file_id_by_path(from_path).await?;
        let to_id = self.get_file_id_by_path(to_path).await?;

        if let (Some(from), Some(to)) = (from_id, to_id) {
            self.create_relationship(from, to, RelationType::DependsOn).await?;
        }

        Ok(())
    }

    async fn upsert_file_by_path(&self, file_path: &str) -> Result<()> {
        // Generate deterministic ID for File nodes based on path hash (same as Neo4j)
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        file_path.hash(&mut hasher);
        let file_id = hasher.finish() as i64;

        let props = NodeProperties {
            id: file_id, // Use same hash-based ID as Neo4j
            name: std::path::Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            path: Some(file_path.to_string()),
            start_line: None,
            end_line: None,
            signature: None,
            body_snippet: None,
            docstring: None,
            hash: None,
            language: None,
            file_sha256: None,
            mtime: None,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        };

        self.upsert_entity(NodeLabel::File, props).await
    }

    // === Query Operations ===

    async fn get_entity_by_id(&self, id: i64) -> Result<Option<EntityResult>> {
        self.get_entity_from_sqlite(id).await
    }

    async fn get_file_entities(&self, file_path: &str) -> Result<Vec<EntityResult>> {
        let query = r#"
            SELECT id, name, entity_type, file_path, line_start, line_end,
                   signature, docstring, language, body_snippet
            FROM code_entities 
            WHERE file_path = ?
            ORDER BY line_start, id
        "#;

        let results = self
            .execute_sql_query(query, vec![("file_path", Value::String(file_path.to_string()))])
            .await?;

        let mut entities = Vec::new();
        for result in results {
            if let Value::Object(obj) = result {
                entities.push(EntityResult {
                    id: obj.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                    name: obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    label: obj
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    path: obj.get("file_path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    start_line: obj.get("line_start").and_then(|v| v.as_i64()),
                    end_line: obj.get("line_end").and_then(|v| v.as_i64()),
                    signature: obj.get("signature").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    body_snippet: obj
                        .get("body_snippet")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    created_at: None,
                    last_modified_at: None,
                    change_count: None,
                    author_count: None,
                });
            }
        }

        Ok(entities)
    }

    async fn get_function_callees(&self, function_id: i64) -> Result<Vec<EntityResult>> {
        let query = r#"
            SELECT e.id, e.name, e.entity_type, e.file_path, e.line_start, e.line_end,
                   e.signature, e.docstring, e.language, e.body_snippet
            FROM code_entities e
            JOIN code_edges edge ON e.id = edge.dst_entity_id
            WHERE edge.src_entity_id = ? AND edge.edge_type = 'CALLS'
            ORDER BY e.name, e.id
        "#;

        let results = self
            .execute_sql_query(query, vec![("function_id", Value::Number(function_id.into()))])
            .await?;

        let mut callees = Vec::new();
        for result in results {
            if let Value::Object(obj) = result {
                callees.push(EntityResult {
                    id: obj.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                    name: obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    label: obj
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    path: obj.get("file_path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    start_line: obj.get("line_start").and_then(|v| v.as_i64()),
                    end_line: obj.get("line_end").and_then(|v| v.as_i64()),
                    signature: obj.get("signature").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    body_snippet: obj
                        .get("body_snippet")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    created_at: None,
                    last_modified_at: None,
                    change_count: None,
                    author_count: None,
                });
            }
        }

        Ok(callees)
    }

    async fn get_function_callers(&self, function_id: i64) -> Result<Vec<EntityResult>> {
        let query = r#"
            SELECT e.id, e.name, e.entity_type, e.file_path, e.line_start, e.line_end,
                   e.signature, e.docstring, e.language, e.body_snippet
            FROM code_entities e
            JOIN code_edges edge ON e.id = edge.src_entity_id
            WHERE edge.dst_entity_id = ? AND edge.edge_type = 'CALLS'
            ORDER BY e.name, e.id
        "#;

        let results = self
            .execute_sql_query(query, vec![("function_id", Value::Number(function_id.into()))])
            .await?;

        let mut callers = Vec::new();
        for result in results {
            if let Value::Object(obj) = result {
                callers.push(EntityResult {
                    id: obj.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                    name: obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    label: obj
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    path: obj.get("file_path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    start_line: obj.get("line_start").and_then(|v| v.as_i64()),
                    end_line: obj.get("line_end").and_then(|v| v.as_i64()),
                    signature: obj.get("signature").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    body_snippet: obj
                        .get("body_snippet")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    created_at: None,
                    last_modified_at: None,
                    change_count: None,
                    author_count: None,
                });
            }
        }

        Ok(callers)
    }

    async fn find_entities_by_name(&self, name: &str) -> Result<Vec<EntityResult>> {
        let query = r#"
            SELECT id, name, entity_type, file_path, line_start, line_end,
                   signature, docstring, language, body_snippet
            FROM code_entities 
            WHERE name LIKE ? || '%'
            ORDER BY file_path, line_start, id
        "#;

        let results =
            self.execute_sql_query(query, vec![("name", Value::String(name.to_string()))]).await?;

        let mut entities = Vec::new();
        for result in results {
            if let Value::Object(obj) = result {
                entities.push(EntityResult {
                    id: obj.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                    name: obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    label: obj
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    path: obj.get("file_path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    start_line: obj.get("line_start").and_then(|v| v.as_i64()),
                    end_line: obj.get("line_end").and_then(|v| v.as_i64()),
                    signature: obj.get("signature").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    body_snippet: obj
                        .get("body_snippet")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    created_at: None,
                    last_modified_at: None,
                    change_count: None,
                    author_count: None,
                });
            }
        }

        Ok(entities)
    }

    async fn get_entities_by_type(&self, label: NodeLabel) -> Result<Vec<EntityResult>> {
        let entity_type_str = format!("{:?}", Self::node_label_to_entity_type(label));
        let query = r#"
            SELECT id, name, entity_type, file_path, line_start, line_end,
                   signature, docstring, language, body_snippet
            FROM code_entities 
            WHERE entity_type = ?
            ORDER BY name, file_path, line_start, id
        "#;

        let results = self
            .execute_sql_query(query, vec![("entity_type", Value::String(entity_type_str))])
            .await?;

        let mut entities = Vec::new();
        for result in results {
            if let Value::Object(obj) = result {
                entities.push(EntityResult {
                    id: obj.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                    name: obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    label: obj
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    path: obj.get("file_path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    start_line: obj.get("line_start").and_then(|v| v.as_i64()),
                    end_line: obj.get("line_end").and_then(|v| v.as_i64()),
                    signature: obj.get("signature").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    body_snippet: obj
                        .get("body_snippet")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    created_at: None,
                    last_modified_at: None,
                    change_count: None,
                    author_count: None,
                });
            }
        }

        Ok(entities)
    }

    async fn get_neighbors(&self, entity_id: i64) -> Result<Vec<EntityResult>> {
        let query = r#"
            SELECT DISTINCT e.id, e.name, e.entity_type, e.file_path, e.line_start, e.line_end,
                   e.signature, e.docstring, e.language, e.body_snippet
            FROM code_entities e
            JOIN code_edges edge1 ON e.id = edge1.dst_entity_id
            WHERE edge1.src_entity_id = ?
            UNION
            SELECT DISTINCT e.id, e.name, e.entity_type, e.file_path, e.line_start, e.line_end,
                   e.signature, e.docstring, e.language, e.body_snippet
            FROM code_entities e
            JOIN code_edges edge2 ON e.id = edge2.src_entity_id
            WHERE edge2.dst_entity_id = ?
            ORDER BY e.name, e.id
        "#;

        let results = self
            .execute_sql_query(
                query,
                vec![
                    ("entity_id", Value::Number(entity_id.into())),
                    ("entity_id", Value::Number(entity_id.into())),
                ],
            )
            .await?;

        let mut neighbors = Vec::new();
        for result in results {
            if let Value::Object(obj) = result {
                neighbors.push(EntityResult {
                    id: obj.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                    name: obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    label: obj
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    path: obj.get("file_path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    start_line: obj.get("line_start").and_then(|v| v.as_i64()),
                    end_line: obj.get("line_end").and_then(|v| v.as_i64()),
                    signature: obj.get("signature").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    body_snippet: obj
                        .get("body_snippet")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    created_at: None,
                    last_modified_at: None,
                    change_count: None,
                    author_count: None,
                });
            }
        }

        Ok(neighbors)
    }

    async fn find_orphan_entities(&self) -> Result<Vec<EntityResult>> {
        let query = r#"
            SELECT e.id, e.name, e.entity_type, e.file_path, e.line_start, e.line_end,
                   e.signature, e.docstring, e.language, e.body_snippet
            FROM code_entities e
            LEFT JOIN code_edges edge1 ON e.id = edge1.src_entity_id
            LEFT JOIN code_edges edge2 ON e.id = edge2.dst_entity_id
            WHERE edge1.src_entity_id IS NULL AND edge2.dst_entity_id IS NULL
            ORDER BY e.name, e.id
        "#;

        let results = self.execute_sql_query(query, vec![]).await?;

        let mut orphans = Vec::new();
        for result in results {
            if let Value::Object(obj) = result {
                orphans.push(EntityResult {
                    id: obj.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                    name: obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    label: obj
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    path: obj.get("file_path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    start_line: obj.get("line_start").and_then(|v| v.as_i64()),
                    end_line: obj.get("line_end").and_then(|v| v.as_i64()),
                    signature: obj.get("signature").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    body_snippet: obj
                        .get("body_snippet")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    created_at: None,
                    last_modified_at: None,
                    change_count: None,
                    author_count: None,
                });
            }
        }

        Ok(orphans)
    }

    // === Statistics and Validation ===

    async fn count_entities_by_type(&self) -> Result<Vec<(String, i64)>> {
        let query = r#"
            SELECT entity_type, COUNT(*) as count
            FROM code_entities
            GROUP BY entity_type
            ORDER BY entity_type
        "#;

        let results = self.execute_sql_query(query, vec![]).await?;

        let mut counts = Vec::new();
        for result in results {
            if let Value::Object(obj) = result {
                if let (Some(entity_type), Some(count)) = (
                    obj.get("entity_type").and_then(|v| v.as_str()),
                    obj.get("count").and_then(|v| v.as_i64()),
                ) {
                    counts.push((entity_type.to_string(), count));
                }
            }
        }

        Ok(counts)
    }

    async fn validate_structure(&self) -> Result<GraphStats> {
        let total_nodes_query = "SELECT COUNT(*) as count FROM code_entities";
        let total_edges_query = "SELECT COUNT(*) as count FROM code_edges";
        let orphan_query = r#"
            SELECT COUNT(*) as count
            FROM code_entities e
            LEFT JOIN code_edges edge1 ON e.id = edge1.src_entity_id
            LEFT JOIN code_edges edge2 ON e.id = edge2.dst_entity_id
            WHERE edge1.src_entity_id IS NULL AND edge2.dst_entity_id IS NULL
        "#;

        let total_nodes_result = self.execute_sql_query(total_nodes_query, vec![]).await?;
        let total_edges_result = self.execute_sql_query(total_edges_query, vec![]).await?;
        let orphan_result = self.execute_sql_query(orphan_query, vec![]).await?;

        let total_nodes = total_nodes_result
            .first()
            .and_then(|r| r.get("count"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let total_edges = total_edges_result
            .first()
            .and_then(|r| r.get("count"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let orphan_count = orphan_result
            .first()
            .and_then(|r| r.get("count"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let entity_types = self.count_entities_by_type().await?;
        let edge_types = self.count_edges_by_type().await?;

        Ok(GraphStats {
            total_nodes,
            total_edges,
            orphan_count,
            entity_types,
            edge_types,
        })
    }

    // === Metadata Operations ===

    async fn update_git_metadata(
        &self,
        _id: i64,
        _created_at: Option<String>,
        _last_modified_at: Option<String>,
        _change_count: Option<i64>,
        _author_count: Option<i64>,
    ) -> Result<()> {
        // SQLite schema doesn't have these columns, so this is a no-op
        // In a real implementation, you'd add these columns to the schema
        Ok(())
    }

    // === Specialized Operations ===

    async fn create_task_node(&self, id: i64, title: &str, status: &str) -> Result<()> {
        // SQLite doesn't have specialized task tables - create as generic entity
        let props = NodeProperties {
            id,
            name: title.to_string(),
            path: None,
            start_line: None,
            end_line: None,
            signature: Some(format!("status: {}", status)),
            body_snippet: None,
            docstring: None,
            hash: None,
            language: Some("task".to_string()),
            file_sha256: None,
            mtime: None,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        };

        self.upsert_entity(NodeLabel::Constant, props).await // Use Constant as generic type
    }

    async fn create_subtask_relationship(&self, parent_id: i64, child_id: i64) -> Result<()> {
        self.create_relationship(parent_id, child_id, RelationType::Contains).await
    }

    async fn create_memory_node(&self, key: &str, value: &str) -> Result<()> {
        let props = NodeProperties {
            id: self.get_next_id().await?,
            name: key.to_string(),
            path: None,
            start_line: None,
            end_line: None,
            signature: Some(value.to_string()),
            body_snippet: None,
            docstring: None,
            hash: None,
            language: Some("memory".to_string()),
            file_sha256: None,
            mtime: None,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        };

        self.upsert_entity(NodeLabel::Constant, props).await // Use Constant as generic type
    }

    async fn create_embedding_node(&self, id: i64, text: &str, hash: &str) -> Result<()> {
        let props = NodeProperties {
            id,
            name: hash.to_string(),
            path: None,
            start_line: None,
            end_line: None,
            signature: Some(text.to_string()),
            body_snippet: None,
            docstring: None,
            hash: Some(hash.to_string()),
            language: Some("embedding".to_string()),
            file_sha256: None,
            mtime: None,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        };

        self.upsert_entity(NodeLabel::Constant, props).await // Use Constant as generic type
    }

    async fn link_embedding_to_task(&self, embedding_id: i64, task_id: i64) -> Result<()> {
        self.create_relationship(embedding_id, task_id, RelationType::Uses).await
    }
}

// Private helper methods
impl SQLiteGraphBackend {
    /// Get the next available ID for entity creation
    async fn get_next_id(&self) -> Result<i64> {
        let conn = self.code_graph.db_conn();
        let db = conn.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?;

        let next_id: i64 =
            db.query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM code_entities", [], |row| {
                row.get(0)
            })?;

        Ok(next_id)
    }

    /// Get file ID by path
    async fn get_file_id_by_path(&self, file_path: &str) -> Result<Option<i64>> {
        let query = r#"
            SELECT id FROM code_entities 
            WHERE file_path = ? AND entity_type = 'File'
            ORDER BY id
            LIMIT 1
        "#;

        let results = self
            .execute_sql_query(query, vec![("file_path", Value::String(file_path.to_string()))])
            .await?;

        if let Some(result) = results.first() {
            if let Value::Object(obj) = result {
                Ok(obj.get("id").and_then(|v| v.as_i64()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Count edges by type
    async fn count_edges_by_type(&self) -> Result<Vec<(String, i64)>> {
        let query = r#"
            SELECT edge_type, COUNT(*) as count
            FROM code_edges
            GROUP BY edge_type
            ORDER BY edge_type
        "#;

        let results = self.execute_sql_query(query, vec![]).await?;

        let mut counts = Vec::new();
        for result in results {
            if let Value::Object(obj) = result {
                if let (Some(edge_type), Some(count)) = (
                    obj.get("edge_type").and_then(|v| v.as_str()),
                    obj.get("count").and_then(|v| v.as_i64()),
                ) {
                    counts.push((edge_type.to_string(), count));
                }
            }
        }

        Ok(counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;

    #[test]
    fn test_entity_type_conversion() {
        assert_eq!(
            SQLiteGraphBackend::entity_type_to_node_label(EntityType::Function),
            NodeLabel::Function
        );
        assert_eq!(
            SQLiteGraphBackend::node_label_to_entity_type(NodeLabel::Function),
            EntityType::Function
        );
    }

    #[tokio::test]
    async fn test_sqlitegraph_connect() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();
        let backend = SQLiteGraphBackend::connect(db_path_str, "", "", "test").await?;
        assert_eq!(backend.namespace(), "test");
        Ok(())
    }
}
