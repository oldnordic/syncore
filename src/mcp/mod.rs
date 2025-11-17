//! MCP Server Modular Architecture
//!
//! Organizes MCP tool handlers into focused submodules for maintainability.
//! Each module contains related tool handlers that integrate with SynCore's
//! multi-system architecture (SQLite, Neo4j, FAISS, etc.)

pub mod code_relationship_tools;
pub mod server;
pub mod protocol;
pub mod code_graph_tools;

pub use code_relationship_tools::CodeRelationshipTools;
pub use server::McpServer;

// Re-export protocol types for backward compatibility
pub use protocol::{
    ToolInfo, MCPRequest, MCPResponse, MCPError,
    list_tools, describe_server, handle_mcp_request, SynCoreState,
};

// Re-export code graph tools
pub use code_graph_tools::{
    handle_code_graph_index, handle_code_graph_query,
    handle_code_graph_explain, handle_code_graph_impact,
    handle_code_graph_refactor_check, handle_code_graph_refactor_symbol,
};
