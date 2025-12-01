//! Fusion Router - Auto-selects optimal fusion mode
//!
//! Analyzes query characteristics and selects appropriate fusion mode:
//! - Mode A (Simple): Short queries, symbol lookups
//! - Mode B (Attention): Semantic queries, explanations
//! - Mode C (Reasoning): Causal tracing, multi-file reasoning
//!
//! Selection rules:
//! - IF query < 4 tokens → Mode A
//! - IF query is path or symbol → Mode A
//! - IF multi-sentence or "why/explain/trace" → Mode B
//! - IF multi-file or causal reasoning → Mode C
//! - IF ambiguous → Mode B

/// Fusion mode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FusionMode {
    /// Mode A: Simple linear weighted fusion
    Simple,
    /// Mode B: Attention-based dynamic fusion
    Attention,
    /// Mode C: Multi-hop reasoning fusion
    Reasoning,
}

/// Router for automatic fusion mode selection
pub struct FusionRouter;

impl FusionRouter {
    /// Create new fusion router
    pub fn new() -> Self {
        Self
    }

    /// Select optimal fusion mode based on query characteristics
    ///
    /// # Arguments
    /// * `query` - User query string
    ///
    /// # Returns
    /// Selected fusion mode
    pub fn select_mode(&self, query: &str) -> FusionMode {
        let query_lower = query.to_lowercase();
        let tokens: Vec<&str> = query.split_whitespace().collect();

        // Rule 1: Short queries → Simple
        if tokens.len() < 4 {
            return FusionMode::Simple;
        }

        // Rule 2: Path or symbol patterns → Simple
        if query.contains("::") || query.contains("->") || query.contains('/') {
            return FusionMode::Simple;
        }

        // Rule 3: Causal/trace keywords → Reasoning
        let reasoning_keywords = ["trace", "dependency", "from", "to", "path", "chain"];
        if reasoning_keywords.iter().filter(|kw| query_lower.contains(*kw)).count() >= 2 {
            return FusionMode::Reasoning;
        }

        // Rule 4: Semantic/explanation keywords → Attention
        let semantic_keywords = ["why", "explain", "how", "what", "understand", "describe"];
        if semantic_keywords.iter().any(|kw| query_lower.contains(*kw)) {
            return FusionMode::Attention;
        }

        // Rule 5: Multi-sentence → Attention
        if query.contains('.') || query.contains('?') {
            return FusionMode::Attention;
        }

        // Default: Attention (handles ambiguous cases)
        FusionMode::Attention
    }
}

impl Default for FusionRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_query_simple() {
        let router = FusionRouter::new();
        assert_eq!(router.select_mode("fmt"), FusionMode::Simple);
        assert_eq!(router.select_mode("test fn"), FusionMode::Simple);
    }

    #[test]
    fn test_path_query_simple() {
        let router = FusionRouter::new();
        assert_eq!(router.select_mode("std::fmt::Display"), FusionMode::Simple);
        assert_eq!(router.select_mode("src/main.rs"), FusionMode::Simple);
    }

    #[test]
    fn test_semantic_query_attention() {
        let router = FusionRouter::new();
        assert_eq!(router.select_mode("explain why function A fails"), FusionMode::Attention);
        assert_eq!(router.select_mode("how does this work"), FusionMode::Attention);
    }

    #[test]
    fn test_causal_query_reasoning() {
        let router = FusionRouter::new();
        assert_eq!(router.select_mode("trace dependency from A to D"), FusionMode::Reasoning);
        assert_eq!(router.select_mode("path from module A to module B"), FusionMode::Reasoning);
    }

    #[test]
    fn test_multi_sentence_attention() {
        let router = FusionRouter::new();
        assert_eq!(router.select_mode("Find the bug. Check the tests."), FusionMode::Attention);
    }
}
