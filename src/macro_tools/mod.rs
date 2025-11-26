//! Macro Tools Layer
//!
//! This module provides a simplified interface to SynCore's 49 underlying tools
//! by exposing 10 high-level "macro tools" that route based on an action parameter.
//!
//! ## Architecture
//!
//! Each macro tool accepts a generic request with an `action` field that determines
//! which underlying tool to invoke. This reduces cognitive load for LLMs while
//! maintaining full functionality.
//!
//! ## Macro Tools
//!
//! 1. **syncore.memory** - routes to memory_store, memory_query
//! 2. **syncore.task** - routes to task_create, intellitask_*
//! 3. **syncore.vector** - routes to vector_insert, vector_search
//! 4. **syncore.code** - routes to parser_*, code_*
//! 5. **syncore.document** - routes to document_index, document_search
//! 6. **syncore.graph** - routes to graph_query, graph_insert, graph_relate
//! 7. **syncore.agent** - routes to agent_send, agent_recv, agent_*
//! 8. **syncore.mapping** - routes to mapping_record, mapping_get, mapping_search, mapping_deps
//! 9. **syncore.reasoning** - routes to sequential_cycle, sequential_record, sequential_get, sequential_search
//! 10. **syncore.logs** - routes to logs_tail

pub mod agent;
pub mod code;
pub mod document;
pub mod executor_real;
pub mod executor_stub;
pub mod graph;
pub mod import_extractor;
pub mod logs;
pub mod mapping;
pub mod memory;
pub mod path_filter;
pub mod planner;
pub mod reasoning;
pub mod router;
pub mod task;
pub mod vector;

// Re-export key types for convenience
pub use code::*;
pub use router::*;
pub use task::*;
