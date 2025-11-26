//! Neo4j relationship creation for code graph edges
//!
//! This module handles creating Neo4j relationships between CodeEntity nodes
//! based on CodeEdge data from the code graph.

use super::types::{CodeEdge, EdgeType};
use crate::graph::Neo4jClient;
use crate::databases::neo4j::{RelationType, create_relationship};
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

// Deprecated: Use edge_type_to_relation_type() instead
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
