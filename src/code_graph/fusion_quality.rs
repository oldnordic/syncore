//! Fusion Quality Module - Evaluates fusion results and recommends deep-read
//!
//! PHASE C: Implements fusion quality metrics and token efficiency guards:
//! - Quality score computation from fusion results
//! - Deep-read recommendation based on score thresholds
//! - Token efficiency guard (prevents over-fetching)
//!
//! Key Metrics:
//! - Confidence: How confident are we in the fusion result?
//! - Completeness: Did we find enough related entities?
//! - Diversity: Are results spread across multiple sources?

/// Configuration for fusion quality evaluation
#[derive(Debug, Clone)]
pub struct FusionQualityConfig {
    /// Minimum confidence for "good" result (0.0 to 1.0)
    pub min_confidence: f32,
    /// Threshold below which deep-read is recommended
    pub deep_read_threshold: f32,
    /// Maximum tokens to return in snippet mode
    pub max_snippet_tokens: usize,
    /// Maximum tokens to return in deep-read mode
    pub max_deep_read_tokens: usize,
    /// Weight for recency scoring in fusion evaluation (0.0 to 1.0)
    pub recency: f32,
}

impl Default for FusionQualityConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.6,
            deep_read_threshold: 0.4,
            max_snippet_tokens: 500,
            max_deep_read_tokens: 2000,
            recency: 0.05,
        }
    }
}

/// Result of fusion quality evaluation
#[derive(Debug, Clone)]
pub struct FusionQualityResult {
    /// Overall quality score (0.0 to 1.0)
    pub quality_score: f32,
    /// Confidence in the results
    pub confidence: f32,
    /// Completeness score
    pub completeness: f32,
    /// Diversity score
    pub diversity: f32,
    /// Whether deep-read is recommended
    pub recommend_deep_read: bool,
    /// Recommended token budget
    pub token_budget: usize,
    /// Reason for recommendation
    pub recommendation_reason: String,
}

/// Scored entity from fusion
#[derive(Debug, Clone)]
pub struct ScoredEntity {
    pub entity_id: i64,
    pub vector_score: f32,
    pub graph_score: f32,
    pub path_score: f32,
    pub fused_score: f32,
    pub source_file: String,
}

/// Fusion Quality Evaluator
pub struct FusionQualityEvaluator {
    config: FusionQualityConfig,
}

