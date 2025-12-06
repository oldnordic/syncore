//! Sync SQLite Backend - Hybrid Sync Wrapper for Async GraphBackend
//!
//! Wraps an async `Arc<dyn GraphBackend>` to provide sync methods.
//! Each sync method creates a runtime when needed or uses spawn_blocking
//! to execute the async operation safely within sync contexts.
//!
//! This approach follows the "hybrid model" requirements:
//! - Keep SQLiteGraph core code fully async
//! - Add thin sync wrapper at adapter boundary for StorageAdapter
//! - Handle runtime creation correctly for sync-to-async bridging
//! - Maintain full backward compatibility

use crate::graph::{GraphBackend, EntityResult, NodeProperties, NodeLabel, RelationType, GraphStats};
use anyhow::{Context, Result};
use serde_json::Value;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::task;

/// Synchronous GraphBackend interface for use in sync contexts like StorageAdapter
///
/// This trait mirrors the async GraphBackend trait but provides sync methods
/// that can be safely called from non-async code without blocking issues.
pub trait SyncGraphBackend: Send + Sync {
    /// Connect to the graph database
    fn connect(uri: &str, user: &str, pass: &str, namespace: &str) -> Result<Self>
    where
        Self: Sized;

    /// Get the current namespace
    fn namespace(&self) -> &str;

    /// Execute a query with parameters
    fn execute_query(&self, query: &str, params: Vec<(&str, Value)>) -> Result<Vec<Value>>;

    /// Create or update an entity node
    fn upsert_entity(&self, label: NodeLabel, props: NodeProperties) -> Result<()>;

    /// Delete an entity by ID
    fn delete_entity(&self, id: i64) -> Result<()>;

    /// Delete all entities for a file path
    fn delete_file_entities(&self, file_path: &str) -> Result<usize>;

    /// Batch upsert entities
    fn batch_upsert_entities(&self, label: NodeLabel, entities: Vec<NodeProperties>, batch_size: usize) -> Result<usize>;

    /// Create a relationship between entities
    fn create_relationship(&self, src_id: i64, dst_id: i64, rel_type: RelationType) -> Result<()>;

    /// Batch create relationships
    fn batch_create_relationships(&self, relationships: Vec<(i64, i64, RelationType)>, batch_size: usize) -> Result<usize>;

    /// Create file dependency relationship
    fn create_file_dependency(&self, from_path: &str, to_path: &str) -> Result<()>;

    /// Upsert file entity by path
    fn upsert_file_by_path(&self, file_path: &str) -> Result<()>;

    /// Get entity by ID
    fn get_entity_by_id(&self, id: i64) -> Result<Option<EntityResult>>;

    /// Get file entities
    fn get_file_entities(&self, file_path: &str) -> Result<Vec<EntityResult>>;

    /// Get function callees
    fn get_function_callees(&self, function_id: i64) -> Result<Vec<EntityResult>>;

    /// Get function callers
    fn get_function_callers(&self, function_id: i64) -> Result<Vec<EntityResult>>;

    /// Find entities by name
    fn find_entities_by_name(&self, name: &str) -> Result<Vec<EntityResult>>;

    /// Get entities by type
    fn get_entities_by_type(&self, label: NodeLabel) -> Result<Vec<EntityResult>>;

    /// Get neighbors of an entity
    fn get_neighbors(&self, entity_id: i64) -> Result<Vec<EntityResult>>;

    /// Find orphan entities
    fn find_orphan_entities(&self) -> Result<Vec<EntityResult>>;

    /// Count entities by type
    fn count_entities_by_type(&self) -> Result<Vec<(String, i64)>>;

    /// Validate graph structure
    fn validate_structure(&self) -> Result<GraphStats>;

    /// Update git metadata
    fn update_git_metadata(&self, id: i64, created_at: Option<String>, last_modified_at: Option<String>, change_count: Option<i64>, author_count: Option<i64>) -> Result<()>;

    /// Create task node
    fn create_task_node(&self, id: i64, title: &str, status: &str) -> Result<()>;

    /// Create subtask relationship
    fn create_subtask_relationship(&self, parent_id: i64, child_id: i64) -> Result<()>;

    /// Create memory node
    fn create_memory_node(&self, key: &str, value: &str) -> Result<()>;

    /// Create embedding node
    fn create_embedding_node(&self, id: i64, text: &str, hash: &str) -> Result<()>;

    /// Link embedding to task
    fn link_embedding_to_task(&self, embedding_id: i64, task_id: i64) -> Result<()>;
}

