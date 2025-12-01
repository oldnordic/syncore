//! Reasoning Pattern Engine
//!
//! Mines, stores, and recommends reasoning patterns from historical episodes.
//! Provides:
//! - Pattern mining from reasoning episodes
//! - Pattern storage in LTMC Memory
//! - Pattern recommendation based on intent and mode

use super::intent_classifier::QueryIntent;
use super::reasoning_ledger::ReasoningEpisode;
use crate::memory::Memory;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Reasoning Pattern ID
pub type ReasoningPatternId = i64;

/// Pattern Graph Usage Level
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternGraphUsage {
    None,
    Light,
    Heavy,
}

/// Reasoning Pattern
///
/// Represents a learned pattern from successful/failed reasoning episodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPattern {
    pub id: ReasoningPatternId,
    pub intent_type: QueryIntent,
    pub selected_mode: String,
    pub tool_sequence: Vec<String>,
    pub graph_usage: PatternGraphUsage,
    pub success_count: u64,
    pub failure_count: u64,
    pub success_rate: f32,
    pub last_updated: i64,
    /// Client ID for tracking which LLM contributed to this pattern (metadata only)
    /// Patterns aggregate data from ALL clients within the same namespace
    #[serde(default)]
    pub client_id: Option<String>,
}

/// Mine patterns from reasoning episodes
///
/// Groups episodes by (intent_type, selected_mode, tool_sequence)
/// and computes success/failure statistics
///
/// # Arguments
/// * `episodes` - List of reasoning episodes to mine
///
/// # Returns
/// Vector of mined patterns
pub fn mine_patterns_from_episodes(episodes: &[ReasoningEpisode]) -> Vec<ReasoningPattern> {
    // Group key: (intent_type_str, selected_mode, sorted_tool_sequence)
    let mut groups: HashMap<String, PatternGroup> = HashMap::new();

    for episode in episodes {
        // Classify intent from query
        let intent = classify_intent_from_query(&episode.user_query);

        // Normalize tool sequence (sort for consistency)
        let mut tools = episode.tool_calls.clone();
        tools.sort();
        let tools_key = tools.join(",");

        // Create group key
        let key = format!("{}|{}|{}", intent_to_string(&intent), episode.selected_mode, tools_key);

        let group = groups.entry(key.clone()).or_insert_with(|| PatternGroup {
            intent_type: intent,
            selected_mode: episode.selected_mode.clone(),
            tool_sequence: tools,
            success_count: 0,
            failure_count: 0,
            graph_usage_score: 0,
            last_timestamp: episode.timestamp,
        });

        // Update counts
        if episode.outcome == "success" {
            group.success_count += 1;
        } else {
            group.failure_count += 1;
        }

        // Update graph usage score
        group.graph_usage_score += compute_graph_usage_score(&episode.tool_calls);
        group.last_timestamp = group.last_timestamp.max(episode.timestamp);
    }

    // Convert groups to patterns
    let mut patterns = Vec::new();
    let mut id_counter = 1;

    for (_key, group) in groups {
        let total = group.success_count + group.failure_count;
        let success_rate = if total > 0 {
            group.success_count as f32 / total as f32
        } else {
            0.0
        };

        // Determine graph usage level
        let avg_graph_score = if total > 0 {
            group.graph_usage_score as f32 / total as f32
        } else {
            0.0
        };

        let graph_usage = if avg_graph_score >= 2.0 {
            PatternGraphUsage::Heavy
        } else if avg_graph_score >= 1.0 {
            PatternGraphUsage::Light
        } else {
            PatternGraphUsage::None
        };

        patterns.push(ReasoningPattern {
            id: id_counter,
            intent_type: group.intent_type,
            selected_mode: group.selected_mode,
            tool_sequence: group.tool_sequence,
            graph_usage,
            success_count: group.success_count,
            failure_count: group.failure_count,
            success_rate,
            last_updated: group.last_timestamp,
            client_id: None, // Patterns aggregate from all clients
        });

        id_counter += 1;
    }

    patterns
}

/// Store patterns to Memory
///
/// # Arguments
/// * `patterns` - Patterns to store
/// * `memory` - Memory instance
/// * `namespace` - Namespace for pattern storage
pub fn store_patterns_to_memory(
    patterns: &[ReasoningPattern],
    memory: &Memory,
    namespace: &str,
) -> Result<()> {
    // Serialize all patterns to JSON
    let patterns_json = serde_json::to_string(patterns)?;

    // Store with namespace key
    let key = format!("reasoning_patterns:{}", namespace);
    memory.store(&key, &patterns_json)?;

    Ok(())
}

