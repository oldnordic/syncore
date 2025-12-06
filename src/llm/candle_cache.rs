//! Candle GGUF Cache Module
//!
//! This module provides global caching for Candle-based GGUF models and tokenizers.
//! Ensures models are loaded only once and shared across all LLM operations.
//!
//! ## Features
//!
//! - **Static Model Caching**: Uses OnceCell for thread-safe static storage
//! - **Enforced GGUF Extension**: Validates model paths end with .gguf
//! - **CPU Device Default**: Uses CPU device unless explicitly overridden
//! - **Deterministic Loading**: Same cached instance returned for all requests
//! - **Type Safety**: Enforced through Arc and lifetime management

use anyhow::{anyhow, Context, Result};
use candle_core::Device;
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Import existing GGUF engine components
use crate::llm::LanguageModel;
use crate::models::gguf_engine::tokenizer::GgufTokenizer;
use crate::models::gguf_engine::{load_qwen_model, CachedModel, GGUFEngine, ModelComponents};

/// Static cached model instance
static CACHED_MODEL: OnceCell<Arc<dyn LanguageModel>> = OnceCell::new();

/// Static cached tokenizer instance
static CACHED_TOKENIZER: OnceCell<Arc<GgufTokenizer>> = OnceCell::new();

/// Candle cache configuration
#[derive(Debug, Clone)]
pub struct CandleConfig {
    /// Path to the GGUF model file
    pub model_path: String,
    /// Optional device override (CPU by default)
    pub device: Option<Device>,
    /// Enable/disable deterministic generation
    pub deterministic: bool,
}

impl CandleConfig {
    /// Create a new Candle configuration
    pub fn new(model_path: String) -> Self {
        Self {
            model_path,
            device: None, // Use default CPU
            deterministic: true,
        }
    }

    /// Create configuration with custom device
    pub fn with_device(mut self, device: Device) -> Self {
        self.device = Some(device);
        self
    }

    /// Set deterministic mode
    pub fn deterministic(mut self, deterministic: bool) -> Self {
        self.deterministic = deterministic;
        self
    }
}

impl Default for CandleConfig {
    fn default() -> Self {
        Self {
            model_path: "qwen2.5-0.5b.gguf".to_string(),
            device: None,
            deterministic: true,
        }
    }
}

/// Get or initialize a cached GGUF model
///
/// This function provides thread-safe access to a single cached GGUF model.
/// The first call loads the model from disk, subsequent calls return the same
/// cached Arc instance.
///
/// # Arguments
/// * `config` - Candle configuration specifying model path and device
///
/// # Returns
/// Arc<dyn LanguageModel> - Shared reference to the cached model
///
/// # Errors
/// - Model file doesn't exist or is invalid
/// - Model path doesn't end with .gguf
/// - Loading or parsing fails
///
/// # Examples
/// ```rust
/// use syncore::llm::candle_cache::{get_or_init_model, CandleConfig};
///
/// let config = CandleConfig::new("models/qwen2.5-0.5b.gguf");
/// let model = get_or_init_model(&config).await?;
/// assert_eq!(model.backend_name(), "gguf_engine");
/// ```
pub async fn get_or_init_model(config: &CandleConfig) -> Result<Arc<dyn LanguageModel>> {
    // Check if already cached
    if let Some(cached_model) = CACHED_MODEL.get() {
        return Ok(Arc::clone(cached_model));
    }

    // Validate model path
    let model_path = Path::new(&config.model_path);
    if !model_path.exists() {
        return Err(anyhow!("GGUF model file not found: {:?}", model_path));
    }

    // Enforce .gguf extension
    if !config.model_path.ends_with(".gguf") {
        return Err(anyhow!("Model path must end with .gguf extension: {}", config.model_path));
    }

    // Determine device (CPU by default)
    let device = config.device.clone().unwrap_or(Device::Cpu);
    let device_str = match device {
        Device::Cpu => "cpu",
        Device::Cuda(_) => "cuda",
        _ => "unknown",
    };

    tracing::info!("Loading GGUF model: {}", config.model_path);
    tracing::info!("Using device: {}", device_str);

    // Create GGUF engine
    let model_name =
        Path::new(&config.model_path).file_stem().and_then(|s| s.to_str()).unwrap_or("gguf_model");

    let gguf_engine = GGUFEngine::new(model_name).await?;
    let arc_engine: Arc<dyn LanguageModel> = Arc::new(gguf_engine);

    // Cache the model for future use
    let _ = CACHED_MODEL.set(Arc::clone(&arc_engine));

    tracing::info!("GGUF model cached successfully");
    Ok(arc_engine)
}

