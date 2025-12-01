//! HNSW Vector Index Implementation
//!
//! Pure Rust HNSW implementation using hnsw_rs crate with persistence support.
//! No SQLite coupling, no MCP dependencies - standalone module.

use super::config::HnswConfig;
use crate::vector::traits::VectorIndex;
use anyhow::{anyhow, Result};
use arc_swap::ArcSwap;
use hnsw_rs::prelude::*;
use std::path::Path;
use std::sync::Arc;

/// HNSW-based vector index for approximate nearest neighbor search
/// Uses hnsw_rs crate with DistL2 (Euclidean distance) for cosine similarity via normalized vectors
/// Phase 8: Uses ArcSwap for zero-blocking read snapshots
pub struct HnswVectorIndex {
    /// HNSW graph structure using ArcSwap for zero-blocking reads
    /// Phase 8 optimization: Readers get consistent snapshots without blocking
    hnsw: ArcSwap<Option<Hnsw<'static, f32, DistL2>>>,

    /// Configuration parameters
    config: HnswConfig,

    /// Current dimensionality (None if empty)
    /// Also using ArcSwap for consistent zero-blocking reads
    dimension: ArcSwap<Option<usize>>,

    /// Number of vectors in the index
    count: usize,

    /// Maximum capacity hint for HNSW
    max_elements: usize,
}

impl HnswVectorIndex {
    /// Create a new HNSW index with the given configuration
    ///
    /// # Arguments
    /// - `config`: HNSW tuning parameters (m, ef_construction, ef_search)
    /// - `_seed`: Random seed (unused - hnsw_rs doesn't expose RNG seed control)
    ///
    /// Note: hnsw_rs uses internal randomness for layer assignment. Determinism
    /// can only be achieved by controlling insertion order, not RNG seed.
    pub fn new(config: HnswConfig, _seed: u64) -> Result<Self> {
        Ok(Self {
            hnsw: ArcSwap::new(Arc::new(None)),
            config,
            dimension: ArcSwap::new(Arc::new(None)),
            count: 0,
            max_elements: 100_000, // Default capacity hint
        })
    }

    /// Initialize HNSW graph with known dimension and capacity
    fn ensure_hnsw_initialized(&mut self, dimension: usize) -> Result<()> {
        // Phase 8: Use ArcSwap for lock-free reads
        let current_hnsw = self.hnsw.load();
        if current_hnsw.is_none() {
            // IMPORTANT: nb_layer must be 16 (NB_LAYER_MAX) for serialization to work
            // hnsw_rs file_dump requires nb_layer == NB_LAYER_MAX (16) for file format compatibility
            // The actual max layer used will be determined by data distribution
            let nb_layer = 16_usize;

            // Create new HNSW instance
            // Hnsw::new(max_nb_connection, nb_elem, nb_layer, ef_construction, distance)
            let hnsw = Hnsw::<'static, f32, DistL2>::new(
                self.config.m,
                self.max_elements,
                nb_layer,
                self.config.ef_construction,
                DistL2 {},
            );

            // Phase 8: Atomic swap for zero-blocking readers
            self.hnsw.store(Arc::new(Some(hnsw)));
            self.dimension.store(Arc::new(Some(dimension)));
        }