/// Load patterns from Memory
///
/// # Arguments
/// * `memory` - Memory instance
/// * `namespace` - Namespace to load from
///
/// # Returns
/// Vector of loaded patterns
pub fn load_patterns_from_memory(
    memory: &Memory,
    namespace: &str,
) -> Result<Vec<ReasoningPattern>> {
    let key = format!("reasoning_patterns:{}", namespace);

    if let Some(patterns_json) = memory.query(&key)? {
        let patterns: Vec<ReasoningPattern> = serde_json::from_str(&patterns_json)?;
        Ok(patterns)
    } else {
        Ok(Vec::new())
    }
}

/// Recommend patterns for a query
///
/// Filters by intent and mode, sorts by success_rate and success_count
///
/// # Arguments
/// * `intent` - Query intent
/// * `selected_mode` - Selected fusion mode
/// * `memory` - Memory instance
/// * `namespace` - Namespace to query
/// * `max_patterns` - Maximum patterns to return
///
/// # Returns
/// Recommended patterns, sorted by success rate
pub fn recommend_patterns_for_query(
    intent: &QueryIntent,
    selected_mode: &str,
    memory: &Memory,
    namespace: &str,
    max_patterns: usize,
) -> Result<Vec<ReasoningPattern>> {
    let patterns = load_patterns_from_memory(memory, namespace)?;

    // Filter by intent and mode
    let mut filtered: Vec<ReasoningPattern> = patterns
        .into_iter()
        .filter(|p| intent_matches(&p.intent_type, intent) && p.selected_mode == selected_mode)
        .collect();

    // Sort by success_rate (descending), then success_count (descending)
    filtered.sort_by(|a, b| {
        b.success_rate
            .partial_cmp(&a.success_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.success_count.cmp(&a.success_count))
    });

    // Limit results
    filtered.truncate(max_patterns);

    Ok(filtered)
}

// Helper: Pattern group accumulator
struct PatternGroup {
    intent_type: QueryIntent,
    selected_mode: String,
    tool_sequence: Vec<String>,
    success_count: u64,
    failure_count: u64,
    graph_usage_score: u32,
    last_timestamp: i64,
}

// Helper: Classify intent from query string
fn classify_intent_from_query(query: &str) -> QueryIntent {
    use super::intent_classifier::classify_intent;
    classify_intent(query)
}

// Helper: Convert intent to string
fn intent_to_string(intent: &QueryIntent) -> String {
    match intent {
        QueryIntent::Symbolic => "symbolic".to_string(),
        QueryIntent::Semantic => "semantic".to_string(),
        QueryIntent::Causal => "causal".to_string(),
        QueryIntent::Unknown => "unknown".to_string(),
    }
}

// Helper: Compute graph usage score from tool calls
fn compute_graph_usage_score(tool_calls: &[String]) -> u32 {
    let graph_tools = [
        "code_graph_fusion_query",
        "code_graph_sync_neo4j",
        "graph_query",
        "graph_insert",
        "graph_relate",
        "raggraph_query",
        "raggraph_multihop",
    ];

    let mut score = 0;
    for tool in tool_calls {
        if graph_tools.iter().any(|gt| tool.contains(gt)) {
            score += 1;
        }
    }
    score
}

// Helper: Check if intents match
fn intent_matches(a: &QueryIntent, b: &QueryIntent) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_to_string() {
        assert_eq!(intent_to_string(&QueryIntent::Symbolic), "symbolic");
        assert_eq!(intent_to_string(&QueryIntent::Semantic), "semantic");
    }

    #[test]
    fn test_graph_usage_score() {
        let tools1 = vec!["code_index".to_string()];
        assert_eq!(compute_graph_usage_score(&tools1), 0);

        let tools2 = vec!["code_graph_fusion_query".to_string()];
        assert_eq!(compute_graph_usage_score(&tools2), 1);

        let tools3 = vec!["code_graph_fusion_query".to_string(), "raggraph_query".to_string()];
        assert_eq!(compute_graph_usage_score(&tools3), 2);
    }
}
