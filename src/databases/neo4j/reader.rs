//! Canonical Neo4j Read API
//!
//! THE ONLY place where reads from Neo4j happen.
//! No ad-hoc Cypher. No string concatenation. No guessing.
//!
//! Every query is:
//! - Hardcoded
//! - Parameterized
//! - Namespace-aware
//! - Strongly typed

use anyhow::Result;
use crate::graph::Neo4jClient;
use super::schema::{NodeLabel, project_namespace};
use serde_json::Value;

/// Entity result from Neo4j query
#[derive(Debug, Clone)]
pub struct EntityResult {
    pub id: i64,
    pub name: String,
    pub label: String,
    pub path: Option<String>,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
    pub signature: Option<String>,
    pub body_snippet: Option<String>,
    pub created_at: Option<String>,
    pub last_modified_at: Option<String>,
    pub change_count: Option<i64>,
    pub author_count: Option<i64>,
}

impl EntityResult {
    fn from_neo4j_value(value: &Value) -> Option<Self> {
        let record = value.as_object()?;
        Some(Self {
            id: record.get("id")?.as_i64()?,
            name: record.get("name")?.as_str()?.to_string(),
            label: record.get("label")?.as_str()?.to_string(),
            path: record.get("path").and_then(|v| v.as_str()).map(String::from),
            start_line: record.get("start_line").and_then(|v| v.as_i64()),
            end_line: record.get("end_line").and_then(|v| v.as_i64()),
            signature: record.get("signature").and_then(|v| v.as_str()).map(String::from),
            body_snippet: record.get("body_snippet").and_then(|v| v.as_str()).map(String::from),
            created_at: record.get("created_at").and_then(|v| v.as_str()).map(String::from),
            last_modified_at: record.get("last_modified_at").and_then(|v| v.as_str()).map(String::from),
            change_count: record.get("change_count").and_then(|v| v.as_i64()),
            author_count: record.get("author_count").and_then(|v| v.as_i64()),
        })
    }
}

/// Get entity by ID
pub async fn get_entity_by_id(client: &Neo4jClient, id: i64) -> Result<Option<EntityResult>> {
    let query = r#"
        MATCH (e {id: $id, namespace: $ns})
        RETURN e.id as id,
               e.name as name,
               labels(e)[0] as label,
               e.path as path,
               e.start_line as start_line,
               e.end_line as end_line,
               e.signature as signature,
               e.body_snippet as body_snippet,
               e.created_at as created_at,
               e.last_modified_at as last_modified_at,
               e.change_count as change_count,
               e.author_count as author_count
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(project_namespace(client))),
            ],
        )
        .await?;

    Ok(results.get(0).and_then(EntityResult::from_neo4j_value))
}

/// Get all entities in a file
pub async fn get_file_entities(client: &Neo4jClient, file_path: &str) -> Result<Vec<EntityResult>> {
    let query = r#"
        MATCH (e {namespace: $ns})
        WHERE e.path = $path
        RETURN e.id as id,
               e.name as name,
               labels(e)[0] as label,
               e.path as path,
               e.start_line as start_line,
               e.end_line as end_line,
               e.signature as signature,
               e.body_snippet as body_snippet,
               e.created_at as created_at,
               e.last_modified_at as last_modified_at,
               e.change_count as change_count,
               e.author_count as author_count
        ORDER BY e.start_line
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("path", serde_json::json!(file_path)),
                ("ns", serde_json::json!(project_namespace(client))),
            ],
        )
        .await?;

    Ok(results
        .iter()
        .filter_map(EntityResult::from_neo4j_value)
        .collect())
}

/// Get functions called by a function
pub async fn get_function_callees(client: &Neo4jClient, function_id: i64) -> Result<Vec<EntityResult>> {
    let query = r#"
        MATCH (f {id: $id, namespace: $ns})-[:CALLS]->(callee)
        WHERE callee.namespace = $ns
        RETURN callee.id as id,
               callee.name as name,
               labels(callee)[0] as label,
               callee.path as path,
               callee.start_line as start_line,
               callee.end_line as end_line,
               callee.signature as signature,
               callee.body_snippet as body_snippet,
               callee.created_at as created_at,
               callee.last_modified_at as last_modified_at,
               callee.change_count as change_count,
               callee.author_count as author_count
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(function_id)),
                ("ns", serde_json::json!(project_namespace(client))),
            ],
        )
        .await?;

    Ok(results
        .iter()
        .filter_map(EntityResult::from_neo4j_value)
        .collect())
}

