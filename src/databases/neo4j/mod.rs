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
//! 8. All entities use double label pattern: `:Function:SynCore`

pub mod schema;
pub mod writer;
pub mod reader;

// Re-export main types for convenience
pub use schema::{
    NodeLabel,
    RelationType,
    NodeProperties,
    PROJECT_LABEL,
    project_namespace,
};

pub use writer::{
    upsert_entity,
    create_relationship,
    update_git_metadata,
    batch_upsert_entities,
    batch_create_relationships,
    delete_entity,
    delete_file_entities,
    upsert_file_by_path,
    create_file_dependency,
};

pub use reader::{
    EntityResult,
    GraphStats,
    get_entity_by_id,
    get_file_entities,
    get_function_callees,
    get_function_callers,
    find_entities_by_name,
    get_entities_by_type,
    count_entities_by_type,
    get_neighbors,
    find_orphan_entities,
    validate_structure,
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
        assert_eq!(_label_str, "SynCore");
    }
}
