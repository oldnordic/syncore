//! Mode A: Linear Weighted Hybrid Fusion
//!
//! PHASE 5: Implements 4-component linear combination:
//! S = α*S_vector + β*S_graph + τ*S_temporal + γ*S_graph_embedding
//!
//! Default weights: α=0.5, β=0.2, τ=0.1, γ=0.2
//!
//! This mode is optimal for:
//! - Short queries (< 4 tokens)
//! - Symbol/path lookups
//! - Fast recall operations

/// Simple linear fusion combiner (PHASE 5: 4-component with GraphBERT)
pub struct FusionSimple {
    /// Weight for vector score (alpha)
    alpha: f32,
    /// Weight for graph score (beta)
    beta: f32,
    /// Weight for temporal score (tau)
    tau: f32,
    /// Weight for graph embedding score (gamma) - GraphBERT
    gamma: f32,
}

impl FusionSimple {
    /// Create new simple fusion with given weights (PHASE 5: 4-component)
    ///
    /// # Arguments
    /// * `alpha` - Weight for vector score (0.0 to 1.0)
    /// * `beta` - Weight for graph score (0.0 to 1.0)
    /// * `tau` - Weight for temporal score (0.0 to 1.0)
    /// * `gamma` - Weight for graph embedding score (GraphBERT) (0.0 to 1.0)
    ///
    /// # Returns
    /// New FusionSimple instance with normalized weights
    pub fn new(alpha: f32, beta: f32, tau: f32, gamma: f32) -> Self {
        let total = alpha + beta + tau + gamma;
        Self {
            alpha: (alpha / total).clamp(0.0, 1.0),
            beta: (beta / total).clamp(0.0, 1.0),
            tau: (tau / total).clamp(0.0, 1.0),
            gamma: (gamma / total).clamp(0.0, 1.0),
        }
    }

    /// Combine vector, graph, temporal, and graph embedding scores (PHASE 5)
    ///
    /// # Arguments
    /// * `vector_score` - Score from vector search (0.0 to 1.0)
    /// * `graph_score` - Score from graph traversal (0.0 to 1.0)
    /// * `temporal_score` - Score from temporal metadata (0.0 to 1.0)
    /// * `graph_embedding_score` - Score from GraphBERT/graph embeddings (0.0 to 1.0)
    ///
    /// # Returns
    /// Combined score using S = α*S_v + β*S_g + τ*S_t + γ*S_ge, clamped to [0.0, 1.0]
    pub fn combine(
        &self,
        vector_score: f32,
        graph_score: f32,
        temporal_score: f32,
        graph_embedding_score: f32,
    ) -> f32 {
        let score = self.alpha * vector_score
            + self.beta * graph_score
            + self.tau * temporal_score
            + self.gamma * graph_embedding_score;
        score.clamp(0.0, 1.0)
    }
}

impl Default for FusionSimple {
    fn default() -> Self {
        // PHASE 5: Default weights (4-component with GraphBERT)
        // α=0.5 (vector), β=0.2 (graph), τ=0.1 (temporal), γ=0.2 (GraphBERT)
        Self::new(0.5, 0.2, 0.1, 0.2)
    }
}

/// Compute graph score from multi-hop depth (TASK B)
///
/// Formula: graph_score = 1.0 / (1.0 + depth)
///
/// # Arguments
/// * `depth` - Minimum depth from multi-hop traversal (None if unreachable)
///
/// # Returns
/// Graph score in range [0.0, 1.0]
///
/// # Implementation
/// - None (isolated): 0.0
/// - depth=0 (self): 1.0
/// - depth=1 (direct neighbor): 0.5
/// - depth=2: 0.33
/// - depth→∞: approaches 0.0
pub fn compute_graph_score(depth: Option<usize>) -> f32 {
    match depth {
        None => 0.0,
        Some(d) => (1.0 / (1.0 + d as f32)).clamp(0.0, 1.0),
    }
}

/// Compute temporal score from recency and code churn metadata (PHASE 5)
///
/// Formula: S_temporal = 0.5*recency + 0.5*churn_normalized
///
/// # Arguments
/// * `last_modified_at` - Unix timestamp of last modification
/// * `change_count` - Number of commits touching this entity
/// * `author_count` - Number of unique authors
///
/// # Returns
/// Temporal score in range [0.0, 1.0]
///
/// # Implementation
/// - Recency: Normalized by 1 year window (entities modified within last year get higher scores)
/// - Churn: log(1 + change_count) normalized by log(101) for [0.0, 1.0] range
pub fn compute_temporal_score(last_modified_at: i64, change_count: i32, _author_count: i32) -> f32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Recency score: 1.0 for recent (< 1 year), decays to 0.0 over time
    let seconds_since_mod = (now - last_modified_at).max(0) as f64;
    let one_year_secs = 365.25 * 24.0 * 3600.0;
    let recency = (1.0 - (seconds_since_mod / one_year_secs)).clamp(0.0, 1.0);

    // Churn score: log normalization for change count
    // log(1 + change_count) / log(101) maps [0, 100] changes to [0.0, 1.0]
    let churn = ((1.0 + change_count as f64).ln() / 101_f64.ln()).clamp(0.0, 1.0);

    // Combine recency and churn (equal weights)
    let temporal = 0.5 * recency + 0.5 * churn;
    temporal as f32
}

