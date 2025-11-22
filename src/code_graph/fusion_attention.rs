//! Mode B: Attention Fusion
//!
//! Uses dynamic attention weights based on query context embeddings.
//! Computes adaptive weighting between vector and graph scores.
//!
//! This mode is optimal for:
//! - Multi-sentence queries
//! - "why/explain/trace" questions
//! - Semantic precision tasks

use crate::vector::Embeddings;
use anyhow::Result;

/// Attention-based fusion combiner
pub struct FusionAttention {
    /// Embedding model for context encoding
    embeddings: Box<dyn Embeddings>,
}

impl FusionAttention {
    /// Create new attention fusion with embedding model
    ///
    /// # Arguments
    /// * `embeddings` - Embedding model for encoding contexts
    ///
    /// # Returns
    /// New FusionAttention instance
    pub fn new(embeddings: Box<dyn Embeddings>) -> Self {
        Self { embeddings }
    }

    /// Combine scores using attention-based dynamic weighting
    ///
    /// # Arguments
    /// * `vector_score` - Score from vector search
    /// * `graph_score` - Score from graph traversal
    /// * `context` - Query context for attention computation
    ///
    /// # Returns
    /// Combined score with dynamic weights
    pub fn combine(&self, vector_score: f32, graph_score: f32, context: &str) -> Result<f32> {
        // Encode context to get embedding features
        let embedding = self.embeddings.embed(context)?;

        // Use token count and embedding variance as attention signal
        let token_count = context.split_whitespace().count();
        let mean: f32 = embedding.iter().sum::<f32>() / embedding.len() as f32;
        let variance: f32 =
            embedding.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / embedding.len() as f32;

        // Combine token count and variance for dynamic weighting
        // More tokens + higher variance → more semantic (vector)
        // Fewer tokens + lower variance → more structural (graph)
        let complexity = (token_count as f32 / 10.0) + variance * 100.0;
        let alpha = (0.3 + complexity / 5.0).clamp(0.3, 0.7);

        Ok(alpha * vector_score + (1.0 - alpha) * graph_score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::HuggingFaceEmbeddings;

    #[test]
    fn test_attention_varies_with_context() -> Result<()> {
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let fusion = FusionAttention::new(embeddings);

        let result1 = fusion.combine(0.7, 0.5, "test")?;
        let result2 = fusion.combine(0.7, 0.5, "complex explanation")?;

        // Should produce different results
        assert_ne!(result1, result2);

        Ok(())
    }
}
