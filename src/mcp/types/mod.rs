//! Type system for SynCore MCP tools
//!
//! Provides unified error handling, standardized request/response schemas,
//! and tool metadata for all 49 MCP tools.

pub mod error_kinds;
pub mod tool_errors;
pub mod tool_requests;
pub mod tool_responses;

pub use error_kinds::ErrorType;
pub use tool_errors::{SynCoreError, SynCoreResult};
pub use tool_requests::ToolRequest;
pub use tool_responses::ToolResponse;
