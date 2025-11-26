//! APEX 1.9-G: GRAPH Embedding Service (Architecture)
//!
//! Defines the architecture for graph-aware embeddings for code entities by combining:
//! - CODE domain embeddings (BGE-small-en-v1.5) for code text
//! - Graph structural features (degree, neighbors, edge types)
//!
//! ## Architecture
//!
//! GraphEmbeddingService provides the design for bridging CODE embeddings and
//! graph topology to create GRAPH domain embeddings.
//!
//! ## Graph-BERT Seam (PRIMARY PURPOSE OF THIS MODULE)
//!
//! The `GraphEmbeddingStrategy` trait defines how CODE embeddings and graph
//! features are combined. This is the **extensibility point** for plugging in
//! Graph-BERT or other GNN models.
//!
//! - **Default**: SimpleFeatureCombiner (deterministic feature concatenation)
//! - **Future**: Graph-BERT model (learned graph neural embeddings)
//!
//! ## Example
//!
//! ```no_run
//! use syncore::code_graph::graph_embeddings::{GraphEmbeddingStrategy, SimpleFeatureCombiner};
//!
//! // Use default strategy
//! let strategy: Box<dyn GraphEmbeddingStrategy> = Box::new(SimpleFeatureCombiner);
//!
//! // Future: Plug in Graph-BERT
//! // let strategy: Box<dyn GraphEmbeddingStrategy> = Box::new(GraphBertModel::new());
//! ```

use anyhow::Result;
use std::collections::HashMap;

// ============================================================================
// Graph Feature Extraction (Design Interface)
// ============================================================================

/// Structural graph features for a code entity
///
/// These features capture the topological position of an entity in the code graph:
/// - Connectivity (in/out degree)
/// - Relationship types (CALLS, DEFINES, IMPORTS, USES)
/// - Future: k-hop neighborhoods, PageRank, centrality metrics
#[derive(Debug, Clone)]
pub struct GraphFeatures {
    /// Incoming edge count (how many entities reference this one)
    pub degree_in: u32,

    /// Outgoing edge count (how many entities this one references)
    pub degree_out: u32,

    /// Edge type distribution (e.g., "CALLS": 5, "DEFINES": 2)
    pub edge_types: HashMap<String, u32>,
}

impl GraphFeatures {
    /// Create empty graph features (for entities not yet in graph)
    pub fn empty() -> Self {
        Self {
            degree_in: 0,
            degree_out: 0,
            edge_types: HashMap::new(),
        }
    }

    /// Convert to normalized feature vector for embedding combination
    ///
    /// Returns 6-dim vector: [degree_in_norm, degree_out_norm, calls_norm,
    ///                         defines_norm, imports_norm, uses_norm]
    ///
    /// Normalization: Divide by max_degree and clip to [0, 1]
    pub fn to_vector(&self) -> Vec<f32> {
        let max_degree = 100.0; // Normalization constant
        let degree_in_norm = (self.degree_in as f32 / max_degree).min(1.0);
        let degree_out_norm = (self.degree_out as f32 / max_degree).min(1.0);

        let calls_norm = (self.edge_types.get("CALLS").copied().unwrap_or(0) as f32 / max_degree).min(1.0);
        let defines_norm = (self.edge_types.get("DEFINES").copied().unwrap_or(0) as f32 / max_degree).min(1.0);
        let imports_norm = (self.edge_types.get("IMPORTS").copied().unwrap_or(0) as f32 / max_degree).min(1.0);
        let uses_norm = (self.edge_types.get("USES").copied().unwrap_or(0) as f32 / max_degree).min(1.0);

        vec![degree_in_norm, degree_out_norm, calls_norm, defines_norm, imports_norm, uses_norm]
    }
}

// ============================================================================
// Graph-BERT Plugin Seam (Strategy Pattern) - PRIMARY DELIVERABLE
// ============================================================================

