//! MCP Server Wrapper
//!
//! Provides a high-level API for MCP tools that can be used in tests and integrations.
//! This wraps the underlying CodeRelationshipTools with a simpler interface.

use anyhow::Result;
use serde_json::Value;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use super::CodeRelationshipTools;

/// MCP Server providing code relationship analysis tools
pub struct McpServer {
    relationship_tools: CodeRelationshipTools,
}

impl McpServer {
    /// Create a new MCP server with default database path
    pub async fn new() -> Result<Self> {
        let db_path = PathBuf::from("./data/mcp_server.db");
        Self::new_with_path(db_path).await
    }

    /// Create a new MCP server with custom database path
    pub async fn new_with_path(db_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let relationship_tools = CodeRelationshipTools::new(&db_path).await?;

        Ok(Self { relationship_tools })
    }

    /// Subscribe to events from the server
    pub async fn subscribe_to_events<F>(&self, callback: F)
    where
        F: Fn(&str, &Value) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static,
    {
        self.relationship_tools.subscribe_to_events(callback).await;
    }

    /// Index a source file for code relationships
    pub async fn handle_code_relationship_index(&self, params: Value) -> Result<Value> {
        self.relationship_tools
            .handle_code_relationship_index(params)
            .await
    }

    /// Query code relationships (imports, calls, implementors)
    pub async fn handle_code_relationship_query(&self, params: Value) -> Result<Value> {
        self.relationship_tools
            .handle_code_relationship_query(params)
            .await
    }

    /// Search for semantically similar functions
    pub async fn handle_code_similarity_search(&self, params: Value) -> Result<Value> {
        self.relationship_tools
            .handle_code_similarity_search(params)
            .await
    }
}
