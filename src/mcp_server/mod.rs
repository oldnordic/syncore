//! MCP Server Module
//!
//! Organizes MCP protocol implementation into:
//! - types: Request parameter definitions for all 69 MCP tools
//! - server: Server implementation with tool handlers

pub mod types;
mod server;

// Re-export server components
pub use server::*;
