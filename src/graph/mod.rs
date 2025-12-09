//! Graph database module for SynCore
//!
//! Provides SQLite graph database backend with Neo4j infrastructure frozen
//! Neo4j integration is disabled - all Neo4j code exists as stubs only

pub mod backend;
pub mod backend_selector;
pub mod neo4j_backend_adapter;
pub mod neo4j_client;
pub mod sqlitegraph_impl;

// Neo4j modules are frozen and stubbed out
pub use backend::{
    EntityResult, GraphBackend, GraphStats, NodeLabel, NodeProperties,
    RelationType,
};
pub use backend_selector::{
    create_default_graph_backend, create_graph_backend, validate_graph_config,
};
// Neo4j types are frozen stubs for compilation compatibility
pub use neo4j_backend_adapter::Neo4jBackendAdapter;
pub use neo4j_client::Neo4jClient;
// Create stub Neo4jBackend type alias for compatibility
pub type Neo4jBackend = Neo4jBackendAdapter;
pub use sqlitegraph_impl::SQLiteGraphBackend;
