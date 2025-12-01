//! Language Model abstraction layer for SynCore
//!
//! Provides a unified interface for LLM backends (Ollama, OpenAI, test, etc.)
//! Used by IntelliTask for AI-powered task breakdown and prioritization.

use anyhow::Result;

pub mod factory;
pub mod ollama;
pub mod test;

/// Prompt sent to a language model
#[derive(Debug, Clone)]
pub struct Prompt {
    /// System instructions that set the model's behavior
    pub system: String,
    /// User message or question
    pub user: String,
    /// Optional temperature (0.0-1.0, lower = more deterministic)
    pub temperature: Option<f32>,
    /// Optional max tokens to generate
    pub max_tokens: Option<u32>,
}

impl Prompt {
    /// Create a new prompt with system and user messages
    pub fn new(system: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            system: system.into(),
            user: user.into(),
            temperature: None,
            max_tokens: None,
        }
    }

    /// Set temperature for generation
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set max tokens for generation
    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }
}

/// Completion response from a language model
#[derive(Debug, Clone)]
pub struct Completion {
    /// Generated text response
    pub text: String,
    /// Optional metadata (model name, tokens used, etc.)
    pub metadata: Option<serde_json::Value>,
}

impl Completion {
    /// Create a simple completion with just text
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            metadata: None,
        }
    }

    /// Create a completion with metadata
    pub fn with_metadata(text: impl Into<String>, metadata: serde_json::Value) -> Self {
        Self {
            text: text.into(),
            metadata: Some(metadata),
        }
    }
}

/// Unified interface for language model backends
///
/// This trait abstracts over different LLM providers (Ollama CLI, OpenAI API, local models, etc.)
/// allowing IntelliTask and other AI-dependent features to work with any backend.
///
/// # Implementations
/// - `OllamaLanguageModel`: Ollama CLI via subprocess (production)
/// - `TestLanguageModel`: Deterministic responses for testing (no network)
pub trait LanguageModel: Send + Sync {
    /// Generate a completion from a prompt
    ///
    /// This is the core method all implementations must provide.
    /// Returns the model's generated text response.
    ///
    /// # Errors
    /// - Backend unreachable (network error, service down, etc.)
    /// - Invalid prompt (too long, malformed, etc.)
    /// - Timeout
    fn complete(&self, prompt: &Prompt) -> Result<Completion>;

    /// Optional: Health check for the backend
    ///
    /// Returns true if the backend is reachable and functional.
    /// Default implementation tries a simple completion.
    fn health_check(&self) -> Result<bool> {
        let test_prompt =
            Prompt::new("System: You are a test bot.", "Say 'OK' if you can read this.");
        self.complete(&test_prompt).map(|_| true)
    }

    /// Optional: Get backend name for diagnostics
    fn backend_name(&self) -> &str {
        "unknown"
    }
}
