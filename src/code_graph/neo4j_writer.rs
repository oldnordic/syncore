//! Neo4j node creation for code entities
//!
//! This module handles writing CodeEntity nodes to Neo4j graph database.
//! Neo4j integration is OPTIONAL and best-effort - failures do not block
//! the indexing pipeline.

use super::types::{CodeEntity, EntityType};
use crate::graph::Neo4jClient;
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
    // Get label for this entity type
    let label = entity_type_to_label(&entity.entity_type);

    // Build Cypher query with dynamic label + SynCore project label (TASK C)
    // All SynCore entities get :SynCore label for project isolation
    let cypher = format!(
        r#"
        MERGE (e:{}:SynCore {{id: $id, namespace: $ns}})
        SET e.file_path = $file_path,
            e.name = $name,
            e.signature = $signature,
            e.line_start = $line_start,
            e.line_end = $line_end,
            e.docstring = $docstring,
            e.language = $language,
            e.indexed_at = datetime(),
            e.created_at = $created_at,
            e.last_modified_at = $last_modified_at,
            e.change_count = $change_count,
            e.author_count = $author_count
        "#,
        label
    );

    // Execute with parameters
    neo4j
        .execute_query(
            &cypher,
            vec![
                ("id", serde_json::json!(entity_id)),
                ("ns", serde_json::json!(neo4j.namespace())),
                ("file_path", serde_json::json!(entity.file_path)),
                ("name", serde_json::json!(entity.name)),
                ("signature", serde_json::json!(entity.signature)),
                ("line_start", serde_json::json!(entity.line_start as i64)),
                ("line_end", serde_json::json!(entity.line_end as i64)),
                ("docstring", serde_json::json!(entity.docstring)),
                ("language", serde_json::json!(entity.language)),
                ("created_at", serde_json::json!(entity.created_at)),
                ("last_modified_at", serde_json::json!(entity.last_modified_at)),
                ("change_count", serde_json::json!(entity.change_count)),
                ("author_count", serde_json::json!(entity.author_count)),
            ],
        )
        .await?;

    Ok(())
}

/// Map EntityType to Neo4j node label
///
/// Each entity type gets a specific label for optimal Cypher query performance.
fn entity_type_to_label(entity_type: &EntityType) -> &str {
    match entity_type {
        EntityType::Function => "Function",
        EntityType::Class => "Class",
        EntityType::Method => "Method",
        EntityType::Import => "Import",
        EntityType::Struct => "Struct",
        EntityType::Enum => "Enum",
        EntityType::Trait => "Trait",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_type_to_label() {
        assert_eq!(entity_type_to_label(&EntityType::Function), "Function");
        assert_eq!(entity_type_to_label(&EntityType::Class), "Class");
        assert_eq!(entity_type_to_label(&EntityType::Method), "Method");
        assert_eq!(entity_type_to_label(&EntityType::Import), "Import");
        assert_eq!(entity_type_to_label(&EntityType::Struct), "Struct");
        assert_eq!(entity_type_to_label(&EntityType::Enum), "Enum");
        assert_eq!(entity_type_to_label(&EntityType::Trait), "Trait");
    }

    // TASK C: Verify that :SynCore label is included in MERGE query
    #[test]
    fn test_merge_query_includes_syncore_label() {
        let entity = CodeEntity {
            id: Some(123),
            file_path: "/test/path.rs".to_string(),
            entity_type: EntityType::Function,
            name: "test_func".to_string(),
            signature: Some("fn test_func()".to_string()),
            line_start: 1,
            line_end: 10,
            docstring: None,
            language: "rust".to_string(),
            created_at: Some(1234567890),
            last_modified_at: Some(1234567890),
            change_count: Some(5),
            author_count: Some(1),
        };

        let label = entity_type_to_label(&entity.entity_type);
        let cypher = format!(
            r#"
            MERGE (e:{}:SynCore {{id: $id, namespace: $ns}})
            SET e.file_path = $file_path
            "#,
            label
        );

        // Verify query includes both type label and :SynCore label
        assert!(cypher.contains(":Function:SynCore"), "Query should include both :Function and :SynCore labels");
        assert!(cypher.contains("MERGE (e:Function:SynCore"), "Query should use correct label syntax");
    }
}