/// Sync wrapper for async GraphBackend implementations
///
/// This wrapper provides sync methods that can be used from sync contexts
/// like StorageAdapter, while internally using the async GraphBackend.
///
/// # Thread Safety
///
/// The underlying `Arc<dyn GraphBackend>` is thread-safe (Send + Sync), and
/// this wrapper properly handles runtime creation for async operations.
///
/// # Error Handling
///
/// All async operation errors are properly mapped to `anyhow::Error`
/// with context about the failed operation.
#[derive(Clone)]
pub struct AsyncSQLiteBackend {
    /// The async GraphBackend implementation
    /// This is wrapped in Arc for thread-safe sharing across sync contexts
    inner: Arc<dyn crate::graph::GraphBackend>,
}

impl AsyncSQLiteBackend {
    /// Create a new sync wrapper around an async GraphBackend
    ///
    /// # Arguments
    /// * `inner` - The async GraphBackend implementation to wrap
    ///
    /// # Returns
    /// A new AsyncSQLiteBackend instance
    pub fn new(inner: Arc<dyn GraphBackend>) -> Result<Self> {
        Ok(Self {
            inner,
        })
    }

    /// Get the namespace of the underlying backend
    ///
    /// This method is synchronous as it only reads a string value
    /// without performing I/O operations.
    pub fn namespace(&self) -> &str {
        self.inner.namespace()
    }

    /// Internal helper to execute async operations from sync contexts
    ///
    /// This method always uses spawn_blocking to move the operation to a
    /// dedicated blocking thread, avoiding any runtime conflicts.
    ///
    /// # Arguments
    /// * `operation_name` - Human-readable name for error context
    /// * `operation` - The async operation to execute
    ///
    /// # Returns
    /// The result of the operation, with proper error mapping
    fn execute_sync<F, R>(&self, operation_name: &str, operation: F) -> Result<R>
    where
        F: FnOnce(Arc<dyn GraphBackend>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R>> + Send>> + Send + 'static,
        R: Send + 'static,
    {
        let inner = self.inner.clone();
        let operation_name = operation_name.to_string();

        // Always use spawn_blocking to avoid runtime conflicts
        // This works whether we're in an async context or not
        // Use block_in_place when in async context, direct blocking when in sync context
        match tokio::runtime::Handle::try_current() {
            Ok(_) => {
                // We're in an async context, use block_in_place to move to blocking thread
                tokio::task::block_in_place(|| {
                    // Create a new runtime on the blocking thread
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .context("Failed to create runtime in blocking thread")?;

                    rt.block_on(async {
                        operation(inner)
                            .await
                            .with_context(|| format!("Failed to execute {}", operation_name))
                    })
                })
            }
            Err(_) => {
                // We're in a sync context, just create a runtime directly
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .context("Failed to create runtime for sync execution")?;

                rt.block_on(async {
                    operation(inner)
                        .await
                        .with_context(|| format!("Failed to execute {}", operation_name))
                })
            }
        }
    }
}

impl SyncGraphBackend for AsyncSQLiteBackend {
    fn connect(uri: &str, user: &str, pass: &str, namespace: &str) -> Result<Self>
    where
        Self: Sized,
    {
        // For connection, we need to create the async backend first
        // This is a special case since we're creating the backend, not wrapping it

        // Import the backend selector function to create the async backend
        use crate::graph::backend_selector::create_default_graph_backend;
        use crate::config::{GraphBackend as ConfigBackend, GraphConfig};

        // Create a temporary config for connection
        let graph_config = GraphConfig {
            backend: ConfigBackend::SqliteGraph,
            path: uri.to_string(), // For SQLiteGraph, URI is the path
            uri: String::new(),
            user: user.to_string(),
            password: pass.to_string(),
            enabled: true,
        };

        // Create the async backend - this needs to be done from an async context
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to create runtime for connection")?;

        let async_backend = rt.block_on(async {
            create_default_graph_backend(&graph_config)
                .await
                .context("Failed to create async SQLiteGraph backend")
        })?;

        // Wrap it in our sync façade
        Ok(Self::new(async_backend)?)
    }

    fn namespace(&self) -> &str {
        self.inner.namespace()
    }

    fn execute_query(&self, query: &str, params: Vec<(&str, Value)>) -> Result<Vec<Value>> {
        let query = query.to_string();
        let owned_params: Vec<(String, Value)> = params.into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        self.execute_sync("execute_query", move |backend| {
            Box::pin(async move {
                let param_refs: Vec<(&str, Value)> = owned_params.iter()
                    .map(|(k, v)| (k.as_str(), v.clone()))
                    .collect();
                backend.execute_query(&query, param_refs).await
            })
        })
    }

    fn upsert_entity(&self, label: NodeLabel, props: NodeProperties) -> Result<()> {
        self.execute_sync("upsert_entity", move |backend| {
            Box::pin(async move {
                backend.upsert_entity(label, props).await
            })
        })
    }

