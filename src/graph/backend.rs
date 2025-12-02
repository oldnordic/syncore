//! Graph Backend Trait - Database Abstraction Layer
//!
//! Provides a unified interface for different graph database implementations
//! while maintaining compatibility with the existing Neo4j API surface.
//!
//! This trait abstracts:
//! - Connection management and namespace isolation
//! - Entity CRUD operations (upsert/delete)
//! - Relationship creation operations
//! - Query execution with parameter binding
//! - Error handling patterns
//! - Namespace-based multi-tenancy

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Entity result from graph query
#[derive(Debug, Clone)]
pub struct EntityResult {
    pub id: i64,
    pub name: String,
    pub label: String,
    pub path: Option<String>,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
    pub signature: Option<String>,
    pub body_snippet: Option<String>,
    pub created_at: Option<String>,
    pub last_modified_at: Option<String>,
    pub change_count: Option<i64>,
    pub author_count: Option<i64>,
}

/// Graph statistics for validation
#[derive(Debug)]
pub struct GraphStats {
    pub total_nodes: i64,
    pub total_edges: i64,
    pub orphan_count: i64,
    pub entity_types: Vec<(String, i64)>,
    pub edge_types: Vec<(String, i64)>,
}

/// Node labels - must match Neo4j schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeLabel {
    File,
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    Import,
    Constant,
    TypeAlias,
}

impl NodeLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Function => "Function",
            Self::Struct => "Struct",
            Self::Enum => "Enum",
            Self::Trait => "Trait",
            Self::Impl => "Impl",
            Self::Module => "Module",
            Self::Import => "Import",
            Self::Constant => "Constant",
            Self::TypeAlias => "TypeAlias",
        }
    }
}

/// Relationship types - must match Neo4j schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationType {
    Declares,
    Calls,
    HasMember,
    Implements,
    Imports,
    Uses,
    Owns,
    References,
    Inherits,
    Contains,
    UsesField,
    UsesType,
    ModuleChild,
    DependsOn,
}

impl RelationType {
    pub fn try_parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "DECLARES" => Some(Self::Declares),
            "CALLS" => Some(Self::Calls),
            "HAS_MEMBER" => Some(Self::HasMember),
            "IMPLEMENTS" => Some(Self::Implements),
            "IMPORTS" => Some(Self::Imports),
            "USES" => Some(Self::Uses),
            "OWNS" => Some(Self::Owns),
            "REFERENCES" => Some(Self::References),
            "INHERITS" => Some(Self::Inherits),
            "CONTAINS" => Some(Self::Contains),
            "USES_FIELD" => Some(Self::UsesField),
            "USES_TYPE" => Some(Self::UsesType),
            "MODULE_CHILD" => Some(Self::ModuleChild),
            "DEPENDS_ON" => Some(Self::DependsOn),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Declares => "DECLARES",
            Self::Calls => "CALLS",
            Self::HasMember => "HAS_MEMBER",
            Self::Implements => "IMPLEMENTS",
            Self::Imports => "IMPORTS",
            Self::Uses => "USES",
            Self::Owns => "OWNS",
            Self::References => "REFERENCES",
            Self::Inherits => "INHERITS",
            Self::Contains => "CONTAINS",
            Self::UsesField => "USES_FIELD",
            Self::UsesType => "USES_TYPE",
            Self::ModuleChild => "MODULE_CHILD",
            Self::DependsOn => "DEPENDS_ON",
        }
    }
}

/// Node properties - must match Neo4j schema
#[derive(Debug, Clone)]
pub struct NodeProperties {
    // Identity (required for all nodes)
    pub id: i64,
    pub name: String,

    // Location (required for code entities)
    pub path: Option<String>,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,

    // Content
    pub signature: Option<String>,
    pub body_snippet: Option<String>,
    pub docstring: Option<String>,

    // Metadata
    pub hash: Option<String>,
    pub language: Option<String>,

    // Timestamps (file-level)
    pub file_sha256: Option<String>,
    pub mtime: Option<i64>,

