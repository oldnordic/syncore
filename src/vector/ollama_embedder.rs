//! APEX 2.0-E: GPU Embedding Fast Path via Ollama
//!
//! Provides GPU-accelerated embeddings using Ollama's /api/embed endpoint.
//! Replaces CPU HuggingFace embeddings for CODE and GENERAL domains.
//!
//! ## Supported Models
//!
//! - **bge-m3**: 1024-dim embeddings (CODE and GENERAL default)
//! - **qwen2.5-coder:3b**: 2048-dim embeddings (CODE alternate)
//! - **nomic-embed-text**: 768-dim embeddings (GENERAL fallback, CPU)
//!
//! ## Architecture
//!
//! OllamaEmbedder implements the Embeddings trait and calls Ollama's HTTP API.
//! When Ollama is unavailable, DualEmbeddingService falls back to CPU embeddings.

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// HTTP request to Ollama /api/embed endpoint
#[derive(Debug, Serialize)]
struct EmbedRequest {
    model: String,
    input: String,
}

/// HTTP response from Ollama /api/embed endpoint
#[derive(Debug, Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

/// GPU-accelerated embedder using Ollama HTTP API
///
/// ## Usage
///
/// ```no_run
/// use syncore::vector::ollama_embedder::OllamaEmbedder;
///
/// let embedder = OllamaEmbedder::new("bge-m3", 1024)?;
/// let embedding = embedder.embed("fn main() {}")?;
/// assert_eq!(embedding.len(), 1024);
/// ```
pub struct OllamaEmbedder {
    model: String,
    dimension: usize,
    client: Client,
    endpoint: String,
}

impl OllamaEmbedder {
    /// Create OllamaEmbedder with specified model and dimension
    ///
    /// # Arguments
    /// * `model` - Ollama model name (e.g., "bge-m3", "qwen2.5-coder:3b")
    /// * `dimension` - Expected embedding dimension (1024, 2048, 768)
    ///
    /// # Panics
    /// Does not panic - returns Result for HTTP client creation
    pub fn new(model: &str, dimension: usize) -> Result<Self> {
        Ok(Self {
            model: model.to_string(),
            dimension,
            client: Client::new(),
            endpoint: "http://localhost:11434/api/embed".to_string(),
        })
    }

    /// Create OllamaEmbedder with custom endpoint (for testing)
    pub fn with_endpoint(model: &str, dimension: usize, endpoint: String) -> Result<Self> {
        Ok(Self {
            model: model.to_string(),
            dimension,
            client: Client::new(),
            endpoint,
        })
    }

    /// Call Ollama /api/embed and return embedding vector
    ///
    /// # HTTP Request Format
    /// ```json
    /// {
    ///   "model": "bge-m3",
    ///   "input": "text to embed"
    /// }
    /// ```
    ///
    /// # HTTP Response Format
    /// ```json
    /// {
    ///   "embeddings": [[0.123, -0.456, ...]]
    /// }
    /// ```
    ///
    /// # Errors
    /// Returns error if:
    /// - Ollama server unreachable
    /// - HTTP request fails
    /// - Response parsing fails
    /// - Returned dimension doesn't match expected
    fn embed_http(&self, text: &str) -> Result<Vec<f32>> {
        let request = EmbedRequest {
            model: self.model.clone(),
            input: text.to_string(),
        };

        // Blocking HTTP call (sync interface required by Embeddings trait)
        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .context("Failed to call Ollama /api/embed")?;

        let embed_response: EmbedResponse = response
            .json()
            .context("Failed to parse Ollama response")?;

        // Extract first embedding (single input → single output)
        let embedding = embed_response
            .embeddings
            .into_iter()
            .next()
            .context("Ollama returned empty embeddings array")?;

        // Verify dimension matches expected
        if embedding.len() != self.dimension {
            anyhow::bail!(
                "Dimension mismatch: expected {} but got {} from model {}",
                self.dimension,
                embedding.len(),
                self.model
            );
        }

        Ok(embedding)
    }
}

impl crate::vector::Embeddings for OllamaEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_http(text)
    }

    fn dim(&self) -> usize {
        self.dimension
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::Embeddings;

    #[test]
    fn test_ollama_embedder_creation() {
        let embedder = OllamaEmbedder::new("bge-m3", 1024);
        assert!(embedder.is_ok());
        assert_eq!(embedder.unwrap().dim(), 1024);
    }

    #[test]
    fn test_ollama_embedder_dimension() {
        let embedder = OllamaEmbedder::new("qwen2.5-coder:3b", 2048).unwrap();
        assert_eq!(embedder.dim(), 2048);
    }

    #[test]
    fn test_ollama_embedder_custom_endpoint() {
        let embedder = OllamaEmbedder::with_endpoint(
            "bge-m3",
            1024,
            "http://custom:8080/api/embed".to_string(),
        )
        .unwrap();
        assert_eq!(embedder.endpoint, "http://custom:8080/api/embed");
    }

    // Integration test: Requires Ollama running
    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_ollama_embedder_live_api() -> Result<()> {
        let embedder = OllamaEmbedder::new("bge-m3", 1024)?;
        let embedding = embedder.embed("fn main() {}")?;
        assert_eq!(embedding.len(), 1024);
        assert!(embedding.iter().any(|&x| x != 0.0), "Embedding should be non-zero");
        Ok(())
    }

    // Integration test: Determinism check
    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_ollama_embedder_deterministic() -> Result<()> {
        let embedder = OllamaEmbedder::new("bge-m3", 1024)?;
        let emb1 = embedder.embed("test text")?;
        let emb2 = embedder.embed("test text")?;
        assert_eq!(emb1, emb2, "Embeddings should be deterministic");
        Ok(())
    }

    // Integration test: Dimension mismatch detection
    #[test]
    #[ignore]
    fn test_ollama_embedder_dimension_mismatch() {
        let embedder = OllamaEmbedder::new("bge-m3", 2048).unwrap(); // Wrong dimension
        let result = embedder.embed("test");
        assert!(result.is_err(), "Should fail with dimension mismatch");
        assert!(result.unwrap_err().to_string().contains("Dimension mismatch"));
    }
}
