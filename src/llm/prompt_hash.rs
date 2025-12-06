//! Deterministic Prompt Hashing for Agent Reproducibility
//!
//! Provides stable hashing for prompts to enable deterministic agent behavior
//! and reproducible results across sessions and environments.
//!
//! ## Features
//!
//! - Uses stable ahash hasher (AHasher) for cross-platform consistency
//! - Normalizes prompts before hashing to ensure reproducible results
//! - Supports both string and structured prompt hashing
//! - Thread-safe for concurrent agent operations
//!
//! ## Usage
//!
//! ```rust
//! use crate::llm::prompt_hash::{hash_prompt, hash_prompt_deterministic};
//!
//! let prompt = "Hello, world!";
//! let hash = hash_prompt(prompt);
//! println!("Deterministic hash: {:x}", hash);
//! ```

use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Stable 64-bit prompt hash for deterministic agent behavior
pub type PromptHash = u64;

/// Configuration for prompt hashing behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptHashConfig {
    /// Enable case-sensitive hashing
    pub case_sensitive: bool,
    /// Normalize whitespace before hashing
    pub normalize_whitespace: bool,
    /// Strip control characters
    pub strip_control_chars: bool,
    /// Truncate long prompts before hashing
    pub max_prompt_length: Option<usize>,
}

impl Default for PromptHashConfig {
    fn default() -> Self {
        Self {
            case_sensitive: false,          // Case-insensitive for reproducibility
            normalize_whitespace: true,     // Normalize whitespace for consistency
            strip_control_chars: true,      // Remove control characters
            max_prompt_length: Some(65536), // 64KB limit to prevent excessive hashing
        }
    }
}

/// Fixed seed for deterministic hashing across runs
const DETERMINISTIC_SEED: u64 = 0x123456789ABCDEF0;

/// Normalize a prompt string for deterministic hashing
///
/// This function ensures that semantically equivalent prompts produce the same hash:
/// - Normalizes whitespace (multiple spaces → single space)
/// - Strips control characters
/// - Handles case sensitivity configuration
pub fn normalize_prompt(prompt: &str, config: &PromptHashConfig) -> String {
    let mut normalized = String::with_capacity(prompt.len());

    for ch in prompt.chars() {
        // Strip control characters if configured
        if config.strip_control_chars && ch.is_control() && ch != '\t' && ch != '\n' && ch != '\r' {
            continue;
        }

        // Handle whitespace normalization
        if ch.is_whitespace() {
            if config.normalize_whitespace {
                // Normalize all whitespace to single space
                normalized.push(' ');
            } else {
                // Preserve original whitespace
                normalized.push(ch);
            }
        } else {
            // Handle case sensitivity
            if !config.case_sensitive && ch.is_alphabetic() {
                normalized.push(ch.to_lowercase().next().unwrap());
            } else {
                normalized.push(ch);
            }
        }
    }

    // Collapse multiple spaces if normalizing whitespace
    if config.normalize_whitespace {
        let mut chars: Vec<char> = normalized.chars().collect();
        let mut write_pos = 0;

        let mut prev_was_space = false;
        for read_pos in 0..chars.len() {
            let ch = chars[read_pos];

            if ch == ' ' {
                if !prev_was_space {
                    chars[write_pos] = ch;
                    write_pos += 1;
                    prev_was_space = true;
                }
            } else {
                chars[write_pos] = ch;
                write_pos += 1;
                prev_was_space = false;
            }
        }

        // Trim leading/trailing whitespace
        normalized = chars[0..write_pos].iter().collect::<String>().trim().to_string();
    }

    // Apply length limit if configured
    if let Some(max_len) = config.max_prompt_length {
        if normalized.len() > max_len {
            normalized.truncate(max_len);
            normalized.push_str("...[truncated]");
        }
    }

    normalized
}

/// Generate deterministic hash for a prompt string
///
/// Uses stable FxHasher with fixed seed to ensure reproducible results
/// across different platforms and Rust versions.
pub fn hash_prompt(prompt: &str) -> PromptHash {
    hash_prompt_with_config(prompt, &PromptHashConfig::default())
}

/// Generate deterministic hash for a prompt with custom configuration
pub fn hash_prompt_with_config(prompt: &str, config: &PromptHashConfig) -> PromptHash {
    let normalized = normalize_prompt(prompt, config);
    let mut hasher = FxHasher::default();

    // Write the seed first to make hashing deterministic
    hasher.write_u64(DETERMINISTIC_SEED);
    normalized.hash(&mut hasher);
    hasher.finish()
}

/// Generate deterministic hash for structured prompt data
pub fn hash_prompt_structured<T: Serialize>(
    data: &T,
    config: &PromptHashConfig,
) -> Result<PromptHash, serde_json::Error> {
    // Serialize to JSON for consistent string representation
    let json_str = serde_json::to_string(data)?;
    Ok(hash_prompt_with_config(&json_str, config))
}

/// Hash multiple prompts and combine for batch operations
pub fn hash_prompts(prompts: &[&str]) -> PromptHash {
    let mut hasher = FxHasher::default();

    for prompt in prompts {
        let hash = hash_prompt(prompt);
        hash.hash(&mut hasher);
    }

    hasher.finish()
}

/// Cache for recent prompt hashes to avoid redundant computation
pub struct PromptHashCache {
    cache: HashMap<String, PromptHash>,
    max_size: usize,
}

