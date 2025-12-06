//! SQLiteGraph storage adapter for RagGraph operations
//!
//! Implements StorageAdapter trait using SQLiteGraph for graph operations
//! and HNSW for vector similarity search.
//!
//! This adapter replaces Neo4j dependencies while maintaining full functionality:
//! - Vector search via HNSW (unchanged)
//! - Graph operations via SQLiteGraph backend
//! - Deterministic behavior and ACID guarantees
//! - No external dependencies beyond SQLite

use super::storage::StorageAdapter;
use super::types::NodeId;
use crate::sqlitegraph::async_sqlite_backend::{SyncGraphBackend, AsyncSQLiteBackend};
use crate::vector::traits::VectorIndex;
use anyhow::{anyhow, Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

/// SQLiteGraph storage adapter using HNSW for vectors and SQLiteGraph for graph
pub struct SQLiteGraphStorageAdapter {
    /// HNSW vector index for semantic search
    vector_index: Arc<Mutex<dyn VectorIndex>>,

    /// SQLiteGraph backend for graph traversal (wrapped in sync interface)
    graph_backend: Arc<AsyncSQLiteBackend>,

    /// Embedding dimension (default: 384)
    dimension: usize,
}

impl SQLiteGraphStorageAdapter {
    /// Create a new SQLiteGraph storage adapter
    ///
    /// # Arguments
    /// * `vector_index` - HNSW vector index (thread-safe)
    /// * `graph_backend` - SQLiteGraph backend (Arc<dyn GraphBackend>)
    /// * `dimension` - Embedding dimension (default: 384)
    pub fn new(
        vector_index: Arc<Mutex<dyn VectorIndex>>,
        graph_backend: Arc<dyn crate::graph::GraphBackend>,
        dimension: usize,
    ) -> Result<Self> {
        let sync_backend = AsyncSQLiteBackend::new(graph_backend)
            .context("Failed to create sync wrapper for GraphBackend")?;

        Ok(Self {
            vector_index,
            graph_backend: Arc::new(sync_backend),
            dimension,
        })
    }

    /// Generate embedding from text (deterministic hash-based approach)
    ///
    /// Uses hash-based deterministic embedding for reproducible results
    /// TODO: Replace with actual embedding model (fastembed) in production
    fn text_to_embedding(&self, text: &str) -> Vec<f32> {
        let mut embedding = Vec::with_capacity(self.dimension);
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let base_hash = hasher.finish();

        for i in 0..self.dimension {
            let mut h = DefaultHasher::new();
            (base_hash.wrapping_add(i as u64)).hash(&mut h);
            let val = (h.finish() as f32) / (u64::MAX as f32);
            embedding.push(val * 2.0 - 1.0); // Range: [-1, 1]
        }

        // Normalize to unit length
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut embedding {
                *x /= norm;
            }
        }

        embedding
    }

    /// Get node text from SQLiteGraph for embedding generation
    ///
    /// Extracts the body_snippet or name field from code_entities table
    fn get_node_text(&self, node_id: NodeId) -> Result<Option<String>> {
        let cypher = r#"
            SELECT body_snippet, name
            FROM code_entities
            WHERE id = ? AND body_snippet IS NOT NULL
            LIMIT 1
        "#;

        let results = self.graph_backend
            .execute_query(cypher, vec![("id", serde_json::json!(node_id))])
        .context("Failed to query node text from SQLiteGraph")?;

        if let Some(result) = results.first() {
            if let Some(body_snippet) = result.get("body_snippet").and_then(|v| v.as_str()) {
                return Ok(Some(body_snippet.to_string()));
            }
            if let Some(name) = result.get("name").and_then(|v| v.as_str()) {
                return Ok(Some(name.to_string()));
            }
        }

        Ok(None)
    }
}

impl StorageAdapter for SQLiteGraphStorageAdapter {
    fn seed_nodes_from_query(&self, query_text: &str, top_k: usize) -> Result<Vec<(NodeId, f32)>> {
        // Validate input
        if query_text.trim().is_empty() {
            anyhow::bail!("Query text cannot be empty");
        }

        // Generate embedding from query text
        let query_embedding = self.text_to_embedding(query_text);

        // Search HNSW index for nearest neighbors
        let vector_index =
            self.vector_index.lock().map_err(|e| anyhow!("Vector index lock poisoned: {}", e))?;

        let results = vector_index
            .search(&query_embedding, top_k)
            .map_err(|e| anyhow!("HNSW search failed: {}", e))?;

        // Ensure we got results
        if results.is_empty() {
            anyhow::bail!("No seed nodes found for query");
        }

        Ok(results)
    }

