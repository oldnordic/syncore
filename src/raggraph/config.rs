//! RagGraph configuration types

use serde::{Deserialize, Serialize};

/// Backend mode for RagGraph storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaggraphBackendMode {
    /// Use mock storage (deterministic hash-based seeds and synthetic graphs)
    Mock,
    /// Use real storage (HNSW vector search + Neo4j graph database)
    Real,
}

impl Default for RaggraphBackendMode {
    fn default() -> Self {
        Self::Mock
    }
}

/// Configuration for RagGraph operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagGraphConfig {
    pub num_hops: usize,
    pub alpha: f32,
    pub top_k: usize,
    pub embedding_dim: usize,
    pub backend_mode: RaggraphBackendMode,
}

impl Default for RagGraphConfig {
    fn default() -> Self {
        Self {
            num_hops: 3,
            alpha: 0.85,
            top_k: 50,
            embedding_dim: 384,
            backend_mode: RaggraphBackendMode::Mock,
        }
    }
}

impl RagGraphConfig {
    /// Create config from environment variables
    ///
    /// Reads SYNCORE_RAGGRAPH_BACKEND env var:
    /// - "real" or "REAL" -> RaggraphBackendMode::Real
    /// - anything else or unset -> RaggraphBackendMode::Mock (default)
    pub fn from_env() -> Self {
        let backend_mode = std::env::var("SYNCORE_RAGGRAPH_BACKEND")
            .ok()
            .map(|v| v.to_lowercase())
            .and_then(|v| {
                if v == "real" {
                    Some(RaggraphBackendMode::Real)
                } else {
                    None
                }
            })
            .unwrap_or(RaggraphBackendMode::Mock);

        Self {
            backend_mode,
            ..Default::default()
        }
    }
}
