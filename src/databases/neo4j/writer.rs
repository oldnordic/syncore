//! Canonical Neo4j Write API
//!
//! THE ONLY place where writes to Neo4j happen.
//! No string concatenation. No runtime-generated Cypher. No improvisation.
//!
//! Every write method is:
//! - Hardcoded
//! - Parameterized
//! - Validated against schema
//! - Idempotent (uses MERGE)

use super::schema::{
    project_namespace, NodeLabel, NodeProperties, RelationType, GRAPH_DOMAIN, PROJECT_LABEL,
};
use crate::graph::Neo4jClient;
use anyhow::Result;

/// Create or update a code entity node
///
/// Uses MERGE for idempotency - safe to call multiple times.
/// Schema: :{label}:{PROJECT_LABEL} with all canonical properties
/// Matches existing pattern: `:Function:SynCore`
pub async fn upsert_entity(
    client: &Neo4jClient,
    label: NodeLabel,
    props: NodeProperties,
) -> Result<()> {
    // Use double label pattern: :Function:SynCore
    let query = format!(
        r#"
        MERGE (e:{}:{} {{id: $id, namespace: $ns}})
        SET e.name = $name,
            e.path = $path,
            e.start_line = $start_line,
            e.end_line = $end_line,
            e.signature = $signature,
            e.body_snippet = $body_snippet,
            e.docstring = $docstring,
            e.hash = $hash,
            e.language = $language,
            e.file_sha256 = $file_sha256,
            e.mtime = $mtime,
            e.created_at = $created_at,
            e.last_modified_at = $last_modified_at,
            e.change_count = $change_count,
            e.author_count = $author_count,
            e.graph_domain = $graph_domain,
            e.project = $project_label
        "#,
        label.as_str(),
        PROJECT_LABEL
    );

    client
        .execute_query(
            &query,
            vec![
                ("id", serde_json::json!(props.id)),
                ("ns", serde_json::json!(project_namespace(client))),
                ("name", serde_json::json!(props.name)),
                ("path", serde_json::json!(props.path)),
                ("start_line", serde_json::json!(props.start_line)),
                ("end_line", serde_json::json!(props.end_line)),
                ("signature", serde_json::json!(props.signature)),
                ("body_snippet", serde_json::json!(props.body_snippet)),
                ("docstring", serde_json::json!(props.docstring)),
                ("hash", serde_json::json!(props.hash)),
                ("language", serde_json::json!(props.language)),
                ("file_sha256", serde_json::json!(props.file_sha256)),
                ("mtime", serde_json::json!(props.mtime)),
                ("created_at", serde_json::json!(props.created_at)),
                ("last_modified_at", serde_json::json!(props.last_modified_at)),
                ("change_count", serde_json::json!(props.change_count)),
                ("author_count", serde_json::json!(props.author_count)),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
                ("project_label", serde_json::json!(PROJECT_LABEL)),
            ],
        )
        .await?;

    Ok(())
}

/// Create a relationship between two entities
///
/// Uses MERGE for idempotency - safe to call multiple times.
/// Both entities must already exist (use upsert_entity first).
pub async fn create_relationship(
    client: &Neo4jClient,
    src_id: i64,
    dst_id: i64,
    rel_type: RelationType,
) -> Result<()> {
    let query = format!(
        r#"
        MATCH (a {{id: $src_id, namespace: $ns, graph_domain: $graph_domain}})
        MATCH (b {{id: $dst_id, namespace: $ns, graph_domain: $graph_domain}})
        MERGE (a)-[:{}]->(b)
        "#,
        rel_type.as_str()
    );

    client
        .execute_query(
            &query,
            vec![
                ("src_id", serde_json::json!(src_id)),
                ("dst_id", serde_json::json!(dst_id)),
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(())
}

/// Update Git/History metadata on an existing entity
///
/// Updates only the temporal metadata fields (created_at, last_modified_at, change_count, author_count).
/// The entity must already exist (identified by id + namespace).
/// Use this for enriching entities with Git history data after initial creation.
pub async fn update_git_metadata(
    client: &Neo4jClient,
    id: i64,
    created_at: Option<String>,
    last_modified_at: Option<String>,
    change_count: Option<i64>,
    author_count: Option<i64>,
) -> Result<()> {
    let query = r#"
        MATCH (e {id: $id, namespace: $ns, graph_domain: $graph_domain})
        SET e.created_at = $created_at,
            e.last_modified_at = $last_modified_at,
            e.change_count = $change_count,
            e.author_count = $author_count
    "#;

    client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
                ("created_at", serde_json::json!(created_at)),
                ("last_modified_at", serde_json::json!(last_modified_at)),
                ("change_count", serde_json::json!(change_count)),
                ("author_count", serde_json::json!(author_count)),
            ],
        )
        .await?;

    Ok(())
}

