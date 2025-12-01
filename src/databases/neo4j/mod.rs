//! Canonical Neo4j Module - Single Source of Truth
//!
//! This is THE ONLY module that touches Neo4j.
//! Everything else uses this module's API.
//!
//! Architecture:
//! - schema.rs: Defines labels, properties, relationships (no other file defines schema)
//! - writer.rs: All write operations (no other file writes to Neo4j)
//! - reader.rs: All read operations (no other file reads from Neo4j)
//!
//! Rules:
//! 1. No ad-hoc Cypher queries outside this module
//! 2. No string concatenation for Cypher
//! 3. No runtime-generated schema
//! 4. Namespace from Neo4jClient (defaults to "syncore_default")
//! 5. All writes use MERGE (idempotent)
//! 6. All queries parameterized (no SQL injection)
//! 7. All operations namespace-aware
//! 8. All entities use double label pattern: `:Function:CodeGraph`

pub mod reader;
pub mod schema;
pub mod writer;

// Re-export main types for convenience
pub use schema::{project_namespace, NodeLabel, NodeProperties, RelationType, PROJECT_LABEL};

pub use writer::{
    batch_create_relationships, batch_upsert_entities, create_file_dependency, create_relationship,
    delete_entity, delete_file_entities, update_git_metadata, upsert_entity, upsert_file_by_path,
};

pub use reader::{
    count_entities_by_type, find_entities_by_name, find_orphan_entities, get_entities_by_type,
    get_entity_by_id, get_file_entities, get_function_callees, get_function_callers, get_neighbors,
    validate_structure, EntityResult, GraphStats,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Compile-time check: All expected types are exported
        let _label: NodeLabel = NodeLabel::Function;
        let _rel: RelationType = RelationType::Calls;
        let _label_str: &str = PROJECT_LABEL;
        assert_eq!(_label_str, "CodeGraph");
    }
}
