//! Portfolio Graph Write Operations
//!
//! THE ONLY place where portfolio graph writes to Neo4j happen.
//! Follows same patterns as canonical Neo4j and RAG graph modules.

use anyhow::Result;
use crate::graph::Neo4jClient;
use super::schema::{
    NodeLabel, PatchProperties, StepProperties, TaskProperties,
    RelationType, PORTFOLIO_PROJECT_LABEL, portfolio_namespace
};

/// Create or update a Patch node
pub async fn upsert_patch(
    client: &Neo4jClient,
    props: PatchProperties,
) -> Result<()> {
    let query = format!(
        r#"
        MERGE (p:{}:{} {{id: $id, namespace: $ns}})
        SET p.metadata = $metadata
        "#,
        NodeLabel::Patch.as_str(),
        PORTFOLIO_PROJECT_LABEL
    );

    client
        .execute_query(
            &query,
            vec![
                ("id", serde_json::json!(props.id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
                ("metadata", serde_json::json!(props.metadata)),
            ],
        )
        .await?;

    Ok(())
}

/// Create or update a Step node
pub async fn upsert_step(
    client: &Neo4jClient,
    props: StepProperties,
) -> Result<()> {
    let query = format!(
        r#"
        MERGE (s:{}:{} {{id: $id, namespace: $ns}})
        SET s.step_number = $step_number,
            s.metadata = $metadata
        "#,
        NodeLabel::Step.as_str(),
        PORTFOLIO_PROJECT_LABEL
    );

    client
        .execute_query(
            &query,
            vec![
                ("id", serde_json::json!(props.id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
                ("step_number", serde_json::json!(props.step_number)),
                ("metadata", serde_json::json!(props.metadata)),
            ],
        )
        .await?;

    Ok(())
}

/// Create or update a Task node
pub async fn upsert_task(
    client: &Neo4jClient,
    props: TaskProperties,
) -> Result<()> {
    let query = format!(
        r#"
        MERGE (t:{}:{} {{id: $id, namespace: $ns}})
        SET t.metadata = $metadata
        "#,
        NodeLabel::Task.as_str(),
        PORTFOLIO_PROJECT_LABEL
    );

    client
        .execute_query(
            &query,
            vec![
                ("id", serde_json::json!(props.id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
                ("metadata", serde_json::json!(props.metadata)),
            ],
        )
        .await?;

    Ok(())
}

/// Create a FOR_TASK relationship (Patch/Step → Task)
pub async fn create_for_task_relationship(
    client: &Neo4jClient,
    src_id: i64,
    task_id: i64,
) -> Result<()> {
    let query = format!(
        r#"
        MATCH (src {{id: $src_id, namespace: $ns}})
        MATCH (t:Task {{id: $task_id, namespace: $ns}})
        MERGE (src)-[:{}]->(t)
        "#,
        RelationType::ForTask.as_str()
    );

    client
        .execute_query(
            &query,
            vec![
                ("src_id", serde_json::json!(src_id)),
                ("task_id", serde_json::json!(task_id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
            ],
        )
        .await?;

    Ok(())
}

/// Create an APPLIES_TO relationship (Patch → File)
///
/// Note: File nodes use path as the global key (not namespace-filtered)
pub async fn create_applies_to_relationship(
    client: &Neo4jClient,
    patch_id: i64,
    file_path: &str,
) -> Result<()> {
    let query = format!(
        r#"
        MERGE (p:Patch {{id: $patch_id, namespace: $ns}})
        MERGE (f:File {{path: $path}})
        MERGE (p)-[:{}]->(f)
        "#,
        RelationType::AppliesTo.as_str()
    );

    client
        .execute_query(
            &query,
            vec![
                ("patch_id", serde_json::json!(patch_id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
                ("path", serde_json::json!(file_path)),
            ],
        )
        .await?;

    Ok(())
}

/// Create a FOLLOWS relationship (Step → Step)
pub async fn create_follows_relationship(
    client: &Neo4jClient,
    prev_step_id: i64,
    next_step_id: i64,
) -> Result<()> {
    let query = format!(
        r#"
        MATCH (prev:Step {{id: $prev_id, namespace: $ns}})
        MATCH (next:Step {{id: $next_id, namespace: $ns}})
        MERGE (prev)-[:{}]->(next)
        "#,
        RelationType::Follows.as_str()
    );

    client
        .execute_query(
            &query,
            vec![
                ("prev_id", serde_json::json!(prev_step_id)),
                ("next_id", serde_json::json!(next_step_id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
            ],
        )
        .await?;

    Ok(())
}

/// Delete a patch node
pub async fn delete_patch(client: &Neo4jClient, id: i64) -> Result<()> {
    let query = r#"
        MATCH (p:Patch {id: $id, namespace: $ns})
        DETACH DELETE p
    "#;

    client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
            ],
        )
        .await?;

    Ok(())
}

/// Delete a step node
pub async fn delete_step(client: &Neo4jClient, id: i64) -> Result<()> {
    let query = r#"
        MATCH (s:Step {id: $id, namespace: $ns})
        DETACH DELETE s
    "#;

    client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
            ],
        )
        .await?;

    Ok(())
}

/// Delete a task node
pub async fn delete_task(client: &Neo4jClient, id: i64) -> Result<()> {
    let query = r#"
        MATCH (t:Task {id: $id, namespace: $ns})
        DETACH DELETE t
    "#;

    client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
            ],
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_validation() {
        let label = NodeLabel::Patch;
        let rel = RelationType::ForTask;

        assert_eq!(label.as_str(), "Patch");
        assert_eq!(rel.as_str(), "FOR_TASK");
    }

    #[test]
    fn test_project_label_is_set() {
        assert_eq!(PORTFOLIO_PROJECT_LABEL, "SynCore");
    }
}
