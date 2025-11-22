//! Batch Edge Pusher - Efficient batch MERGE operations to Neo4j
//!
//! Features:
//! - Batch processing for efficiency
//! - Idempotent MERGE operations (safe to re-run)
//! - Uniqueness enforcement via (src_id, dst_id, type)
//! - Support for all EdgeType variants

use crate::code_graph::EdgeType;
use crate::graph::Neo4jClient;
use anyhow::{Context, Result};

/// BatchEdgePusher handles efficient batch edge creation in Neo4j
pub struct BatchEdgePusher {
    neo4j: Neo4jClient,
    batch_size: usize,
}

impl BatchEdgePusher {
    /// Create a new BatchEdgePusher with default batch size of 100
    pub fn new(neo4j: Neo4jClient) -> Self {
        Self {
            neo4j,
            batch_size: 100,
        }
    }

    /// Create with custom batch size
    pub fn with_batch_size(neo4j: Neo4jClient, batch_size: usize) -> Self {
        Self { neo4j, batch_size }
    }

    /// Push a batch of edges to Neo4j using MERGE for idempotency
    ///
    /// Each edge is (src_id, dst_id, EdgeType). Uses MERGE to ensure
    /// uniqueness based on (src_id, dst_id, type) combination.
    ///
    /// Returns the number of edges processed
    pub async fn push_edges(&self, edges: &[(i64, i64, EdgeType)]) -> Result<usize> {
        let mut total_pushed = 0;

        for chunk in edges.chunks(self.batch_size) {
            let count = self.push_edge_batch(chunk).await?;
            total_pushed += count;
        }

        Ok(total_pushed)
    }

    /// Push a single batch of edges
    async fn push_edge_batch(&self, edges: &[(i64, i64, EdgeType)]) -> Result<usize> {
        if edges.is_empty() {
            return Ok(0);
        }

        // Build edge data as JSON array for UNWIND
        let edge_data: Vec<serde_json::Value> = edges
            .iter()
            .map(|(src_id, dst_id, edge_type)| {
                serde_json::json!({
                    "src": src_id,
                    "dst": dst_id,
                    "type": edge_type_to_rel(edge_type)
                })
            })
            .collect();

        // Use dynamic relationship creation with APOC or native Cypher
        // Since we can't use dynamic relationship types in pure Cypher MERGE,
        // we'll handle each edge type separately
        let mut total = 0;

        // Group edges by type for efficient batch processing
        let mut calls_edges = Vec::new();
        let mut imports_edges = Vec::new();
        let mut uses_edges = Vec::new();
        let mut inherits_edges = Vec::new();
        let mut references_edges = Vec::new();
        let mut contains_edges = Vec::new();
        let mut implements_edges = Vec::new();
        let mut uses_field_edges = Vec::new();
        let mut uses_type_edges = Vec::new();
        let mut module_child_edges = Vec::new();

        for (src, dst, et) in edges {
            match et {
                EdgeType::Calls => calls_edges.push((*src, *dst)),
                EdgeType::Imports => imports_edges.push((*src, *dst)),
                EdgeType::Uses => uses_edges.push((*src, *dst)),
                EdgeType::Inherits => inherits_edges.push((*src, *dst)),
                EdgeType::References => references_edges.push((*src, *dst)),
                EdgeType::Contains => contains_edges.push((*src, *dst)),
                EdgeType::Implements => implements_edges.push((*src, *dst)),
                EdgeType::UsesField => uses_field_edges.push((*src, *dst)),
                EdgeType::UsesType => uses_type_edges.push((*src, *dst)),
                EdgeType::ModuleChild => module_child_edges.push((*src, *dst)),
            }
        }

        // Push each type
        total += self.push_typed_edges(&calls_edges, "CALLS").await?;
        total += self.push_typed_edges(&imports_edges, "IMPORTS").await?;
        total += self.push_typed_edges(&uses_edges, "USES").await?;
        total += self.push_typed_edges(&inherits_edges, "INHERITS").await?;
        total += self.push_typed_edges(&references_edges, "REFERENCES").await?;
        total += self.push_typed_edges(&contains_edges, "CONTAINS").await?;
        total += self.push_typed_edges(&implements_edges, "IMPLEMENTS").await?;
        total += self.push_typed_edges(&uses_field_edges, "USES_FIELD").await?;
        total += self.push_typed_edges(&uses_type_edges, "USES_TYPE").await?;
        total += self.push_typed_edges(&module_child_edges, "MODULE_CHILD").await?;

        Ok(total)
    }