    // Git/History metadata
    pub created_at: Option<String>,
    pub last_modified_at: Option<String>,
    pub change_count: Option<i64>,
    pub author_count: Option<i64>,
}

impl NodeProperties {
    pub fn minimal(id: i64, name: String) -> Self {
        Self {
            id,
            name,
            path: None,
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
        }
    }

    pub fn full(
        id: i64,
        name: String,
        path: String,
        start_line: i64,
        end_line: i64,
        language: String,
    ) -> Self {
        Self {
            id,
            name,
            path: Some(path),
            start_line: Some(start_line),
            end_line: Some(end_line),
            signature: None,
            body_snippet: None,
            docstring: None,
            hash: None,
            language: Some(language),
            file_sha256: None,
            mtime: None,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        }
    }
}

/// Graph Backend Trait
///
/// Provides a database-agnostic interface for graph operations.
/// All implementations must support namespace-based multi-tenancy
/// and follow the same error handling patterns.
#[async_trait]
pub trait GraphBackend: Send + Sync {
    /// Connect to the graph database
    ///
    /// # Arguments
    /// * `uri` - Connection URI (database-specific format)
    /// * `user` - Username for authentication
    /// * `pass` - Password for authentication
    /// * `namespace` - Namespace for multi-tenant isolation
    async fn connect(uri: &str, user: &str, pass: &str, namespace: &str) -> Result<Self>
    where
        Self: Sized;

    /// Get the current namespace
    fn namespace(&self) -> &str;

    /// Execute a query with parameters
    ///
    /// # Arguments
    /// * `query` - Query string (database-specific language)
    /// * `params` - Query parameters
    ///
    /// # Returns
    /// Vector of JSON objects representing each row
    async fn execute_query(&self, query: &str, params: Vec<(&str, Value)>) -> Result<Vec<Value>>;

    // === Entity Operations ===

    /// Create or update an entity node
    ///
    /// Uses MERGE for idempotency - safe to call multiple times.
    async fn upsert_entity(&self, label: NodeLabel, props: NodeProperties) -> Result<()>;

    /// Delete a single entity by ID
    ///
    /// Also deletes all relationships connected to this entity.
    async fn delete_entity(&self, id: i64) -> Result<()>;

    /// Delete all entities in a file
    ///
    /// Useful for re-indexing a single file.
    async fn delete_file_entities(&self, file_path: &str) -> Result<usize>;

    /// Batch upsert entities (efficient for bulk imports)
    ///
    /// Processes entities in batches to avoid overwhelming the database.
    async fn batch_upsert_entities(
        &self,
        label: NodeLabel,
        entities: Vec<NodeProperties>,
        batch_size: usize,
    ) -> Result<usize>;

    // === Relationship Operations ===

    /// Create a relationship between two entities
    ///
    /// Uses MERGE for idempotency - safe to call multiple times.
    /// Both entities must already exist.
    async fn create_relationship(
        &self,
        src_id: i64,
        dst_id: i64,
        rel_type: RelationType,
    ) -> Result<()>;

    /// Batch create relationships (efficient for bulk imports)
    ///
    /// Processes relationships in batches to avoid overwhelming the database.
    async fn batch_create_relationships(
        &self,
        relationships: Vec<(i64, i64, RelationType)>,
        batch_size: usize,
    ) -> Result<usize>;

    /// Create a dependency relationship between two files (by path)
    ///
    /// Creates DEPENDS_ON relationship between files identified by path.
    /// Both source and target File nodes are created if they don't exist.
    async fn create_file_dependency(&self, from_path: &str, to_path: &str) -> Result<()>;

    /// Upsert a lightweight File node by path (for application mapping)
    ///
    /// Creates a minimal File node identified by path only.
    async fn upsert_file_by_path(&self, file_path: &str) -> Result<()>;

    // === Query Operations ===

    /// Get entity by ID
    async fn get_entity_by_id(&self, id: i64) -> Result<Option<EntityResult>>;

    /// Get all entities in a file
    async fn get_file_entities(&self, file_path: &str) -> Result<Vec<EntityResult>>;