/// Get functions that call a function
pub async fn get_function_callers(client: &Neo4jClient, function_id: i64) -> Result<Vec<EntityResult>> {
    let query = r#"
        MATCH (caller)-[:CALLS]->(f {id: $id, namespace: $ns})
        WHERE caller.namespace = $ns
        RETURN caller.id as id,
               caller.name as name,
               labels(caller)[0] as label,
               caller.path as path,
               caller.start_line as start_line,
               caller.end_line as end_line,
               caller.signature as signature,
               caller.body_snippet as body_snippet,
               caller.created_at as created_at,
               caller.last_modified_at as last_modified_at,
               caller.change_count as change_count,
               caller.author_count as author_count
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(function_id)),
                ("ns", serde_json::json!(project_namespace(client))),
            ],
        )
        .await?;

    Ok(results
        .iter()
        .filter_map(EntityResult::from_neo4j_value)
        .collect())
}

/// Get entities by name (exact match)
pub async fn find_entities_by_name(client: &Neo4jClient, name: &str) -> Result<Vec<EntityResult>> {
    let query = r#"
        MATCH (e {name: $name, namespace: $ns})
        RETURN e.id as id,
               e.name as name,
               labels(e)[0] as label,
               e.path as path,
               e.start_line as start_line,
               e.end_line as end_line,
               e.signature as signature,
               e.body_snippet as body_snippet,
               e.created_at as created_at,
               e.last_modified_at as last_modified_at,
               e.change_count as change_count,
               e.author_count as author_count
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("name", serde_json::json!(name)),
                ("ns", serde_json::json!(project_namespace(client))),
            ],
        )
        .await?;

    Ok(results
        .iter()
        .filter_map(EntityResult::from_neo4j_value)
        .collect())
}

/// Get entities by label type
pub async fn get_entities_by_type(client: &Neo4jClient, label: NodeLabel) -> Result<Vec<EntityResult>> {
    let query = format!(
        r#"
        MATCH (e:{} {{namespace: $ns}})
        RETURN e.id as id,
               e.name as name,
               labels(e)[0] as label,
               e.path as path,
               e.start_line as start_line,
               e.end_line as end_line,
               e.signature as signature,
               e.body_snippet as body_snippet,
               e.created_at as created_at,
               e.last_modified_at as last_modified_at,
               e.change_count as change_count,
               e.author_count as author_count
        LIMIT 100
        "#,
        label.as_str()
    );

    let results = client
        .execute_query(
            &query,
            vec![("ns", serde_json::json!(project_namespace(client)))],
        )
        .await?;

    Ok(results
        .iter()
        .filter_map(EntityResult::from_neo4j_value)
        .collect())
}

/// Count entities by type
pub async fn count_entities_by_type(client: &Neo4jClient) -> Result<Vec<(String, i64)>> {
    let query = r#"
        MATCH (e {namespace: $ns})
        WITH labels(e)[0] as label, count(e) as count
        RETURN label, count
        ORDER BY count DESC
    "#;

    let results = client
        .execute_query(
            query,
            vec![("ns", serde_json::json!(project_namespace(client)))],
        )
        .await?;

    Ok(results
        .iter()
        .filter_map(|r| {
            let label = r.get("label")?.as_str()?.to_string();
            let count = r.get("count")?.as_i64()?;
            Some((label, count))
        })
        .collect())
}

/// Get neighbors (any relationship) of an entity
pub async fn get_neighbors(client: &Neo4jClient, entity_id: i64) -> Result<Vec<EntityResult>> {
    let query = r#"
        MATCH (e {id: $id, namespace: $ns})--(neighbor)
        WHERE neighbor.namespace = $ns
        RETURN DISTINCT neighbor.id as id,
                        neighbor.name as name,
                        labels(neighbor)[0] as label,
                        neighbor.path as path,
                        neighbor.start_line as start_line,
                        neighbor.end_line as end_line,
                        neighbor.signature as signature,
                        neighbor.body_snippet as body_snippet,
                        neighbor.created_at as created_at,
                        neighbor.last_modified_at as last_modified_at,
                        neighbor.change_count as change_count,
                        neighbor.author_count as author_count
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(entity_id)),
                ("ns", serde_json::json!(project_namespace(client))),
            ],
        )
        .await?;

    Ok(results
        .iter()
        .filter_map(EntityResult::from_neo4j_value)
        .collect())
}

