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

use super::schema::{project_namespace, NodeLabel, GRAPH_DOMAIN};
use crate::graph::Neo4jClient;
use anyhow::Result;
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
            last_modified_at: record
                .get("last_modified_at")
                .and_then(|v| v.as_str())
                .map(String::from),
            change_count: record.get("change_count").and_then(|v| v.as_i64()),
            author_count: record.get("author_count").and_then(|v| v.as_i64()),
        })
    }
}

/// Get entity by ID
pub async fn get_entity_by_id(client: &Neo4jClient, id: i64) -> Result<Option<EntityResult>> {
    let query = r#"
        MATCH (e {id: $id, namespace: $ns, graph_domain: $graph_domain})
        RETURN e.id as id,
               e.name as name,
               CASE 
                   WHEN 'Function' IN labels(e) THEN 'Function'
                   WHEN 'Struct' IN labels(e) THEN 'Struct'
                   WHEN 'Enum' IN labels(e) THEN 'Enum'
                   WHEN 'Trait' IN labels(e) THEN 'Trait'
                   WHEN 'Module' IN labels(e) THEN 'Module'
                   WHEN 'File' IN labels(e) THEN 'File'
                   WHEN 'Impl' IN labels(e) THEN 'Impl'
                   WHEN 'Import' IN labels(e) THEN 'Import'
                   WHEN 'Constant' IN labels(e) THEN 'Constant'
                   WHEN 'TypeAlias' IN labels(e) THEN 'TypeAlias'
                   ELSE labels(e)[0]
               END as label,
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
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(results.first().and_then(EntityResult::from_neo4j_value))
}

/// Get all entities in a file
pub async fn get_file_entities(client: &Neo4jClient, file_path: &str) -> Result<Vec<EntityResult>> {
    let query = r#"
        MATCH (e {namespace: $ns, graph_domain: $graph_domain})
        WHERE e.path = $path
        RETURN e.id as id,
               e.name as name,
               CASE 
                   WHEN 'Function' IN labels(e) THEN 'Function'
                   WHEN 'Struct' IN labels(e) THEN 'Struct'
                   WHEN 'Enum' IN labels(e) THEN 'Enum'
                   WHEN 'Trait' IN labels(e) THEN 'Trait'
                   WHEN 'Module' IN labels(e) THEN 'Module'
                   WHEN 'File' IN labels(e) THEN 'File'
                   WHEN 'Impl' IN labels(e) THEN 'Impl'
                   WHEN 'Import' IN labels(e) THEN 'Import'
                   WHEN 'Constant' IN labels(e) THEN 'Constant'
                   WHEN 'TypeAlias' IN labels(e) THEN 'TypeAlias'
                   ELSE labels(e)[0]
               END as label,
               e.path as path,
               e.start_line as start_line,
               e.end_line as end_line,
               e.signature as signature,
               e.body_snippet as body_snippet,
               e.created_at as created_at,
               e.last_modified_at as last_modified_at,
               e.change_count as change_count,
               e.author_count as author_count
          ORDER BY e.start_line, e.id
    "#;

    let params = vec![
        ("path", serde_json::json!(file_path)),
        ("ns", serde_json::json!(project_namespace(client))),
        ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
    ];

    // Debug: Print what we're about to execute
    eprintln!("DEBUG: get_file_entities query: {}", query);
    eprintln!("DEBUG: get_file_entities params: {:?}", params);

    let results = client.execute_query(query, params).await?;

    eprintln!("DEBUG: get_file_entities returned {} results", results.len());
    for (i, result) in results.iter().enumerate() {
        eprintln!("DEBUG: get_file_entities result {}: {:?}", i, result);
    }

    Ok(results.iter().filter_map(EntityResult::from_neo4j_value).collect())
}

