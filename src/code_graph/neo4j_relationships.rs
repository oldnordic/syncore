//! Neo4j relationship creation for code graph edges
//!
//! This module handles creating Neo4j relationships between CodeEntity nodes
//! based on CodeEdge data from the code graph.

use super::types::{CodeEdge, EdgeType};
use crate::databases::neo4j::{create_relationship, RelationType};
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
    // Map EdgeType to canonical RelationType
    let rel_type = edge_type_to_relation_type(&edge.edge_type);

    // Use canonical create_relationship (handles namespace, :SynCore filtering, idempotency)
    create_relationship(neo4j, edge.src_entity_id, edge.dst_entity_id, rel_type).await
}

/// Map EdgeType to canonical RelationType
fn edge_type_to_relation_type(edge_type: &EdgeType) -> RelationType {
    match edge_type {
        EdgeType::Calls => RelationType::Calls,
        EdgeType::Imports => RelationType::Imports,
        EdgeType::Inherits => RelationType::Inherits,
        EdgeType::References => RelationType::References,
        EdgeType::Uses => RelationType::Uses,
        EdgeType::Contains => RelationType::Contains,
        EdgeType::UsesField => RelationType::UsesField,
        EdgeType::Implements => RelationType::Implements,
        EdgeType::UsesType => RelationType::UsesType,
        EdgeType::ModuleChild => RelationType::ModuleChild,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_type_to_relation_type() {
        assert_eq!(edge_type_to_relation_type(&EdgeType::Calls), RelationType::Calls);
        assert_eq!(edge_type_to_relation_type(&EdgeType::Imports), RelationType::Imports);
        assert_eq!(edge_type_to_relation_type(&EdgeType::Inherits), RelationType::Inherits);
        assert_eq!(edge_type_to_relation_type(&EdgeType::References), RelationType::References);
        assert_eq!(edge_type_to_relation_type(&EdgeType::Uses), RelationType::Uses);
        assert_eq!(edge_type_to_relation_type(&EdgeType::Contains), RelationType::Contains);
    }
}
