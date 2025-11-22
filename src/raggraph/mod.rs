//! RagGraph: Retrieval-Augmented Generation with Graph Intelligence
//!
//! Combines HNSW vector search with Neo4j graph traversal for multi-hop reasoning.

pub mod attention;
pub mod config;
pub mod diffusion;
pub mod fusion;
pub mod hopgraph;
pub mod rag_query;
pub mod storage;
pub mod storage_adapter;
pub mod types;
pub mod validation;

pub use config::{RagGraphConfig, RaggraphBackendMode};
pub use hopgraph::HopGraphTransformer;
pub use rag_query::RagQuery;
pub use storage::{RealStorageAdapter, StorageAdapter, StorageError};
pub use types::{RagGraphEdge, RagGraphNode, RagGraphResult};
pub use validation::{validate_real_backend, ValidationError};