    /// Get functions called by a function
    async fn get_function_callees(&self, function_id: i64) -> Result<Vec<EntityResult>>;

    /// Get functions that call a function
    async fn get_function_callers(&self, function_id: i64) -> Result<Vec<EntityResult>>;

    /// Get entities by name (exact match)
    async fn find_entities_by_name(&self, name: &str) -> Result<Vec<EntityResult>>;

    /// Get entities by label type
    async fn get_entities_by_type(&self, label: NodeLabel) -> Result<Vec<EntityResult>>;

    /// Get neighbors (any relationship) of an entity
    async fn get_neighbors(&self, entity_id: i64) -> Result<Vec<EntityResult>>;

    /// Find orphan entities (no relationships)
    async fn find_orphan_entities(&self) -> Result<Vec<EntityResult>>;

    // === Statistics and Validation ===

    /// Count entities by type
    async fn count_entities_by_type(&self) -> Result<Vec<(String, i64)>>;

    /// Validate graph structure (returns stats)
    async fn validate_structure(&self) -> Result<GraphStats>;

    // === Metadata Operations ===

    /// Update Git/History metadata on an existing entity
    ///
    /// Updates only the temporal metadata fields.
    /// The entity must already exist.
    async fn update_git_metadata(
        &self,
        id: i64,
        created_at: Option<String>,
        last_modified_at: Option<String>,
        change_count: Option<i64>,
        author_count: Option<i64>,
    ) -> Result<()>;

    // === Specialized Operations (for compatibility) ===

    /// Create a Task node
    async fn create_task_node(&self, id: i64, title: &str, status: &str) -> Result<()>;

    /// Create a HAS_SUBTASK relationship between two tasks
    async fn create_subtask_relationship(&self, parent_id: i64, child_id: i64) -> Result<()>;

    /// Create a Memory node
    async fn create_memory_node(&self, key: &str, value: &str) -> Result<()>;

    /// Create an Embedding node
    async fn create_embedding_node(&self, id: i64, text: &str, hash: &str) -> Result<()>;

    /// Link an embedding to a task
    async fn link_embedding_to_task(&self, embedding_id: i64, task_id: i64) -> Result<()>;
}

/// Neo4j implementation of GraphBackend
pub mod neo4j_impl {
    use super::*;
    use crate::databases::neo4j::{reader, schema, writer};
    use crate::graph::Neo4jClient;

    /// Neo4j backend implementation
    #[derive(Clone)]
    pub struct Neo4jBackend {
        client: Neo4jClient,
    }

    impl Neo4jBackend {
        /// Create a new Neo4jBackend from an existing Neo4jClient
        pub fn new(client: Neo4jClient) -> Self {
            Self {
                client,
            }
        }

        /// Get the underlying Neo4j client (for advanced use cases)
        pub fn client(&self) -> &Neo4jClient {
            &self.client
        }

        /// Convert Neo4j EntityResult to backend EntityResult
        fn convert_entity_result(neo4j_entity: reader::EntityResult) -> super::EntityResult {
            super::EntityResult {
                id: neo4j_entity.id,
                name: neo4j_entity.name,
                label: neo4j_entity.label,
                path: neo4j_entity.path,
                start_line: neo4j_entity.start_line,
                end_line: neo4j_entity.end_line,
                signature: neo4j_entity.signature,
                body_snippet: neo4j_entity.body_snippet,
                created_at: neo4j_entity.created_at,
                last_modified_at: neo4j_entity.last_modified_at,
                change_count: neo4j_entity.change_count,
                author_count: neo4j_entity.author_count,
            }
        }

        /// Convert Neo4j GraphStats to backend GraphStats
        fn convert_graph_stats(neo4j_stats: reader::GraphStats) -> super::GraphStats {
            super::GraphStats {
                total_nodes: neo4j_stats.total_nodes,
                total_edges: neo4j_stats.total_edges,
                orphan_count: neo4j_stats.orphan_count,
                entity_types: neo4j_stats.entity_types,
                edge_types: neo4j_stats.edge_types,
            }
        }
    }

