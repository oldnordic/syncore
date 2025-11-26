//! Mode A: Linear Weighted Hybrid Fusion
//!
//! PHASE 5: Implements 3-component linear combination:
//! S = α*S_vector + β*S_graph + τ*S_temporal
//!
//! Default weights: α=0.65, β=0.25, τ=0.10
//!
//! This mode is optimal for:
//! - Short queries (< 4 tokens)
//! - Symbol/path lookups
//! - Fast recall operations

/// Simple linear fusion combiner (PHASE 5: 3-component)
pub struct FusionSimple {
    /// Weight for vector score (alpha)
    alpha: f32,
    /// Weight for graph score (beta)
    beta: f32,
    /// Weight for temporal score (tau)
    tau: f32,
}

impl FusionSimple {
    /// Create new simple fusion with given weights (PHASE 5: 3-component)
    ///
    /// # Arguments
    /// * `alpha` - Weight for vector score (0.0 to 1.0)
    /// * `beta` - Weight for graph score (0.0 to 1.0)
    /// * `tau` - Weight for temporal score (0.0 to 1.0)
    ///
    /// # Returns
    /// New FusionSimple instance with normalized weights
    pub fn new(alpha: f32, beta: f32, tau: f32) -> Self {
        let total = alpha + beta + tau;
        Self {
            alpha: (alpha / total).clamp(0.0, 1.0),
            beta: (beta / total).clamp(0.0, 1.0),
            tau: (tau / total).clamp(0.0, 1.0),
        }
    }

    /// Combine vector, graph, and temporal scores (PHASE 5)
    ///
    /// # Arguments
    /// * `vector_score` - Score from vector search (0.0 to 1.0)
    /// * `graph_score` - Score from graph traversal (0.0 to 1.0)
    /// * `temporal_score` - Score from temporal metadata (0.0 to 1.0)
    ///
    /// # Returns
    /// Combined score using S = α*S_v + β*S_g + τ*S_t, clamped to [0.0, 1.0]
    pub fn combine(&self, vector_score: f32, graph_score: f32, temporal_score: f32) -> f32 {
        let score = self.alpha * vector_score + self.beta * graph_score + self.tau * temporal_score;
        score.clamp(0.0, 1.0)
    }
}

impl Default for FusionSimple {
    fn default() -> Self {
        // PHASE 5: Default weights per directive
        Self::new(0.65, 0.25, 0.10)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_3component_combination() {
        // PHASE 5: Test 3-component formula
        let fusion = FusionSimple::new(0.6, 0.3, 0.1);
        let result = fusion.combine(0.8, 0.4, 0.9);
        // Expected: 0.6*0.8 + 0.3*0.4 + 0.1*0.9 = 0.48 + 0.12 + 0.09 = 0.69
        assert!((result - 0.69).abs() < 0.001);
    }

    #[test]
    fn test_weight_normalization() {
        // PHASE 5: Weights should be normalized to sum to 1.0
        let fusion = FusionSimple::new(0.65, 0.25, 0.10);
        let total = fusion.alpha + fusion.beta + fusion.tau;
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_clamping() {
        // PHASE 5: Result should be clamped to [0.0, 1.0]
        let fusion = FusionSimple::new(0.5, 0.3, 0.2);
        let result = fusion.combine(1.2, 1.5, 1.8);
        assert!(result <= 1.0);
        assert!(result >= 0.0);
    }

    #[test]
    fn test_default_weights() {
        // PHASE 5: Default should use α=0.65, β=0.25, τ=0.10
        let fusion = FusionSimple::default();
        assert!((fusion.alpha - 0.65).abs() < 0.001);
        assert!((fusion.beta - 0.25).abs() < 0.001);
        assert!((fusion.tau - 0.10).abs() < 0.001);
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
