//! Graph database module for SynCore
//!
//! Provides Neo4j integration alongside SQLite and FAISS

pub mod neo4j_client;

pub use neo4j_client::Neo4jClient;
