//! Reasoning Ledger Module
//!
//! Hybrid SQL + Graph storage for reasoning episodes.
//! Provides:
//! - SQL ledger for chronological queryable log
//! - Graph ledger for entity relationship tracking
//! - Hybrid fetch combining both sources

use crate::graph::Neo4jClient;
use crate::memory::Memory;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Reasoning Episode ID
pub type ReasoningEpisodeId = i64;

/// Reasoning Episode
///
/// Captures a complete reasoning cycle including:
/// - User query
/// - Selected fusion mode
/// - Important code entities
/// - Tool calls invoked
/// - Outcome status
/// - Optional reasoning summary
/// - Client ID for multi-LLM support (metadata only, NOT for isolation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningEpisode {
    pub id: ReasoningEpisodeId,
    pub timestamp: i64,
    pub user_query: String,
    pub selected_mode: String,
    pub important_entities: Vec<String>,
    pub tool_calls: Vec<String>,
    pub outcome: String,
    pub notes: Option<String>,
    /// Client ID for tracking which LLM created this episode (metadata only)
    /// All episodes are shared across all clients within the same namespace
    #[serde(default = "default_client_id")]
    pub client_id: Option<String>,
}

fn default_client_id() -> Option<String> {
    None
}

/// Store reasoning episode to SQL ledger
///
/// Uses Memory's key-value store for episode persistence
///
/// # Arguments
/// * `memory` - Memory instance
/// * `episode` - Episode to store
pub fn store_episode_sql(memory: &Memory, episode: &ReasoningEpisode) -> Result<()> {
    // Serialize episode to JSON
    let episode_json = serde_json::to_string(episode)?;

    // Store with composite key: episode_{id}
    let key = format!("episode_{}", episode.id);
    memory.store(&key, &episode_json)?;

    // Also store in an index for query lookup
    let query_index_key = format!(
        "episode_query_index_{}",
        episode.user_query.to_lowercase().replace(' ', "_")
    );
    let existing_index = memory.query(&query_index_key)?;

    let mut episode_ids: Vec<i64> = if let Some(index_json) = existing_index {
        serde_json::from_str(&index_json).unwrap_or_default()
    } else {
        Vec::new()
    };

    if !episode_ids.contains(&episode.id) {
        episode_ids.push(episode.id);
        memory.store(&query_index_key, &serde_json::to_string(&episode_ids)?)?;
    }

    Ok(())
}