    #[async_trait]
    impl GraphBackend for Neo4jBackend {
        async fn connect(uri: &str, user: &str, pass: &str, namespace: &str) -> Result<Self> {
            // Temporarily set namespace environment variable for Neo4jClient
            std::env::set_var("GRAPH_NAMESPACE", namespace);

            let client = Neo4jClient::connect(uri, user, pass).await?;
            Ok(Self {
                client,
            })
        }

        fn namespace(&self) -> &str {
            self.client.namespace()
        }

        async fn execute_query(
            &self,
            query: &str,
            params: Vec<(&str, Value)>,
        ) -> Result<Vec<Value>> {
            self.client.execute_query(query, params).await
        }

        // === Entity Operations ===

        async fn upsert_entity(&self, label: NodeLabel, props: NodeProperties) -> Result<()> {
            let neo4j_label = match label {
                NodeLabel::File => crate::databases::neo4j::schema::NodeLabel::File,
                NodeLabel::Function => crate::databases::neo4j::schema::NodeLabel::Function,
                NodeLabel::Struct => crate::databases::neo4j::schema::NodeLabel::Struct,
                NodeLabel::Enum => crate::databases::neo4j::schema::NodeLabel::Enum,
                NodeLabel::Trait => crate::databases::neo4j::schema::NodeLabel::Trait,
                NodeLabel::Impl => crate::databases::neo4j::schema::NodeLabel::Impl,
                NodeLabel::Module => crate::databases::neo4j::schema::NodeLabel::Module,
                NodeLabel::Import => crate::databases::neo4j::schema::NodeLabel::Import,
                NodeLabel::Constant => crate::databases::neo4j::schema::NodeLabel::Constant,
                NodeLabel::TypeAlias => crate::databases::neo4j::schema::NodeLabel::TypeAlias,
            };

            let neo4j_props = crate::databases::neo4j::schema::NodeProperties {
                id: props.id,
                name: props.name,
                path: props.path,
                start_line: props.start_line,
                end_line: props.end_line,
                signature: props.signature,
                body_snippet: props.body_snippet,
                docstring: props.docstring,
                hash: props.hash,
                language: props.language,
                file_sha256: props.file_sha256,
                mtime: props.mtime,
                created_at: props.created_at,
                last_modified_at: props.last_modified_at,
                change_count: props.change_count,
                author_count: props.author_count,
            };

            writer::upsert_entity(&self.client, neo4j_label, neo4j_props).await
        }

        async fn delete_entity(&self, id: i64) -> Result<()> {
            writer::delete_entity(&self.client, id).await
        }

        async fn delete_file_entities(&self, file_path: &str) -> Result<usize> {
            writer::delete_file_entities(&self.client, file_path).await
        }

        async fn batch_upsert_entities(
            &self,
            label: NodeLabel,
            entities: Vec<NodeProperties>,
            batch_size: usize,
        ) -> Result<usize> {
            let neo4j_label = match label {
                NodeLabel::File => crate::databases::neo4j::schema::NodeLabel::File,
                NodeLabel::Function => crate::databases::neo4j::schema::NodeLabel::Function,
                NodeLabel::Struct => crate::databases::neo4j::schema::NodeLabel::Struct,
                NodeLabel::Enum => crate::databases::neo4j::schema::NodeLabel::Enum,
                NodeLabel::Trait => crate::databases::neo4j::schema::NodeLabel::Trait,
                NodeLabel::Impl => crate::databases::neo4j::schema::NodeLabel::Impl,
                NodeLabel::Module => crate::databases::neo4j::schema::NodeLabel::Module,
                NodeLabel::Import => crate::databases::neo4j::schema::NodeLabel::Import,
                NodeLabel::Constant => crate::databases::neo4j::schema::NodeLabel::Constant,
                NodeLabel::TypeAlias => crate::databases::neo4j::schema::NodeLabel::TypeAlias,
            };

            let neo4j_entities: Vec<crate::databases::neo4j::schema::NodeProperties> = entities
                .into_iter()
                .map(|props| crate::databases::neo4j::schema::NodeProperties {
                    id: props.id,
                    name: props.name,
                    path: props.path,
                    start_line: props.start_line,
                    end_line: props.end_line,
                    signature: props.signature,
                    body_snippet: props.body_snippet,
                    docstring: props.docstring,
                    hash: props.hash,
                    language: props.language,
                    file_sha256: props.file_sha256,
                    mtime: props.mtime,
                    created_at: props.created_at,
                    last_modified_at: props.last_modified_at,
                    change_count: props.change_count,
                    author_count: props.author_count,
                })
                .collect();

            writer::batch_upsert_entities(&self.client, neo4j_label, neo4j_entities, batch_size)
                .await
        }