/// Strategy for combining CODE embeddings with graph features
///
/// **This trait is the Graph-BERT plugin seam.** It defines how CODE embeddings
/// and graph structural features are combined to produce GRAPH embeddings.
///
/// ## Implementations
///
/// - `SimpleFeatureCombiner`: Default deterministic strategy (feature concat + norm)
/// - `GraphBertModel` (FUTURE): Learned graph neural network embeddings
///
/// ## Contract
///
/// Implementations must:
/// 1. Accept 384-dim CODE embedding + GraphFeatures
/// 2. Return 384-dim GRAPH embedding (same dimensionality for compatibility)
/// 3. Be deterministic OR clearly document randomness/reproducibility
/// 4. Be Send + Sync for async/thread-safe usage
pub trait GraphEmbeddingStrategy: Send + Sync {
    /// Combine CODE embedding with graph structural features
    ///
    /// # Arguments
    /// * `code_embedding` - 384-dim CODE domain embedding from BGE-small-en-v1.5
    /// * `graph_features` - Structural graph features (degree, edge types)
    ///
    /// # Returns
    /// 384-dim GRAPH domain embedding
    ///
    /// # Graph-BERT Future
    /// When implementing Graph-BERT, this method should:
    /// - Take node_id, neighbor embeddings, edge features
    /// - Run through pretrained Graph-BERT encoder
    /// - Return learned 384-dim embedding
    fn embed_with_graph(&self, code_embedding: &[f32], graph_features: &GraphFeatures) -> Vec<f32>;
}

// ============================================================================
// Default Strategy: Simple Feature Combiner
// ============================================================================

/// Default strategy: Simple feature concatenation + normalization
///
/// **APEX 1.9-G Phase 2**: This is the initial implementation that provides
/// functional GRAPH embeddings without requiring Graph-BERT.
///
/// ## Algorithm
///
/// 1. Extract graph features as 6-dim normalized vector
/// 2. Replace first 6 dims of CODE embedding with graph features
/// 3. Renormalize entire vector to unit length
///
/// ## Rationale
///
/// This approach injects graph topology information into the CODE embedding
/// while maintaining dimensionality compatibility (384-dim). The first 6
/// dimensions act as "graph metadata" that downstream systems can use.
///
/// ## Graph-BERT Migration
///
/// When Graph-BERT is added, simply swap this implementation:
/// ```ignore
/// // PHASE 2 (current):
/// let strategy: Box<dyn GraphEmbeddingStrategy> = Box::new(SimpleFeatureCombiner);
///
/// // PHASE 3 (future):
/// let strategy: Box<dyn GraphEmbeddingStrategy> = Box::new(GraphBertModel::new());
/// ```
pub struct SimpleFeatureCombiner;

impl GraphEmbeddingStrategy for SimpleFeatureCombiner {
    fn embed_with_graph(&self, code_embedding: &[f32], graph_features: &GraphFeatures) -> Vec<f32> {
        let mut result = code_embedding.to_vec();
        let graph_vec = graph_features.to_vector();

        // Inject graph features into first 6 dimensions
        for (i, &val) in graph_vec.iter().enumerate() {
            if i < result.len() {
                result[i] = val;
            }
        }

        // Normalize to unit length (standard for semantic similarity)
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in result.iter_mut() {
                *val /= norm;
            }
        }

        result
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_features_empty() {
        let features = GraphFeatures::empty();
        assert_eq!(features.degree_in, 0);
        assert_eq!(features.degree_out, 0);
        assert!(features.edge_types.is_empty());
    }

    #[test]
    fn test_graph_features_to_vector() {
        let mut features = GraphFeatures::empty();
        features.degree_in = 5;
        features.degree_out = 10;
        features.edge_types.insert("CALLS".to_string(), 3);

        let vec = features.to_vector();
        assert_eq!(vec.len(), 6);
        assert!(vec[0] > 0.0); // degree_in normalized
        assert!(vec[1] > 0.0); // degree_out normalized
        assert!(vec[2] > 0.0); // CALLS normalized
    }

    #[test]
    fn test_simple_feature_combiner_dimensions() {
        let combiner = SimpleFeatureCombiner;
        let code_emb = vec![0.5; 384];
        let features = GraphFeatures::empty();

        let graph_emb = combiner.embed_with_graph(&code_emb, &features);
        assert_eq!(graph_emb.len(), 384);
    }

    #[test]
    fn test_simple_feature_combiner_normalization() {
        let combiner = SimpleFeatureCombiner;
        let code_emb = vec![1.0; 384];
        let features = GraphFeatures::empty();

        let graph_emb = combiner.embed_with_graph(&code_emb, &features);

        // Check unit length (L2 norm ≈ 1.0)
        let norm: f32 = graph_emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001, "Embedding should be unit normalized");
    }

    #[test]
    fn test_simple_feature_combiner_injects_graph_features() {
        let combiner = SimpleFeatureCombiner;
        let code_emb = vec![0.5; 384];

        let mut features = GraphFeatures::empty();
        features.degree_in = 10;
        features.degree_out = 20;

        let graph_emb = combiner.embed_with_graph(&code_emb, &features);

        // First 6 dims should differ from CODE embedding (graph features injected)
        assert_ne!(graph_emb[0], code_emb[0]);
        assert_ne!(graph_emb[1], code_emb[1]);
    }
}