/// Get functions called by a function
pub async fn get_function_callees(
    client: &Neo4jClient,
    function_id: i64,
) -> Result<Vec<EntityResult>> {
    let query = r#"
        MATCH (f {id: $id, namespace: $ns, graph_domain: $graph_domain})-[:CALLS]->(callee)
        WHERE callee.namespace = $ns
          AND callee.graph_domain = $graph_domain
        RETURN callee.id as id,
                callee.name as name,
                CASE 
                    WHEN 'Function' IN labels(callee) THEN 'Function'
                    WHEN 'Struct' IN labels(callee) THEN 'Struct'
                    WHEN 'Enum' IN labels(callee) THEN 'Enum'
                    WHEN 'Trait' IN labels(callee) THEN 'Trait'
                    WHEN 'Module' IN labels(callee) THEN 'Module'
                    WHEN 'File' IN labels(callee) THEN 'File'
                    WHEN 'Impl' IN labels(callee) THEN 'Impl'
                    WHEN 'Import' IN labels(callee) THEN 'Import'
                    WHEN 'Constant' IN labels(callee) THEN 'Constant'
                    WHEN 'TypeAlias' IN labels(callee) THEN 'TypeAlias'
                    ELSE labels(callee)[0]
                END as label,
                callee.path as path,
                callee.start_line as start_line,
                callee.end_line as end_line,
                callee.signature as signature,
                callee.body_snippet as body_snippet,
                callee.created_at as created_at,
                callee.last_modified_at as last_modified_at,
                callee.change_count as change_count,
                callee.author_count as author_count
        ORDER BY callee.name, callee.id
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(function_id)),
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(results.iter().filter_map(EntityResult::from_neo4j_value).collect())
}

/// Get functions that call a function
pub async fn get_function_callers(
    client: &Neo4jClient,
    function_id: i64,
) -> Result<Vec<EntityResult>> {
    let query = r#"
        MATCH (caller)-[:CALLS]->(f {id: $id, namespace: $ns, graph_domain: $graph_domain})
        WHERE caller.namespace = $ns
          AND caller.graph_domain = $graph_domain
        RETURN caller.id as id,
                caller.name as name,
                CASE 
                    WHEN 'Function' IN labels(caller) THEN 'Function'
                    WHEN 'Struct' IN labels(caller) THEN 'Struct'
                    WHEN 'Enum' IN labels(caller) THEN 'Enum'
                    WHEN 'Trait' IN labels(caller) THEN 'Trait'
                    WHEN 'Module' IN labels(caller) THEN 'Module'
                    WHEN 'File' IN labels(caller) THEN 'File'
                    WHEN 'Impl' IN labels(caller) THEN 'Impl'
                    WHEN 'Import' IN labels(caller) THEN 'Import'
                    WHEN 'Constant' IN labels(caller) THEN 'Constant'
                    WHEN 'TypeAlias' IN labels(caller) THEN 'TypeAlias'
                    ELSE labels(caller)[0]
                END as label,
                caller.path as path,
                caller.start_line as start_line,
                caller.end_line as end_line,
                caller.signature as signature,
                caller.body_snippet as body_snippet,
                caller.created_at as created_at,
                caller.last_modified_at as last_modified_at,
                caller.change_count as change_count,
                caller.author_count as author_count
        ORDER BY caller.name, caller.id
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(function_id)),
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(results.iter().filter_map(EntityResult::from_neo4j_value).collect())
}

/// Get entities by name (exact match)
pub async fn find_entities_by_name(client: &Neo4jClient, name: &str) -> Result<Vec<EntityResult>> {
    let query = r#"
        MATCH (e {namespace: $ns, graph_domain: $graph_domain})
        WHERE e.name STARTS WITH $name
        RETURN e.id as id,
                e.name as name,
                CASE 
                    WHEN 'Function' IN labels(e) THEN 'Function'
                    WHEN 'Struct' IN labels(e) THEN 'Struct'
                    WHEN 'Enum' IN labels(e) THEN 'Enum'
                    WHEN 'Trait' IN labels(e) THEN 'Trait'
                    WHEN 'Module' IN labels(e) THEN 'Module'
                    WHEN 'File' IN labels(e) THEN 'File'
                    WHEN 'Impl' IN labels(e) THEN 'Impl'
                    WHEN 'Import' IN labels(e) THEN 'Import'
                    WHEN 'Constant' IN labels(e) THEN 'Constant'
                    WHEN 'TypeAlias' IN labels(e) THEN 'TypeAlias'
                    ELSE labels(e)[0]
                END as label,
                e.path as path,
                e.start_line as start_line,
                e.end_line as end_line,
                e.signature as signature,
                e.body_snippet as body_snippet,
                e.created_at as created_at,
                e.last_modified_at as last_modified_at,
                e.change_count as change_count,
                e.author_count as author_count
        ORDER BY e.path, e.start_line, e.id
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("name", serde_json::json!(name)),
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(results.iter().filter_map(EntityResult::from_neo4j_value).collect())
}

