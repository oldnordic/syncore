//! APEX 1.8 REFRAG - Selective Expansion Layer
//!
//! Implements REFRAG-style RAG with deterministic chunk selection:
//! - ChunkCompressionLayer: Reuse existing embeddings (no re-embedding)
//! - PerceiveSelector: Deterministic selection based on fusion/graph/structural scores
//! - ExpandStage: Selective expansion (RAW vs COMPRESSED)
//! - HybridPromptBuilder: Assemble final LLM prompts

pub mod builder;
pub mod compression;
pub mod expand;
pub mod perceive;
pub mod pipeline;
pub mod types;

// Re-exports for convenient API access
pub use builder::HybridPromptBuilder;
pub use compression::ChunkCompressionLayer;
pub use expand::{ChunkFormat, ExpandStage, ExpandedChunk};
pub use perceive::{PerceiveSelector, SelectionPolicy, SelectionResult};
pub use pipeline::{RefragPipeline, RefragResult};
pub use types::{ChunkMetadata, Domain};

/// REFRAG pipeline configuration
#[derive(Debug, Clone)]
pub struct RefragConfig {
    /// Maximum number of raw chunks to include
    pub top_k_raw: usize,
    /// Maximum total tokens allowed in final prompt
    pub max_tokens: usize,
    /// Selection policy for perceive stage
    pub selection_policy: SelectionPolicy,
}

impl Default for RefragConfig {
    fn default() -> Self {
        Self {
            top_k_raw: 5,
            max_tokens: 4000,
            selection_policy: SelectionPolicy::default(),
        }
    }
}
