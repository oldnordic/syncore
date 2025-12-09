//! Fusion bridge for RAGGraph API
//!
//! Handles integration between different fusion modes and score combination.

use super::super::fusion_attention::FusionAttention;
use super::super::fusion_reasoning::FusionReasoning;
use super::super::fusion_router::{FusionMode, FusionRouter};
use super::super::fusion_simple::FusionSimple;
use crate::graph::GraphBackend;
use std::collections::HashMap;

/// Apply simple linear fusion (PHASE 5: 5-component with GraphBERT + Recency)
pub fn apply_simple_fusion(
    vector_score: f32,
    graph_score: f32,
    temporal_score: f32,
    graph_embedding_score: f32,
    recency_score: f32,
) -> f32 {
    let fusion = FusionSimple::default();
    fusion.combine(vector_score, graph_score, temporal_score, graph_embedding_score, recency_score)
}

/// Apply attention-based fusion with context awareness
pub fn apply_attention_fusion(
    vector_score: f32,
    graph_score: f32,
    context: &str,
    debug_info: &mut HashMap<String, String>,
) -> f32 {
    // For now, use simple weighted average as fallback
    // TODO: Implement proper attention-based fusion when Embeddings dependency is resolved
    debug_info.insert("fusion_method".to_string(), "simple_weighted_fallback".to_string());
    let combined = 0.7 * vector_score + 0.3 * graph_score;
    combined.clamp(0.0, 1.0)
}

/// Apply reasoning fusion with higher-order terms
pub fn apply_reasoning_fusion(
    vector_score: f32,
    graph_score: f32,
    debug_info: &mut HashMap<String, String>,
) -> f32 {
    // For now, use simple reasoning fusion
    // In a full implementation, this would use FusionReasoning
    let combined_score = 0.6 * vector_score + 0.4 * graph_score;
    debug_info.insert("reasoning_formula".to_string(), "0.6*vector + 0.4*graph".to_string());
    combined_score
}

/// Bridge between router selection and fusion mode application
pub fn apply_selected_fusion(
    mode: FusionMode,
    vector_score: f32,
    graph_score: f32,
    temporal_score: Option<f32>,
    graph_embedding_score: Option<f32>,
    recency_score: Option<f32>,
    query: &str,
    debug_info: &mut HashMap<String, String>,
) -> f32 {
    debug_info.insert("fusion_mode".to_string(), format!("{:?}", mode));

    match mode {
        FusionMode::Simple => {
            let temporal = temporal_score.unwrap_or(0.0);
            let graph_emb = graph_embedding_score.unwrap_or(0.0);
            let recency = recency_score.unwrap_or(0.0);
            let result = apply_simple_fusion(vector_score, graph_score, temporal, graph_emb, recency);
            debug_info.insert(
                "fusion_components".to_string(),
                format!(
                    "vector:{:.3}, graph:{:.3}, temporal:{:.3}, embedding:{:.3}, recency:{:.3}",
                    vector_score, graph_score, temporal, graph_emb, recency
                ),
            );
            result
        }
        FusionMode::Attention => {
            let recency = recency_score.unwrap_or(0.5); // Use recency as context factor
            let result = apply_attention_fusion(vector_score, graph_score, query, debug_info);
            let adjusted_result = result * (0.7 + 0.3 * recency); // Boost recent results
            debug_info.insert(
                "fusion_components".to_string(),
                format!("vector:{:.3}, graph:{:.3}, context:{}, recency:{:.3}", vector_score, graph_score, query, recency),
            );
            debug_info.insert("recency_boost".to_string(), format!("{:.3}", adjusted_result - result));
            adjusted_result
        }
        FusionMode::Reasoning => {
            let recency = recency_score.unwrap_or(0.5);
            let result = apply_reasoning_fusion(vector_score, graph_score, debug_info);
            let adjusted_result = result * (0.8 + 0.2 * recency); // Slight boost for recent results
            debug_info.insert(
                "fusion_components".to_string(),
                format!("vector:{:.3}, graph:{:.3}, recency:{:.3}", vector_score, graph_score, recency),
            );
            debug_info.insert("recency_boost".to_string(), format!("{:.3}", adjusted_result - result));
            adjusted_result
        }
    }
}

/// Fusion configuration options
#[derive(Debug, Clone)]
pub struct FusionConfig {
    /// Weight for vector similarity scores
    pub vector_weight: f32,
    /// Weight for graph connectivity scores
    pub graph_weight: f32,
    /// Weight for temporal relevance scores
    pub temporal_weight: f32,
    /// Weight for graph embedding scores
    pub embedding_weight: f32,
    /// Enable adaptive fusion based on query characteristics
    pub adaptive_fusion: bool,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            vector_weight: 0.4,
            graph_weight: 0.3,
            temporal_weight: 0.2,
            embedding_weight: 0.1,
            adaptive_fusion: true,
        }
    }
}

/// Advanced fusion with custom configuration
pub fn apply_configured_fusion(
    config: &FusionConfig,
    vector_score: f32,
    graph_score: f32,
    temporal_score: Option<f32>,
    graph_embedding_score: Option<f32>,
    debug_info: &mut HashMap<String, String>,
) -> f32 {
    let temporal = temporal_score.unwrap_or(0.0);
    let graph_emb = graph_embedding_score.unwrap_or(0.0);

    let combined_score = config.vector_weight * vector_score
        + config.graph_weight * graph_score
        + config.temporal_weight * temporal
        + config.embedding_weight * graph_emb;

    debug_info.insert(
        "fusion_config".to_string(),
        format!(
            "weights: vector={:.2}, graph={:.2}, temporal={:.2}, embedding={:.2}",
            config.vector_weight,
            config.graph_weight,
            config.temporal_weight,
            config.embedding_weight
        ),
    );

    combined_score.min(1.0) // Ensure score doesn't exceed 1.0
}

/// Normalize scores to 0.0-1.0 range
pub fn normalize_scores(scores: &mut [f32]) {
    if scores.is_empty() {
        return;
    }

    let max_score = scores.iter().fold(0.0f32, |a, &b| a.max(b));
    if max_score > 0.0 {
        for score in scores.iter_mut() {
            *score = *score / max_score;
        }
    }
}

/// Apply score calibration based on historical performance
pub fn calibrate_scores(scores: &[f32], calibration_factor: f32) -> Vec<f32> {
    scores
        .iter()
        .map(|&score| {
            // Apply sigmoid-like calibration
            let calibrated = 1.0 / (1.0 + (-((score - 0.5) * calibration_factor)).exp());
            calibrated.clamp(0.0, 1.0)
        })
        .collect()
}
