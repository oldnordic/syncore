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

/// Create a ReasoningSession node for Tree-of-Thought reasoning
///
/// Creates a new reasoning session with optional task reference and metadata.
/// Uses MERGE for idempotency - safe to call multiple times.
pub async fn create_session(client: &Neo4jClient, props: ReasoningSessionProperties) -> Result<()> {
    let query = format!(
        r#"
        MERGE (s:{}:{} {{id: $id, namespace: $ns}})
        SET s.task_id = $task_id,
            s.metadata = $metadata,
            s.created_at = $created_at,
            s.graph_domain = $graph_domain,
            s.project = $project_label,
            s.total_nodes = $total_nodes,
            s.depth = $depth,
            s.breadth = $breadth,
            s.identical_expansions = $identical_expansions,
            s.consecutive_errors = $consecutive_errors
        "#,
        NodeLabel::ReasoningSession.as_str(),
        COGNITION_PROJECT_LABEL
    );

    client
        .execute_query(
            &query,
            vec![
                ("id", serde_json::json!(props.id)),
                ("task_id", serde_json::json!(props.task_id)),
                ("metadata", serde_json::json!(props.metadata)),
                ("created_at", serde_json::json!(props.created_at)),
                ("total_nodes", serde_json::json!(props.total_nodes)),
                ("depth", serde_json::json!(props.depth)),
                ("breadth", serde_json::json!(props.breadth)),
                ("identical_expansions", serde_json::json!(props.identical_expansions)),
                ("consecutive_errors", serde_json::json!(props.consecutive_errors)),
                ("ns", serde_json::json!(cognition_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
                ("project_label", serde_json::json!(COGNITION_PROJECT_LABEL)),
            ],
        )
        .await?;

    Ok(())
}

/// Add a ThoughtNode to a ReasoningSession
///
/// Creates a thought node with optional parent reference for tree structure.
/// Uses MERGE for idempotency - safe to call multiple times.
pub async fn add_thought_node(client: &Neo4jClient, props: ThoughtNodeProperties) -> Result<()> {
    let query = format!(
        r#"
        MERGE (t:{}:{} {{id: $id, namespace: $ns}})
        SET t.session_id = $session_id,
            t.parent_id = $parent_id,
            t.step_index = $step_index,
            t.content = $content,
            t.score = $score,
            t.graph_domain = $graph_domain,
            t.project = $project_label
        "#,
        NodeLabel::ThoughtNode.as_str(),
        COGNITION_PROJECT_LABEL
    );

    client
        .execute_query(
            &query,
            vec![
                ("id", serde_json::json!(props.id)),
                ("session_id", serde_json::json!(props.session_id)),
                ("parent_id", serde_json::json!(props.parent_id)),
                ("step_index", serde_json::json!(props.step_index)),
                ("content", serde_json::json!(props.content)),
                ("score", serde_json::json!(props.score)),
                ("ns", serde_json::json!(cognition_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
                ("project_label", serde_json::json!(COGNITION_PROJECT_LABEL)),
            ],
        )
        .await?;

    // Create BELONGS_TO relationship to session
    let session_query = format!(
        r#"
        MATCH (t:{}:{} {{id: $node_id, namespace: $ns, graph_domain: $graph_domain}})
        MATCH (s:{}:{} {{id: $session_id, namespace: $ns, graph_domain: $graph_domain}})
        MERGE (t)-[:{}]->(s)
        "#,
        NodeLabel::ThoughtNode.as_str(),
        COGNITION_PROJECT_LABEL,
        NodeLabel::ReasoningSession.as_str(),
        COGNITION_PROJECT_LABEL,
        RelationType::BelongsTo.as_str()
    );

    client
        .execute_query(
            &session_query,
            vec![
                ("node_id", serde_json::json!(props.id)),
                ("session_id", serde_json::json!(props.session_id)),
                ("ns", serde_json::json!(cognition_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    // Create HAS_CHILD relationship if parent exists
    if let Some(ref parent_id) = props.parent_id {
        let parent_query = format!(
            r#"
            MATCH (parent:{}:{} {{id: $parent_id, namespace: $ns, graph_domain: $graph_domain}})
            MATCH (child:{}:{} {{id: $node_id, namespace: $ns, graph_domain: $graph_domain}})
            MERGE (parent)-[:{}]->(child)
            "#,
            NodeLabel::ThoughtNode.as_str(),
            COGNITION_PROJECT_LABEL,
            NodeLabel::ThoughtNode.as_str(),
            COGNITION_PROJECT_LABEL,
            RelationType::HasChild.as_str()
        );

        client
            .execute_query(
                &parent_query,
                vec![
                    ("parent_id", serde_json::json!(parent_id)),
                    ("node_id", serde_json::json!(props.id)),
                    ("ns", serde_json::json!(cognition_namespace(client))),
                    ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
                ],
            )
            .await?;
    }

    Ok(())
}
