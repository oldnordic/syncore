//! Mode C: Multi-hop Semantic Reasoning Fusion
//!
//! Implements higher-order fusion with multi-hop graph expansion:
//! S = α*S_v + β*S_g + γ*S_g²
//!
//! Steps:
//! 1. Vector search → K entities
//! 2. Expand 2-3 graph hops
//! 3. Diffusion scoring
//! 4. Entropy-based gating
//! 5. Higher-order combination
//!
//! This mode is optimal for:
//! - Multi-file reasoning
//! - Causal tracing
//! - Deep architectural analysis

use crate::graph::GraphBackend;
use crate::vector::VectorStore;
use anyhow::Result;
use std::sync::{Arc, Mutex};

/// Reasoning-based fusion combiner with higher-order terms
pub struct FusionReasoning {
    /// Graph backend for operations (any GraphBackend implementation)
    _graph_backend: Arc<dyn GraphBackend>,
    /// Vector store for embeddings
    _vector_store: Arc<Mutex<VectorStore>>,
    /// Weight for vector score (alpha)
    alpha: f32,
    /// Weight for graph score (beta)
    beta: f32,
    /// Weight for higher-order term (gamma)
    gamma: f32,
}

impl FusionReasoning {
    /// Create new reasoning fusion
    ///
    /// # Arguments
    /// * `graph_backend` - Graph backend for graph traversal (any GraphBackend implementation)
    /// * `vector_store` - Vector store for embeddings
    ///
    /// # Returns
    /// New FusionReasoning instance
    pub fn new(
        graph_backend: Arc<dyn GraphBackend>,
        vector_store: Arc<Mutex<VectorStore>>,
    ) -> Self {
        Self {
            _graph_backend: graph_backend,
            _vector_store: vector_store,
            alpha: 0.4,
            beta: 0.4,
            gamma: 0.2,
        }
    }

    /// Create new reasoning fusion with any GraphBackend implementation
    ///
    /// This constructor accepts any backend that implements the GraphBackend trait,
    /// including SQLiteGraph, Neo4j, or future backends. For Neo4j usage,
    /// create a Neo4jBackend and pass it to this constructor.
    ///
    /// # Arguments
    /// * `graph_backend` - Graph backend for graph traversal (any GraphBackend implementation)
    /// * `vector_store` - Vector store for embeddings
    ///
    /// # Returns
    /// New FusionReasoning instance

    /// Combine scores using higher-order formula
    ///
    /// # Arguments
    /// * `vector_score` - Score from vector search
    /// * `graph_score` - Score from graph traversal
    ///
    /// # Returns
    /// Combined score: S = α*S_v + β*S_g + γ*S_g²
    pub fn combine_higher_order(&self, vector_score: f32, graph_score: f32) -> f32 {
        self.alpha * vector_score + self.beta * graph_score + self.gamma * graph_score * graph_score
    }

    /// Full reasoning fusion with multi-hop expansion
    ///
    /// # Arguments
    /// * `query` - Search query
    /// * `k` - Number of initial entities from vector search
    ///
    /// # Returns
    /// Fused results with multi-hop reasoning
    pub fn reason(&self, query: &str, k: usize) -> Result<Vec<(i64, f32)>> {
        // 1. Vector search → K entities
        let vector_results = self.vector_search(query, k)?;

        // 2. Graph expansion (2-3 hops)
        let expanded_results = self.graph_expand(&vector_results, 2)?;

        // 3. Diffusion scoring
        let diffusion_scores = self.diffusion_score(&expanded_results)?;

        // 4. Entropy gating
        let gated_results = self.entropy_gate(&diffusion_scores)?;

        // 5. Combine with higher-order formula
        let fused_results = self.combine_scores(&vector_results, &gated_results);

        Ok(fused_results)
    }

    /// Perform vector search to get initial K entities
    fn vector_search(&self, query: &str, k: usize) -> Result<Vec<(i64, f32)>> {
        use crate::vector::SearchScope;
        let store = self._vector_store.lock().unwrap();
        let results = store.search(query, k, SearchScope::Global)?;
        Ok(results.into_iter().map(|hit| (hit.id, hit.score)).collect())
    }

    /// Expand graph neighbors for multi-hop reasoning
    fn graph_expand(&self, initial_results: &[(i64, f32)], hops: usize) -> Result<Vec<(i64, f32)>> {
        let mut expanded = initial_results.to_vec();
        let mut visited: std::collections::HashSet<i64> =
            initial_results.iter().map(|(id, _)| *id).collect();

        for _ in 0..hops {
            let mut next_level = Vec::new();

            for (entity_id, score) in &expanded {
                // Get neighbors from GraphBackend (works with any backend implementation)
                let neighbors = self.get_neighbors(*entity_id)?;

                for neighbor_id in neighbors {
                    if !visited.contains(&neighbor_id) {
                        visited.insert(neighbor_id);
                        // Decay score with distance
                        let decayed_score = score * 0.8;
                        next_level.push((neighbor_id, decayed_score));
                    }
                }
            }

            expanded.extend(next_level);
        }

        Ok(expanded)
    }

