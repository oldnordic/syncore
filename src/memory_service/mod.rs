//! Memory Service Module
//!
//! Provides unified short-term (RAM) and long-term (RagGraph + SQL) memory
//! for TOON internal reasoning.

mod error;
pub mod ltm_adapter;
mod ram_cache;
pub mod toon;
pub mod toon_controller;
pub mod toon_decoder;
pub mod toon_engine;
pub mod toon_error;
pub mod toon_prompt;
pub mod toon_result;

pub use error::MemoryError;
pub use ltm_adapter::{LongTermStore, LtmAdapter, LtmStats};
pub use ram_cache::{MemoryEntry, RamCache};
pub use toon_controller::ToonController;
pub use toon_decoder::ToonDecoder;
pub use toon_engine::{ToonExecutor, ToonGraph, ToonInstr, ToonNode};
pub use toon_error::ToonError;
pub use toon_prompt::ToonPromptBuilder;
pub use toon_result::{ToonResult, ToonStepResult};

use crate::common::fast_map::FastHashMap;
use crate::db::DbManager;

/// Memory Service with short-term (RAM) and long-term (persistent) storage
pub struct MemoryService {
    ram_cache: RamCache,
    ltm: Option<LtmAdapter>,
    dimension: usize,
    capacity: usize,
}

impl MemoryService {
    /// Create new MemoryService with RAM-only storage
    ///
    /// # Arguments
    /// * `dimension` - Embedding vector dimension
    /// * `capacity` - RAM cache capacity
    pub fn new(dimension: usize, capacity: usize) -> Self {
        Self {
            ram_cache: RamCache::new(dimension, capacity),
            ltm: None,
            dimension,
            capacity,
        }
    }

    /// Create new MemoryService with RAM + LTM storage
    ///
    /// # Arguments
    /// * `dimension` - Embedding vector dimension
    /// * `capacity` - RAM cache capacity
    /// * `db_manager` - Database manager for LTM storage
    pub fn new_with_ltm(
        dimension: usize,
        capacity: usize,
        db_manager: DbManager,
    ) -> Result<Self, MemoryError> {
        let ltm = LtmAdapter::new_with_mock(db_manager, dimension)?;

        Ok(Self {
            ram_cache: RamCache::new(dimension, capacity),
            ltm: Some(ltm),
            dimension,
            capacity,
        })
    }

    /// Store memory entry
    ///
    /// Returns the ID of the stored entry
    pub fn store(&mut self, entry: MemoryEntry) -> Result<String, MemoryError> {
        let id = entry.id.clone();

        // Store in RAM cache
        self.ram_cache.insert(entry.clone())?;

        // Store in LTM if available
        if let Some(ref mut ltm) = self.ltm {
            ltm.ltm_store(&entry)?;
        }

        Ok(id)
    }

    /// Retrieve memory entries by query
    ///
    /// Searches both RAM and LTM (if available), merges results, deduplicates,
    /// and returns top-k entries sorted by similarity.
    ///
    /// # Arguments
    /// * `query_embedding` - Query vector
    /// * `k` - Number of results to return
    pub fn retrieve(&self, query_embedding: &[f32], k: usize) -> Vec<MemoryEntry> {
        // Search RAM cache
        let ram_results = self.ram_cache.search(query_embedding, k * 2);

        // Search LTM if available
        let ltm_results = if let Some(ref ltm) = self.ltm {
            ltm.ltm_query(query_embedding, k * 2)
                .unwrap_or_else(|_| vec![])
        } else {
            vec![]
        };

        // Merge and deduplicate results
        let mut entries_by_id: FastHashMap<String, (MemoryEntry, f32)> = FastHashMap::default();

        // Add RAM results
        for entry in ram_results {
            let similarity = Self::cosine_similarity(query_embedding, &entry.embedding);
            entries_by_id.insert(entry.id.clone(), (entry, similarity));
        }

        // Add LTM results (don't overwrite if already in RAM)
        for entry in ltm_results {
            if !entries_by_id.contains_key(&entry.id) {
                let similarity = Self::cosine_similarity(query_embedding, &entry.embedding);
                entries_by_id.insert(entry.id.clone(), (entry, similarity));
            }
        }

        // Sort deterministically: similarity DESC, then ID ASC
        let mut scored: Vec<(String, f32, MemoryEntry)> = entries_by_id
            .into_iter()
            .map(|(id, (entry, similarity))| (id, similarity, entry))
            .collect();

        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        // Take top-k results
        scored
            .into_iter()
            .take(k)
            .map(|(_, _, entry)| entry)
            .collect()
    }

    /// Get the embedding dimension used by this memory service
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Get cache statistics
    pub fn stats(&self) -> MemoryStats {
        let (ltm_nodes, ltm_edges) = if let Some(ref ltm) = self.ltm {
            let ltm_stats = ltm.ltm_stats().unwrap_or(LtmStats {
                node_count: 0,
                edge_count: 0,
                sql_rows: 0,
            });
            (ltm_stats.sql_rows, ltm_stats.edge_count)
        } else {
            (0, 0)
        };

        MemoryStats {
            ram_size: self.ram_cache.len(),
            ram_capacity: self.capacity,
            dimension: self.dimension,
            ltm_nodes,
            ltm_edges,
        }
    }

    /// Compute cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }
}

/// Memory service statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub ram_size: usize,
    pub ram_capacity: usize,
    pub dimension: usize,
    pub ltm_nodes: usize,
    pub ltm_edges: usize,
}
