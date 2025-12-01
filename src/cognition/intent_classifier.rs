//! Intent Classification Module
//!
//! Analyzes user queries and classifies them into intent categories:
//! - Symbolic: Short, code-symbol-like queries (e.g., "format_string", "MyClass")
//! - Semantic: Explanatory queries (e.g., "why does X fail", "explain Y")
//! - Causal: Dependency/flow queries (e.g., "trace from A to B", "how does X affect Y")
//! - Unknown: Queries that don't fit above categories
//!
//! This module uses pure pattern matching and heuristics - no LLM calls.

use serde::{Deserialize, Serialize};

/// Query intent classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryIntent {
    /// Short, symbol-like query (function name, class name, etc.)
    Symbolic,
    /// Explanatory query asking "why", "how", "what", "explain"
    Semantic,
    /// Dependency/flow query with "trace", "dependency", "flow", "call chain"
    Causal,
    /// Unrecognized query type
    Unknown,
}

/// Classify user query intent based on linguistic patterns
///
/// # Arguments
/// * `query` - The user's input query text
///
/// # Returns
/// QueryIntent classification
///
/// # Examples
/// ```
/// use syncore::cognition::intent_classifier::{classify_intent, QueryIntent};
///
/// assert_eq!(classify_intent("format_string"), QueryIntent::Symbolic);
/// assert_eq!(classify_intent("explain why parse fails"), QueryIntent::Semantic);
/// assert_eq!(classify_intent("trace dependency from A to B"), QueryIntent::Causal);
/// ```
pub fn classify_intent(query: &str) -> QueryIntent {
    let query_lower = query.to_lowercase();
    let words: Vec<&str> = query_lower.split_whitespace().collect();
    let word_count = words.len();

    // Symbolic: Short queries (1-3 words) without question words
    // Likely code symbols: "format_string", "MyClass", "parse_config"
    if word_count <= 3 && !contains_question_words(&query_lower) {
        // Check if it looks like a code identifier
        if looks_like_code_symbol(query) {
            return QueryIntent::Symbolic;
        }
    }

    // Causal: Contains dependency/flow keywords
    if contains_causal_keywords(&query_lower) {
        return QueryIntent::Causal;
    }

    // Semantic: Contains explanatory keywords
    if contains_semantic_keywords(&query_lower) {
        return QueryIntent::Semantic;
    }

    // Default: Unknown
    QueryIntent::Unknown
}

/// Check if query contains question words (why, how, what, etc.)
fn contains_question_words(query: &str) -> bool {
    let question_words = ["why", "how", "what", "where", "when", "explain"];
    question_words.iter().any(|&word| query.contains(word))
}

/// Check if query looks like a code symbol (function, class, variable)
fn looks_like_code_symbol(query: &str) -> bool {
    // Code symbols typically:
    // - Use snake_case or camelCase
    // - Don't contain spaces
    // - May contain dots (for module paths)
    // - May contain :: (for Rust paths)

    let trimmed = query.trim();

    // Must not be empty
    if trimmed.is_empty() {
        return false;
    }

    // Check for typical code patterns
    let has_underscore = trimmed.contains('_');
    let has_camel =
        trimmed.chars().any(|c| c.is_uppercase()) && trimmed.chars().any(|c| c.is_lowercase());
    let has_separator = trimmed.contains("::") || trimmed.contains('.');
    let no_spaces = !trimmed.contains(' ');

    // At least one code-like characteristic and no spaces
    no_spaces && (has_underscore || has_camel || has_separator)
}

/// Check if query contains causal/dependency keywords
fn contains_causal_keywords(query: &str) -> bool {
    let causal_keywords = [
        "trace",
        "dependency",
        "dependencies",
        "flow",
        "call chain",
        "calling",
        "leads to",
        "affects",
        "impact",
        "propagate",
        "ripple",
        "from",
        "to",
    ];

    // Check for multi-word patterns first
    if query.contains("call chain") || query.contains("leads to") {
        return true;
    }

    // Then check individual keywords
    causal_keywords
        .iter()
        .filter(|&&kw| kw.len() < 10) // Skip multi-word already checked
        .any(|&kw| query.split_whitespace().any(|word| word == kw))
}

/// Check if query contains semantic/explanatory keywords
fn contains_semantic_keywords(query: &str) -> bool {
    let semantic_keywords = [
        "explain",
        "why",
        "how",
        "what",
        "describe",
        "tell me",
        "understand",
        "meaning",
        "purpose",
        "reason",
        "does",
        "work",
    ];

    semantic_keywords.iter().any(|&kw| query.split_whitespace().any(|word| word == kw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbolic_single_word() {
        assert_eq!(classify_intent("format_string"), QueryIntent::Symbolic);
        assert_eq!(classify_intent("MyClass"), QueryIntent::Symbolic);
        assert_eq!(classify_intent("parse_config"), QueryIntent::Symbolic);
    }

    #[test]
    fn test_symbolic_with_path() {
        assert_eq!(classify_intent("std::fmt::Display"), QueryIntent::Symbolic);
        assert_eq!(classify_intent("module.function"), QueryIntent::Symbolic);
    }

    #[test]
    fn test_semantic_why() {
        assert_eq!(classify_intent("why does parse function fail"), QueryIntent::Semantic);
        assert_eq!(classify_intent("explain how this works"), QueryIntent::Semantic);
    }

    #[test]
    fn test_causal_trace() {
        assert_eq!(classify_intent("trace dependency from A to B"), QueryIntent::Causal);
        assert_eq!(classify_intent("show call chain for format"), QueryIntent::Causal);
    }

    #[test]
    fn test_unknown() {
        assert_eq!(classify_intent("hello world"), QueryIntent::Unknown);
        assert_eq!(classify_intent(""), QueryIntent::Unknown);
    }
}