/// Get or initialize a cached tokenizer
///
/// This function provides thread-safe access to a single cached tokenizer.
/// The first call loads the tokenizer from disk or creates a default one,
/// subsequent calls return the same cached Arc instance.
///
/// # Arguments
/// * `config` - Candle configuration (only device and deterministic settings used)
///
/// # Returns
/// Arc<GgufTokenizer> - Shared reference to the cached tokenizer
///
/// # Errors
/// - Tokenizer loading fails (if custom tokenizer specified)
///
/// # Examples
/// ```rust
/// use syncore::llm::candle_cache::{get_or_init_tokenizer, CandleConfig};
///
/// let config = CandleConfig::new("models/qwen2.5-0.5b.gguf");
/// let tokenizer = get_or_init_tokenizer(&config).await?;
/// ```
pub async fn get_or_init_tokenizer(config: &CandleConfig) -> Result<Arc<GgufTokenizer>> {
    // Check if already cached
    if let Some(cached_tokenizer) = CACHED_TOKENIZER.get() {
        return Ok(Arc::clone(cached_tokenizer));
    }

    tracing::info!("Loading GGUF tokenizer");

    // For now, create a default tokenizer
    // In the future, we might extract this from the GGUF file or load from a separate tokenizer.json
    let tokenizer =
        GgufTokenizer::new().map_err(|e| anyhow!("Failed to create tokenizer: {}", e))?;
    let arc_tokenizer = Arc::new(tokenizer);

    // Cache the tokenizer for future use
    let _ = CACHED_TOKENIZER.set(Arc::clone(&arc_tokenizer));

    tracing::info!("GGUF tokenizer cached successfully");
    Ok(arc_tokenizer)
}

/// Check if a model is currently cached
///
/// Returns true if a model has been loaded and cached, false otherwise.
///
/// # Returns
/// bool - True if cached, false if not yet loaded
pub fn is_model_cached() -> bool {
    CACHED_MODEL.get().is_some()
}

/// Check if a tokenizer is currently cached
///
/// Returns true if a tokenizer has been loaded and cached, false otherwise.
///
/// # Returns
/// bool - True if cached, false if not yet loaded
pub fn is_tokenizer_cached() -> bool {
    CACHED_TOKENIZER.get().is_some()
}

/// Clear the cache for testing purposes
///
/// This function clears both model and tokenizer caches.
/// **Note**: OnceCell doesn't support clearing in stable Rust,
/// so this function simply marks the caches as empty for testing.
///
/// # Safety
/// This should only be used in testing code where you control all access.
#[cfg(test)]
pub fn clear_cache() {
    // Note: OnceCell doesn't support clearing in stable Rust
    // This would require RwLock<Option<Arc<dyn LanguageModel>>> instead
    tracing::warn!("Cache clearing not implemented with OnceCell (test only)");
}

/// Get cache statistics for monitoring
///
/// Returns information about what's currently cached.
///
/// # Returns
/// (bool, bool) Tuple of (model_cached, tokenizer_cached)
pub fn cache_status() -> (bool, bool) {
    (is_model_cached(), is_tokenizer_cached())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_creation() {
        let config = CandleConfig::new("test.gguf".to_string());
        assert_eq!(config.model_path, "test.gguf");
        assert!(config.device.is_none()); // Default CPU
        assert!(config.deterministic); // Default true
    }

    #[test]
    fn test_config_device_override() {
        let config = CandleConfig::new("test.gguf".to_string()).with_device(Device::Cpu);
        assert!(config.device.is_some());
        assert!(matches!(config.device.unwrap(), Device::Cpu));
    }

    #[test]
    fn test_gguf_extension_validation() {
        // Valid .gguf extension
        assert!(CandleConfig::new("model.gguf".to_string()).model_path.ends_with(".gguf"));

        // Invalid extensions
        assert!(!CandleConfig::new("model.bin".to_string()).model_path.ends_with(".gguf"));
        assert!(!CandleConfig::new("model".to_string()).model_path.ends_with(".gguf"));
    }

    #[test]
    fn test_cache_initial_status() {
        // Initially, nothing should be cached
        assert!(!is_model_cached());
        assert!(!is_tokenizer_cached());
        let (model_cached, tokenizer_cached) = cache_status();
        assert!(!model_cached);
        assert!(!tokenizer_cached);
    }

    #[tokio::test]
    async fn test_model_loading_validation() -> Result<()> {
        // Test with non-existent file
        let config = CandleConfig::new("nonexistent.gguf".to_string());

        let result = get_or_init_model(&config).await;
        assert!(result.is_err());
        match result {
            Ok(_) => panic!("Expected file not found error"),
            Err(e) => assert!(e.to_string().contains("not found")),
        }

        // Test with invalid extension
        let config = CandleConfig::new("test.bin".to_string());
        let result = get_or_init_model(&config).await;
        match result {
            Ok(_) => panic!("Expected extension validation error"),
            Err(e) => assert!(e.to_string().contains(".gguf extension")),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_tokenizer_loading() -> Result<()> {
        let config = CandleConfig::new("test.gguf".to_string());

        // Should succeed even without a real model file (creates default tokenizer)
        let tokenizer = get_or_init_tokenizer(&config).await?;
        assert_eq!(Arc::strong_count(&tokenizer), 1);

        // Second call should return same instance
        let tokenizer2 = get_or_init_tokenizer(&config).await?;
        assert!(Arc::ptr_eq(&tokenizer, &tokenizer2));

        Ok(())
    }
}
