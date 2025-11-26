//! RAG Graph Read Operations
//!
//! THE ONLY place where RAG graph reads from Neo4j happen.
//! Provides type-safe queries for embedding retrieval and neighbor discovery.

use anyhow::Result;
use crate::graph::Neo4jClient;
use super::schema::{rag_namespace, RelationType};

/// Result of an embedding query
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    pub id: i64,
    pub text: String,
    pub metadata: Option<String>,
}

impl EmbeddingResult {
    /// Parse from Neo4j query result
    pub fn from_neo4j_value(value: &serde_json::Value) -> Option<Self> {
        Some(EmbeddingResult {
            id: value.get("id")?.as_i64()?,
            text: value.get("text")?.as_str()?.to_string(),
            metadata: value
                .get("metadata")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }
}

/// Neighbor with relationship information
#[derive(Debug, Clone)]
pub struct NeighborResult {
    pub id: i64,
    pub weight: Option<f32>,
    pub rel_type: Option<RelationType>,
}

/// Get an embedding by ID
pub async fn get_embedding_by_id(client: &Neo4jClient, id: i64) -> Result<Option<EmbeddingResult>> {
    let query = r#"
        MATCH (e:Embedding {id: $id, namespace: $ns})
        RETURN e.id as id,
               e.text as text,
               e.metadata as metadata
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(rag_namespace(client))),
            ],
        )
        .await?;

    Ok(results
        .first()
        .and_then(EmbeddingResult::from_neo4j_value))
}

/// Get neighbors of an embedding (any relationship)
///
/// Returns list of (neighbor_id, weight, rel_type) tuples.
/// Sorted by neighbor_id for deterministic ordering.
pub async fn get_neighbors(
    client: &Neo4jClient,
    entity_id: i64,
) -> Result<Vec<NeighborResult>> {
    let query = r#"
        MATCH (e {id: $entity_id, namespace: $ns})-[r]-(neighbor)
        WHERE neighbor.namespace = $ns
        RETURN neighbor.id as neighbor_id,
               COALESCE(r.weight, 1.0) as weight,
               type(r) as rel_type
        ORDER BY neighbor.id
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("entity_id", serde_json::json!(entity_id)),
                ("ns", serde_json::json!(rag_namespace(client))),
            ],
        )
        .await?;

    let neighbors: Vec<NeighborResult> = results
        .iter()
        .filter_map(|record| {
            let id = record.get("neighbor_id")?.as_i64()?;
            let weight = record.get("weight").and_then(|v| v.as_f64()).map(|w| w as f32);
            let rel_type = record
                .get("rel_type")
                .and_then(|v| v.as_str())
                .and_then(RelationType::from_str);

            Some(NeighborResult {
                id,
                weight,
                rel_type,
            })
        })
        .collect();

    Ok(neighbors)
}

/// Get text for an embedding (lightweight query for vector search)
pub async fn get_embedding_text(client: &Neo4jClient, id: i64) -> Result<Option<String>> {
    let query = r#"
        MATCH (e:Embedding {id: $id, namespace: $ns})
        RETURN e.text as text
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(rag_namespace(client))),
            ],
        )
        .await?;

    Ok(results
        .first()
        .and_then(|r| r.get("text"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string()))
}

/// Count embeddings in the graph
pub async fn count_embeddings(client: &Neo4jClient) -> Result<i64> {
    let query = r#"
        MATCH (e:Embedding {namespace: $ns})
        RETURN count(e) as count
    "#;

    let results = client
        .execute_query(
            query,
            vec![("ns", serde_json::json!(rag_namespace(client)))],
        )
        .await?;

    Ok(results
        .first()
        .and_then(|r| r.get("count"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_result_parsing() {
        let json = serde_json::json!({
            "id": 42,
            "text": "test embedding",
            "metadata": "{\"source\": \"test\"}"
        });

        let result = EmbeddingResult::from_neo4j_value(&json).unwrap();
        assert_eq!(result.id, 42);
        assert_eq!(result.text, "test embedding");
        assert_eq!(result.metadata, Some("{\"source\": \"test\"}".to_string()));
    }
}
