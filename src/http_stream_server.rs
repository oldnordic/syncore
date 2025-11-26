/// HTTP Streamable Server for MCP Protocol (2025-03-26 Spec)
///
/// This server exposes ALL MCP tools via the Streamable HTTP transport.
/// Uses rmcp's StreamableHttpService for proper MCP protocol compliance.
///
/// Endpoints:
/// - POST /mcp - JSON-RPC messages (initialize, tool calls)
/// - GET /mcp - SSE stream for server-initiated notifications
///
/// This replaces the old custom HTTP implementation with proper MCP transport.
use anyhow::Result;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::mcp_server::SynCoreMCPServer;
use crate::router::SynCoreState;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};

/// HTTP Streamable Server using rmcp's proper MCP transport
#[derive(Clone)]
pub struct HttpStreamServer {
    state: SynCoreState,
}

impl HttpStreamServer {
    /// Create new HTTP streaming server with unified MCP router
    pub fn new(state: SynCoreState) -> Self {
        Self { state }
    }

    /// Start HTTP streamable server with proper MCP protocol support
    pub async fn start(self, addr: SocketAddr) -> Result<()> {
        // Create the StreamableHttpService using rmcp's proper transport
        // This handles:
        // - POST /mcp for JSON-RPC messages (initialize, tool calls)
        // - GET /mcp for SSE streams (server notifications)
        // - Session management with Mcp-Session-Id headers
        let state_for_factory = self.state.clone();

        let service = StreamableHttpService::new(
            // Service factory - creates a new SynCoreMCPServer for each session
            move || {
                Ok(SynCoreMCPServer::new(
                    (*Arc::new(state_for_factory.clone())).clone(),
                ))
            },
            // Session manager for handling multiple clients
            Arc::new(LocalSessionManager::default()),
            // Default configuration
            StreamableHttpServerConfig::default(),
        );

        // Mount the MCP service at /mcp endpoint
        // The service handles both POST and GET automatically per MCP spec
        let app = Router::new().nest_service("/mcp", service);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!("HTTP Streamable MCP server listening on {}", addr);
        eprintln!("[MCP HTTP] Server listening on http://{}/mcp", addr);

        axum::serve(listener, app).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;
    use crate::tasks::Tasks;
    use crate::vector::{StubEmbeddings, VectorStore};
    use std::sync::Mutex;

    #[tokio::test]
    async fn test_http_stream_server_creation() {
        let memory = Memory::new(":memory:").unwrap();
        let tasks = Tasks::new(":memory:").unwrap();
        let embeddings = Box::new(StubEmbeddings::new(384).unwrap());
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let state = SynCoreState::new(memory, tasks, vector_store);

        let _server = HttpStreamServer::new(state);
        // Server created successfully
    }
}
