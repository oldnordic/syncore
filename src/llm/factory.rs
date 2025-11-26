//! Factory for creating LanguageModel instances from configuration
//!
//! Supports multiple backends: ollama, test, and future extensions (openai, anthropic, etc.)

use super::{ollama::OllamaLanguageModel, test::TestLanguageModel, LanguageModel};
use crate::ollama::OllamaConfig;
use anyhow::{anyhow, Result};

/// Language model backend type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmBackend {
    /// Ollama CLI backend (production)
    Ollama,
    /// Test backend (deterministic, no network)
    Test,
}

impl LlmBackend {
    /// Parse backend from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "ollama" => Ok(Self::Ollama),
            "test" => Ok(Self::Test),
            _ => Err(anyhow!(
                "Unknown LLM backend '{}'. Supported: ollama, test",
                s
            )),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Ollama => "ollama",
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
            backend: LlmBackend::Ollama,
            model: "qwen2.5-coder:3b".to_string(),
            url: "http://localhost:11434".to_string(),
            timeout_seconds: 30,
        }
    }
}

impl LlmConfig {
    /// Create LLM config from central config file with environment variable overrides
    ///
    /// Priority (highest to lowest):
    /// 1. Environment variables (LLM_BACKEND, LLM_MODEL, LLM_URL, LLM_TIMEOUT)
    /// 2. Central config file (config/syncore.toml [llm] section)
    /// 3. Hardcoded defaults
    ///
    /// This allows users to set defaults in syncore.toml and override per-run with env vars.
    pub fn from_env() -> Self {
        // Try to load from global config first
        let config = crate::config::SyncoreConfig::try_global();

        let (default_backend, default_model, default_url, default_timeout) = match config {
            Some(cfg) => (
                cfg.llm.backend.clone(),
                cfg.llm.model.clone(),
                cfg.llm.url.clone(),
                cfg.llm.timeout_seconds,
            ),
            None => (
                "ollama".to_string(),
                "qwen2.5-coder:3b".to_string(),
                "http://localhost:11434".to_string(),
                30,
            ),
        };

        // Environment variables override config file
        let backend_str = std::env::var("LLM_BACKEND").unwrap_or(default_backend);
        let backend = LlmBackend::from_str(&backend_str).unwrap_or(LlmBackend::Ollama);

        Self {
            backend,
            model: std::env::var("LLM_MODEL").unwrap_or(default_model),
            url: std::env::var("LLM_URL").unwrap_or(default_url),
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
    /// A boxed LanguageModel trait object configured for the specified backend
    ///
    /// # Errors
    /// - Invalid backend name
    /// - Backend initialization failure (e.g., Ollama not installed)
    pub fn from_config(config: &LlmConfig) -> Result<Box<dyn LanguageModel>> {
        match config.backend {
            LlmBackend::Ollama => {
                let ollama_config = OllamaConfig {
                    model: config.model.clone(),
                    timeout_secs: config.timeout_seconds,
                    temperature: 0.0, // Deterministic for structured output
                    max_tokens: 2048,
                };

                let model = OllamaLanguageModel::new(ollama_config).map_err(|e| {
                    anyhow!(
                        "Failed to initialize Ollama backend: {}. Is ollama installed and running?",
                        e
                    )
                })?;

                Ok(Box::new(model))
            }
            LlmBackend::Test => {
                // Test backend with predefined response
                let model = TestLanguageModel::predefined(
                    r#"{"success": true, "message": "Test backend response"}"#,
                );
                Ok(Box::new(model))
            }
        }
    }

    /// Create a LanguageModel from environment variables
    ///
    /// This is a convenience method that reads LLM_BACKEND, LLM_MODEL, etc.
    /// from environment and creates the appropriate backend.
    pub fn from_env() -> Result<Box<dyn LanguageModel>> {
        let config = LlmConfig::from_env();
        Self::from_config(&config)
    }

    /// Create a test backend for use in tests
    ///
    /// This is a convenience method that always returns TestLanguageModel,
    /// regardless of environment configuration.
    pub fn test_backend() -> Box<dyn LanguageModel> {
        Box::new(TestLanguageModel::echo())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Prompt;

    #[test]
    fn test_backend_from_str() {
        assert_eq!(LlmBackend::from_str("ollama").unwrap(), LlmBackend::Ollama);
        assert_eq!(LlmBackend::from_str("Ollama").unwrap(), LlmBackend::Ollama);
        assert_eq!(LlmBackend::from_str("TEST").unwrap(), LlmBackend::Test);
        assert!(LlmBackend::from_str("invalid").is_err());
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.backend, LlmBackend::Ollama);
        assert_eq!(config.model, "qwen2.5-coder:3b");
        assert_eq!(config.timeout_seconds, 30);
    }

    #[test]
    fn test_factory_creates_test_backend() {
        let config = LlmConfig {
            backend: LlmBackend::Test,
            model: "test-model".to_string(),
            url: "".to_string(),
            timeout_seconds: 10,
        };

        let model = LlmFactory::from_config(&config).unwrap();
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

    #[test]
    #[ignore] // Requires ollama installed
    fn test_factory_creates_ollama_backend() {
        let config = LlmConfig {
            backend: LlmBackend::Ollama,
            model: "qwen2.5-coder:3b".to_string(),
            url: "http://localhost:11434".to_string(),
            timeout_seconds: 30,
        };

        // This will fail if ollama is not installed, which is expected
        let result = LlmFactory::from_config(&config);
        if let Ok(model) = result {
            assert_eq!(model.backend_name(), "ollama_cli");
        }
    }
}
