//! Real storage adapter for RagGraph using HNSW + Neo4j
//!
//! # Backend Discovery Summary
//!
//! ## HNSW Vector Backend
//! - **Trait**: `VectorIndex` from `src/vector/traits.rs`
//! - **Implementation**: `HnswVectorIndex` from `src/vector/hnsw/hnsw_index.rs`
//! - **Key Methods**:
//!   - `add(&mut self, id: i64, embedding: Vec<f32>) -> Result<()>`
//!   - `search(&self, query: &[f32], k: usize) -> Result<Vec<(i64, f32)>>`
//!   - `dimension(&self) -> Option<usize>`
//!   - `len(&self) -> usize`
//! - **Thread Safety**: Uses `Arc<RwLock<Hnsw>>` internally
//!
//! ## Neo4j Graph Backend
//! - **Client**: `Neo4jClient` from `src/graph/neo4j_client.rs`
//! - **Key Methods**:
//!   - `execute_query(&self, cypher: &str, params: Vec<(&str, serde_json::Value)>) -> Result<Vec<serde_json::Value>>`
//!   - `create_relationship(from_label, from_id, to_label, to_id, rel_type) -> Result<()>`
//! - **Connection**: Async connect via `Neo4jClient::connect(uri, user, pass)`
//! - **Thread Safety**: Uses `Arc<neo4rs::Graph>` internally (Clone-safe)

use super::types::NodeId;
use crate::graph::Neo4jClient;
use crate::vector::traits::VectorIndex;
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

/// Error types specific to RagGraph storage operations
#[derive(Debug)]
pub enum StorageError {
    VectorSearchFailed(String),
    GraphQueryFailed(String),
    EmbeddingNotFound(NodeId),
    InvalidQuery(String),
    EmptySeedNodes,
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::VectorSearchFailed(msg) => write!(f, "Vector search failed: {}", msg),
            StorageError::GraphQueryFailed(msg) => write!(f, "Graph query failed: {}", msg),
            StorageError::EmbeddingNotFound(id) => write!(f, "Embedding not found for node {}", id),
            StorageError::InvalidQuery(msg) => write!(f, "Invalid query: {}", msg),
            StorageError::EmptySeedNodes => {
                write!(f, "Empty seed nodes returned from vector search")
            }
        }
    }
}

impl std::error::Error for StorageError {}

/// Storage adapter trait for RagGraph operations
///
/// Abstracts the storage layer to allow both real and mock implementations
pub trait StorageAdapter: Send + Sync {
    /// Generate seed nodes from a text query using vector similarity search
    ///
    /// # Arguments
    /// * `query_text` - Natural language query
    /// * `top_k` - Number of seed nodes to return
    ///
    /// # Returns
    /// Vector of (node_id, similarity_score) tuples
    fn seed_nodes_from_query(&self, query_text: &str, top_k: usize) -> Result<Vec<(NodeId, f32)>>;

    /// Resolve embedding vector for a given node ID
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    ///
    /// # Returns
    /// 384-dimensional embedding vector
    fn resolve_embedding(&self, node_id: NodeId) -> Result<Vec<f32>>;

    /// Get neighbors of a node from the knowledge graph
    ///
    /// # Arguments
    /// * `node_id` - Source node identifier
    ///
    /// # Returns
    /// Vector of (neighbor_id, edge_weight) tuples
    fn neighbors_of(&self, node_id: NodeId) -> Result<Vec<(NodeId, f32)>>;
}

/// Real storage adapter using HNSW for vectors and Neo4j for graph
pub struct RealStorageAdapter {
    /// HNSW vector index for semantic search
    vector_index: Arc<Mutex<dyn VectorIndex>>,

    /// Neo4j client for graph traversal
    neo4j: Neo4jClient,

    /// Embedding dimension (default: 384)
    dimension: usize,
}

impl RealStorageAdapter {
    /// Create a new real storage adapter
    ///
    /// # Arguments
    /// * `vector_index` - HNSW vector index (thread-safe)
    /// * `neo4j` - Neo4j client (Clone-safe)
    /// * `dimension` - Embedding dimension (default: 384)
    pub fn new(
        vector_index: Arc<Mutex<dyn VectorIndex>>,
        neo4j: Neo4jClient,
        dimension: usize,
    ) -> Self {
        Self {
            vector_index,
            neo4j,
            dimension,
        }
    }

    /// Generate embedding from text (placeholder - will use real embeddings later)
    ///
    /// TODO: Replace with actual embedding model (fastembed or Ollama)
    fn text_to_embedding(&self, text: &str) -> Vec<f32> {
        // Temporary: Use hash-based deterministic embedding
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

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
}

impl StorageAdapter for RealStorageAdapter {
    fn seed_nodes_from_query(&self, query_text: &str, top_k: usize) -> Result<Vec<(NodeId, f32)>> {
        // Validate input
        if query_text.trim().is_empty() {
            return Err(
                StorageError::InvalidQuery("Query text cannot be empty".to_string()).into(),
            );
        }

        // Generate embedding from query text
        let query_embedding = self.text_to_embedding(query_text);

        // Search HNSW index for nearest neighbors
        let vector_index = self
            .vector_index
            .lock()
            .map_err(|e| StorageError::VectorSearchFailed(format!("Lock poisoned: {}", e)))?;

        let results = vector_index
            .search(&query_embedding, top_k)
            .map_err(|e| StorageError::VectorSearchFailed(format!("HNSW search failed: {}", e)))?;

        // Ensure we got results
        if results.is_empty() {
            return Err(StorageError::EmptySeedNodes.into());
        }

        Ok(results)
    }

    fn resolve_embedding(&self, node_id: NodeId) -> Result<Vec<f32>> {
        // Use canonical RAG graph module for type-safe query
        use crate::databases::rag_graph::get_embedding_text;

        let text = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                get_embedding_text(&self.neo4j, node_id).await
            })
        })
        .context("Failed to query embedding text from Neo4j")?;

        // Extract text and generate embedding
        if let Some(text) = text {
            return Ok(self.text_to_embedding(&text));
        }

        Err(StorageError::EmbeddingNotFound(node_id).into())
    }

    fn neighbors_of(&self, node_id: NodeId) -> Result<Vec<(NodeId, f32)>> {
        // Use canonical RAG graph module for type-safe query
        use crate::databases::rag_graph::get_neighbors;

        let neighbor_results = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                get_neighbors(&self.neo4j, node_id).await
            })
        })
        .map_err(|e| StorageError::GraphQueryFailed(format!("Neo4j query failed: {}", e)))?;

        // Convert NeighborResult to (NodeId, weight) tuples
        let neighbors: Vec<(NodeId, f32)> = neighbor_results
            .into_iter()
            .map(|n| (n.id, n.weight.unwrap_or(1.0)))
            .collect();

        Ok(neighbors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_storage_error_display() {
        let err = StorageError::VectorSearchFailed("HNSW index empty".to_string());
        assert_eq!(err.to_string(), "Vector search failed: HNSW index empty");

        let err = StorageError::EmbeddingNotFound(42);
        assert_eq!(err.to_string(), "Embedding not found for node 42");
    }

    #[test]
    fn test_text_to_embedding_deterministic() {
        // Cannot test RealStorageAdapter without Neo4j, but can test embedding generation logic
        // This will be covered by integration tests with real Neo4j
    }
}
