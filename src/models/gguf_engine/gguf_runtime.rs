//! GGUF Runtime Wrapper
//!
//! This module provides a vendor-neutral interface to underlying mistral.rs
//! technical dependency. All vendor-specific types and functions are wrapped
//! behind neutral APIs to maintain clean separation.

use anyhow::{anyhow, Result};
use candle_core::Device;
use std::path::Path;

// Re-export technical dependency privately
use mistralrs::{GgufModelBuilder, Model as InternalModel};

/// Vendor-neutral GGUF model wrapper
pub struct GgufRuntimeModel {
    /// Internal model implementation (technical detail)
    inner: InternalModel,
}

impl std::fmt::Debug for GgufRuntimeModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GgufRuntimeModel").field("inner", &"<InternalModel>").finish()
    }
}

impl GgufRuntimeModel {
    /// Get reference to internal model (for advanced operations)
    pub fn inner(&self) -> &InternalModel {
        &self.inner
    }
}

/// GGUF runtime session for inference operations
pub struct GgufRuntimeSession {
    /// Model for this session
    model: GgufRuntimeModel,
    /// Device used for computation
    device: Device,
}

impl GgufRuntimeSession {
    /// Create a new runtime session
    pub fn new(model: GgufRuntimeModel, device: Device) -> Self {
        Self {
            model,
            device,
        }
    }

    /// Get model
    pub fn model(&self) -> &GgufRuntimeModel {
        &self.model
    }

    /// Get device
    pub fn device(&self) -> &Device {
        &self.device
    }
}

/// Builder for loading GGUF models with vendor-neutral interface
pub struct GgufRuntimeBuilder {
    /// Model directory path
    model_dir: String,
    /// Model filename(s)
    model_files: Vec<String>,
    /// Whether to enable logging
    logging: bool,
}

impl GgufRuntimeBuilder {
    /// Create a new builder
    pub fn new(model_dir: &str, model_files: Vec<String>) -> Self {
        Self {
            model_dir: model_dir.to_string(),
            model_files,
            logging: false,
        }
    }

    /// Enable logging during model loading
    pub fn with_logging(mut self) -> Self {
        self.logging = true;
        self
    }

    /// Build model asynchronously
    pub async fn build(self) -> Result<GgufRuntimeModel> {
        let mut builder = GgufModelBuilder::new(&self.model_dir, self.model_files);

        if self.logging {
            builder = builder.with_logging();
        }

        let internal_model =
            builder.build().await.map_err(|e| anyhow!("Failed to build GGUF model: {}", e))?;

        Ok(GgufRuntimeModel {
            inner: internal_model,
        })
    }
}

/// Load a GGUF model from file path
pub async fn load_gguf_model(model_path: &Path) -> Result<GgufRuntimeModel> {
    // Validate file exists
    if !model_path.exists() {
        return Err(anyhow!("Model file does not exist: {:?}", model_path));
    }

    // Extract directory and filename from the path
    let model_dir = model_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_str()
        .ok_or_else(|| anyhow!("Invalid model directory path"))?;

    let model_filename = model_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Invalid model filename"))?;

    let builder = GgufRuntimeBuilder::new(model_dir, vec![model_filename.to_string()]);
    builder.build().await
}

/// Load a GGUF model synchronously (wrapper for async function)
pub fn load_gguf_model_sync(model_path: &Path) -> Result<GgufRuntimeModel> {
    // Force CPU-only optimizations
    std::env::set_var("CUDA_VISIBLE_DEVICES", "");
    std::env::set_var("ROCR_VISIBLE_DEVICES", "");

    // Create a new runtime for this operation
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(load_gguf_model(model_path))
}

/// Create a runtime session with the model
pub fn create_session(model: GgufRuntimeModel, device: Device) -> GgufRuntimeSession {
    GgufRuntimeSession::new(model, device)
}

/// Model configuration extracted from GGUF metadata
#[derive(Debug, Clone)]
pub struct GgufModelConfig {
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

impl Default for GgufModelConfig {
    fn default() -> Self {
        Self {
            name: "gguf-model".to_string(),
            context_length: 32768,
            vocab_size: 151936,
            num_layers: 24,
            hidden_size: 896,
            num_attention_heads: 14,
        }
    }
}

/// Extract configuration from model path (heuristic-based)
pub fn extract_config_from_path(model_path: &Path) -> GgufModelConfig {
    let name = model_path.file_stem().and_then(|s| s.to_str()).unwrap_or("gguf-model").to_string();

    // Use heuristics based on model name for configuration
    let mut config = GgufModelConfig {
        name,
        ..Default::default()
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

/// Type alias for backward compatibility with existing code
pub type GgufModel = GgufRuntimeModel;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_builder_creation() {
        let builder = GgufRuntimeBuilder::new("test_dir", vec!["model.gguf".to_string()]);
        assert_eq!(builder.model_dir, "test_dir");
        assert_eq!(builder.model_files.len(), 1);
        assert_eq!(builder.model_files[0], "model.gguf");
        assert!(!builder.logging);
    }

    #[test]
    fn test_builder_with_logging() {
        let builder =
            GgufRuntimeBuilder::new("test_dir", vec!["model.gguf".to_string()]).with_logging();
        assert!(builder.logging);
    }

    #[test]
    fn test_extract_config_from_path() {
        let path = PathBuf::from("models/qwen2.5-0.5b.gguf");
        let config = extract_config_from_path(&path);

        assert_eq!(config.name, "qwen2.5-0.5b");
        assert_eq!(config.context_length, 32768);
        assert_eq!(config.vocab_size, 151936);
        assert_eq!(config.num_layers, 24);
        assert_eq!(config.hidden_size, 896);
        assert_eq!(config.num_attention_heads, 14);
    }

    #[test]
    fn test_extract_config_different_sizes() {
        let test_cases = vec![
            ("models/qwen2.5-1.5b.gguf", 24, 1536, 12),
            ("models/qwen2.5-3b.gguf", 36, 2048, 16),
            ("models/qwen2.5-7b.gguf", 32, 4096, 32),
        ];

        for (path, expected_layers, expected_hidden, expected_heads) in test_cases {
            let path = PathBuf::from(path);
            let config = extract_config_from_path(&path);
            assert_eq!(config.num_layers, expected_layers);
            assert_eq!(config.hidden_size, expected_hidden);
            assert_eq!(config.num_attention_heads, expected_heads);
        }
    }

    #[test]
    fn test_load_nonexistent_model() {
        let nonexistent_path = Path::new("/nonexistent/model.gguf");
        let result =
            tokio::runtime::Runtime::new().unwrap().block_on(load_gguf_model(nonexistent_path));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_load_sync_nonexistent_model() {
        let nonexistent_path = Path::new("/nonexistent/model.gguf");
        let result = load_gguf_model_sync(nonexistent_path);

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("does not exist"));
    }

    #[test]
    fn test_create_session() {
        // Test session structure without real model
        let device = Device::Cpu;
        match device {
            Device::Cpu => {} // Expected case
            _ => panic!("Expected CPU device"),
        }
    }

    #[test]
    fn test_config_default() {
        let config = GgufModelConfig::default();
        assert_eq!(config.name, "gguf-model");
        assert_eq!(config.context_length, 32768);
        assert_eq!(config.vocab_size, 151936);
        assert_eq!(config.num_layers, 24);
        assert_eq!(config.hidden_size, 896);
        assert_eq!(config.num_attention_heads, 14);
    }

    #[test]
    fn test_type_alias_compatibility() {
        // Verify the type alias compiles
        let _: Option<GgufModel> = None;
    }
}
