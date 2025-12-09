//! APEX 1.8 REFRAG - Shared Types
//!
//! Common data structures used across REFRAG modules.
//! Kept under 100 LOC as per requirements.

use serde::{Deserialize, Serialize};

/// Domain classification for chunks (reuses APEX 1.7 domains)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Domain {
    /// CODE domain: code entities with code-optimized embeddings
    Code,
    /// GENERAL domain: documents, tasks, notes with general embeddings
    General,
    /// GRAPH domain: graph entities, nodes, edges, relationships
    Graph,
}

impl From<crate::vector::domain::EmbeddingDomain> for Domain {
    fn from(domain: crate::vector::domain::EmbeddingDomain) -> Self {
        match domain {
            crate::vector::domain::EmbeddingDomain::Code => Domain::Code,
            crate::vector::domain::EmbeddingDomain::General => Domain::General,
            crate::vector::domain::EmbeddingDomain::Graph => Domain::Graph,
        }
    }
}

impl From<Domain> for crate::vector::domain::EmbeddingDomain {
    fn from(domain: Domain) -> Self {
        match domain {
            Domain::Code => crate::vector::domain::EmbeddingDomain::Code,
            Domain::General => crate::vector::domain::EmbeddingDomain::General,
            Domain::Graph => crate::vector::domain::EmbeddingDomain::Graph,
        }
    }
}

/// Metadata for a single chunk from compression layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Unique chunk identifier (from vector store)
    pub chunk_id: i64,

    /// Domain classification
    pub domain: Domain,

    /// Precomputed embedding vector (384-dim HuggingFace)
    pub embedding: Option<Vec<f32>>,

    /// File path (if available)
    pub file_path: Option<String>,

    /// Entity type (Function, Class, Struct, etc.)
    pub entity_type: Option<String>,

    /// Extracted symbols (function names, struct names, etc.)
    pub symbols: Vec<String>,

    /// Line range in source file
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,

    /// Fusion score (from tri-mode fusion: Simple/Attention/Reasoning)
    pub fusion_score: f32,

    /// Graph connectivity score (k-hop from Neo4j)
    pub graph_score: f32,

    /// Structural importance score (Function > Impl > Block > Import)
    pub structural_score: f32,

    /// Optional perplexity score (LLM-based, fallback only)
    pub perplexity_score: Option<f32>,

    /// Graph hops from query origin (1 = direct, 2 = 2-hop, etc.)
    pub graph_hops: Option<u32>,

    /// Full text content (from Hit.text)
    pub text: String,
}

impl ChunkMetadata {
    /// Create new chunk metadata with minimal required fields
    pub fn new(chunk_id: i64, domain: Domain, text: String) -> Self {
        Self {
            chunk_id,
            domain,
            embedding: None,
            file_path: None,
            entity_type: None,
            symbols: Vec::new(),
            line_start: None,
            line_end: None,
            fusion_score: 0.0,
            graph_score: 0.0,
            structural_score: 0.0,
            perplexity_score: None,
            graph_hops: None,
            text,
        }
    }
}