impl FusionQualityEvaluator {
    /// Create new evaluator with default config
    pub fn new() -> Self {
        Self {
            config: FusionQualityConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: FusionQualityConfig) -> Self {
        Self {
            config,
        }
    }

    /// Evaluate fusion quality from scored entities
    ///
    /// # Arguments
    /// * `entities` - List of scored entities from fusion
    /// * `query` - Original query string
    ///
    /// # Returns
    /// FusionQualityResult with metrics and recommendation
    pub fn evaluate(&self, entities: &[ScoredEntity], query: &str) -> FusionQualityResult {
        if entities.is_empty() {
            return FusionQualityResult {
                quality_score: 0.0,
                confidence: 0.0,
                completeness: 0.0,
                diversity: 0.0,
                recommend_deep_read: true,
                token_budget: self.config.max_deep_read_tokens,
                recommendation_reason: "No results found - deep read recommended".to_string(),
            };
        }

        // Compute confidence from top scores
        let confidence = self.compute_confidence(entities);

        // Compute completeness from result count vs expected
        let completeness = self.compute_completeness(entities, query);

        // Compute diversity from source files
        let diversity = self.compute_diversity(entities);

        // Overall quality score
        let quality_score = 0.5 * confidence + 0.3 * completeness + 0.2 * diversity;

        // Deep-read recommendation
        let (recommend_deep_read, reason) =
            self.should_recommend_deep_read(quality_score, confidence, entities);

        let token_budget = if recommend_deep_read {
            self.config.max_deep_read_tokens
        } else {
            self.config.max_snippet_tokens
        };

        FusionQualityResult {
            quality_score,
            confidence,
            completeness,
            diversity,
            recommend_deep_read,
            token_budget,
            recommendation_reason: reason,
        }
    }

    /// Compute confidence from score distribution
    fn compute_confidence(&self, entities: &[ScoredEntity]) -> f32 {
        if entities.is_empty() {
            return 0.0;
        }

        // Use top-1 score and score gap as confidence indicators
        let top_score = entities.first().map(|e| e.fused_score).unwrap_or(0.0);

        // Score gap between top-1 and top-2 indicates distinctiveness
        let score_gap = if entities.len() >= 2 {
            entities[0].fused_score - entities[1].fused_score
        } else {
            0.3 // Single result gets moderate gap
        };

        // Confidence = weighted combination of top score and gap
        let confidence = 0.7 * top_score + 0.3 * (score_gap * 3.0).min(1.0);
        confidence.clamp(0.0, 1.0)
    }

    /// Compute completeness based on result count and query complexity
    fn compute_completeness(&self, entities: &[ScoredEntity], query: &str) -> f32 {
        let query_tokens = query.split_whitespace().count();

        // Estimate expected results based on query complexity
        let expected_results = match query_tokens {
            0..=2 => 3, // Simple query expects few results
            3..=5 => 5, // Medium query
            _ => 8,     // Complex query
        };

        // Completeness = actual / expected, clamped to 1.0
        let ratio = entities.len() as f32 / expected_results as f32;
        ratio.min(1.0)
    }

    /// Compute diversity from unique source files
    fn compute_diversity(&self, entities: &[ScoredEntity]) -> f32 {
        if entities.is_empty() {
            return 0.0;
        }

        // Count unique source files
        let unique_files: std::collections::HashSet<_> =
            entities.iter().map(|e| &e.source_file).collect();

        // Diversity = unique files / total entities, with bonus for multiple files
        let base_diversity = unique_files.len() as f32 / entities.len() as f32;
        let file_bonus = if unique_files.len() > 1 {
            0.2
        } else {
            0.0
        };

        (base_diversity + file_bonus).min(1.0)
    }

    /// Determine if deep-read should be recommended
    fn should_recommend_deep_read(
        &self,
        quality_score: f32,
        confidence: f32,
        entities: &[ScoredEntity],
    ) -> (bool, String) {
        // Rule 1: Low quality score
        if quality_score < self.config.deep_read_threshold {
            return (
                true,
                format!(
                    "Low quality score ({:.2}) below threshold ({:.2})",
                    quality_score, self.config.deep_read_threshold
                ),
            );
        }

        // Rule 2: Low confidence with results
        if confidence < self.config.min_confidence && !entities.is_empty() {
            return (
                true,
                format!("Low confidence ({:.2}) - results may be incomplete", confidence),
            );
        }

        // Rule 3: High vector score but low graph score suggests isolated entity
        if let Some(top) = entities.first() {
            if top.vector_score > 0.8 && top.graph_score < 0.3 {
                return (
                    true,
                    "High semantic match but low graph connectivity - context needed".to_string(),
                );
            }
        }

        // Rule 4: Score distribution too flat (no clear winner)
        if entities.len() >= 3 {
            let score_range =
                entities.first().unwrap().fused_score - entities.last().unwrap().fused_score;
            if score_range < 0.1 {
                return (true, "Multiple equally-ranked results - more context needed".to_string());
            }
        }

        // Good quality - snippet mode sufficient
        (false, "Good fusion quality - snippet mode sufficient".to_string())
    }

    /// Token efficiency guard - limits tokens based on quality
    ///
    /// # Arguments
    /// * `quality` - Quality evaluation result
    /// * `requested_tokens` - Number of tokens requested
    ///
    /// # Returns
    /// Adjusted token count that respects efficiency bounds
    pub fn guard_token_budget(
        &self,
        quality: &FusionQualityResult,
        requested_tokens: usize,
    ) -> usize {
        let max_allowed = quality.token_budget;

        // If quality is low, allow more tokens
        if quality.quality_score < 0.3 {
            return max_allowed;
        }

        // Scale based on quality - higher quality needs fewer tokens
        let quality_factor = 1.0 - (quality.quality_score * 0.3);
        let adjusted = (requested_tokens as f32 * quality_factor) as usize;

        adjusted.min(max_allowed).max(100) // At least 100 tokens
    }
}

impl Default for FusionQualityEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(fused: f32, vector: f32, graph: f32, file: &str) -> ScoredEntity {
        ScoredEntity {
            entity_id: 1,
            vector_score: vector,
            graph_score: graph,
            path_score: 0.5,
            fused_score: fused,
            source_file: file.to_string(),
        }
    }

