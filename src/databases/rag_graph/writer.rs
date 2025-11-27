//! RAG Graph Write Operations
//!
//! THE ONLY place where RAG graph writes to Neo4j happen.
//! Follows same patterns as canonical Neo4j module:
//! - Hardcoded queries
//! - Parameterized
//! - Validated against schema
//! - Idempotent (uses MERGE)
//! - Namespace isolated

use anyhow::Result;
use crate::graph::Neo4jClient;
use super::schema::{NodeLabel, EmbeddingProperties, RelationType, RAG_PROJECT_LABEL, rag_namespace};

/// Create or update an embedding node
///
/// Uses MERGE for idempotency - safe to call multiple times.
/// Schema: :Embedding:SynCore with RAG-specific properties
pub async fn upsert_embedding(
    client: &Neo4jClient,
    props: EmbeddingProperties,
) -> Result<()> {
    // Use double label pattern: :Embedding:SynCore
    let query = format!(
        r#"
        MERGE (e:{}:{} {{id: $id, namespace: $ns}})
        SET e.text = $text,
            e.metadata = $metadata
        "#,
        NodeLabel::Embedding.as_str(),
        RAG_PROJECT_LABEL
    );

    client
        .execute_query(
            &query,
            vec![
                ("id", serde_json::json!(props.id)),
                ("ns", serde_json::json!(rag_namespace(client))),
                ("text", serde_json::json!(props.text)),
                ("metadata", serde_json::json!(props.metadata)),
            ],
        )
        .await?;

    Ok(())
}

/// Create a relationship between two embeddings or embedding and entity
///
/// Uses MERGE for idempotency. Supports optional weight property.
pub async fn create_relationship(
    client: &Neo4jClient,
    src_id: i64,
    dst_id: i64,
    rel_type: RelationType,
    weight: Option<f32>,
) -> Result<()> {
    let query = if let Some(_w) = weight {
        format!(
            r#"
            MATCH (a {{id: $src_id, namespace: $ns}})
            MATCH (b {{id: $dst_id, namespace: $ns}})
            MERGE (a)-[r:{}]->(b)
            SET r.weight = $weight
            "#,
            rel_type.as_str()
        )
    } else {
        format!(
            r#"
            MATCH (a {{id: $src_id, namespace: $ns}})
            MATCH (b {{id: $dst_id, namespace: $ns}})
            MERGE (a)-[:{}]->(b)
            "#,
            rel_type.as_str()
        )
    };

    let mut params = vec![
        ("src_id", serde_json::json!(src_id)),
        ("dst_id", serde_json::json!(dst_id)),
        ("ns", serde_json::json!(rag_namespace(client))),
    ];

    if let Some(w) = weight {
        params.push(("weight", serde_json::json!(w)));
    }

    client.execute_query(&query, params).await?;

    Ok(())
}

/// Batch upsert embeddings (efficient for bulk imports)
pub async fn batch_upsert_embeddings(
    client: &Neo4jClient,
    embeddings: Vec<EmbeddingProperties>,
    batch_size: usize,
) -> Result<usize> {
    let mut total = 0;

    for chunk in embeddings.chunks(batch_size) {
        for props in chunk {
            upsert_embedding(client, props.clone()).await?;
            total += 1;
        }
    }

    Ok(total)
}

/// Delete an embedding by ID
pub async fn delete_embedding(client: &Neo4jClient, id: i64) -> Result<()> {
    let query = r#"
        MATCH (e {id: $id, namespace: $ns})
        DETACH DELETE e
    "#;

    client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(rag_namespace(client))),
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
        let label = NodeLabel::Embedding;
        let rel = RelationType::SimilarTo;

        assert_eq!(label.as_str(), "Embedding");
        assert_eq!(rel.as_str(), "SIMILAR_TO");
    }

    #[test]
    fn test_project_label_is_set() {
        // Project label for double-label pattern
        assert_eq!(RAG_PROJECT_LABEL, "SynCore");
    }
}