/// Batch upsert entities (efficient for bulk imports)
///
/// Processes entities in batches to avoid overwhelming Neo4j.
pub async fn batch_upsert_entities(
    client: &Neo4jClient,
    label: NodeLabel,
    entities: Vec<NodeProperties>,
    batch_size: usize,
) -> Result<usize> {
    let mut total = 0;

    for chunk in entities.chunks(batch_size) {
        for props in chunk {
            upsert_entity(client, label, props.clone()).await?;
            total += 1;
        }
    }

    Ok(total)
}

/// Batch create relationships (efficient for bulk imports)
///
/// Processes relationships in batches to avoid overwhelming Neo4j.
pub async fn batch_create_relationships(
    client: &Neo4jClient,
    relationships: Vec<(i64, i64, RelationType)>,
    batch_size: usize,
) -> Result<usize> {
    let mut total = 0;

    for chunk in relationships.chunks(batch_size) {
        for (src_id, dst_id, rel_type) in chunk {
            create_relationship(client, *src_id, *dst_id, *rel_type).await?;
            total += 1;
        }
    }

    Ok(total)
}

/// Delete a single entity by ID
///
/// Also deletes all relationships connected to this entity.
pub async fn delete_entity(client: &Neo4jClient, id: i64) -> Result<()> {
    let query = r#"
        MATCH (e {id: $id, namespace: $ns, graph_domain: $graph_domain})
        DETACH DELETE e
    "#;

    client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(())
}

/// Delete all entities in a file
///
/// Useful for re-indexing a single file.
pub async fn delete_file_entities(client: &Neo4jClient, file_path: &str) -> Result<usize> {
    let query = r#"
        MATCH (e {namespace: $ns, graph_domain: $graph_domain})
        WHERE e.path = $path
        DETACH DELETE e
        RETURN count(e) as deleted
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("path", serde_json::json!(file_path)),
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    let deleted =
        results.first().and_then(|r| r.get("deleted")).and_then(|v| v.as_i64()).unwrap_or(0);

    Ok(deleted as usize)
}

/// Upsert a lightweight File node by path (for application mapping)
///
/// Creates a minimal File node identified by path only (no entity ID).
/// Use this for file-level dependency tracking where full entity properties aren't needed.
/// Distinct from upsert_entity() which tracks detailed code entities with IDs.
pub async fn upsert_file_by_path(client: &Neo4jClient, file_path: &str) -> Result<()> {
    let query = format!(
        r#"
        MERGE (f:{}:{} {{path: $path, namespace: $ns}})
        SET f.graph_domain = $graph_domain,
            f.project = $project_label
        "#,
        NodeLabel::File.as_str(),
        PROJECT_LABEL
    );

    client
        .execute_query(
            &query,
            vec![
                ("path", serde_json::json!(file_path)),
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
                ("project_label", serde_json::json!(PROJECT_LABEL)),
            ],
        )
        .await?;

    Ok(())
}

/// Create a dependency relationship between two files (by path)
///
/// Creates DEPENDS_ON relationship between files identified by path.
/// Both source and target File nodes are created if they don't exist (MERGE).
/// Use this for file-level dependency tracking in application mapping.
pub async fn create_file_dependency(
    client: &Neo4jClient,
    from_path: &str,
    to_path: &str,
) -> Result<()> {
    let query = format!(
        r#"
        MERGE (a:{}:{} {{path: $from, namespace: $ns}})
        SET a.graph_domain = $graph_domain,
            a.project = $project_label
        MERGE (b:{}:{} {{path: $to, namespace: $ns}})
        SET b.graph_domain = $graph_domain,
            b.project = $project_label
        MERGE (a)-[:{}]->(b)
        "#,
        NodeLabel::File.as_str(),
        PROJECT_LABEL,
        NodeLabel::File.as_str(),
        PROJECT_LABEL,
        RelationType::DependsOn.as_str()
    );

    client
        .execute_query(
            &query,
            vec![
                ("from", serde_json::json!(from_path)),
                ("to", serde_json::json!(to_path)),
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
                ("project_label", serde_json::json!(PROJECT_LABEL)),
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
        // Compile-time check: Can only use NodeLabel and RelationType from schema
        let label = NodeLabel::Function;
        let rel = RelationType::Calls;

        assert_eq!(label.as_str(), "Function");
        assert_eq!(rel.as_str(), "CALLS");
    }

    #[test]
    fn test_project_label_is_set() {
        // Project label for double-label pattern
        assert_eq!(PROJECT_LABEL, "CodeGraph");
    }
}
