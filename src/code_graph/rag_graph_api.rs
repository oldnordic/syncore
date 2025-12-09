//! RAGGraph API - Legacy compatibility layer
//!
//! This file provides backward compatibility by re-exporting the modular implementation
//! from the rag_graph/ subdirectory while maintaining the exact same public API surface.

// Re-export main public API types
pub use super::rag_graph::{
    FusionMode, RagGraphAPI, RagGraphQueryRequest, RagGraphQueryResponse, RankedEntity,
};