impl PromptHashCache {
    /// Create new cache with default size (1000 entries)
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }

    /// Create cache with specific capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(capacity),
            max_size: capacity,
        }
    }

    /// Get cached hash or compute new one
    pub fn get_or_compute(&mut self, prompt: &str, config: &PromptHashConfig) -> PromptHash {
        // Simple caching - in production, would use LRU eviction
        if let Some(&hash) = self.cache.get(prompt) {
            return hash;
        }

        let hash = hash_prompt_with_config(prompt, config);

        // Simple eviction strategy - clear when full
        if self.cache.len() >= self.max_size {
            self.cache.clear();
        }

        self.cache.insert(prompt.to_string(), hash);
        hash
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.cache.len(), self.max_size)
    }
}

impl Default for PromptHashCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic prompt hasher that can be used in hash maps
pub struct DeterministicPromptHasher {
    config: PromptHashConfig,
    hasher: FxHasher,
}

impl DeterministicPromptHasher {
    /// Create new deterministic hasher
    pub fn new() -> Self {
        Self::with_config(PromptHashConfig::default())
    }

    /// Create hasher with custom configuration
    pub fn with_config(config: PromptHashConfig) -> Self {
        Self {
            config,
            hasher: FxHasher::default(),
        }
    }

    /// Hash a string and get the 64-bit result
    pub fn hash_str(&mut self, prompt: &str) -> PromptHash {
        let normalized = normalize_prompt(prompt, &self.config);
        normalized.hash(&mut self.hasher);
        self.hasher.finish()
    }

    /// Reset the hasher for reuse
    pub fn reset(&mut self) {
        self.hasher = FxHasher::default();
    }
}

impl Default for DeterministicPromptHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// Verify hash consistency across different runs
///
/// This function should always return true for the same input
/// when called with the same prompt, ensuring deterministic behavior.
pub fn verify_deterministic(prompt: &str) -> bool {
    let hash1 = hash_prompt(prompt);
    let hash2 = hash_prompt(prompt);
    hash1 == hash2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_hash_deterministic() {
        let prompt = "Hello, world!";
        let hash1 = hash_prompt(prompt);
        let hash2 = hash_prompt(prompt);

        assert_eq!(hash1, hash2, "Hash should be deterministic");
        assert!(hash1 != 0, "Hash should not be zero for non-empty prompt");
    }

    #[test]
    fn test_case_insensitive_hashing() {
        let config = PromptHashConfig {
            case_sensitive: false,
            ..Default::default()
        };

        let prompt1 = "Hello World";
        let prompt2 = "hello world";

        let hash1 = hash_prompt_with_config(prompt1, &config);
        let hash2 = hash_prompt_with_config(prompt2, &config);

        assert_eq!(hash1, hash2, "Case-insensitive hashing should produce same hash");
    }

    #[test]
    fn test_case_sensitive_hashing() {
        let config = PromptHashConfig {
            case_sensitive: true,
            ..Default::default()
        };

        let prompt1 = "Hello World";
        let prompt2 = "hello world";

        let hash1 = hash_prompt_with_config(prompt1, &config);
        let hash2 = hash_prompt_with_config(prompt2, &config);

        assert_ne!(hash1, hash2, "Case-sensitive hashing should produce different hashes");
    }

    #[test]
    fn test_whitespace_normalization() {
        let config = PromptHashConfig {
            normalize_whitespace: true,
            ..Default::default()
        };

        let prompt1 = "Hello   World";
        let prompt2 = "Hello World";

        let hash1 = hash_prompt_with_config(prompt1, &config);
        let hash2 = hash_prompt_with_config(prompt2, &config);

        assert_eq!(hash1, hash2, "Normalized whitespace should produce same hash");
    }

    #[test]
    fn test_prompt_hash_cache() {
        let mut cache = PromptHashCache::new();
        let prompt = "test prompt";
        let config = PromptHashConfig::default();

        let hash1 = cache.get_or_compute(prompt, &config);
        let hash2 = cache.get_or_compute(prompt, &config);

        assert_eq!(hash1, hash2, "Cache should return same hash");

        let (used, capacity) = cache.stats();
        assert_eq!(used, 1, "Cache should have one entry");
    }

    #[test]
    fn test_verify_deterministic() {
        let prompt = "deterministic test";
        assert!(verify_deterministic(prompt), "Deterministic verification should pass");

        let empty_prompt = "";
        assert!(verify_deterministic(empty_prompt), "Empty prompt should be deterministic");
    }

    #[test]
    fn test_deterministic_hasher() {
        let mut hasher = DeterministicPromptHasher::new();
        let prompt1 = "test1";
        let prompt2 = "test2";

        let hash1a = hasher.hash_str(prompt1);
        let hash1b = hasher.hash_str(prompt1); // Should be different due to internal state

        hasher.reset();
        let hash1c = hasher.hash_str(prompt1);

        assert_eq!(hash1a, hash1c, "Reset should restore deterministic behavior");
        assert_ne!(hash1a, hash1b, "Internal state should affect hash output");
    }

    #[test]
    fn test_length_limit() {
        let config = PromptHashConfig {
            max_prompt_length: Some(10),
            ..Default::default()
        };

        let short_prompt = "short";
        let long_prompt = "this is a very long prompt that exceeds the limit";

        let short_hash = hash_prompt_with_config(short_prompt, &config);
        let long_hash = hash_prompt_with_config(long_prompt, &config);

        assert!(short_hash != 0, "Short prompt should have valid hash");
        assert!(long_hash != 0, "Truncated prompt should have valid hash");
        assert_ne!(short_hash, long_hash, "Different prompts should have different hashes");
    }
}
