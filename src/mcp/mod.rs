//! MCP Server Modular Architecture
//!
//! Organizes MCP tool handlers into focused submodules for maintainability.
//! Each module contains related tool handlers that integrate with SynCore's
//! multi-system architecture (SQLite, Neo4j, FAISS, etc.)

pub mod code_graph_tools;
pub mod code_relationship_tools;
pub mod protocol;
pub mod server;
pub mod tool_metadata;
pub mod types;

pub use code_relationship_tools::CodeRelationshipTools;
pub use server::McpServer;
pub use tool_metadata::{SideEffects, ToolCategory, ToolCost, ToolMetadata};
pub use types::{SynCoreError, SynCoreResult, ToolRequest, ToolResponse};

// Re-export protocol types for backward compatibility
pub use protocol::{
    describe_server, handle_mcp_request, list_tools, MCPError, MCPRequest, MCPResponse,
    SynCoreState, ToolInfo,
};

// Re-export code graph tools
pub use code_graph_tools::{
    handle_code_graph_explain, handle_code_graph_impact, handle_code_graph_index,
    handle_code_graph_query, handle_code_graph_refactor_check, handle_code_graph_refactor_symbol,
    handle_project_macro_expand,
};