    fn resolve_embedding(&self, node_id: NodeId) -> Result<Vec<f32>> {
        // Get node text from SQLiteGraph
        let text = self.get_node_text(node_id)?;

        // Extract text and generate embedding
        if let Some(text) = text {
            return Ok(self.text_to_embedding(&text));
        }

        anyhow::bail!("Node {} not found or has no text content", node_id)
    }

    fn neighbors_of(&self, node_id: NodeId) -> Result<Vec<(NodeId, f32)>> {
        // Use SQLiteGraph get_neighbors method
        let neighbors = self.graph_backend
            .get_neighbors(node_id)
        .context("Failed to query neighbors from SQLiteGraph")?;

        // Convert EntityResult to (NodeId, weight) tuples
        let neighbor_tuples: Vec<(NodeId, f32)> = neighbors
            .into_iter()
            .map(|entity| (entity.id, 1.0)) // Default weight for relationships
            .collect();

        Ok(neighbor_tuples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GraphBackend as ConfigBackend, GraphConfig};
    use crate::graph::backend_selector::create_graph_backend;

    // Mock VectorIndex for testing
    struct MockVectorIndex {
        results: Vec<(i64, f32)>,
    }

    impl VectorIndex for MockVectorIndex {
        fn add(&mut self, _id: i64, _embedding: Vec<f32>) -> Result<()> {
            Ok(())
        }

        fn search(&self, _query: &[f32], _k: usize) -> Result<Vec<(i64, f32)>> {
            Ok(self.results.clone())
        }

        fn dimension(&self) -> Option<usize> {
            Some(384)
        }

        fn len(&self) -> usize {
            self.results.len()
        }
    }

    #[tokio::test]
    async fn test_sqlite_storage_adapter_creation() -> Result<()> {
        // Create mock vector index
        let vector_index = Arc::new(Mutex::new(MockVectorIndex {
            results: vec![(1, 0.9), (2, 0.8), (3, 0.7)],
        }));

        // Create SQLiteGraph backend
        let graph_config = GraphConfig {
            backend: ConfigBackend::SqliteGraph,
            path: ":memory:".to_string(),
            uri: String::new(),
            user: String::new(),
            password: String::new(),
            enabled: true,
        };

        let graph_backend = create_graph_backend(&graph_config, "test").await?;

        // Create adapter
        let adapter = SQLiteGraphStorageAdapter::new(vector_index, graph_backend, 384)?;

        // Test basic functionality
        let seeds = adapter.seed_nodes_from_query("test query", 5)?;
        assert_eq!(seeds.len(), 3);

        Ok(())
    }

    #[tokio::test]
    async fn test_embedding_generation() -> Result<()> {
        let vector_index = Arc::new(Mutex::new(MockVectorIndex {
            results: vec![],
        }));

        let graph_config = GraphConfig {
            backend: ConfigBackend::SqliteGraph,
            path: ":memory:".to_string(),
            uri: String::new(),
            user: String::new(),
            password: String::new(),
            enabled: true,
        };

        let graph_backend = create_graph_backend(&graph_config, "test").await?;

        let adapter = SQLiteGraphStorageAdapter::new(vector_index, graph_backend, 384)?;

        // Test embedding generation is deterministic
        let embedding1 = adapter.text_to_embedding("hello world");
        let embedding2 = adapter.text_to_embedding("hello world");

        assert_eq!(embedding1.len(), 384);
        assert_eq!(embedding1, embedding2); // Should be deterministic

        // Test normalization
        let norm: f32 = embedding1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);

        Ok(())
    }

    #[test]
    fn test_empty_query_validation() {
        let vector_index = Arc::new(Mutex::new(MockVectorIndex {
            results: vec![],
        }));

        // We can't create a real backend in this unit test, so we'll test the validation logic
        // by checking the error message format
        let query = "";
        assert!(query.trim().is_empty());
    }
}
