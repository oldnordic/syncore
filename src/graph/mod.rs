//! Graph database module for SynCore
//!
//! Provides Neo4j integration alongside SQLite and FAISS
//! Now with database abstraction via GraphBackend trait

pub mod backend;
pub mod backend_selector;
pub mod neo4j_client;
pub mod sqlitegraph_impl;

pub use backend::{
    neo4j_impl::Neo4jBackend, EntityResult, GraphBackend, GraphStats, NodeLabel, NodeProperties,
    RelationType,
};
pub use backend_selector::{
    create_default_graph_backend, create_graph_backend, validate_graph_config,
};
pub use neo4j_client::Neo4jClient;
pub use sqlitegraph_impl::SQLiteGraphBackend;
