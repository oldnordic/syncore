//! Tokenizer for GGUFEngine
//!
//! Provides text encoding/decoding functionality for Qwen models.
//! Uses tokenizers crate for proper tokenization with fallback to basic encoding.

use anyhow::Result;
use std::path::{Path, PathBuf};

/// GGUFEngine-compatible tokenizer wrapper for Qwen models
pub struct GgufTokenizer {
    /// Tokenizer implementation
    tokenizer_impl: TokenizerImpl,
    /// Vocabulary size
    vocab_size: usize,
}

/// Wrapper type for tokenizer compatibility with cache system
pub type TokenizerWrapper = GgufTokenizer;

/// Internal tokenizer implementation
enum TokenizerImpl {
    /// HuggingFace tokenizer (preferred)
    HfTokenizer(Box<tokenizers::Tokenizer>),
    /// Basic fallback tokenizer
    Basic,
}

impl GgufTokenizer {
    /// Create a new tokenizer instance for Qwen2.5 models
    pub fn new() -> Result<Self> {
        Self::new_with_path(None)
    }

    /// Create a new tokenizer instance with optional path
    pub fn new_with_path(tokenizer_path: Option<PathBuf>) -> Result<Self> {
        // Try to load from provided path first, then config, then fallback to basic
        if let Some(path) = tokenizer_path {
            match Self::from_file(&path) {
                Ok(tokenizer) => return Ok(tokenizer),
                Err(e) => {
                    tracing::warn!("Failed to load tokenizer from provided path: {}", e);
                }
            }
        }

        match Self::load_from_config() {
            Ok(tokenizer) => {
                let vocab_size = tokenizer.get_vocab_size(false);
                tracing::info!("Loaded Qwen2.5 tokenizer from config path");
                Ok(Self {
                    tokenizer_impl: TokenizerImpl::HfTokenizer(Box::new(tokenizer)),
                    vocab_size,
                })
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load tokenizer from config: {}, using basic tokenizer",
                    e
                );
                Ok(Self {
                    tokenizer_impl: TokenizerImpl::Basic,
                    vocab_size: 151936, // Qwen2.5 vocab size
                })
            }
        }
    }

    /// Load tokenizer from configuration path
    fn load_from_config() -> Result<tokenizers::Tokenizer> {
        // Try to get global config
        if let Some(config) = crate::config::SyncoreConfig::try_global() {
            let tokenizer_path = &config.llm.tokenizer_path;

            // Check if tokenizer file exists
            if std::path::Path::new(tokenizer_path).exists() {
                let tokenizer = Self::from_file(std::path::Path::new(tokenizer_path))?;
                match tokenizer.tokenizer_impl {
                    TokenizerImpl::HfTokenizer(ht) => return Ok(*ht),
                    TokenizerImpl::Basic => {
                        return Err(anyhow::anyhow!(
                            "Basic tokenizer not supported for external path"
                        ));
                    }
                }
            } else {
                tracing::debug!("Tokenizer file not found at: {}", tokenizer_path);
            }
        }

        // Fallback to basic tokenizer creation
        Err(anyhow::anyhow!("No valid tokenizer configuration found"))
    }

    /// Load tokenizer from local file (if available)
    pub fn from_file(tokenizer_path: &Path) -> Result<Self> {
        if !tokenizer_path.exists() {
            return Err(anyhow::anyhow!("Tokenizer file not found: {:?}", tokenizer_path));
        }

        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer from file: {}", e))?;

        let vocab_size = tokenizer.get_vocab_size(false);

        tracing::info!("Loaded tokenizer from file: {:?}", tokenizer_path);
        Ok(Self {
            tokenizer_impl: TokenizerImpl::HfTokenizer(Box::new(tokenizer)),
            vocab_size,
        })
    }

    /// Encode text to token IDs
    pub fn encode(&self, text: &str) -> Result<Vec<u32>> {
        match &self.tokenizer_impl {
            TokenizerImpl::HfTokenizer(tokenizer) => {
                let encoding = tokenizer
                    .encode(text, false)
                    .map_err(|e| anyhow::anyhow!("Failed to encode text: {}", e))?;

                let ids: Vec<u32> = encoding.get_ids().to_vec();

                Ok(ids)
            }
            TokenizerImpl::Basic => {
                // Basic UTF-8 byte encoding as fallback
                let tokens: Vec<u32> = text.bytes().map(|b| b as u32).collect();
                Ok(tokens)
            }
        }
    }

    /// Decode token IDs back to text
    pub fn decode(&self, tokens: &[u32]) -> Result<String> {
        match &self.tokenizer_impl {
            TokenizerImpl::HfTokenizer(tokenizer) => {
                let ids: Vec<u32> = tokens.to_vec();

                let text = tokenizer
                    .decode(&ids, true)
                    .map_err(|e| anyhow::anyhow!("Failed to decode tokens: {}", e))?;

                Ok(text)
            }
            TokenizerImpl::Basic => {
                // Basic UTF-8 byte decoding as fallback
                let bytes: Vec<u8> = tokens
                    .iter()
                    .filter_map(|&t| {
                        if t <= 255 {
                            Some(t as u8)
                        } else {
                            None // Skip invalid bytes
                        }
                    })
                    .collect();

                Ok(String::from_utf8_lossy(&bytes).to_string())
            }
        }
    }

    /// Get vocabulary size
    pub fn vocab_size(&self) -> usize {
        self.vocab_size
    }

    /// Get special token IDs if available
    pub fn get_special_tokens(&self) -> SpecialTokens {
        match &self.tokenizer_impl {
            TokenizerImpl::HfTokenizer(tokenizer) => SpecialTokens {
                pad_id: tokenizer.token_to_id("<pad>"),
                unk_id: tokenizer.token_to_id("<unk>"),
                bos_id: tokenizer.token_to_id("<s>"),
                eos_id: tokenizer.token_to_id("</s>"),
            },
            TokenizerImpl::Basic => SpecialTokens {
                pad_id: Some(0),
                unk_id: Some(3),
                bos_id: Some(1),
                eos_id: Some(2),
            },
        }
    }
}

