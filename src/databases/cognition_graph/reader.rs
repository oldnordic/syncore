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
        MATCH (e:{}:{} {{id: $id, namespace: $ns, graph_domain: $graph_domain}})
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
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
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
        user_query: row.get("user_query").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        selected_mode: row.get("selected_mode").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        outcome: row.get("outcome").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        notes: row.get("notes").and_then(|v| v.as_str()).map(|s| s.to_string()),
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
        MATCH (e:{}:{} {{namespace: $ns, graph_domain: $graph_domain}})-[:{}]->(ent:{} {{namespace: $ns, graph_domain: $graph_domain}})
        WHERE ent.id IN $entity_ids
        WITH DISTINCT e
        RETURN e.id as id
        ORDER BY e.timestamp DESC
        LIMIT $limit
        "#,
        NodeLabel::ReasoningEpisode.as_str(),
        COGNITION_PROJECT_LABEL,
        RelationType::Uses.as_str(),
        NodeLabel::CodeReference.as_str()
    );

    let results = client
        .execute_query(
            &query,
            vec![
                ("entity_ids", serde_json::json!(entity_ids)),
                ("limit", serde_json::json!(limit)),
                ("ns", serde_json::json!(cognition_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    let episode_ids =
        results.iter().filter_map(|row| row.get("id").and_then(|v| v.as_i64())).collect();

    Ok(episode_ids)
}

/// Count total reasoning episodes
pub async fn count_reasoning_episodes(client: &Neo4jClient) -> Result<i64> {
    let query = format!(
        r#"
        MATCH (e:{}:{} {{namespace: $ns, graph_domain: $graph_domain}})
        RETURN count(e) as count
        "#,
        NodeLabel::ReasoningEpisode.as_str(),
        COGNITION_PROJECT_LABEL
    );

    let results = client
        .execute_query(
            &query,
            vec![
                ("ns", serde_json::json!(cognition_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    let count = results.first().and_then(|r| r.get("count")).and_then(|v| v.as_i64()).unwrap_or(0);

    Ok(count)
}

/// Result type for ReasoningSession queries
#[derive(Debug, Clone)]
pub struct SessionResult {
    pub id: String,
    pub task_id: Option<String>,
    pub metadata: Option<String>,
    pub created_at: i64,
    // PHASE ST-6: Circuit breaker session counters
    pub total_nodes: i64,
    pub depth: i64,
    pub breadth: i64,
    pub identical_expansions: i64,
    pub consecutive_errors: i64,
}

/// Result type for ThoughtNode queries
#[derive(Debug, Clone)]
pub struct ThoughtNodeResult {
    pub id: String,
    pub session_id: String,
    pub parent_id: Option<String>,
    pub step_index: i64,
    pub content: String,
    pub score: Option<f64>,
}

impl SessionResult {
    pub fn from_neo4j_value(value: &serde_json::Value) -> Option<Self> {
        Some(SessionResult {
            id: value.get("id")?.as_str()?.to_string(),
            task_id: value.get("task_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
            metadata: value.get("metadata").and_then(|v| v.as_str()).map(|s| s.to_string()),
            created_at: value.get("created_at")?.as_i64().unwrap_or(0),
            // PHASE ST-6: Circuit breaker session counters
            total_nodes: value.get("total_nodes").and_then(|v| v.as_i64()).unwrap_or(0),
            depth: value.get("depth").and_then(|v| v.as_i64()).unwrap_or(0),
            breadth: value.get("breadth").and_then(|v| v.as_i64()).unwrap_or(0),
            identical_expansions: value
                .get("identical_expansions")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            consecutive_errors: value
                .get("consecutive_errors")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
        })
    }
}

impl ThoughtNodeResult {
    pub fn from_neo4j_value(value: &serde_json::Value) -> Option<Self> {
        Some(ThoughtNodeResult {
            id: value.get("id")?.as_str()?.to_string(),
            session_id: value.get("session_id")?.as_str()?.to_string(),
            parent_id: value.get("parent_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
            step_index: value.get("step_index")?.as_i64().unwrap_or(0),
            content: value.get("content")?.as_str()?.to_string(),
            score: value.get("score").and_then(|v| v.as_f64()),
        })
    }
}

/// Get reasoning session by ID
pub async fn get_session(client: &Neo4jClient, session_id: &str) -> Result<Option<SessionResult>> {
    let query = format!(
        r#"
        MATCH (s:{}:{} {{id: $id, namespace: $ns, graph_domain: $graph_domain}})
        RETURN s.id as id, s.task_id as task_id, s.metadata as metadata, s.created_at as created_at,
               s.total_nodes as total_nodes, s.depth as depth, s.breadth as breadth,
               s.identical_expansions as identical_expansions, s.consecutive_errors as consecutive_errors
        "#,
        NodeLabel::ReasoningSession.as_str(),
        COGNITION_PROJECT_LABEL
    );

    let results = client
        .execute_query(
            &query,
            vec![
                ("id", serde_json::json!(session_id)),
                ("ns", serde_json::json!(cognition_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    if results.is_empty() {
        return Ok(None);
    }

    let row = &results[0];
    Ok(SessionResult::from_neo4j_value(row))
}

/// Get all thought nodes for a session
///
/// Returns nodes ordered by step_index ascending.
/// Includes parent-child relationships for tree structure.
pub async fn get_nodes_for_session(
    client: &Neo4jClient,
    session_id: &str,
) -> Result<Vec<ThoughtNodeResult>> {
    let query = format!(
        r#"
        MATCH (t:{}:{} {{session_id: $session_id, namespace: $ns, graph_domain: $graph_domain}})
        RETURN t.id as id, t.session_id as session_id, t.parent_id as parent_id,
               t.step_index as step_index, t.content as content, t.score as score
        ORDER BY t.step_index ASC
        "#,
        NodeLabel::ThoughtNode.as_str(),
        COGNITION_PROJECT_LABEL
    );

    let results = client
        .execute_query(
            &query,
            vec![
                ("session_id", serde_json::json!(session_id)),
                ("ns", serde_json::json!(cognition_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    let nodes: Vec<ThoughtNodeResult> =
        results.iter().filter_map(|row| ThoughtNodeResult::from_neo4j_value(row)).collect();

    Ok(nodes)
}
