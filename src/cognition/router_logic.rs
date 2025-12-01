//! Router Logic Module
//!
//! Determines RAGGraph invocation strategy based on intent classification:
//! - Decides whether to call RAGGraph
//! - Selects fusion mode (simple/attention/reasoning)
//! - Suggests top_k parameter
//! - Provides expected information depth
//!
//! Integrates with tri-mode fusion logic from R2.4/R2.5.

use super::intent_classifier::QueryIntent;
use serde::{Deserialize, Serialize};

/// Routing decision for query processing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Whether to invoke RAGGraph before LLM call
    pub should_call_raggraph: bool,
    /// Suggested fusion mode: "simple", "attention", or "reasoning"
    pub mode_hint: Option<String>,
    /// Suggested top_k for vector/graph search
    pub top_k: Option<u32>,
    /// Expected information depth (0.0 = shallow, 1.0 = deep)
    pub depth: f32,
    /// Reasoning about why this decision was made
    pub reasoning: String,
}

/// Route a query based on its intent classification
///
/// # Arguments
/// * `intent` - The classified query intent
/// * `query` - Original query text (for additional context)
///
/// # Returns
/// RoutingDecision with RAGGraph invocation strategy
///
/// # Examples
/// ```
/// use syncore::cognition::intent_classifier::QueryIntent;
/// use syncore::cognition::router_logic::route_query;
///
/// let decision = route_query(&QueryIntent::Symbolic, "format_string");
/// assert!(decision.should_call_raggraph);
/// assert_eq!(decision.mode_hint, Some("simple".to_string()));
/// ```
pub fn route_query(intent: &QueryIntent, query: &str) -> RoutingDecision {
    match intent {
        QueryIntent::Symbolic => {
            // Symbolic queries: Fast, focused lookup
            // Use simple fusion mode with low top_k
            RoutingDecision {
                should_call_raggraph: true,
                mode_hint: Some("simple".to_string()),
                top_k: Some(5),
                depth: 0.3,
                reasoning: format!(
                    "Symbolic query '{}' requires focused code lookup. Using simple fusion mode.",
                    query
                ),
            }
        }

        QueryIntent::Semantic => {
            // Semantic queries: Explanatory, need context
            // Use attention fusion mode with moderate top_k
            RoutingDecision {
                should_call_raggraph: true,
                mode_hint: Some("attention".to_string()),
                top_k: Some(10),
                depth: 0.6,
                reasoning: format!(
                    "Semantic query '{}' needs contextual understanding. Using attention fusion mode.",
                    query
                ),
            }
        }

        QueryIntent::Causal => {
            // Causal queries: Dependency tracing, flow analysis
            // Use reasoning fusion mode with higher top_k
            RoutingDecision {
                should_call_raggraph: true,
                mode_hint: Some("reasoning".to_string()),
                top_k: Some(15),
                depth: 0.9,
                reasoning: format!(
                    "Causal query '{}' requires dependency analysis. Using reasoning fusion mode.",
                    query
                ),
            }
        }

        QueryIntent::Unknown => {
            // Unknown queries: Skip RAGGraph to avoid noise
            RoutingDecision {
                should_call_raggraph: false,
                mode_hint: None,
                top_k: None,
                depth: 0.0,
                reasoning: format!(
                    "Query '{}' doesn't match known patterns. Skipping RAGGraph.",
                    query
                ),
            }
        }
    }
}

/// Select fusion mode based on query characteristics
///
/// This is a helper function for backward compatibility with explicit mode selection.
pub fn select_fusion_mode(intent: &QueryIntent) -> Option<String> {
    match intent {
        QueryIntent::Symbolic => Some("simple".to_string()),
        QueryIntent::Semantic => Some("attention".to_string()),
        QueryIntent::Causal => Some("reasoning".to_string()),
        QueryIntent::Unknown => None,
    }
}

/// Suggest top_k parameter based on intent
pub fn suggest_top_k(intent: &QueryIntent) -> u32 {
    match intent {
        QueryIntent::Symbolic => 5,
        QueryIntent::Semantic => 10,
        QueryIntent::Causal => 15,
        QueryIntent::Unknown => 5, // Default fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_symbolic() {
        let decision = route_query(&QueryIntent::Symbolic, "format_string");
        assert!(decision.should_call_raggraph);
        assert_eq!(decision.mode_hint, Some("simple".to_string()));
        assert_eq!(decision.top_k, Some(5));
        assert!(decision.depth < 0.5);
    }

    #[test]
    fn test_route_semantic() {
        let decision = route_query(&QueryIntent::Semantic, "explain parse");
        assert!(decision.should_call_raggraph);
        assert_eq!(decision.mode_hint, Some("attention".to_string()));
        assert_eq!(decision.top_k, Some(10));
    }

    #[test]
    fn test_route_causal() {
        let decision = route_query(&QueryIntent::Causal, "trace flow");
        assert!(decision.should_call_raggraph);
        assert_eq!(decision.mode_hint, Some("reasoning".to_string()));
        assert_eq!(decision.top_k, Some(15));
        assert!(decision.depth > 0.8);
    }

    #[test]
    fn test_route_unknown() {
        let decision = route_query(&QueryIntent::Unknown, "hello");
        assert!(!decision.should_call_raggraph);
        assert_eq!(decision.mode_hint, None);
    }

    #[test]
    fn test_select_fusion_mode() {
        assert_eq!(select_fusion_mode(&QueryIntent::Symbolic), Some("simple".to_string()));
        assert_eq!(select_fusion_mode(&QueryIntent::Semantic), Some("attention".to_string()));
        assert_eq!(select_fusion_mode(&QueryIntent::Causal), Some("reasoning".to_string()));
        assert_eq!(select_fusion_mode(&QueryIntent::Unknown), None);
    }
}
