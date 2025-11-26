//! Portfolio Graph Canonical Module - Change Tracking & Task Management
//!
//! This module provides type-safe Neo4j operations for portfolio tracking entities:
//! - Patches: Code change tracking
//! - Steps: Sequential reasoning history
//! - Tasks: Task management and dependencies
//!
//! Separate from code entity and RAG schemas to maintain clear domain boundaries.
//!
//! Architecture:
//! - schema.rs: Defines :Patch, :Step, :Task nodes and their relationships
//! - writer.rs: All write operations for portfolio entities
//! - reader.rs: All read operations for portfolio entities
//!
//! Rules (same as other canonical modules):
//! 1. No ad-hoc Cypher queries outside this module
//! 2. No string concatenation for Cypher
//! 3. No runtime-generated schema
//! 4. Namespace from Neo4jClient (defaults to "syncore_default")
//! 5. All writes use MERGE (idempotent)
//! 6. All queries parameterized
//! 7. All operations namespace-aware
//! 8. All entities use double label pattern: `:Patch:SynCore`

pub mod schema;
pub mod writer;
pub mod reader;

// Re-export main types for convenience
pub use schema::{
    NodeLabel,
    RelationType,
    PatchProperties,
    StepProperties,
    TaskProperties,
    PORTFOLIO_PROJECT_LABEL,
    portfolio_namespace,
};

pub use writer::{
    upsert_patch,
    upsert_step,
    upsert_task,
    create_for_task_relationship,
    create_applies_to_relationship,
    create_follows_relationship,
    delete_patch,
    delete_step,
    delete_task,
};

pub use reader::{
    PatchResult,
    StepResult,
    TaskResult,
    get_patch_by_id,
    get_step_by_id,
    get_task_by_id,
    get_patches_for_task,
    get_steps_for_task,
    get_patch_files,
    count_portfolio_nodes,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Compile-time check: All expected types are exported
        let _label: NodeLabel = NodeLabel::Patch;
        let _rel: RelationType = RelationType::ForTask;
        let _label_str: &str = PORTFOLIO_PROJECT_LABEL;
        assert_eq!(_label_str, "SynCore");
    }
}
