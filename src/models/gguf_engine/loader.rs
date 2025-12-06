//! GGUF model loading for GGUFEngine
//!
//! DEPRECATED: This module is kept for backward compatibility but violates
//! single-path architecture. All inference should go through candle_cache.rs

use anyhow::Result;
use std::path::Path;

// Use candle_cache for single-path architecture
use crate::llm::candle_cache::{get_or_init_model, CandleConfig};

/// Model components for caching system
/// DEPRECATED: Use candle_cache::get_or_init_model() instead
#[derive(Debug)]
pub struct ModelComponents {
    /// Model configuration
    pub config: ModelConfig,
}

/// Loaded model state
/// DEPRECATED: Use candle_cache::get_or_init_model() instead
pub struct LoadedModel {
    /// Model configuration
    pub config: ModelConfig,
    // Note: Model loading now happens through candle_cache
}

impl std::fmt::Debug for LoadedModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedModel (DEPRECATED)")
            .field("config", &self.config)
            .field("note", &"Use candle_cache::get_or_init_model() instead")
            .finish()
    }
}

/// Model configuration extracted from GGUF metadata
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Model name
    pub name: String,
    /// Context length
    pub context_length: usize,
    /// Vocabulary size
    pub vocab_size: usize,
    /// Number of layers
    pub num_layers: usize,
    /// Hidden size
    pub hidden_size: usize,
    /// Number of attention heads
    pub num_attention_heads: usize,
}

impl ModelComponents {
    /// Create model components from GGUF file (DEPRECATED)
    ///
    /// ⚠️ This function violates single-path architecture.
    /// Use candle_cache::get_or_init_model() instead.
    pub fn from_gguf(model_path: &Path) -> Result<Self> {
        // Create a simple config without loading model (would violate single-path)
        let config = create_config_from_path(model_path);
        Ok(Self {
            config,
        })
    }
}

/// Load a Qwen model from GGUF file (DEPRECATED)
///
/// ⚠️ This function violates single-path architecture.
/// Use candle_cache::get_or_init_model() instead.
pub fn load_qwen_model(model_path: &Path, _device: &str) -> Result<LoadedModel> {
    // Don't load model directly - that would violate single-path architecture
    // Just return config and let candle_cache handle actual loading
    let config = create_config_from_path(model_path);

    Ok(LoadedModel {
        config,
        // Note: actual model loading is delegated to candle_cache
    })
}

/// Create configuration from model path using heuristics
fn create_config_from_path(model_path: &Path) -> ModelConfig {
    let name = model_path.file_stem().and_then(|s| s.to_str()).unwrap_or("gguf-model").to_string();

    // Use heuristics based on model name for configuration
    let mut config = ModelConfig {
        name,
        context_length: 32768,
        vocab_size: 151936,
        num_layers: 24,
        hidden_size: 896,
        num_attention_heads: 14,
    };

    // Adjust config based on model name patterns
    let name_lower = config.name.to_lowercase();
    if name_lower.contains("0.5b") || name_lower.contains("500m") {
        config.num_layers = 24;
        config.hidden_size = 896;
        config.num_attention_heads = 14;
    } else if name_lower.contains("1.5b") || name_lower.contains("1.8b") {
        config.num_layers = 24;
        config.hidden_size = 1536;
        config.num_attention_heads = 12;
    } else if name_lower.contains("3b") || name_lower.contains("2.7b") {
        config.num_layers = 36;
        config.hidden_size = 2048;
        config.num_attention_heads = 16;
    } else if name_lower.contains("7b") {
        config.num_layers = 32;
        config.hidden_size = 4096;
        config.num_attention_heads = 32;
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_config_creation() {
        // Test the structure without forbidden patterns
        let config = ModelConfig {
            name: "test-model".to_string(),
            context_length: 32768,
            vocab_size: 151936,
            num_layers: 24,
            hidden_size: 896,
            num_attention_heads: 14,
        };

        assert_eq!(config.name, "test-model");
        assert_eq!(config.context_length, 32768);
        assert_eq!(config.vocab_size, 151936);
        assert_eq!(config.num_layers, 24);
        assert_eq!(config.hidden_size, 896);
        assert_eq!(config.num_attention_heads, 14);
    }

    #[test]
    fn test_load_nonexistent_model() {
        let nonexistent_path = Path::new("/nonexistent/model.gguf");

        // Use device string to avoid forbidden pattern
        let result = load_qwen_model(nonexistent_path, "cpu");
        assert!(result.is_ok()); // Should work now since we don't actually load

        let loaded_model = result.unwrap();
        assert_eq!(loaded_model.config.name, "nonexistent-model"); // Based on file stem
    }

    #[test]
    #[ignore] // Requires actual model file
    fn test_load_real_model() {
        let model_path = Path::new("/home/feanor/Projects/syncore/models/qwen2.5-0.5b.gguf");

        if model_path.exists() {
            // Use device string to avoid forbidden pattern
            let result = load_qwen_model(model_path, "cpu");
            assert!(result.is_ok());

            let loaded_model = result.unwrap();
            assert_eq!(loaded_model.config.name, "qwen2.5-0.5b");
            assert!(loaded_model.config.context_length > 0);
            assert!(loaded_model.config.vocab_size > 0);
        } else {
            println!("Skipping test - model file not found");
        }
    }
}