    /// Push edges of a specific relationship type
    async fn push_typed_edges(&self, edges: &[(i64, i64)], rel_type: &str) -> Result<usize> {
        if edges.is_empty() {
            return Ok(0);
        }

        let ns = self.neo4j.namespace();

        // Build edge list for UNWIND
        let edge_list: Vec<serde_json::Value> = edges
            .iter()
            .map(|(src, dst)| serde_json::json!({"src": src, "dst": dst}))
            .collect();

        // Use format! to create query with embedded relationship type
        // MERGE ensures idempotency - won't create duplicates
        let query = format!(
            r#"
            UNWIND $edges AS edge
            MATCH (src {{id: edge.src, namespace: $ns}})
            MATCH (dst {{id: edge.dst, namespace: $ns}})
            MERGE (src)-[r:{}]->(dst)
            RETURN count(r) as cnt
            "#,
            rel_type
        );

        let result = self
            .neo4j
            .execute_query(
                &query,
                vec![
                    ("edges", serde_json::json!(edge_list)),
                    ("ns", serde_json::json!(ns)),
                ],
            )
            .await
            .context("Failed to push edges to Neo4j")?;

        // Count from result
        let count = result
            .first()
            .and_then(|r| r.get("cnt"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Ok(count as usize)
    }

    /// Delete all edges from Neo4j (for rebuild)
    pub async fn clear_all_edges(&self) -> Result<u64> {
        let query = r#"
            MATCH ()-[r]->()
            WHERE startNode(r).namespace = $ns AND endNode(r).namespace = $ns
            DELETE r
            RETURN count(r) as deleted
        "#;

        let result = self
            .neo4j
            .execute_query(query, vec![("ns", serde_json::json!(self.neo4j.namespace()))])
            .await?;

        let deleted = result
            .first()
            .and_then(|r| r.get("deleted"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Ok(deleted)
    }
}

/// Push edges by name - creates nodes if they don't exist
/// This is useful when you don't have SQLite IDs but have extracted edges from source
impl BatchEdgePusher {
    /// Push edges by entity names (creates nodes if missing)
    ///
    /// Each edge is (src_name, dst_name, EdgeType). Uses MERGE to create nodes
    /// if they don't exist, then creates the relationship.
    pub async fn push_edges_by_name(&self, edges: &[(&str, &str, EdgeType)]) -> Result<usize> {
        let mut total_pushed = 0;

        for chunk in edges.chunks(self.batch_size) {
            let count = self.push_named_edge_batch(chunk).await?;
            total_pushed += count;
        }

        Ok(total_pushed)
    }

    /// Push a batch of named edges (grouped by type)
    async fn push_named_edge_batch(&self, edges: &[(&str, &str, EdgeType)]) -> Result<usize> {
        if edges.is_empty() {
            return Ok(0);
        }

        let mut total = 0;

        // Group by edge type and process each type
        for edge_type in [
            EdgeType::Calls, EdgeType::Imports, EdgeType::Uses,
            EdgeType::Inherits, EdgeType::References, EdgeType::Contains,
            EdgeType::Implements, EdgeType::UsesField, EdgeType::UsesType,
            EdgeType::ModuleChild,
        ] {
            let typed_edges: Vec<_> = edges.iter()
                .filter(|(_, _, et)| std::mem::discriminant(et) == std::mem::discriminant(&edge_type))
                .map(|(s, d, _)| (*s, *d))
                .collect();

            if typed_edges.is_empty() {
                continue;
            }

            let count = self.push_typed_named_edges(&typed_edges, &edge_type).await?;
            total += count;
        }

        Ok(total)
    }

    /// Push edges of a single type by name
    /// Uses individual MERGE queries per edge since neo4rs doesn't support array of maps well
    async fn push_typed_named_edges(&self, edges: &[(&str, &str)], edge_type: &EdgeType) -> Result<usize> {
        if edges.is_empty() {
            return Ok(0);
        }

        let rel_type = edge_type_to_rel(edge_type);
        eprintln!("  Pushing {} {} edges...", edges.len(), rel_type);

        let mut count = 0;
        let ns = self.neo4j.namespace().to_string();

        // Process each edge individually - not as efficient but reliable
        for (src, dst) in edges {
            let query = format!(r#"
                MERGE (s:CodeEntity {{name: $src, namespace: $ns}})
                MERGE (d:CodeEntity {{name: $dst, namespace: $ns}})
                MERGE (s)-[r:{}]->(d)
                RETURN count(r) as cnt
            "#, rel_type);

            let result = self.neo4j
                .execute_query(
                    &query,
                    vec![
                        ("src", serde_json::json!(src)),
                        ("dst", serde_json::json!(dst)),
                        ("ns", serde_json::json!(&ns)),
                    ],
                )
                .await;

            match result {
                Ok(_) => count += 1,
                Err(e) => {
                    eprintln!("    Warning: Failed to push edge {} -> {}: {}", src, dst, e);
                }
            }
        }

        eprintln!("  Created {} edges", count);
        Ok(count)
    }
}

/// Convert EdgeType to Neo4j relationship type string
fn edge_type_to_rel(et: &EdgeType) -> &'static str {
    match et {
        EdgeType::Calls => "CALLS",
        EdgeType::Imports => "IMPORTS",
        EdgeType::Uses => "USES",
        EdgeType::Inherits => "INHERITS",
        EdgeType::References => "REFERENCES",
        EdgeType::Contains => "CONTAINS",
        EdgeType::Implements => "IMPLEMENTS",
        EdgeType::UsesField => "USES_FIELD",
        EdgeType::UsesType => "USES_TYPE",
        EdgeType::ModuleChild => "MODULE_CHILD",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_type_to_rel() {
        assert_eq!(edge_type_to_rel(&EdgeType::Calls), "CALLS");
        assert_eq!(edge_type_to_rel(&EdgeType::Imports), "IMPORTS");
        assert_eq!(edge_type_to_rel(&EdgeType::Uses), "USES");
        assert_eq!(edge_type_to_rel(&EdgeType::Inherits), "INHERITS");
        assert_eq!(edge_type_to_rel(&EdgeType::Implements), "IMPLEMENTS");
    }
}