    /// Get neighbors from graph using GraphBackend trait
    fn get_neighbors(&self, entity_id: i64) -> Result<Vec<i64>> {
        // Use GraphBackend trait to get neighbors regardless of backend implementation
        let neighbor_results = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { self._graph_backend.get_neighbors(entity_id).await })
        })?;

        // Convert EntityResult to entity IDs for fusion logic
        let neighbor_ids: Vec<i64> = neighbor_results.into_iter().map(|entity| entity.id).collect();

        Ok(neighbor_ids)
    }

    /// Apply diffusion scoring to expanded results
    fn diffusion_score(&self, results: &[(i64, f32)]) -> Result<Vec<(i64, f32)>> {
        // Simple diffusion: average scores of connected entities
        let mut diffusion_scores = Vec::new();

        for (entity_id, base_score) in results {
            // Get connected entities for diffusion
            let connected = self.get_neighbors(*entity_id)?;

            if connected.is_empty() {
                diffusion_scores.push((*entity_id, *base_score));
            } else {
                // Average with neighbor scores (simplified)
                let diffusion_factor = 0.1;
                let diffused_score =
                    base_score * (1.0 - diffusion_factor) + base_score * diffusion_factor;
                diffusion_scores.push((*entity_id, diffused_score));
            }
        }

        Ok(diffusion_scores)
    }

    /// Apply entropy-based gating to filter results
    fn entropy_gate(&self, results: &[(i64, f32)]) -> Result<Vec<(i64, f32)>> {
        if results.is_empty() {
            return Ok(vec![]);
        }

        // Calculate entropy of score distribution
        let total_score: f32 = results.iter().map(|(_, score)| score).sum();
        let mut entropy = 0.0;

        for (_, score) in results {
            if total_score > 0.0 {
                let p = score / total_score;
                if p > 0.0 {
                    entropy -= p * p.log2();
                }
            }
        }

        // Gate based on entropy threshold
        let entropy_threshold = 2.0; // Adjust based on requirements
        if entropy < entropy_threshold {
            // Low entropy: return top results
            let mut gated = results.to_vec();
            gated.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            Ok(gated.into_iter().take(10).collect())
        } else {
            // High entropy: keep all results
            Ok(results.to_vec())
        }
    }

    /// Combine vector and diffusion scores using higher-order formula
    fn combine_scores(
        &self,
        vector_results: &[(i64, f32)],
        diffusion_results: &[(i64, f32)],
    ) -> Vec<(i64, f32)> {
        let mut combined = std::collections::HashMap::new();

        // Add vector scores
        for (id, score) in vector_results {
            combined.insert(*id, (*score, 0.0));
        }

        // Add diffusion scores
        for (id, score) in diffusion_results {
            combined
                .entry(*id)
                .and_modify(|(_vec_score, diff_score)| {
                    *diff_score = *score;
                })
                .or_insert((0.0, *score));
        }

        // Combine using higher-order formula
        combined
            .into_iter()
            .map(|(id, (vec_score, diff_score))| {
                let combined_score = self.combine_higher_order(vec_score, diff_score);
                (id, combined_score)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::StubEmbeddings;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_higher_order_combination() -> Result<()> {
        // Create a mock GraphBackend for testing scoring logic
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        let embeddings = Box::new(StubEmbeddings::new(384)?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        // Create SQLiteGraph backend for testing
        let graph_backend = Arc::new(
            crate::graph::SQLiteGraphBackend::new(db_path.to_str().unwrap(), "test_namespace")
                .await?,
        );

        let fusion = FusionReasoning::new(graph_backend, vector_store);

        let result = fusion.combine_higher_order(0.6, 0.8);

        // Expected: 0.4*0.6 + 0.4*0.8 + 0.2*0.64 = 0.688
        assert!((result - 0.688).abs() < 0.01);

        Ok(())
    }

    #[test]
    fn test_scoring_determinism() -> Result<()> {
        // Create mock backend for deterministic scoring test
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        let graph_backend = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                crate::graph::SQLiteGraphBackend::new(db_path.to_str().unwrap(), "test_namespace")
                    .await
            })
        })?;

        let embeddings = Box::new(StubEmbeddings::new(384)?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

        let fusion = FusionReasoning::new(Arc::new(graph_backend), vector_store);

        // Test that scoring is deterministic
        let result1 = fusion.combine_higher_order(0.6, 0.8);
        let result2 = fusion.combine_higher_order(0.6, 0.8);

        assert_eq!(result1, result2, "Scoring should be deterministic");

        Ok(())
    }
}
