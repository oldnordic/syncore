//! MCP Server Module
//!
//! Organizes MCP protocol implementation into:
//! - types: Request parameter definitions for all 69 MCP tools
//! - server: Server implementation with tool handlers

mod server;
mod stdio_backcompat;
pub mod types;

// Unified reasoning module for consistent MCP tool behavior
pub mod reasoning;

// Re-export server components
pub use server::*;
pub use stdio_backcompat::BackcompatStdIo;