        // === Relationship Operations ===

        async fn create_relationship(
            &self,
            src_id: i64,
            dst_id: i64,
            rel_type: RelationType,
        ) -> Result<()> {
            let neo4j_rel = match rel_type {
                RelationType::Declares => crate::databases::neo4j::schema::RelationType::Declares,
                RelationType::Calls => crate::databases::neo4j::schema::RelationType::Calls,
                RelationType::HasMember => crate::databases::neo4j::schema::RelationType::HasMember,
                RelationType::Implements => {
                    crate::databases::neo4j::schema::RelationType::Implements
                }
                RelationType::Imports => crate::databases::neo4j::schema::RelationType::Imports,
                RelationType::Uses => crate::databases::neo4j::schema::RelationType::Uses,
                RelationType::Owns => crate::databases::neo4j::schema::RelationType::Owns,
                RelationType::References => {
                    crate::databases::neo4j::schema::RelationType::References
                }
                RelationType::Inherits => crate::databases::neo4j::schema::RelationType::Inherits,
                RelationType::Contains => crate::databases::neo4j::schema::RelationType::Contains,
                RelationType::UsesField => crate::databases::neo4j::schema::RelationType::UsesField,
                RelationType::UsesType => crate::databases::neo4j::schema::RelationType::UsesType,
                RelationType::ModuleChild => {
                    crate::databases::neo4j::schema::RelationType::ModuleChild
                }
                RelationType::DependsOn => crate::databases::neo4j::schema::RelationType::DependsOn,
            };

            writer::create_relationship(&self.client, src_id, dst_id, neo4j_rel).await
        }

        async fn batch_create_relationships(
            &self,
            relationships: Vec<(i64, i64, RelationType)>,
            batch_size: usize,
        ) -> Result<usize> {
            let neo4j_relationships: Vec<(
                i64,
                i64,
                crate::databases::neo4j::schema::RelationType,
            )> = relationships
                .into_iter()
                .map(|(src, dst, rel)| {
                    let neo4j_rel = match rel {
                        RelationType::Declares => {
                            crate::databases::neo4j::schema::RelationType::Declares
                        }
                        RelationType::Calls => crate::databases::neo4j::schema::RelationType::Calls,
                        RelationType::HasMember => {
                            crate::databases::neo4j::schema::RelationType::HasMember
                        }
                        RelationType::Implements => {
                            crate::databases::neo4j::schema::RelationType::Implements
                        }
                        RelationType::Imports => {
                            crate::databases::neo4j::schema::RelationType::Imports
                        }
                        RelationType::Uses => crate::databases::neo4j::schema::RelationType::Uses,
                        RelationType::Owns => crate::databases::neo4j::schema::RelationType::Owns,
                        RelationType::References => {
                            crate::databases::neo4j::schema::RelationType::References
                        }
                        RelationType::Inherits => {
                            crate::databases::neo4j::schema::RelationType::Inherits
                        }
                        RelationType::Contains => {
                            crate::databases::neo4j::schema::RelationType::Contains
                        }
                        RelationType::UsesField => {
                            crate::databases::neo4j::schema::RelationType::UsesField
                        }
                        RelationType::UsesType => {
                            crate::databases::neo4j::schema::RelationType::UsesType
                        }
                        RelationType::ModuleChild => {
                            crate::databases::neo4j::schema::RelationType::ModuleChild
                        }
                        RelationType::DependsOn => {
                            crate::databases::neo4j::schema::RelationType::DependsOn
                        }
                    };
                    (src, dst, neo4j_rel)
                })
                .collect();

            writer::batch_create_relationships(&self.client, neo4j_relationships, batch_size).await
        }