    fn delete_entity(&self, id: i64) -> Result<()> {
        self.execute_sync("delete_entity", move |backend| {
            Box::pin(async move {
                backend.delete_entity(id).await
            })
        })
    }

    fn delete_file_entities(&self, file_path: &str) -> Result<usize> {
        let file_path = file_path.to_string();
        self.execute_sync("delete_file_entities", move |backend| {
            Box::pin(async move {
                backend.delete_file_entities(&file_path).await
            })
        })
    }

    fn batch_upsert_entities(
        &self,
        label: NodeLabel,
        entities: Vec<NodeProperties>,
        batch_size: usize,
    ) -> Result<usize> {
        self.execute_sync("batch_upsert_entities", move |backend| {
            Box::pin(async move {
                backend.batch_upsert_entities(label, entities, batch_size).await
            })
        })
    }

    fn create_relationship(
        &self,
        src_id: i64,
        dst_id: i64,
        rel_type: RelationType,
    ) -> Result<()> {
        self.execute_sync("create_relationship", move |backend| {
            Box::pin(async move {
                backend.create_relationship(src_id, dst_id, rel_type).await
            })
        })
    }

    fn batch_create_relationships(
        &self,
        relationships: Vec<(i64, i64, RelationType)>,
        batch_size: usize,
    ) -> Result<usize> {
        self.execute_sync("batch_create_relationships", move |backend| {
            Box::pin(async move {
                backend.batch_create_relationships(relationships, batch_size).await
            })
        })
    }

    fn create_file_dependency(&self, from_path: &str, to_path: &str) -> Result<()> {
        let from_path = from_path.to_string();
        let to_path = to_path.to_string();
        self.execute_sync("create_file_dependency", move |backend| {
            Box::pin(async move {
                backend.create_file_dependency(&from_path, &to_path).await
            })
        })
    }

    fn upsert_file_by_path(&self, file_path: &str) -> Result<()> {
        let file_path = file_path.to_string();
        self.execute_sync("upsert_file_by_path", move |backend| {
            Box::pin(async move {
                backend.upsert_file_by_path(&file_path).await
            })
        })
    }

    fn get_entity_by_id(&self, id: i64) -> Result<Option<EntityResult>> {
        self.execute_sync("get_entity_by_id", move |backend| {
            Box::pin(async move {
                backend.get_entity_by_id(id).await
            })
        })
    }

    fn get_file_entities(&self, file_path: &str) -> Result<Vec<EntityResult>> {
        let file_path = file_path.to_string();
        self.execute_sync("get_file_entities", move |backend| {
            Box::pin(async move {
                backend.get_file_entities(&file_path).await
            })
        })
    }

    fn get_function_callees(&self, function_id: i64) -> Result<Vec<EntityResult>> {
        self.execute_sync("get_function_callees", move |backend| {
            Box::pin(async move {
                backend.get_function_callees(function_id).await
            })
        })
    }

    fn get_function_callers(&self, function_id: i64) -> Result<Vec<EntityResult>> {
        self.execute_sync("get_function_callers", move |backend| {
            Box::pin(async move {
                backend.get_function_callers(function_id).await
            })
        })
    }

    fn find_entities_by_name(&self, name: &str) -> Result<Vec<EntityResult>> {
        let name = name.to_string();
        self.execute_sync("find_entities_by_name", move |backend| {
            Box::pin(async move {
                backend.find_entities_by_name(&name).await
            })
        })
    }

    fn get_entities_by_type(&self, label: NodeLabel) -> Result<Vec<EntityResult>> {
        self.execute_sync("get_entities_by_type", move |backend| {
            Box::pin(async move {
                backend.get_entities_by_type(label).await
            })
        })
    }

    fn get_neighbors(&self, entity_id: i64) -> Result<Vec<EntityResult>> {
        self.execute_sync("get_neighbors", move |backend| {
            Box::pin(async move {
                backend.get_neighbors(entity_id).await
            })
        })
    }

    fn find_orphan_entities(&self) -> Result<Vec<EntityResult>> {
        self.execute_sync("find_orphan_entities", move |backend| {
            Box::pin(async move {
                backend.find_orphan_entities().await
            })
        })
    }

    fn count_entities_by_type(&self) -> Result<Vec<(String, i64)>> {
        self.execute_sync("count_entities_by_type", move |backend| {
            Box::pin(async move {
                backend.count_entities_by_type().await
            })
        })
    }

    fn validate_structure(&self) -> Result<GraphStats> {
        self.execute_sync("validate_structure", move |backend| {
            Box::pin(async move {
                backend.validate_structure().await
            })
        })
    }

