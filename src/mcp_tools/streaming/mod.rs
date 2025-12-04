//! Streaming contract enforcement module
//!
//! Provides output limiting and truncation capabilities for MCP tools
//! to prevent large responses from overwhelming LLM context.

pub mod contract_enforcer;

pub use contract_enforcer::OutputLimiter;