        async fn create_file_dependency(&self, from_path: &str, to_path: &str) -> Result<()> {
            writer::create_file_dependency(&self.client, from_path, to_path).await
        }

        async fn upsert_file_by_path(&self, file_path: &str) -> Result<()> {
            writer::upsert_file_by_path(&self.client, file_path).await
        }

        // === Query Operations ===

        async fn get_entity_by_id(&self, id: i64) -> Result<Option<super::EntityResult>> {
            let result = reader::get_entity_by_id(&self.client, id).await?;
            Ok(result.map(Self::convert_entity_result))
        }

        async fn get_file_entities(&self, file_path: &str) -> Result<Vec<super::EntityResult>> {
            let results = reader::get_file_entities(&self.client, file_path).await?;
            Ok(results.into_iter().map(Self::convert_entity_result).collect())
        }

        async fn get_function_callees(&self, function_id: i64) -> Result<Vec<super::EntityResult>> {
            let results = reader::get_function_callees(&self.client, function_id).await?;
            Ok(results.into_iter().map(Self::convert_entity_result).collect())
        }

        async fn get_function_callers(&self, function_id: i64) -> Result<Vec<super::EntityResult>> {
            let results = reader::get_function_callers(&self.client, function_id).await?;
            Ok(results.into_iter().map(Self::convert_entity_result).collect())
        }

        async fn find_entities_by_name(&self, name: &str) -> Result<Vec<super::EntityResult>> {
            let results = reader::find_entities_by_name(&self.client, name).await?;
            Ok(results.into_iter().map(Self::convert_entity_result).collect())
        }

        async fn get_entities_by_type(&self, label: NodeLabel) -> Result<Vec<EntityResult>> {
            let neo4j_label = match label {
                NodeLabel::File => crate::databases::neo4j::schema::NodeLabel::File,
                NodeLabel::Function => crate::databases::neo4j::schema::NodeLabel::Function,
                NodeLabel::Struct => crate::databases::neo4j::schema::NodeLabel::Struct,
                NodeLabel::Enum => crate::databases::neo4j::schema::NodeLabel::Enum,
                NodeLabel::Trait => crate::databases::neo4j::schema::NodeLabel::Trait,
                NodeLabel::Impl => crate::databases::neo4j::schema::NodeLabel::Impl,
                NodeLabel::Module => crate::databases::neo4j::schema::NodeLabel::Module,
                NodeLabel::Import => crate::databases::neo4j::schema::NodeLabel::Import,
                NodeLabel::Constant => crate::databases::neo4j::schema::NodeLabel::Constant,
                NodeLabel::TypeAlias => crate::databases::neo4j::schema::NodeLabel::TypeAlias,
            };

            let results = reader::get_entities_by_type(&self.client, neo4j_label).await?;
            Ok(results
                .into_iter()
                .map(|neo4j_entity| EntityResult {
                    id: neo4j_entity.id,
                    name: neo4j_entity.name,
                    label: neo4j_entity.label,
                    path: neo4j_entity.path,
                    start_line: neo4j_entity.start_line,
                    end_line: neo4j_entity.end_line,
                    signature: neo4j_entity.signature,
                    body_snippet: neo4j_entity.body_snippet,
                    created_at: neo4j_entity.created_at,
                    last_modified_at: neo4j_entity.last_modified_at,
                    change_count: neo4j_entity.change_count,
                    author_count: neo4j_entity.author_count,
                })
                .collect())
        }

        async fn get_neighbors(&self, entity_id: i64) -> Result<Vec<EntityResult>> {
            let results = reader::get_neighbors(&self.client, entity_id).await?;
            Ok(results
                .into_iter()
                .map(|neo4j_entity| EntityResult {
                    id: neo4j_entity.id,
                    name: neo4j_entity.name,
                    label: neo4j_entity.label,
                    path: neo4j_entity.path,
                    start_line: neo4j_entity.start_line,
                    end_line: neo4j_entity.end_line,
                    signature: neo4j_entity.signature,
                    body_snippet: neo4j_entity.body_snippet,
                    created_at: neo4j_entity.created_at,
                    last_modified_at: neo4j_entity.last_modified_at,
                    change_count: neo4j_entity.change_count,
                    author_count: neo4j_entity.author_count,
                })
                .collect())
        }

