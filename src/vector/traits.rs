//! Vector index trait for pluggable backends
//!
//! This trait enables switching between different vector indexing implementations
//! (linear scan, HNSW, IVF, etc.) without changing client code.

use anyhow::Result;

/// Core trait for vector indexing backends
///
/// Implementations must handle:
/// - Adding vectors with unique IDs
/// - Nearest-neighbor search queries
/// - Consistent distance metrics (cosine similarity recommended for embeddings)
pub trait VectorIndex: Send + Sync {
    /// Add a vector with its unique identifier to the index
    ///
    /// # Arguments
    /// - `id`: Unique identifier for this vector
    /// - `embedding`: Dense vector representation
    ///
    /// # Errors
    /// - Returns error if dimension mismatch detected
    /// - Returns error if ID already exists (implementation-specific)
    fn add(&mut self, id: i64, embedding: Vec<f32>) -> Result<()>;

    /// Search for k-nearest neighbors to the query vector
    ///
    /// # Arguments
    /// - `query`: Query vector (must match index dimensionality)
    /// - `k`: Number of nearest neighbors to return
    ///
    /// # Returns
    /// - Vec of (id, distance) tuples sorted by distance (closest first)
    /// - Empty vec if index is empty
    /// - May return fewer than k results if index contains < k vectors
    ///
    /// # Distance Metric
    /// - Uses cosine similarity (higher = more similar)
    /// - Implementations should document their metric choice
    fn search(&self, query: &[f32], k: usize) -> Result<Vec<(i64, f32)>>;

    /// Get the dimensionality of vectors in this index
    ///
    /// # Returns
    /// - Dimension of vectors (None if index is empty)
    fn dimension(&self) -> Option<usize>;

    /// Get the number of vectors in the index
    fn len(&self) -> usize;

    /// Check if the index is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
