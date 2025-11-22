//! HNSW (Hierarchical Navigable Small World) Vector Index Module
//!
//! Standalone HNSW implementation for approximate nearest neighbor search.
//! No SQLite coupling, no MCP dependencies - pure vector indexing.

mod config;
mod hnsw_index;

pub use config::HnswConfig;
pub use hnsw_index::HnswVectorIndex;
