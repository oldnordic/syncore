//! Neo4j relationship creation for code graph edges
//!
//! This module handles creating Neo4j relationships between CodeEntity nodes
//! based on CodeEdge data from the code graph.

use super::types::{CodeEdge, EdgeType};
use crate::graph::Neo4jClient;
use anyhow::Result;

/// Create a Neo4j relationship for a CodeEdge
///
/// This function creates a relationship in Neo4j between two CodeEntity nodes
/// with the appropriate relationship type (CALLS, IMPORTS, etc.).
///
/// Uses MERGE for idempotency - can be called multiple times for the same edge.
///
/// # Arguments
/// * `neo4j` - Neo4j client connection
/// * `edge` - CodeEdge describing the relationship
///
/// # Returns
/// Ok(()) on success, or error if Neo4j operation fails
pub async fn create_code_relationship(neo4j: &Neo4jClient, edge: &CodeEdge) -> Result<()> {
    let rel_type = edge_type_to_neo4j_type(&edge.edge_type);

    // Build Cypher query with MERGE for idempotency (TASK C: restrict to :SynCore)
    let cypher = format!(
        r#"
        MATCH (a:SynCore {{id: $src_id, namespace: $ns}}), (b:SynCore {{id: $dst_id, namespace: $ns}})
        MERGE (a)-[:{}]->(b)
        "#,
        rel_type
    );

    neo4j
        .execute_query(
            &cypher,
            vec![
                ("src_id", serde_json::json!(edge.src_entity_id)),
                ("dst_id", serde_json::json!(edge.dst_entity_id)),
                ("ns", serde_json::json!(neo4j.namespace())),
            ],
        )
        .await?;

    Ok(())
}

/// Map EdgeType to Neo4j relationship type string
fn edge_type_to_neo4j_type(edge_type: &EdgeType) -> &str {
    match edge_type {
        EdgeType::Calls => "CALLS",
        EdgeType::Imports => "IMPORTS",
        EdgeType::Inherits => "INHERITS",
        EdgeType::References => "REFERENCES",
        EdgeType::Uses => "USES",
        EdgeType::Contains => "CONTAINS",
        EdgeType::UsesField => "USES_FIELD",
        EdgeType::Implements => "IMPLEMENTS",
        EdgeType::UsesType => "USES_TYPE",
        EdgeType::ModuleChild => "MODULE_CHILD",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_type_to_neo4j_type() {
        assert_eq!(edge_type_to_neo4j_type(&EdgeType::Calls), "CALLS");
        assert_eq!(edge_type_to_neo4j_type(&EdgeType::Imports), "IMPORTS");
        assert_eq!(edge_type_to_neo4j_type(&EdgeType::Inherits), "INHERITS");
        assert_eq!(edge_type_to_neo4j_type(&EdgeType::References), "REFERENCES");
        assert_eq!(edge_type_to_neo4j_type(&EdgeType::Uses), "USES");
        assert_eq!(edge_type_to_neo4j_type(&EdgeType::Contains), "CONTAINS");
    }
}
