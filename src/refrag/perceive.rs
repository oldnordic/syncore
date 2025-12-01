//! APEX 1.8 REFRAG - PerceiveSelector
//!
//! Deterministic selective expansion based on:
//! - fusion_score (from tri-mode fusion)
//! - graph_score (k-hop connectivity)
//! - structural_score (AST node type hierarchy)
//! - perplexity_score (optional LLM fallback)
//!
//! Design:
//! - NO randomness
//! - Deterministic tie-breaking
//! - Policy-based selection (TopK, TopPercent, Weighted)

use super::types::ChunkMetadata;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Selection policy for perceive stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectionPolicy {
    /// Select top K chunks by combined score
    TopK(usize),

    /// Select top N% of chunks
    TopPercent(u8),

    /// Weighted combination of scores
    Weighted {
        fusion_weight: f32,
        graph_weight: f32,
        structural_weight: f32,
    },

    /// Prioritize graph connectivity
    GraphPriority,
}

impl Default for SelectionPolicy {
    fn default() -> Self {
        SelectionPolicy::TopPercent(20)
    }
}

/// Result of chunk selection
#[derive(Debug, Clone)]
pub struct SelectionResult {
    /// Selected chunks (for raw expansion)
    pub selected: Vec<ChunkMetadata>,

    /// Rejected chunks (for compressed summaries)
    pub rejected: Vec<ChunkMetadata>,
}

/// Deterministic chunk selector
pub struct PerceiveSelector {
    policy: SelectionPolicy,
}

impl PerceiveSelector {
    /// Create new selector with policy
    pub fn new(policy: SelectionPolicy) -> Self {
        Self {
            policy,
        }
    }

    /// Select chunks deterministically based on query and candidates
    pub fn select_chunks(
        &self,
        _query: &str,
        mut candidates: Vec<ChunkMetadata>,
    ) -> Result<SelectionResult> {
        if candidates.is_empty() {
            return Ok(SelectionResult {
                selected: Vec::new(),
                rejected: Vec::new(),
            });
        }

        // Compute combined scores for all candidates
        self.compute_combined_scores(&mut candidates)?;

        // Sort by combined score (descending), then by structural score for ties
        candidates.sort_by(|a, b| {
            let score_cmp =
                b.fusion_score.partial_cmp(&a.fusion_score).unwrap_or(std::cmp::Ordering::Equal);

            if score_cmp == std::cmp::Ordering::Equal {
                // Tie-breaker: structural score
                b.structural_score
                    .partial_cmp(&a.structural_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                score_cmp
            }
        });

        // Apply policy to determine selection count
        let select_count = self.compute_selection_count(&candidates);

        // Split into selected and rejected
        let (selected, rejected) = if select_count < candidates.len() {
            let (sel, rej) = candidates.split_at(select_count);
            (sel.to_vec(), rej.to_vec())
        } else {
            (candidates, Vec::new())
        };

        Ok(SelectionResult {
            selected,
            rejected,
        })
    }

    /// Compute combined scores based on policy
    fn compute_combined_scores(&self, candidates: &mut [ChunkMetadata]) -> Result<()> {
        // Compute structural scores first
        self.compute_structural_scores(candidates)?;

        // Apply policy-specific scoring
        match &self.policy {
            SelectionPolicy::Weighted {
                fusion_weight,
                graph_weight,
                structural_weight,
            } => {
                // Normalize structural scores to [0,1] range
                let max_structural =
                    candidates.iter().map(|c| c.structural_score).fold(0.0_f32, f32::max);

                for chunk in candidates.iter_mut() {
                    let normalized_structural = if max_structural > 0.0 {
                        chunk.structural_score / max_structural
                    } else {
                        0.0
                    };

                    // Combined score = weighted sum
                    chunk.fusion_score = chunk.fusion_score * fusion_weight
                        + chunk.graph_score * graph_weight
                        + normalized_structural * structural_weight;
                }
            }
            SelectionPolicy::GraphPriority => {
                // Override fusion score with graph score
                for chunk in candidates.iter_mut() {
                    chunk.fusion_score = chunk.graph_score;
                }
            }
            _ => {
                // TopK and TopPercent use fusion_score as-is
            }
        }

        Ok(())
    }

    /// Compute structural importance scores based on entity type
    ///
    /// Hierarchy: Function(10) > Class(9) > Method(8) > Impl(7) > Struct(6) > Block(3) > Import(1)
    pub fn compute_structural_scores(&self, candidates: &mut [ChunkMetadata]) -> Result<()> {
        for chunk in candidates.iter_mut() {
            chunk.structural_score = match chunk.entity_type.as_deref() {
                Some("Function") => 10.0,
                Some("Class") => 9.0,
                Some("Method") => 8.0,
                Some("Impl") => 7.0,
                Some("Struct") => 6.0,
                Some("Block") => 3.0,
                Some("Import") => 1.0,
                _ => 5.0, // Default for unknown types
            };
        }

        Ok(())
    }

    /// Compute how many chunks to select based on policy
    fn compute_selection_count(&self, candidates: &[ChunkMetadata]) -> usize {
        match self.policy {
            SelectionPolicy::TopK(k) => k.min(candidates.len()),
            SelectionPolicy::TopPercent(percent) => {
                let count = (candidates.len() as f32 * (percent as f32 / 100.0)).ceil() as usize;
                count.max(1).min(candidates.len())
            }
            SelectionPolicy::Weighted {
                ..
            }
            | SelectionPolicy::GraphPriority => {
                // Default to top 20%
                let count = (candidates.len() as f32 * 0.2).ceil() as usize;
                count.max(1).min(candidates.len())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selector_deterministic() {
        // Basic smoke test - real tests in tests/refrag_perceive_test.rs
        let policy = SelectionPolicy::default();
        let _selector = PerceiveSelector::new(policy);
        // Test passes if no panic
    }
}