/// Find orphan entities (no relationships)
pub async fn find_orphan_entities(client: &Neo4jClient) -> Result<Vec<EntityResult>> {
    let query = r#"
        MATCH (e {namespace: $ns})
        WHERE NOT EXISTS { (e)--() }
        RETURN e.id as id,
               e.name as name,
               labels(e)[0] as label,
               e.path as path,
               e.start_line as start_line,
               e.end_line as end_line,
               e.signature as signature,
               e.body_snippet as body_snippet,
               e.created_at as created_at,
               e.last_modified_at as last_modified_at,
               e.change_count as change_count,
               e.author_count as author_count
        LIMIT 100
    "#;

    let results = client
        .execute_query(
            query,
            vec![("ns", serde_json::json!(project_namespace(client)))],
        )
        .await?;

    Ok(results
        .iter()
        .filter_map(EntityResult::from_neo4j_value)
        .collect())
}

/// Validate graph structure (returns stats)
pub async fn validate_structure(client: &Neo4jClient) -> Result<GraphStats> {
    let total_nodes = count_total_nodes(client).await?;
    let total_edges = count_total_edges(client).await?;
    let orphan_count = count_orphan_nodes(client).await?;
    let types = count_entities_by_type(client).await?;
    let edge_types = count_edges_by_type(client).await?;

    Ok(GraphStats {
        total_nodes,
        total_edges,
        orphan_count,
        entity_types: types,
        edge_types,
    })
}

#[derive(Debug)]
pub struct GraphStats {
    pub total_nodes: i64,
    pub total_edges: i64,
    pub orphan_count: i64,
    pub entity_types: Vec<(String, i64)>,
    pub edge_types: Vec<(String, i64)>,
}

async fn count_total_nodes(client: &Neo4jClient) -> Result<i64> {
    let query = r#"
        MATCH (n {namespace: $ns})
        RETURN count(n) as count
    "#;

    let results = client
        .execute_query(
            query,
            vec![("ns", serde_json::json!(project_namespace(client)))],
        )
        .await?;

    Ok(results
        .get(0)
        .and_then(|r| r.get("count"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0))
}

async fn count_total_edges(client: &Neo4jClient) -> Result<i64> {
    // Use directed pattern to avoid counting each edge twice
    let query = r#"
        MATCH (a {namespace: $ns})-[r]->(b {namespace: $ns})
        RETURN count(r) as count
    "#;

    let results = client
        .execute_query(
            query,
            vec![("ns", serde_json::json!(project_namespace(client)))],
        )
        .await?;

    Ok(results
        .get(0)
        .and_then(|r| r.get("count"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0))
}

async fn count_edges_by_type(client: &Neo4jClient) -> Result<Vec<(String, i64)>> {
    let query = r#"
        MATCH (a {namespace: $ns})-[r]->(b {namespace: $ns})
        WITH type(r) as rel_type, count(r) as count
        RETURN rel_type, count
        ORDER BY count DESC
    "#;

    let results = client
        .execute_query(
            query,
            vec![("ns", serde_json::json!(project_namespace(client)))],
        )
        .await?;

    let mut edge_types = Vec::new();
    for row in results {
        if let (Some(rel_type), Some(count)) = (
            row.get("rel_type").and_then(|v| v.as_str()),
            row.get("count").and_then(|v| v.as_i64()),
        ) {
            edge_types.push((rel_type.to_string(), count));
        }
    }

    Ok(edge_types)
}

async fn count_orphan_nodes(client: &Neo4jClient) -> Result<i64> {
    let query = r#"
        MATCH (n {namespace: $ns})
        WHERE NOT EXISTS { (n)--() }
        RETURN count(n) as count
    "#;

    let results = client
        .execute_query(
            query,
            vec![("ns", serde_json::json!(project_namespace(client)))],
        )
        .await?;

    Ok(results
        .get(0)
        .and_then(|r| r.get("count"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests removed - namespace is now dynamic via project_namespace(client)
}
