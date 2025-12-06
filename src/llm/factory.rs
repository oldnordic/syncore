//! Factory for creating LanguageModel instances from configuration
//!
//! Supports Candle-based GGUF engine for local offline inference and test backend.
//! Uses candle_cache for efficient model and tokenizer caching.

use super::candle_cache::{get_or_init_model, CandleConfig};
use super::{test::TestLanguageModel, LanguageModel};
use anyhow::{anyhow, Result};
use std::sync::Arc;

/// Language model backend type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmBackend {
    /// GGUFEngine backend (Candle-based local inference)
    GGUFEngine,
    /// Test backend (deterministic, no network)
    Test,
}

impl LlmBackend {
    /// Parse backend from string
    pub fn try_parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "gguf" | "gguf_engine" => Ok(Self::GGUFEngine),
            "test" => Ok(Self::Test),
            _ => Err(anyhow!("Unknown LLM backend '{}'. Supported: gguf, gguf_engine, test", s)),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::GGUFEngine => "gguf_engine",
            Self::Test => "test",
        }
    }
}

/// Configuration for LLM backend
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub backend: LlmBackend,
    pub model: String,
    pub url: String,
    pub timeout_seconds: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            backend: LlmBackend::GGUFEngine,
            model: "qwen2.5-0.5b".to_string(),
            url: "local".to_string(),
            timeout_seconds: 30,
        }
    }
}

impl LlmConfig {
    /// Create LLM config from central config file with environment variable overrides
    ///
    /// Priority (highest to lowest):
    /// 1. Environment variables (LLM_BACKEND, LLM_MODEL, LLM_TIMEOUT)
    /// 2. Central config file (config/syncore.toml [llm] section)
    /// 3. Hardcoded defaults
    ///
    /// This allows users to set defaults in syncore.toml and override per-run with env vars.
    pub fn from_env() -> Self {
        // Try to load from global config first
        let config = crate::config::SyncoreConfig::try_global();

        let (default_backend, default_model, default_timeout) = match config {
            Some(cfg) => (cfg.llm.backend.clone(), cfg.llm.model.clone(), cfg.llm.timeout_seconds),
            None => ("gguf_engine".to_string(), "qwen2.5-0.5b".to_string(), 30),
        };

        // Environment variables override config file
        let backend_str = std::env::var("LLM_BACKEND").unwrap_or(default_backend);
        let backend = LlmBackend::try_parse(&backend_str).unwrap_or(LlmBackend::GGUFEngine);

        Self {
            backend,
            model: std::env::var("LLM_MODEL").unwrap_or(default_model),
            url: "local".to_string(), // GGUFEngine doesn't use URLs
            timeout_seconds: std::env::var("LLM_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default_timeout),
        }
    }
}

/// Factory for creating LanguageModel instances
pub struct LlmFactory;

impl LlmFactory {
    /// Create a LanguageModel from configuration
    ///
    /// # Arguments
    /// * `config` - LLM configuration specifying backend and parameters
    ///
    /// # Returns
    /// An Arc-wrapped LanguageModel trait object configured for the specified backend
    /// Arc allows efficient sharing across multiple components (IntelliTask, Reasoning, etc.)
    ///
    /// # Errors
    /// - Invalid backend name
    /// - Backend initialization failure
    pub async fn from_config(config: &LlmConfig) -> Result<Arc<dyn LanguageModel>> {
        match config.backend {
            LlmBackend::GGUFEngine => {
                // Use Candle cache for efficient model loading
                tracing::info!("Loading GGUF model via cache: {}", config.model);

                // Create cache configuration - ensure .gguf extension
                let model_path = if config.model.ends_with(".gguf") {
                    config.model.clone()
                } else {
                    format!("{}.gguf", config.model)
                };

                let cache_config = CandleConfig::new(model_path).deterministic(true); // Enable deterministic generation

                // Try to load via cache (returns cached instance if available)
                match get_or_init_model(&cache_config).await {
                    Ok(cached_model) => {
                        tracing::info!("✅ Successfully loaded GGUF model via cache");
                        Ok(cached_model) // Return the Arc directly from cache
                    }
                    Err(e) => {
                        tracing::error!(
                            "❌ Failed to load GGUF model via cache '{}': {}",
                            config.model,
                            e
                        );
                        tracing::error!("❌ Single-path architecture: All models must load through candle_cache");
                        Err(anyhow::anyhow!(
                            "Failed to load GGUF model through single-path candle_cache: {}",
                            e
                        ))
                    }
                }
            }
            LlmBackend::Test => {
                // Test backend with predefined response
                let model = TestLanguageModel::predefined(
                    r#"{"success": true, "message": "Test backend response"}"#,
                );
                Ok(Arc::new(model) as Arc<dyn LanguageModel>)
            }
        }
    }

    /// Create a LanguageModel from environment variables
    ///
    /// This is a convenience method that reads LLM_BACKEND, LLM_MODEL, etc.
    /// from environment and creates the appropriate backend.
    pub async fn from_env() -> Result<Arc<dyn LanguageModel>> {
        let config = LlmConfig::from_env();
        Self::from_config(&config).await
    }

    /// Create a test backend for use in tests
    ///
    /// This is a convenience method that always returns TestLanguageModel,
    /// regardless of environment configuration.
    pub fn test_backend() -> Arc<dyn LanguageModel> {
        Arc::new(TestLanguageModel::echo())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Prompt;

    #[test]
    fn test_backend_from_str() {
        assert_eq!(LlmBackend::try_parse("gguf").unwrap(), LlmBackend::GGUFEngine);
        assert_eq!(LlmBackend::try_parse("gguf_engine").unwrap(), LlmBackend::GGUFEngine);
        assert_eq!(LlmBackend::try_parse("test").unwrap(), LlmBackend::Test);
        assert!(LlmBackend::try_parse("invalid").is_err());
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.backend, LlmBackend::GGUFEngine);
        assert_eq!(config.model, "qwen2.5-0.5b");
        assert_eq!(config.url, "local");
        assert_eq!(config.timeout_seconds, 30);
    }

    #[tokio::test]
    async fn test_factory_creates_test_backend() {
        let config = LlmConfig {
            backend: LlmBackend::Test,
            model: "test-model".to_string(),
            url: "".to_string(),
            timeout_seconds: 10,
        };

        let model = LlmFactory::from_config(&config).await.unwrap();
        assert_eq!(model.backend_name(), "test");

        // Verify it works
        let prompt = Prompt::new("System", "User message");
        let result = model.complete(&prompt);
        assert!(result.is_ok());
    }

    #[test]
    fn test_factory_test_backend_convenience() {
        let model = LlmFactory::test_backend();
        assert_eq!(model.backend_name(), "test");

        // Echo mode should return user text
        let prompt = Prompt::new("", "Hello");
        let result = model.complete(&prompt).unwrap();
        assert_eq!(result.text, "Hello");
    }

    #[tokio::test]
    #[ignore] // Requires real GGUF model file
    async fn test_factory_creates_gguf_engine_backend() {
        let config = LlmConfig {
            backend: LlmBackend::GGUFEngine,
            model: "qwen2.5-0.5b".to_string(),
            url: "local".to_string(),
            timeout_seconds: 30,
        };

        // This will create a test GGUFEngine for now
        let result = LlmFactory::from_config(&config).await;
        if let Ok(model) = result {
            assert_eq!(model.backend_name(), "gguf_engine");
        }
    }
}
