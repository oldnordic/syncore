//! Canonical Database Modules
//!
//! All database access goes through these modules.
//! Each database type has:
//! - schema.rs: Schema definition (single source of truth)
//! - writer.rs: Write operations (hardcoded, parameterized)
//! - reader.rs: Read operations (hardcoded, parameterized)
//!
//! Modules:
//! - neo4j: Code entity graph (File, Function, Struct, etc.)
//! - rag_graph: RAG embeddings graph (Embedding nodes for semantic search)
//! - portfolio_graph: Portfolio tracking (Patch, Step, Task nodes for change tracking and task management)
//! - cognition_graph: Cognitive reasoning episodes (ReasoningEpisode nodes for LLM reasoning tracking)

pub mod neo4j;
pub mod rag_graph;
pub mod portfolio_graph;
pub mod cognition_graph;
