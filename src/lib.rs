// pub mod agent; // PHASE 4: Agent Planning & Reasoning Engine (APRE) - Temporarily disabled for testing
pub mod autonomy;
pub mod backup;
pub mod bootstrap; // APEX 2.15: Cold/warm start bootstrap logic
pub mod circuit_breaker;
pub mod cli;
pub mod code_directory_indexer;
pub mod code_drift;
pub mod code_graph;
pub mod cognition;
pub mod cognitive;
pub mod cognitive_db;
pub mod common;
pub mod config;
pub mod databases; // Canonical database modules (Neo4j, etc.)
pub mod db;
pub mod dlr;
pub mod document_indexer;
pub mod embedding_refresh; // APEX 2.9: Embedding refresh daemon
pub mod embeddings; // GraphBERT Candle embeddings for GRAPH domain
pub mod fs_watcher; // APEX 2.2-FW: Filesystem watcher
pub mod global_store;
pub mod graph;
pub mod graph_cli;
pub mod graph_rebuilder;
pub mod http_stream_server;
pub mod ingestion; // Global Ingestion Coordinator (GIC)
pub mod intellitask;
pub mod intellitask_persistence;
pub mod live_indexer; // APEX 2.7-LIVE-INDEXER: Real-time continuous indexing
pub mod llm;
pub mod logger;
pub mod lsp_bridge; // APEX 2.5-LSP: External LSP bridge
pub mod macro_tools;
pub mod mcp;
pub mod mcp_server;
pub mod mcp_stdio;
pub mod mcp_tools;
pub mod memory;
pub mod memory_service;
pub mod message_bus;
pub mod sqlitegraph;

pub mod metrics;
pub mod models; // GGUF Engine and model backends
pub mod parser;
pub mod parser_service; // APEX 2.2-FW: Incremental parser
pub mod path_resolver; // APEX v1.7 Phase 5: Central path resolution
pub mod polyglot;
pub mod portfolio;
pub mod project_analysis;
pub mod project_reasoning;
pub mod protocol;
pub mod query;
pub mod raggraph;
pub mod reasoning; // ST-3: Tree-of-Thought reasoning engine
pub mod refrag; // APEX 1.8: REFRAG selective expansion layer
pub mod router;
pub mod runtime;
pub mod rust_tools;
pub mod schema_migration;

pub mod snapshots;
pub mod storage;
pub mod tasks;
pub mod tools_cli;
pub mod validation;
pub mod vector;

// Re-export exact functions users requested
pub use cognitive_db::{recent_steps, store_step};
pub use mcp_server::run_mcp_stdio_server;
pub use mcp_stdio::run_stdio_server;
pub use tasks::{add_task, link_tasks, next_task, update_task};
pub use vector::{insert_text, search};

// Re-export configuration types
pub use config::{
    Config, EmbeddingsConfig, GraphEmbeddingsConfig, HotspotWeights, HttpConfig, IndexingConfig, LlmConfig, Neo4jConfig,
    PathsConfig, ProjectAnalysisConfig, SyncoreConfig, VectorSearchConfig,
};
