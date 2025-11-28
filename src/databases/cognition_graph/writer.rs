//! Cognition Graph Writer - All Write Operations
//!
//! Type-safe write operations for reasoning episodes.

use super::schema::*;
use crate::graph::Neo4jClient;
use anyhow::Result;

/// Upsert a ReasoningEpisode node
///
/// Creates or updates a reasoning episode with full properties.
/// Uses MERGE for idempotency - safe to call multiple times.
pub async fn upsert_reasoning_episode(
    client: &Neo4jClient,
    props: ReasoningEpisodeProperties,
) -> Result<()> {
    let query = format!(
        r#"
        MERGE (e:{}:{} {{id: $id, namespace: $ns}})
        SET e.timestamp = $timestamp,
            e.user_query = $user_query,
            e.selected_mode = $selected_mode,
            e.outcome = $outcome,
            e.notes = $notes,
            e.graph_domain = $graph_domain,
            e.project = $project_label
        "#,
        NodeLabel::ReasoningEpisode.as_str(),
        COGNITION_PROJECT_LABEL
    );

    client
        .execute_query(
            &query,
            vec![
                ("id", serde_json::json!(props.id)),
                ("ns", serde_json::json!(cognition_namespace(client))),
                ("timestamp", serde_json::json!(props.timestamp)),
                ("user_query", serde_json::json!(props.user_query)),
                ("selected_mode", serde_json::json!(props.selected_mode)),
                ("outcome", serde_json::json!(props.outcome)),
                ("notes", serde_json::json!(props.notes)),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
                ("project_label", serde_json::json!(COGNITION_PROJECT_LABEL)),
            ],
        )
        .await?;

    Ok(())
}

/// Create USES relationship between ReasoningEpisode and CodeReference
///
/// Links a reasoning episode to a code entity it references.
/// Creates CodeReference node if it doesn't exist (lightweight reference node).
pub async fn create_uses_relationship(
    client: &Neo4jClient,
    episode_id: i64,
    entity_id: &str,
) -> Result<()> {
    let query = format!(
        r#"
        MATCH (e:{}:{} {{id: $episode_id, namespace: $ns, graph_domain: $graph_domain}})
        MERGE (ent:{}:{} {{id: $entity_id, namespace: $ns}})
        SET ent.graph_domain = $graph_domain,
            ent.project = $project_label
        MERGE (e)-[:{}]->(ent)
        "#,
        NodeLabel::ReasoningEpisode.as_str(),
        COGNITION_PROJECT_LABEL,
        NodeLabel::CodeReference.as_str(),
        COGNITION_PROJECT_LABEL,
        RelationType::Uses.as_str()
    );

    client
        .execute_query(
            &query,
            vec![
                ("episode_id", serde_json::json!(episode_id)),
                ("entity_id", serde_json::json!(entity_id)),
                ("ns", serde_json::json!(cognition_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
                ("project_label", serde_json::json!(COGNITION_PROJECT_LABEL)),
            ],
        )
        .await?;

    Ok(())
}

/// Delete a ReasoningEpisode node and its relationships
pub async fn delete_reasoning_episode(client: &Neo4jClient, episode_id: i64) -> Result<()> {
    let query = format!(
        r#"
        MATCH (e:{}:{} {{id: $id, namespace: $ns, graph_domain: $graph_domain}})
        DETACH DELETE e
        "#,
        NodeLabel::ReasoningEpisode.as_str(),
        COGNITION_PROJECT_LABEL
    );

    client
        .execute_query(
            &query,
            vec![
                ("id", serde_json::json!(episode_id)),
                ("ns", serde_json::json!(cognition_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(())
}
