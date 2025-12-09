//! Embeddings Module
//!
//! Contains various embedding implementations for different domains.
//! Provides both feature-engineered and transformer-based embeddings.

pub mod graphbert_candle;

// Re-export for easier access
pub use graphbert_candle::{GraphBertCandleEmbeddings, GraphBertError};