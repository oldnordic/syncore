//! Cognition Graph Reader - All Read Operations
//!
//! Type-safe read operations for reasoning episodes.

use super::schema::*;
use crate::graph::Neo4jClient;
use anyhow::Result;

/// Result type for ReasoningEpisode queries
#[derive(Debug, Clone)]
pub struct ReasoningEpisodeResult {
    pub id: i64,
    pub timestamp: i64,
    pub user_query: String,
    pub selected_mode: String,
    pub outcome: String,
    pub notes: Option<String>,
}

/// Get reasoning episode by ID
pub async fn get_reasoning_episode_by_id(
    client: &Neo4jClient,
    episode_id: i64,
) -> Result<Option<ReasoningEpisodeResult>> {
    let query = format!(
        r#"
        MATCH (e:{}:{} {{id: $id, namespace: $ns}})
        RETURN e.id as id, e.timestamp as timestamp, e.user_query as user_query,
               e.selected_mode as selected_mode, e.outcome as outcome, e.notes as notes
        "#,
        NodeLabel::ReasoningEpisode.as_str(),
        COGNITION_PROJECT_LABEL
    );

    let results = client
        .execute_query(
            &query,
            vec![
                ("id", serde_json::json!(episode_id)),
                ("ns", serde_json::json!(cognition_namespace(client))),
            ],
        )
        .await?;

    if results.is_empty() {
        return Ok(None);
    }

    let row = &results[0];
    Ok(Some(ReasoningEpisodeResult {
        id: row.get("id").and_then(|v| v.as_i64()).unwrap_or(episode_id),
        timestamp: row.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0),
        user_query: row
            .get("user_query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        selected_mode: row
            .get("selected_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        outcome: row
            .get("outcome")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        notes: row
            .get("notes")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    }))
}

/// Fetch related episodes by entity IDs
///
/// Returns episode IDs for episodes that reference the given code entities.
/// Ordered by timestamp descending (most recent first).
pub async fn fetch_related_episodes(
    client: &Neo4jClient,
    entity_ids: &[String],
    limit: usize,
) -> Result<Vec<i64>> {
    if entity_ids.is_empty() {
        return Ok(Vec::new());
    }

    let query = format!(
        r#"
        MATCH (e:{}:{} {{namespace: $ns}})-[:{}]->(ent:{})
        WHERE ent.id IN $entity_ids
        WITH DISTINCT e
        RETURN e.id as id
        ORDER BY e.timestamp DESC
        LIMIT $limit
        "#,
        NodeLabel::ReasoningEpisode.as_str(),
        COGNITION_PROJECT_LABEL,
        RelationType::Uses.as_str(),
        NodeLabel::CodeEntity.as_str()
    );

    let results = client
        .execute_query(
            &query,
            vec![
                ("entity_ids", serde_json::json!(entity_ids)),
                ("limit", serde_json::json!(limit)),
                ("ns", serde_json::json!(cognition_namespace(client))),
            ],
        )
        .await?;

    let episode_ids = results
        .iter()
        .filter_map(|row| row.get("id").and_then(|v| v.as_i64()))
        .collect();

    Ok(episode_ids)
}

/// Count total reasoning episodes
pub async fn count_reasoning_episodes(client: &Neo4jClient) -> Result<i64> {
    let query = format!(
        r#"
        MATCH (e:{}:{} {{namespace: $ns}})
        RETURN count(e) as count
        "#,
        NodeLabel::ReasoningEpisode.as_str(),
        COGNITION_PROJECT_LABEL
    );

    let results = client
        .execute_query(
            &query,
            vec![("ns", serde_json::json!(cognition_namespace(client)))],
        )
        .await?;

    let count = results
        .first()
        .and_then(|r| r.get("count"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    Ok(count)
}