        async fn find_orphan_entities(&self) -> Result<Vec<EntityResult>> {
            let results = reader::find_orphan_entities(&self.client).await?;
            Ok(results
                .into_iter()
                .map(|neo4j_entity| EntityResult {
                    id: neo4j_entity.id,
                    name: neo4j_entity.name,
                    label: neo4j_entity.label,
                    path: neo4j_entity.path,
                    start_line: neo4j_entity.start_line,
                    end_line: neo4j_entity.end_line,
                    signature: neo4j_entity.signature,
                    body_snippet: neo4j_entity.body_snippet,
                    created_at: neo4j_entity.created_at,
                    last_modified_at: neo4j_entity.last_modified_at,
                    change_count: neo4j_entity.change_count,
                    author_count: neo4j_entity.author_count,
                })
                .collect())
        }

        // === Statistics and Validation ===

        async fn count_entities_by_type(&self) -> Result<Vec<(String, i64)>> {
            reader::count_entities_by_type(&self.client).await
        }

        async fn validate_structure(&self) -> Result<GraphStats> {
            let stats = reader::validate_structure(&self.client).await?;
            Ok(GraphStats {
                total_nodes: stats.total_nodes,
                total_edges: stats.total_edges,
                orphan_count: stats.orphan_count,
                entity_types: stats.entity_types,
                edge_types: stats.edge_types,
            })
        }

        // === Metadata Operations ===

        async fn update_git_metadata(
            &self,
            id: i64,
            created_at: Option<String>,
            last_modified_at: Option<String>,
            change_count: Option<i64>,
            author_count: Option<i64>,
        ) -> Result<()> {
            writer::update_git_metadata(
                &self.client,
                id,
                created_at,
                last_modified_at,
                change_count,
                author_count,
            )
            .await
        }

        // === Specialized Operations ===

        async fn create_task_node(&self, id: i64, title: &str, status: &str) -> Result<()> {
            self.client.create_task_node(id, title, status).await
        }

        async fn create_subtask_relationship(&self, parent_id: i64, child_id: i64) -> Result<()> {
            self.client.create_subtask_relationship(parent_id, child_id).await
        }

        async fn create_memory_node(&self, key: &str, value: &str) -> Result<()> {
            self.client.create_memory_node(key, value).await
        }

        async fn create_embedding_node(&self, id: i64, text: &str, hash: &str) -> Result<()> {
            self.client.create_embedding_node(id, text, hash).await
        }

        async fn link_embedding_to_task(&self, embedding_id: i64, task_id: i64) -> Result<()> {
            self.client.link_embedding_to_task(embedding_id, task_id).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_label_conversion() {
        assert_eq!(NodeLabel::Function.as_str(), "Function");
        assert_eq!(NodeLabel::File.as_str(), "File");
    }

    #[test]
    fn test_relation_type_conversion() {
        assert_eq!(RelationType::Calls.as_str(), "CALLS");
        assert_eq!(RelationType::Declares.as_str(), "DECLARES");
    }

    #[test]
    fn test_node_properties_creation() {
        let props = NodeProperties::minimal(1, "test".to_string());
        assert_eq!(props.id, 1);
        assert_eq!(props.name, "test");
        assert!(props.path.is_none());

        let full_props = NodeProperties::full(
            2,
            "test_fn".to_string(),
            "src/test.rs".to_string(),
            10,
            20,
            "rust".to_string(),
        );
        assert_eq!(full_props.id, 2);
        assert_eq!(full_props.name, "test_fn");
        assert_eq!(full_props.path, Some("src/test.rs".to_string()));
        assert_eq!(full_props.language, Some("rust".to_string()));
    }
}