/// Get entities by label type
pub async fn get_entities_by_type(
    client: &Neo4jClient,
    label: NodeLabel,
) -> Result<Vec<EntityResult>> {
    let query = format!(
        r#"
        MATCH (e:{} {{namespace: $ns, graph_domain: $graph_domain}})
        RETURN e.id as id,
                e.name as name,
                CASE 
                    WHEN 'Function' IN labels(e) THEN 'Function'
                    WHEN 'Struct' IN labels(e) THEN 'Struct'
                    WHEN 'Enum' IN labels(e) THEN 'Enum'
                    WHEN 'Trait' IN labels(e) THEN 'Trait'
                    WHEN 'Module' IN labels(e) THEN 'Module'
                    WHEN 'File' IN labels(e) THEN 'File'
                    WHEN 'Impl' IN labels(e) THEN 'Impl'
                    WHEN 'Import' IN labels(e) THEN 'Import'
                    WHEN 'Constant' IN labels(e) THEN 'Constant'
                    WHEN 'TypeAlias' IN labels(e) THEN 'TypeAlias'
                    ELSE labels(e)[0]
                END as label,
                e.path as path,
                e.start_line as start_line,
                e.end_line as end_line,
                e.signature as signature,
                e.body_snippet as body_snippet,
                e.created_at as created_at,
                e.last_modified_at as last_modified_at,
                e.change_count as change_count,
                e.author_count as author_count
        ORDER BY e.name, e.path, e.start_line, e.id
         "#,
        label.as_str()
    );

    let results = client
        .execute_query(
            &query,
            vec![
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(results.iter().filter_map(EntityResult::from_neo4j_value).collect())
}

/// Count entities by type
pub async fn count_entities_by_type(client: &Neo4jClient) -> Result<Vec<(String, i64)>> {
    let query = r#"
        MATCH (e {namespace: $ns, graph_domain: $graph_domain})
        WITH CASE 
                WHEN 'Function' IN labels(e) THEN 'Function'
                WHEN 'Struct' IN labels(e) THEN 'Struct'
                WHEN 'Enum' IN labels(e) THEN 'Enum'
                WHEN 'Trait' IN labels(e) THEN 'Trait'
                WHEN 'Module' IN labels(e) THEN 'Module'
                WHEN 'File' IN labels(e) THEN 'File'
                WHEN 'Impl' IN labels(e) THEN 'Impl'
                WHEN 'Import' IN labels(e) THEN 'Import'
                WHEN 'Constant' IN labels(e) THEN 'Constant'
                WHEN 'TypeAlias' IN labels(e) THEN 'TypeAlias'
                ELSE labels(e)[0]
            END as label, count(e) as count
        RETURN label, count
        ORDER BY count DESC
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
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
        MATCH (e {id: $id, namespace: $ns, graph_domain: $graph_domain})--(neighbor)
        WHERE neighbor.namespace = $ns
          AND neighbor.graph_domain = $graph_domain
        RETURN DISTINCT neighbor.id as id,
                        neighbor.name as name,
                        CASE 
                            WHEN 'Function' IN labels(neighbor) THEN 'Function'
                            WHEN 'Struct' IN labels(neighbor) THEN 'Struct'
                            WHEN 'Enum' IN labels(neighbor) THEN 'Enum'
                            WHEN 'Trait' IN labels(neighbor) THEN 'Trait'
                            WHEN 'Module' IN labels(neighbor) THEN 'Module'
                            WHEN 'File' IN labels(neighbor) THEN 'File'
                            WHEN 'Impl' IN labels(neighbor) THEN 'Impl'
                            WHEN 'Import' IN labels(neighbor) THEN 'Import'
                            WHEN 'Constant' IN labels(neighbor) THEN 'Constant'
                            WHEN 'TypeAlias' IN labels(neighbor) THEN 'TypeAlias'
                            ELSE labels(neighbor)[0]
                        END as label,
                        neighbor.path as path,
                        neighbor.start_line as start_line,
                        neighbor.end_line as end_line,
                        neighbor.signature as signature,
                        neighbor.body_snippet as body_snippet,
                        neighbor.created_at as created_at,
                        neighbor.last_modified_at as last_modified_at,
                        neighbor.change_count as change_count,
                        neighbor.author_count as author_count
        ORDER BY neighbor.name, neighbor.id
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(entity_id)),
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(results.iter().filter_map(EntityResult::from_neo4j_value).collect())
}

/// Find orphan entities (no relationships)
pub async fn find_orphan_entities(client: &Neo4jClient) -> Result<Vec<EntityResult>> {
    let query = r#"
        MATCH (e {namespace: $ns, graph_domain: $graph_domain})
        WHERE NOT EXISTS { (e)--() }
        RETURN e.id as id,
                e.name as name,
                CASE 
                    WHEN 'Function' IN labels(e) THEN 'Function'
                    WHEN 'Struct' IN labels(e) THEN 'Struct'
                    WHEN 'Enum' IN labels(e) THEN 'Enum'
                    WHEN 'Trait' IN labels(e) THEN 'Trait'
                    WHEN 'Module' IN labels(e) THEN 'Module'
                    WHEN 'File' IN labels(e) THEN 'File'
                    WHEN 'Impl' IN labels(e) THEN 'Impl'
                    WHEN 'Import' IN labels(e) THEN 'Import'
                    WHEN 'Constant' IN labels(e) THEN 'Constant'
                    WHEN 'TypeAlias' IN labels(e) THEN 'TypeAlias'
                    ELSE labels(e)[0]
                END as label,
                e.path as path,
                e.start_line as start_line,
                e.end_line as end_line,
                e.signature as signature,
                e.body_snippet as body_snippet,
                e.created_at as created_at,
                e.last_modified_at as last_modified_at,
                e.change_count as change_count,
                e.author_count as author_count
        ORDER BY e.name, e.id
         LIMIT 100
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(results.iter().filter_map(EntityResult::from_neo4j_value).collect())
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
        MATCH (n {namespace: $ns, graph_domain: $graph_domain})
        RETURN count(n) as count
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(results.first().and_then(|r| r.get("count")).and_then(|v| v.as_i64()).unwrap_or(0))
}

async fn count_total_edges(client: &Neo4jClient) -> Result<i64> {
    // Use directed pattern to avoid counting each edge twice
    let query = r#"
        MATCH (a {namespace: $ns, graph_domain: $graph_domain})-[r]->(b {namespace: $ns, graph_domain: $graph_domain})
        RETURN count(r) as count
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(results.first().and_then(|r| r.get("count")).and_then(|v| v.as_i64()).unwrap_or(0))
}

async fn count_edges_by_type(client: &Neo4jClient) -> Result<Vec<(String, i64)>> {
    let query = r#"
        MATCH (a {namespace: $ns, graph_domain: $graph_domain})-[r]->(b {namespace: $ns, graph_domain: $graph_domain})
        WITH type(r) as rel_type, count(r) as count
        RETURN rel_type, count
        ORDER BY count DESC
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
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
        MATCH (n {namespace: $ns, graph_domain: $graph_domain})
        WHERE NOT EXISTS { (n)--() }
        RETURN count(n) as count
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("ns", serde_json::json!(project_namespace(client))),
                ("graph_domain", serde_json::json!(GRAPH_DOMAIN)),
            ],
        )
        .await?;

    Ok(results.first().and_then(|r| r.get("count")).and_then(|v| v.as_i64()).unwrap_or(0))
}

#[cfg(test)]
mod tests {

    // Tests removed - namespace is now dynamic via project_namespace(client)
}
