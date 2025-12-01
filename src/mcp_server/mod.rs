//! MCP Server Module
//!
//! Organizes MCP protocol implementation into:
//! - types: Request parameter definitions for all 69 MCP tools
//! - server: Server implementation with tool handlers

mod server;
pub mod types;

// Re-export server components
pub use server::*;
