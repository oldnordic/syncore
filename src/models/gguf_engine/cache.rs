//! Thread-safe caching for loaded models and tokenizers
//!
//! This module provides lazy loading and caching to avoid reloading
//! GGUF models and tokenizers on every inference request.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::models::gguf_engine::loader::ModelComponents;
use crate::models::gguf_engine::tokenizer::TokenizerWrapper;

/// Cached model and tokenizer components
#[derive(Debug)]
pub struct CachedModel {
    pub model: ModelComponents,
    pub tokenizer: TokenizerWrapper,
}

impl CachedModel {
    /// Create new cached model instance
    pub fn new(model: ModelComponents, tokenizer: TokenizerWrapper) -> Self {
        Self {
            model,
            tokenizer,
        }
    }

    /// Get reference to the model components
    pub fn model(&self) -> &ModelComponents {
        &self.model
    }

    /// Get reference to the tokenizer
    pub fn tokenizer(&self) -> &TokenizerWrapper {
        &self.tokenizer
    }
}

/// Global cache for loaded models
static MODEL_CACHE: OnceCell<Arc<CachedModel>> = OnceCell::const_new();

/// Load and cache model components (thread-safe)
pub async fn get_cached_model(
    model_path: PathBuf,
    tokenizer_path: Option<PathBuf>,
) -> Result<Arc<CachedModel>> {
    // Check if already cached
    if let Some(cached) = MODEL_CACHE.get() {
        return Ok(Arc::clone(cached));
    }

    // Load the model components
    let cached = load_model_components(model_path, tokenizer_path).await?;
    let cached_arc = Arc::new(cached);

    // Try to set in cache (ignore if another thread set it first)
    let _ = MODEL_CACHE.set(Arc::clone(&cached_arc));

    Ok(cached_arc)
}

/// Load model components from disk
async fn load_model_components(
    model_path: PathBuf,
    tokenizer_path: Option<PathBuf>,
) -> Result<CachedModel> {
    // Load model components
    let model = tokio::task::spawn_blocking(move || ModelComponents::from_gguf(&model_path))
        .await
        .context("Failed to join model loading task")?
        .context("Failed to load model from GGUF")?;

    // Load tokenizer
    let tokenizer =
        tokio::task::spawn_blocking(move || TokenizerWrapper::new_with_path(tokenizer_path))
            .await
            .context("Failed to join tokenizer loading task")?
            .context("Failed to load tokenizer")?;

    Ok(CachedModel::new(model, tokenizer))
}

/// Clear the model cache (useful for testing or model switching)
pub fn clear_cache() {
    // Note: OnceCell doesn't support clearing, so this is a no-op
    // In a production system, you might use RwLock<Option<Arc<CachedModel>>> instead
    tracing::warn!("Model cache clearing not implemented with OnceCell");
}

/// Check if model is cached
pub fn is_cached() -> bool {
    MODEL_CACHE.get().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_cache_initialization() {
        assert!(!is_cached());
    }

    #[tokio::test]
    #[ignore] // Requires actual model files
    async fn test_model_caching() -> Result<()> {
        let model_path = PathBuf::from("models/qwen2.5-0.5b.gguf");

        if !Path::new(&model_path).exists() {
            return Ok(()); // Skip test if model doesn't exist
        }

        // First load should cache the model
        let cached1 = get_cached_model(model_path.clone(), None).await?;
        assert!(is_cached());

        // Second load should return the same cached instance
        let cached2 = get_cached_model(model_path, None).await?;

        // Verify they're the same Arc (same underlying data)
        assert!(Arc::ptr_eq(&cached1, &cached2));

        Ok(())
    }
}
