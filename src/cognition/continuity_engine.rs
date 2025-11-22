//! Continuity Engine Module
//!
//! Provides reasoning continuity by:
//! - Routing to appropriate ledger (SQL/Graph/Hybrid)
//! - Building ReasoningContinuity from historical episodes
//! - Persisting current episode at end of cycle

use super::context_bundle::ContextBundle;
use super::intent_classifier::QueryIntent;
use super::orchestrator::EnrichedContext;
use super::reasoning_ledger::{
    fetch_recent_episodes_sql, fetch_related_episodes_graph, store_episode_graph,
    store_episode_sql, ReasoningEpisode, ReasoningEpisodeId,
};
use crate::graph::Neo4jClient;
use crate::memory::Memory;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Reasoning Continuity
///
/// Container for historical reasoning episodes relevant to current query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningContinuity {
    pub episodes: Vec<ReasoningEpisode>,
    pub sql_used: bool,
    pub graph_used: bool,
    pub summary: Option<String>,
}

impl ReasoningContinuity {
    pub fn new() -> Self {
        Self {
            episodes: Vec::new(),
            sql_used: false,
            graph_used: false,
            summary: None,
        }
    }
}

impl Default for ReasoningContinuity {
    fn default() -> Self {
        Self::new()
    }
}

/// Continuity Route
///
/// Determines which ledger(s) to query
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinuityRoute {
    SqlOnly,
    GraphOnly,
    Hybrid,
    None,
}

/// Decide continuity route based on intent and context
///
/// # Arguments
/// * `intent` - Query intent from classifier
/// * `context_bundle` - Current context bundle
///
/// # Returns
/// Recommended continuity route
pub fn decide_continuity_route(
    intent: &QueryIntent,
    context_bundle: &ContextBundle,
) -> ContinuityRoute {
    let entity_count = context_bundle.raggraph_entities.len()
        + context_bundle.memory_vectors.len()
        + context_bundle.memory_graph.len();

    match intent {
        QueryIntent::Symbolic => {
            // Simple symbolic queries: minimal or no continuity
            if entity_count == 0 {
                ContinuityRoute::None
            } else {
                ContinuityRoute::SqlOnly
            }
        }
        QueryIntent::Semantic => {
            // Semantic queries: hybrid if multiple entities, SQL otherwise
            if entity_count > 3 {
                ContinuityRoute::Hybrid
            } else {
                ContinuityRoute::SqlOnly
            }
        }
        QueryIntent::Causal => {
            // Causal queries: always hybrid (need graph relationships)
            ContinuityRoute::Hybrid
        }
        QueryIntent::Unknown => {
            // Unknown queries: minimal continuity
            if entity_count > 0 {
                ContinuityRoute::SqlOnly
            } else {
                ContinuityRoute::None
            }
        }
    }
}

