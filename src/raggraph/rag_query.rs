//! RAG query engine

use super::config::RagGraphConfig;
use super::hopgraph::HopGraphTransformer;
use super::storage::StorageAdapter;
use super::types::{NodeId, RagGraphResult};
use anyhow::Result;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// RAG query engine combining vector search and graph reasoning
pub struct RagQuery {
    transformer: HopGraphTransformer,
    config: RagGraphConfig,
    storage: Option<Arc<dyn StorageAdapter>>,
}

impl RagQuery {
    pub fn new() -> Self {
        let config = RagGraphConfig::default();
        let transformer = HopGraphTransformer::new(config.clone());
        Self {
            transformer,
            config,
            storage: None,
        }
    }

    pub fn with_storage(config: RagGraphConfig, storage: Arc<dyn StorageAdapter>) -> Self {
        let transformer = HopGraphTransformer::with_storage(config.clone(), storage.clone());
        Self {
            transformer,
            config,
            storage: Some(storage),
        }
    }

    pub fn query(&self, query_text: &str) -> Result<RagGraphResult> {
        if query_text.is_empty() {
            anyhow::bail!("Query text cannot be empty");
        }

        // Generate seed nodes based on backend mode and config
        let seed_nodes = if let Some(ref storage) = self.storage {
            // Real mode: use storage adapter for vector search with config.top_k
            storage
                .seed_nodes_from_query(query_text, self.config.top_k)?
                .into_iter()
                .map(|(id, _score)| id)
                .collect()
        } else {
            // Mock mode: deterministic hash-based seeds, respecting config.top_k
            let mut seeds = self.generate_seed_nodes(query_text);
            // Limit seeds to config.top_k if needed
            if seeds.len() > self.config.top_k {
                seeds.truncate(self.config.top_k);
            }
            seeds
        };

        // Run multi-hop reasoning with HopGraph (uses config internally)
        let result = self.transformer.multi_hop_reasoning(&seed_nodes)?;

        Ok(result)
    }

    /// Get the current configuration
    pub fn config(&self) -> &RagGraphConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: RagGraphConfig) {
        self.config = config.clone();
        if let Some(ref storage) = self.storage {
            self.transformer = HopGraphTransformer::with_storage(config.clone(), storage.clone());
        } else {
            self.transformer = HopGraphTransformer::new(config.clone());
        }
    }

    /// Generate deterministic seed nodes from query text
    /// In production, this would be replaced with HNSW vector search
    fn generate_seed_nodes(&self, query_text: &str) -> Vec<NodeId> {
        let mut hasher = DefaultHasher::new();
        query_text.hash(&mut hasher);
        let hash = hasher.finish();

        // Generate 2-3 seed nodes from hash
        let num_seeds = 2 + (hash % 2) as usize;
        let mut seeds = Vec::new();

        for i in 0..num_seeds {
            let seed = ((hash >> (i * 16)) & 0xFFFF) as NodeId;
            // Ensure seeds are in reasonable range (1-1000)
            let normalized_seed = 1 + (seed % 1000);
            seeds.push(normalized_seed);
        }

        seeds
    }
}