/// Special token IDs for tokenizer
#[derive(Debug, Clone)]
pub struct SpecialTokens {
    pub pad_id: Option<u32>,
    pub unk_id: Option<u32>,
    pub bos_id: Option<u32>,
    pub eos_id: Option<u32>,
}

impl Default for GgufTokenizer {
    fn default() -> Self {
        // Try to create a real tokenizer, fall back to basic if needed
        Self::new().unwrap_or_else(|_| Self {
            tokenizer_impl: TokenizerImpl::Basic,
            vocab_size: 151936,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_creation() {
        let tokenizer = GgufTokenizer::new();
        assert!(tokenizer.is_ok());
    }

    #[test]
    fn test_tokenizer_default() {
        let tokenizer = GgufTokenizer::default();
        assert!(tokenizer.vocab_size() > 0);
    }

    #[test]
    fn test_encode_decode() {
        let tokenizer = GgufTokenizer::default();

        let text = "Hello";
        let result = tokenizer.encode(text);

        // Should succeed even with simplified tokenizer
        if let Ok(tokens) = result {
            assert!(!tokens.is_empty());

            // Try to decode back
            let decoded = tokenizer.decode(&tokens);
            assert!(decoded.is_ok());
        }
    }

    #[test]
    fn test_empty_text() {
        let tokenizer = GgufTokenizer::default();

        let result = tokenizer.encode("");
        assert!(result.is_ok());

        let tokens = result.unwrap();
        // Empty text should produce empty tokens
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_long_text() {
        let tokenizer = GgufTokenizer::default();

        let long_text = "x".repeat(1000);
        let result = tokenizer.encode(&long_text);
        assert!(result.is_ok());

        let tokens = result.unwrap();
        assert!(!tokens.is_empty());

        // Should be able to decode back
        let decoded = tokenizer.decode(&tokens);
        assert!(decoded.is_ok());
    }
}