/// Build reasoning continuity from ledgers
///
/// # Arguments
/// * `query` - Current user query
/// * `entity_ids` - Relevant entity IDs from context
/// * `route` - Continuity route
/// * `memory` - Memory instance for SQL
/// * `neo4j` - Optional Neo4j client for graph
/// * `limit` - Max episodes to fetch
pub async fn build_reasoning_continuity(
    query: &str,
    entity_ids: &[String],
    route: &ContinuityRoute,
    memory: &Memory,
    neo4j: Option<&Neo4jClient>,
    limit: usize,
) -> Result<ReasoningContinuity> {
    let mut continuity = ReasoningContinuity::new();

    match route {
        ContinuityRoute::None => {
            // No continuity needed
        }
        ContinuityRoute::SqlOnly => {
            // Fetch from SQL only
            let sql_episodes = fetch_recent_episodes_sql(memory, query, limit)?;
            continuity.episodes = sql_episodes;
            continuity.sql_used = true;
        }
        ContinuityRoute::GraphOnly => {
            // Fetch from graph only
            if let Some(neo4j_client) = neo4j {
                let episode_ids =
                    fetch_related_episodes_graph(neo4j_client, entity_ids, limit).await?;

                // Fetch full episodes from SQL by IDs
                for id in episode_ids {
                    if let Ok(episodes) = fetch_recent_episodes_sql(memory, "", limit) {
                        for episode in episodes {
                            if episode.id == id {
                                continuity.episodes.push(episode);
                                break;
                            }
                        }
                    }
                }
                continuity.graph_used = true;
            }
        }
        ContinuityRoute::Hybrid => {
            // Fetch from both and merge
            let mut seen_ids: HashSet<ReasoningEpisodeId> = HashSet::new();

            // SQL episodes
            let sql_episodes = fetch_recent_episodes_sql(memory, query, limit)?;
            for episode in sql_episodes {
                if seen_ids.insert(episode.id) {
                    continuity.episodes.push(episode);
                }
            }
            continuity.sql_used = true;

            // Graph episodes
            if let Some(neo4j_client) = neo4j {
                let episode_ids =
                    fetch_related_episodes_graph(neo4j_client, entity_ids, limit).await?;

                // Fetch full episodes from SQL
                for id in episode_ids {
                    if !seen_ids.contains(&id) {
                        if let Ok(all_episodes) = fetch_recent_episodes_sql(memory, "", limit * 2) {
                            for episode in all_episodes {
                                if episode.id == id && seen_ids.insert(episode.id) {
                                    continuity.episodes.push(episode);
                                    break;
                                }
                            }
                        }
                    }
                }
                continuity.graph_used = true;
            }

            // Limit total episodes
            continuity.episodes.truncate(limit);
        }
    }

    // Generate summary
    if !continuity.episodes.is_empty() {
        continuity.summary = Some(format!(
            "Found {} relevant past reasoning episodes",
            continuity.episodes.len()
        ));
    }

    Ok(continuity)
}

/// Persist current episode to both SQL and graph ledgers
///
/// # Arguments
/// * `enriched` - Enriched context from orchestrator
/// * `bundle` - Context bundle
/// * `tool_calls` - Tool calls invoked
/// * `outcome` - Episode outcome (e.g., "success", "error")
/// * `memory` - Memory instance
/// * `neo4j` - Optional Neo4j client
pub async fn persist_current_episode(
    enriched: &EnrichedContext,
    bundle: &ContextBundle,
    tool_calls: &[String],
    outcome: &str,
    memory: &Memory,
    neo4j: Option<&Neo4jClient>,
) -> Result<()> {
    // Generate episode ID (timestamp-based for uniqueness)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    let episode_id = timestamp;

    // Extract important entities from bundle
    let mut important_entities = Vec::new();
    for entity in &bundle.raggraph_entities {
        if let Some(id) = entity.entity_id {
            important_entities.push(format!("entity_{}", id));
        }
    }
    // Add top vector hits
    for (idx, _hit) in bundle.memory_vectors.iter().take(3).enumerate() {
        important_entities.push(format!("vector_{}", idx));
    }

    // Create episode
    let episode = ReasoningEpisode {
        id: episode_id,
        timestamp,
        user_query: enriched.query.clone(),
        selected_mode: enriched
            .selected_mode
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        important_entities,
        tool_calls: tool_calls.to_vec(),
        outcome: outcome.to_string(),
        notes: Some(enriched.debug_info.clone()),
        client_id: None, // Will be set by transport layer if needed
    };

    // Store to SQL (always)
    store_episode_sql(memory, &episode)?;

    // Store to graph (if available)
    if let Some(neo4j_client) = neo4j {
        // Best-effort graph storage (don't fail if graph unavailable)
        let _ = store_episode_graph(neo4j_client, &episode).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::context_bundle::ContextBundle;

    #[test]
    fn test_continuity_route_none_for_empty_symbolic() {
        let intent = QueryIntent::Symbolic;
        let bundle = ContextBundle::new();
        let route = decide_continuity_route(&intent, &bundle);
        assert_eq!(route, ContinuityRoute::None);
    }

    #[test]
    fn test_continuity_route_hybrid_for_causal() {
        let intent = QueryIntent::Causal;
        let bundle = ContextBundle::new();
        let route = decide_continuity_route(&intent, &bundle);
        assert_eq!(route, ContinuityRoute::Hybrid);
    }
}
