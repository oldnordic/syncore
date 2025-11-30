//! Neo4j node creation for code entities
//!
//! This module handles writing CodeEntity nodes to Neo4j graph database.
//! Neo4j integration is OPTIONAL and best-effort - failures do not block
//! the indexing pipeline.

use super::types::{CodeEntity, EntityType};
use crate::graph::Neo4jClient;
use crate::databases::neo4j::{NodeLabel, NodeProperties, upsert_entity};
use anyhow::Result;

/// Create a Neo4j node for a CodeEntity
///
/// This function creates a node in Neo4j with a label matching the entity type
/// (Function, Class, Method, etc.) and stores all CodeEntity properties.
///
/// Uses MERGE for idempotency - can be called multiple times for the same entity.
///
/// # Arguments
/// * `neo4j` - Neo4j client connection
/// * `entity_id` - SQLite entity ID (used as Neo4j node identity)
/// * `entity` - CodeEntity to store
///
/// # Returns
/// Ok(()) on success, or error if Neo4j operation fails
///
/// # Example
/// ```rust
/// let neo4j = Neo4jClient::connect(...).await?;
/// create_code_entity_node(&neo4j, 123, &my_entity).await?;
/// ```
pub async fn create_code_entity_node(
    neo4j: &Neo4jClient,
    entity_id: i64,
    entity: &CodeEntity,
) -> Result<()> {
    // Map EntityType to canonical NodeLabel
    let label = entity_type_to_node_label(&entity.entity_type);

    // Map CodeEntity to canonical NodeProperties
    let props = NodeProperties {
        id: entity_id,
        name: entity.name.clone(),
        path: Some(entity.file_path.clone()),
        start_line: Some(entity.line_start as i64),
        end_line: Some(entity.line_end as i64),
        signature: entity.signature.clone(),
        body_snippet: entity.body_snippet.clone(),
        docstring: entity.docstring.clone(),
        hash: None, // Not available in CodeEntity
        language: Some(entity.language.clone()),
        file_sha256: None, // Not available in CodeEntity
        mtime: None, // Not available in CodeEntity
        created_at: entity.created_at.map(|ts| ts.to_string()),
        last_modified_at: entity.last_modified_at.map(|ts| ts.to_string()),
        change_count: entity.change_count.map(|c| c as i64),
        author_count: entity.author_count.map(|c| c as i64),
    };

    // Use canonical upsert function (handles :EntityType:SynCore, namespace, idempotency)
    upsert_entity(neo4j, label, props).await
}

/// Map EntityType to canonical NodeLabel
///
/// Each entity type gets a specific label for optimal Cypher query performance.
fn entity_type_to_node_label(entity_type: &EntityType) -> NodeLabel {
    match entity_type {
        EntityType::Function => NodeLabel::Function,
        EntityType::Class => NodeLabel::Struct, // Map Class to Struct (closest match)
        EntityType::Method => NodeLabel::Function, // Methods are functions
        EntityType::Import => NodeLabel::Import,
        EntityType::Struct => NodeLabel::Struct,
        EntityType::Enum => NodeLabel::Enum,
        EntityType::Trait => NodeLabel::Trait,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