/// Compute graph embedding score from structural features (GRAPH domain heuristic)
///
/// This function provides a simple heuristic score based on graph topology:
/// - Higher degree (more connections) suggests more important/central code
/// - Balanced in/out degree suggests well-integrated code
/// - Edge type diversity suggests multi-purpose/interface code
///
/// Formula: S_graph_emb = 0.4*degree_norm + 0.3*balance + 0.3*diversity
///
/// # Arguments
/// * `features` - Graph structural features (degree, edge types)
///
/// # Returns
/// Graph embedding score in range [0.0, 1.0]
///
/// # Implementation
/// - Degree normalization: log(1 + total_degree) / log(101) maps [0, 100] to [0.0, 1.0]
/// - Balance: 1.0 - |degree_in - degree_out| / (degree_in + degree_out + 1) favors balanced nodes
/// - Diversity: edge_type_count / 4.0 (normalized by max expected types: CALLS, DEFINES, IMPORTS, USES)
pub fn compute_graph_embedding_score(features: &super::graph_embeddings::GraphFeatures) -> f32 {
    let total_degree = features.degree_in + features.degree_out;

    // Degree score: log-normalized degree (more connections = higher score)
    let degree_norm: f64 = if total_degree > 0 {
        ((1.0_f64 + total_degree as f64).ln() / 101_f64.ln()).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Balance score: penalize highly imbalanced nodes (all in or all out)
    let balance = if total_degree > 0 {
        let diff = (features.degree_in as i32 - features.degree_out as i32).abs();
        1.0 - (diff as f64 / (total_degree + 1) as f64)
    } else {
        1.0 // Isolated nodes are perfectly balanced (trivially)
    };

    // Diversity score: more edge types = more versatile/important
    let edge_type_count = features.edge_types.len();
    let diversity = (edge_type_count as f64 / 4.0).clamp(0.0, 1.0); // Max 4 types

    // Weighted combination
    let score = 0.4 * degree_norm + 0.3 * balance + 0.3 * diversity;
    score as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_4component_combination() {
        // PHASE 5: Test 4-component formula with GraphBERT
        let fusion = FusionSimple::new(0.5, 0.2, 0.1, 0.2);
        let result = fusion.combine(0.8, 0.4, 0.9, 0.7);
        // Expected: 0.5*0.8 + 0.2*0.4 + 0.1*0.9 + 0.2*0.7 = 0.4 + 0.08 + 0.09 + 0.14 = 0.71
        assert!((result - 0.71).abs() < 0.001);
    }

    #[test]
    fn test_weight_normalization() {
        // PHASE 5: Weights should be normalized to sum to 1.0 (4-component)
        let fusion = FusionSimple::new(0.5, 0.2, 0.1, 0.2);
        let total = fusion.alpha + fusion.beta + fusion.tau + fusion.gamma;
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_clamping() {
        // PHASE 5: Result should be clamped to [0.0, 1.0]
        let fusion = FusionSimple::new(0.5, 0.2, 0.1, 0.2);
        let result = fusion.combine(1.2, 1.5, 1.8, 2.0);
        assert!(result <= 1.0);
        assert!(result >= 0.0);
    }

    #[test]
    fn test_default_weights() {
        // PHASE 5: Default should use α=0.5, β=0.2, τ=0.1, γ=0.2
        let fusion = FusionSimple::default();
        assert!((fusion.alpha - 0.5).abs() < 0.001);
        assert!((fusion.beta - 0.2).abs() < 0.001);
        assert!((fusion.tau - 0.1).abs() < 0.001);
        assert!((fusion.gamma - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_zero_graph_bert_backward_compat() {
        // When graph_embedding_score is 0.0, behavior should match old 3-component
        let fusion = FusionSimple::new(0.6, 0.3, 0.1, 0.0);
        let result = fusion.combine(0.8, 0.4, 0.9, 0.0);
        // Expected: 0.6*0.8 + 0.3*0.4 + 0.1*0.9 + 0.0*0.0 = 0.48 + 0.12 + 0.09 = 0.69
        assert!((result - 0.69).abs() < 0.001);
    }

    // Tests for compute_graph_embedding_score function
    #[test]
    fn test_graph_embedding_score_isolated_node() {
        use crate::code_graph::graph_embeddings::GraphFeatures;
        // Isolated node (no connections) should have low score
        // Note: Gets some score from balance (0.3*1.0) but no degree or diversity
        let features = GraphFeatures::empty();
        let score = compute_graph_embedding_score(&features);
        assert!(
            score < 0.5,
            "Isolated node should have low score (< 0.5), got {}",
            score
        );
        assert!(score >= 0.0, "Score should be non-negative");
    }

    #[test]
    fn test_graph_embedding_score_hub_node() {
        use crate::code_graph::graph_embeddings::GraphFeatures;
        // Hub node (many connections) should have high score
        let mut features = GraphFeatures::empty();
        features.degree_in = 50;
        features.degree_out = 50;
        features.edge_types.insert("CALLS".to_string(), 20);
        features.edge_types.insert("DEFINES".to_string(), 15);
        features.edge_types.insert("IMPORTS".to_string(), 10);
        features.edge_types.insert("USES".to_string(), 5);

        let score = compute_graph_embedding_score(&features);
        assert!(
            score > 0.5,
            "Hub node should have high score (> 0.5), got {}",
            score
        );
        assert!(score <= 1.0, "Score should not exceed 1.0");
    }

    #[test]
    fn test_graph_embedding_score_balanced_node() {
        use crate::code_graph::graph_embeddings::GraphFeatures;
        // Balanced node (equal in/out) should score well
        let mut features = GraphFeatures::empty();
        features.degree_in = 10;
        features.degree_out = 10;

        let score = compute_graph_embedding_score(&features);
        assert!(score > 0.0, "Connected node should have positive score");
        assert!(score <= 1.0, "Score should be normalized to [0, 1]");
    }

    #[test]
    fn test_graph_embedding_score_imbalanced_node() {
        use crate::code_graph::graph_embeddings::GraphFeatures;
        // Highly imbalanced node (all out, no in) should be penalized
        let mut features_imbalanced = GraphFeatures::empty();
        features_imbalanced.degree_in = 0;
        features_imbalanced.degree_out = 20;

        let mut features_balanced = GraphFeatures::empty();
        features_balanced.degree_in = 10;
        features_balanced.degree_out = 10;

        let score_imbalanced = compute_graph_embedding_score(&features_imbalanced);
        let score_balanced = compute_graph_embedding_score(&features_balanced);

        assert!(
            score_balanced > score_imbalanced,
            "Balanced node should score higher than imbalanced, got balanced={} imbalanced={}",
            score_balanced,
            score_imbalanced
        );
    }

    // TASK B: Tests for compute_graph_score function
    #[test]
    fn test_compute_graph_score_isolated_node() {
        // Isolated node (no neighbors) should have score 0.0
        let score = compute_graph_score(None);
        assert_eq!(score, 0.0, "Isolated node should have graph_score = 0.0");
    }

    #[test]
    fn test_compute_graph_score_self_node() {
        // Self node (depth 0) should have score 1.0
        let score = compute_graph_score(Some(0));
        assert_eq!(
            score, 1.0,
            "Self node (depth=0) should have graph_score = 1.0"
        );
    }

    #[test]
    fn test_compute_graph_score_direct_neighbor() {
        // Direct neighbor (depth 1) should have score 0.5
        // Formula: 1.0 / (1.0 + 1) = 0.5
        let score = compute_graph_score(Some(1));
        assert!(
            (score - 0.5).abs() < 0.001,
            "Direct neighbor (depth=1) should have graph_score ≈ 0.5"
        );
    }

    #[test]
    fn test_compute_graph_score_depth2() {
        // Depth 2 neighbor should have score ~0.33
        // Formula: 1.0 / (1.0 + 2) = 0.333...
        let score = compute_graph_score(Some(2));
        assert!(
            (score - 0.333).abs() < 0.01,
            "Depth-2 neighbor should have graph_score ≈ 0.33"
        );
    }

    #[test]
    fn test_compute_graph_score_monotonic_decrease() {
        // Score should decrease monotonically with increasing depth
        let score_1 = compute_graph_score(Some(1));
        let score_2 = compute_graph_score(Some(2));
        let score_3 = compute_graph_score(Some(3));

        assert!(
            score_1 > score_2,
            "Depth 1 should score higher than depth 2"
        );
        assert!(
            score_2 > score_3,
            "Depth 2 should score higher than depth 3"
        );
    }
}