/// Fetch recent episodes from SQL ledger
///
/// Uses Memory's key-value store with query index
///
/// # Arguments
/// * `memory` - Memory instance
/// * `query` - Search query (searches in user_query field)
/// * `limit` - Max episodes to return
pub fn fetch_recent_episodes_sql(
    memory: &Memory,
    query: &str,
    limit: usize,
) -> Result<Vec<ReasoningEpisode>> {
    let mut episodes = Vec::new();

    // Try to fetch from query index
    let query_index_key = format!(
        "episode_query_index_{}",
        query.to_lowercase().replace(' ', "_")
    );
    if let Some(index_json) = memory.query(&query_index_key)? {
        let episode_ids: Vec<i64> = serde_json::from_str(&index_json).unwrap_or_default();

        for id in episode_ids.iter().take(limit) {
            let episode_key = format!("episode_{}", id);
            if let Some(episode_json) = memory.query(&episode_key)? {
                if let Ok(episode) = serde_json::from_str::<ReasoningEpisode>(&episode_json) {
                    episodes.push(episode);
                }
            }
        }
    }

    // If no results from index, try to scan all episode keys
    if episodes.is_empty() && !query.is_empty() {
        // List all keys and filter for episodes
        if let Ok(keys) = memory.list_keys(Some(100)) {
            for key in keys {
                if key.starts_with("episode_") && !key.contains("index") {
                    if let Some(episode_json) = memory.query(&key)? {
                        if let Ok(episode) = serde_json::from_str::<ReasoningEpisode>(&episode_json)
                        {
                            if episode
                                .user_query
                                .to_lowercase()
                                .contains(&query.to_lowercase())
                            {
                                episodes.push(episode);
                                if episodes.len() >= limit {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by timestamp descending
    episodes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    episodes.truncate(limit);

    Ok(episodes)
}

/// Store reasoning episode to graph ledger
///
/// Creates:
/// - Episode node
/// - Query node
/// - Entity nodes
/// - Relationships: USES, REFERENCES
///
/// # Arguments
/// * `neo4j` - Neo4j client
/// * `episode` - Episode to store
pub async fn store_episode_graph(neo4j: &Neo4jClient, episode: &ReasoningEpisode) -> Result<()> {
    // Create Episode node
    let cypher = format!(
        "CREATE (e:ReasoningEpisode {{
            id: $id,
            timestamp: $timestamp,
            user_query: $query,
            selected_mode: $mode,
            outcome: $outcome,
            notes: $notes,
            namespace: '{}'
        }})",
        neo4j.namespace()
    );

    neo4j
        .execute_query(
            &cypher,
            vec![
                ("id", serde_json::json!(episode.id)),
                ("timestamp", serde_json::json!(episode.timestamp)),
                ("query", serde_json::json!(episode.user_query)),
                ("mode", serde_json::json!(episode.selected_mode)),
                ("outcome", serde_json::json!(episode.outcome)),
                ("notes", serde_json::json!(episode.notes)),
            ],
        )
        .await?;

    // Link to entities
    for entity_id in &episode.important_entities {
        let link_cypher = format!(
            "MATCH (e:ReasoningEpisode {{id: $episode_id, namespace: '{}'}})
             MERGE (ent:CodeEntity {{id: $entity_id, namespace: '{}'}})
             MERGE (e)-[:USES]->(ent)",
            neo4j.namespace(),
            neo4j.namespace()
        );

        neo4j
            .execute_query(
                &link_cypher,
                vec![
                    ("episode_id", serde_json::json!(episode.id)),
                    ("entity_id", serde_json::json!(entity_id)),
                ],
            )
            .await?;
    }

    Ok(())
}

/// Fetch related episodes from graph ledger
///
/// Finds episodes that reference the given entity IDs
///
/// # Arguments
/// * `neo4j` - Neo4j client
/// * `entity_ids` - Entity IDs to search for
/// * `limit` - Max episodes to return
pub async fn fetch_related_episodes_graph(
    neo4j: &Neo4jClient,
    entity_ids: &[String],
    limit: usize,
) -> Result<Vec<ReasoningEpisodeId>> {
    if entity_ids.is_empty() {
        return Ok(Vec::new());
    }

    let cypher = format!(
        "MATCH (e:ReasoningEpisode {{namespace: '{}'}})-[:USES]->(ent:CodeEntity)
         WHERE ent.id IN $entity_ids
         WITH DISTINCT e
         RETURN e.id as id
         ORDER BY e.timestamp DESC
         LIMIT $limit",
        neo4j.namespace()
    );

    let results = neo4j
        .execute_query(
            &cypher,
            vec![
                ("entity_ids", serde_json::json!(entity_ids)),
                ("limit", serde_json::json!(limit)),
            ],
        )
        .await?;

    let episode_ids = results
        .iter()
        .filter_map(|row| row.get("id").and_then(|v| v.as_i64()))
        .collect();

    Ok(episode_ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_creation() {
        let episode = ReasoningEpisode {
            id: 1,
            timestamp: 123456,
            user_query: "test".to_string(),
            selected_mode: "simple".to_string(),
            important_entities: vec!["e1".to_string()],
            tool_calls: vec!["tool1".to_string()],
            outcome: "success".to_string(),
            notes: Some("test".to_string()),
            client_id: None,
        };

        assert_eq!(episode.id, 1);
        assert_eq!(episode.outcome, "success");
    }
}