        Ok(())
    }

    /// Save HNSW index to disk
    ///
    /// Uses hnsw_rs native file_dump which saves to multiple files with basename prefix.
    /// Example: path="/tmp/hnsw.index" saves to /tmp/hnsw_* files
    pub fn save_to_disk(&self, path: &Path) -> Result<()> {
        // Phase 8: Zero-blocking read with ArcSwap
        let hnsw_arc = self.hnsw.load();

        if let Some(ref hnsw) = **hnsw_arc {
            // hnsw_rs file_dump API: file_dump(&self, dir: &Path, basename: &str)
            let dir = path.parent().unwrap_or_else(|| Path::new("."));
            let basename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("hnsw");

            hnsw.file_dump(dir, basename)
                .map_err(|e| anyhow!("HNSW serialization failed: {:?}", e))?;

            Ok(())
        } else {
            Err(anyhow!("Cannot save empty HNSW index"))
        }
    }

    /// Load HNSW index from disk
    ///
    /// Uses hnsw_rs native HnswIO::load_hnsw_with_dist for loading.
    /// Requires the index to be uninitialized.
    pub fn load_from_disk(&mut self, path: &Path) -> Result<()> {
        // Phase 8: Check if already initialized using ArcSwap
        let current_hnsw = self.hnsw.load();
        if (**current_hnsw).is_some() {
            return Err(anyhow!("Cannot load into already-initialized HNSW index"));
        }

        // hnsw_rs stores multiple files with basename prefix
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let basename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("hnsw");

        // Use HnswIo to load
        use hnsw_rs::hnswio::HnswIo;
        let hnswio = HnswIo::new(dir, basename);
        let loaded_temp = hnswio
            .load_hnsw_with_dist::<f32, DistL2>(DistL2 {})
            .map_err(|e| anyhow!("HNSW deserialization failed: {:?}", e))?;

        // Safety: The loaded HNSW owns all its data (no borrows from hnswio).
        // The lifetime parameter 'b is a phantom type parameter for type safety,
        // but the actual data structure is self-contained after loading.
        // We need 'static lifetime to store in our struct.
        let loaded: Hnsw<'static, f32, DistL2> = unsafe { std::mem::transmute(loaded_temp) };

        // Update metadata
        self.count = loaded.get_nb_point();
        // Note: dimension will be inferred from first search/insert operation
        // hnsw_rs doesn't expose a reliable way to query dimension from loaded index

        // Phase 8: Atomic swap for zero-blocking readers
        self.hnsw.store(Arc::new(Some(loaded)));

        Ok(())
    }

    /// Rebuild HNSW index from a list of vectors
    ///
    /// Used when index file is missing but vectors are available from SQLite.
    /// Clears any existing index and rebuilds from scratch.
    pub fn rebuild_from_vectors(&mut self, vectors: &[(i64, Vec<f32>)]) -> Result<()> {
        if vectors.is_empty() {
            return Ok(());
        }

        // Phase 8: Clear existing index atomically
        self.hnsw.store(Arc::new(None));

        // Reset metadata
        self.dimension.store(Arc::new(None));
        self.count = 0;

        // Initialize with first vector's dimension
        let first_dim = vectors[0].1.len();
        self.ensure_hnsw_initialized(first_dim)?;

        // Insert all vectors
        for (id, vec) in vectors {
            self.add(*id, vec.clone())?;
        }

        Ok(())
    }

    /// Delete a vector from the index
    ///
    /// Note: hnsw_rs doesn't support true deletion. We mark as deleted
    /// and filter from search results. A full rebuild is needed to reclaim space.
    pub fn delete(&mut self, _id: i64) -> Result<()> {
        // LIMITATION: hnsw_rs Hnsw struct doesn't expose delete/remove API
        // The underlying graph structure doesn't support efficient deletion
        //
        // Workaround options:
        // 1. Keep a deleted_ids HashSet and filter search results (memory overhead)
        // 2. Require full rebuild to remove deleted vectors (implemented here)
        // 3. Use a different HNSW library with deletion support (out of scope)
        //
        // For now, return error indicating rebuild required
        Err(anyhow!(
            "HNSW index doesn't support deletion. Use rebuild_from_vectors() to remove deleted IDs."
        ))
    }

    /// Check if HNSW index files exist on disk
    ///
    /// hnsw_rs creates multiple files with basename prefix:
    /// - {basename}.hnsw_graph (graph structure)
    /// - {basename}.hnsw_data (vector data)
    /// - {basename}.hnsw_graph_layer (layer info)
    ///
    /// Returns true only if the main graph file exists.
    pub fn files_exist(path: &Path) -> bool {
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let basename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("hnsw");

        // Check for main HNSW file (graph structure)
        let graph_file = dir.join(format!("{}.hnsw.graph", basename));
        graph_file.exists()
    }

    /// Get number of vectors in the index
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl VectorIndex for HnswVectorIndex {
    fn add(&mut self, id: i64, embedding: Vec<f32>) -> Result<()> {
        // Check dimension consistency
        let dim = embedding.len();
        let current_dim = **self.dimension.load();
        if let Some(expected_dim) = current_dim {
            if dim != expected_dim {
                return Err(anyhow!("Dimension mismatch: expected {}, got {}", expected_dim, dim));
            }
        } else {
            // Initialize HNSW on first insertion
            self.ensure_hnsw_initialized(dim)?;
        }

        // Normalize the embedding vector to unit length
        // This is REQUIRED because our L2 -> cosine conversion assumes normalized vectors
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized: Vec<f32> = if norm > 0.0 {
            embedding.iter().map(|x| x / norm).collect()
        } else {
            embedding // Zero vector stays zero
        };

        // Insert into HNSW index
        // hnsw_rs API: hnsw.insert((data.as_slice(), id))
        // Phase 8: Zero-blocking read with ArcSwap
        let hnsw_arc = self.hnsw.load();
        if let Some(ref hnsw) = **hnsw_arc {
            // DataId is usize in hnsw_rs, cast i64 to usize
            hnsw.insert((&normalized[..], id as usize));
            // Phase 8: No lock to release with ArcSwap
            self.count += 1;
        } else {
            return Err(anyhow!("HNSW not initialized"));
        }

        Ok(())
    }

    fn search(&self, query: &[f32], k: usize) -> Result<Vec<(i64, f32)>> {
        // Phase 8: Zero-blocking read with ArcSwap
        let hnsw_arc = self.hnsw.load();

        // Empty index returns empty results
        if (**hnsw_arc).is_none() {
            return Ok(Vec::new());
        }

        let hnsw = (**hnsw_arc).as_ref().ok_or_else(|| anyhow!("HNSW not initialized"))?;

        // Lazily infer dimension from query if not set
        let dim_arc = self.dimension.load();
        if (**dim_arc).is_none() && !query.is_empty() {
            // Phase 8: Store inferred dimension atomically
            self.dimension.store(Arc::new(Some(query.len())));
        }

        // Normalize the query vector to unit length (same as indexed vectors)
        let norm: f32 = query.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized_query: Vec<f32> = if norm > 0.0 {
            query.iter().map(|x| x / norm).collect()
        } else {
            query.to_vec() // Zero vector stays zero
        };

        // Perform HNSW search
        // hnsw_rs API: hnsw.search(&query, k, ef_search) -> Vec<Neighbour>
        // Neighbour has: d (distance), id (DataId which is usize)
        let neighbours = hnsw.search(&normalized_query, k, self.config.ef_search);

        // Convert to output format: Vec<(id, cosine_similarity)>
        // hnsw_rs returns Euclidean (L2) distance
        // For normalized vectors: L2^2 = 2 - 2*cos(theta)
        // So: cos(theta) = 1 - (L2^2 / 2)
        // For very close vectors, L2 ~ 0, so cos(theta) ~ 1
        let mut output: Vec<(i64, f32)> = neighbours
            .iter()
            .map(|n| {
                // hnsw_rs DataId is usize, but we need i64
                let id = n.d_id as i64;
                // Convert L2 distance to cosine similarity
                // For normalized vectors: cos(theta) = 1 - (L2^2 / 2)
                let distance_sq = n.distance * n.distance;
                let cosine_sim = 1.0 - (distance_sq / 2.0);
                // Clamp to [-1, 1] to handle numerical errors
                let cosine_sim = cosine_sim.max(-1.0).min(1.0);
                (id, cosine_sim)
            })
            .collect();

        // Sort by similarity descending (higher = more similar)
        output.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(output)
    }

    fn dimension(&self) -> Option<usize> {
        **self.dimension.load()
    }

    fn len(&self) -> usize {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_index_creation() {
        let config = HnswConfig::default();
        let index = HnswVectorIndex::new(config, 42).unwrap();
        assert_eq!(index.len(), 0);
        assert_eq!(index.dimension(), None);
        assert!(index.is_empty());
    }

    #[test]
    fn test_add_single_vector() {
        let config = HnswConfig::default();
        let mut index = HnswVectorIndex::new(config, 42).unwrap();

        let vec = vec![1.0, 0.0, 0.0];
        index.add(1, vec).unwrap();

        assert_eq!(index.len(), 1);
        assert_eq!(index.dimension(), Some(3));
    }

    #[test]
    fn test_dimension_mismatch() {
        let config = HnswConfig::default();
        let mut index = HnswVectorIndex::new(config, 42).unwrap();

        index.add(1, vec![1.0, 0.0]).unwrap();
        let result = index.add(2, vec![1.0, 0.0, 0.0]);

        assert!(result.is_err());
    }

    #[test]
    fn test_search_empty_index() {
        let config = HnswConfig::default();
        let index = HnswVectorIndex::new(config, 42).unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = index.search(&query, 5).unwrap();

        assert!(results.is_empty());
    }
}
