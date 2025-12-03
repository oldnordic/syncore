//! GGUF model loading for GGUFEngine
//!
//! Provides functionality to load GGUF format models using technical dependencies.
//! This provides real GGUF support with high-performance inference.

use anyhow::Result;
use candle_core::Device;
use std::path::Path;

use super::gguf_runtime::{load_gguf_model_sync, GgufModel};

/// Model components for caching system
pub struct ModelComponents {
    /// GGUF model
    pub model: GgufModel,
    /// Model configuration
    pub config: ModelConfig,
}

/// Loaded model state
pub struct LoadedModel {
    /// Model configuration
    pub config: ModelConfig,
    /// Device used for computation
    pub device: Device,
    /// GGUF model
    pub model: GgufModel,
}

impl std::fmt::Debug for LoadedModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedModel")
            .field("config", &self.config)
            .field("device", &self.device)
            .field("model", &"<GgufModel>") // Can't debug the actual model
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
    /// Create model components from GGUF file
    pub fn from_gguf(model_path: &Path) -> Result<Self> {
        let loaded = load_qwen_model(model_path, &candle_core::Device::Cpu)?;
        Ok(Self {
            model: loaded.model,
            config: loaded.config,
        })
    }
}

/// Load a Qwen model from GGUF file using gguf_runtime (sync wrapper)
pub fn load_qwen_model(model_path: &Path, _device: &Device) -> Result<LoadedModel> {
    // Force CPU-only optimizations
    std::env::set_var("CUDA_VISIBLE_DEVICES", "");
    std::env::set_var("ROCR_VISIBLE_DEVICES", "");

    // Load model using the vendor-neutral runtime
    let model = load_gguf_model_sync(model_path)?;

    // Extract configuration from model path
    let config = extract_config_from_path(model_path);

    Ok(LoadedModel {
        config,
        device: Device::Cpu, // gguf_runtime handles device internally
        model,
    })
}

/// Extract configuration from model path using gguf_runtime heuristics
fn extract_config_from_path(model_path: &Path) -> ModelConfig {
    let gguf_config = super::gguf_runtime::extract_config_from_path(model_path);

    ModelConfig {
        name: gguf_config.name,
        context_length: gguf_config.context_length,
        vocab_size: gguf_config.vocab_size,
        num_layers: gguf_config.num_layers,
        hidden_size: gguf_config.hidden_size,
        num_attention_heads: gguf_config.num_attention_heads,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;

    #[test]
    fn test_model_config_creation() {
        // This test would require a real GGUF file, so we'll test the structure
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
        let device = Device::Cpu;
        let nonexistent_path = Path::new("/nonexistent/model.gguf");

        let result = load_qwen_model(nonexistent_path, &device);
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("does not exist"));
    }

    #[test]
    #[ignore] // Requires actual model file
    fn test_load_real_model() {
        let model_path = Path::new("/home/feanor/Projects/syncore/models/qwen2.5-0.5b.gguf");
        let device = Device::Cpu;

        if model_path.exists() {
            let result = load_qwen_model(model_path, &device);
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