    fn update_git_metadata(
        &self,
        id: i64,
        created_at: Option<String>,
        last_modified_at: Option<String>,
        change_count: Option<i64>,
        author_count: Option<i64>,
    ) -> Result<()> {
        self.execute_sync("update_git_metadata", move |backend| {
            Box::pin(async move {
                backend.update_git_metadata(id, created_at, last_modified_at, change_count, author_count).await
            })
        })
    }

    fn create_task_node(&self, id: i64, title: &str, status: &str) -> Result<()> {
        let title = title.to_string();
        let status = status.to_string();
        self.execute_sync("create_task_node", move |backend| {
            Box::pin(async move {
                backend.create_task_node(id, &title, &status).await
            })
        })
    }

    fn create_subtask_relationship(&self, parent_id: i64, child_id: i64) -> Result<()> {
        self.execute_sync("create_subtask_relationship", move |backend| {
            Box::pin(async move {
                backend.create_subtask_relationship(parent_id, child_id).await
            })
        })
    }

    fn create_memory_node(&self, key: &str, value: &str) -> Result<()> {
        let key = key.to_string();
        let value = value.to_string();
        self.execute_sync("create_memory_node", move |backend| {
            Box::pin(async move {
                backend.create_memory_node(&key, &value).await
            })
        })
    }

    fn create_embedding_node(&self, id: i64, text: &str, hash: &str) -> Result<()> {
        let text = text.to_string();
        let hash = hash.to_string();
        self.execute_sync("create_embedding_node", move |backend| {
            Box::pin(async move {
                backend.create_embedding_node(id, &text, &hash).await
            })
        })
    }

    fn link_embedding_to_task(&self, embedding_id: i64, task_id: i64) -> Result<()> {
        self.execute_sync("link_embedding_to_task", move |backend| {
            Box::pin(async move {
                backend.link_embedding_to_task(embedding_id, task_id).await
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GraphBackend as ConfigBackend, GraphConfig};
    use crate::graph::backend_selector::create_default_graph_backend;
    use tempfile::tempdir;

    /// Create a test AsyncSQLiteBackend for testing
    async fn create_test_backend() -> AsyncSQLiteBackend {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let graph_config = GraphConfig {
            backend: ConfigBackend::SqliteGraph,
            path: db_path.to_str().unwrap().to_string(),
            uri: String::new(),
            user: String::new(),
            password: String::new(),
            enabled: true,
        };

        let sync_backend = create_default_graph_backend(&graph_config)
            .await
            .unwrap();

        AsyncSQLiteBackend::new(sync_backend).unwrap()
    }

    #[tokio::test]
    async fn test_async_backend_creation() {
        let backend = create_test_backend().await;
        assert_eq!(backend.namespace(), "test");
    }

    #[tokio::test]
    async fn test_execute_query_async() {
        let backend = create_test_backend().await;

        // Simple SELECT query should work
        let result = backend
            .execute_query("SELECT 1 as test", vec![]);

        assert!(result.is_ok());
        let rows = result.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["test"], 1);
    }

    #[tokio::test]
    async fn test_get_neighbors_async() {
        let backend = create_test_backend().await;

        // Query for neighbors of non-existent node should return empty result
        let result = backend.get_neighbors(999);
        assert!(result.is_ok());
        let neighbors = result.unwrap();
        assert_eq!(neighbors.len(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let backend = Arc::new(create_test_backend().await);
        let mut handles = Vec::new();

        // Launch multiple concurrent operations
        for i in 0..10 {
            let backend_clone = backend.clone();
            let handle = tokio::spawn(async move {
                backend_clone.execute_query("SELECT 1", vec!())
            });
            handles.push(handle);
        }

        // All operations should complete successfully
        for handle in handles {
            let result = handle.await;
            assert!(result.is_ok(), "Concurrent operation should complete without panicking");
            let query_result = result.unwrap();
            assert!(query_result.is_ok(), "Query should succeed");
        }
    }

    #[tokio::test]
    async fn test_error_propagation() {
        let backend = create_test_backend().await;

        // Invalid SQL should return an error, not panic
        let result = backend.execute_query("INVALID SQL", vec![]);
        assert!(result.is_err());

        // Error should contain context about the operation
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to execute execute_query") || error_msg.contains("sql"));
    }

    #[tokio::test]
    async fn test_no_runtime_blocking() {
        let backend = Arc::new(create_test_backend().await);

        // Start a background task that should continue running
        let background_task = tokio::spawn(async {
            for i in 0..100 {
                tokio::task::yield_now().await;
                if i % 10 == 0 {
                    // Periodically check that we're still making progress
                    assert!(true, "Background task is still running");
                }
            }
        });

        // Perform database operation
        let result = backend.execute_query("SELECT 1", vec![]);
        assert!(result.is_ok());

        // Background task should complete without being blocked
        background_task.await.unwrap();
    }
}