    #[test]
    fn test_empty_results_recommend_deep_read() {
        let evaluator = FusionQualityEvaluator::new();
        let result = evaluator.evaluate(&[], "test query");

        assert!(result.recommend_deep_read);
        assert_eq!(result.quality_score, 0.0);
        assert_eq!(result.token_budget, 2000);
    }

    #[test]
    fn test_high_quality_snippet_mode() {
        let evaluator = FusionQualityEvaluator::new();
        let entities =
            vec![make_entity(0.95, 0.9, 0.8, "main.rs"), make_entity(0.5, 0.6, 0.4, "util.rs")];

        let result = evaluator.evaluate(&entities, "find main");

        assert!(!result.recommend_deep_read);
        assert!(result.quality_score > 0.5);
        assert_eq!(result.token_budget, 500);
    }

    #[test]
    fn test_low_confidence_deep_read() {
        let evaluator = FusionQualityEvaluator::new();
        let entities = vec![make_entity(0.4, 0.4, 0.3, "test.rs")];

        let result = evaluator.evaluate(&entities, "complex multi word query here");

        // Low completeness should trigger deep read
        assert!(result.recommend_deep_read || result.quality_score < 0.4);
    }

    #[test]
    fn test_isolated_entity_needs_context() {
        let evaluator = FusionQualityEvaluator::new();
        let entities = vec![
            make_entity(0.85, 0.9, 0.1, "isolated.rs"), // High vector, low graph
        ];

        let result = evaluator.evaluate(&entities, "isolated function");

        assert!(result.recommend_deep_read);
        assert!(result.recommendation_reason.contains("context needed"));
    }

    #[test]
    fn test_diversity_multiple_files() {
        let evaluator = FusionQualityEvaluator::new();
        let entities = vec![
            make_entity(0.9, 0.85, 0.7, "file1.rs"),
            make_entity(0.8, 0.75, 0.6, "file2.rs"),
            make_entity(0.7, 0.65, 0.5, "file3.rs"),
        ];

        let result = evaluator.evaluate(&entities, "multi file search");

        assert!(result.diversity > 0.5);
    }

    #[test]
    fn test_token_guard_scales_with_quality() {
        let evaluator = FusionQualityEvaluator::new();

        // High quality = fewer tokens needed
        let high_quality = FusionQualityResult {
            quality_score: 0.9,
            confidence: 0.9,
            completeness: 0.9,
            diversity: 0.8,
            recommend_deep_read: false,
            token_budget: 500,
            recommendation_reason: "Good".to_string(),
        };

        let tokens = evaluator.guard_token_budget(&high_quality, 1000);
        assert!(tokens < 1000);
        assert!(tokens <= 500);

        // Low quality = more tokens allowed
        let low_quality = FusionQualityResult {
            quality_score: 0.2,
            confidence: 0.2,
            completeness: 0.3,
            diversity: 0.1,
            recommend_deep_read: true,
            token_budget: 2000,
            recommendation_reason: "Low quality".to_string(),
        };

        let tokens_low = evaluator.guard_token_budget(&low_quality, 1000);
        assert_eq!(tokens_low, 2000); // Gets max budget
    }
}
