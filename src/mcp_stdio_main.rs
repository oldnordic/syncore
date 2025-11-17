use anyhow::Result;
use std::env;

use syncore::memory::Memory;
use syncore::tasks::Tasks;
use syncore::vector::{VectorStore, HuggingFaceEmbeddings};
use syncore::router::SynCoreState;
use syncore::mcp_server::{run_mcp_stdio_server, SynCoreMCPServer};
use syncore::tools_cli::log_registered_tools;

use rmcp::transport::sse_server::{SseServer, SseServerConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Get configuration from environment
    let db_path = env::var("DB_PATH").unwrap_or_else(|_| "syncore.db".to_string());
    let http_port = env::var("HTTP_PORT").unwrap_or_else(|_| "3001".to_string());
    let http_bind = format!("127.0.0.1:{}", http_port);

    eprintln!("Starting SynCore MCP servers...");
    eprintln!("Database path: {}", db_path);
    eprintln!("HTTP/SSE server: {}", http_bind);

    // Log all registered MCP tools
    log_registered_tools().await;

    // Initialize state
    let memory = Memory::new(&db_path)?;
    let tasks = Tasks::new(&format!("{}_tasks", db_path))?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = std::sync::Arc::new(std::sync::Mutex::new(VectorStore::new(embeddings)));
    let mut state = SynCoreState::new(memory, tasks, vector_store);

    {
        use syncore::message_bus::MessageBus;
        let bus = MessageBus::new();
        state = state.with_message_bus(bus);
    }

    // Connect to Neo4j if available
    {
        use syncore::graph::neo4j_client::Neo4jClient;
        use std::sync::Arc;

        let neo4j_uri = env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
        let neo4j_user = env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
        let neo4j_pass = env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

        eprintln!("Connecting to Neo4j at {}...", neo4j_uri);
        match Neo4jClient::connect(&neo4j_uri, &neo4j_user, &neo4j_pass).await {
            Ok(neo) => {
                state = state.with_neo4j(Arc::new(neo));
                eprintln!("Neo4j connected successfully");
            }
            Err(e) => {
                eprintln!("Neo4j connection failed (graph tools disabled): {}", e);
            }
        }
    }

    eprintln!("State initialized...");

    // Spawn HTTP/SSE server for GLM-4.6 and other HTTP clients
    {
        let http_state = state.clone();
        let ct = tokio_util::sync::CancellationToken::new();
        let ct_clone = ct.clone();

        let config = SseServerConfig {
            bind: http_bind.parse()?,
            sse_path: "/sse".to_string(),
            post_path: "/message".to_string(),
            ct: ct.clone(),
            sse_keep_alive: None,
        };

        let (sse_server, router) = SseServer::new(config);
        let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;

        eprintln!("HTTP/SSE server listening on {}", http_bind);

        // Spawn axum server
        tokio::spawn(async move {
            let server = axum::serve(listener, router).with_graceful_shutdown(async move {
                ct_clone.cancelled().await;
            });
            if let Err(e) = server.await {
                eprintln!("HTTP/SSE server error: {:?}", e);
            }
        });

        // Spawn service handler for SSE connections
        tokio::spawn(async move {
            let _ct = sse_server.with_service(move || SynCoreMCPServer::new(http_state.clone()));
            eprintln!("HTTP/SSE service handler started");
        });
    }

    eprintln!("Starting STDIO server (Claude Code interface)...");

    // Run rmcp-based stdio server (blocks until shutdown)
    // If STDIO closes, we keep HTTP/SSE server running
    match run_mcp_stdio_server(state).await {
        Ok(_) => eprintln!("STDIO server shut down normally"),
        Err(e) => eprintln!("STDIO server error: {:?}", e),
    }

    // Keep the process alive for HTTP clients even if STDIO is closed
    // Wait for ctrl-c signal to shut down
    eprintln!("STDIO closed, HTTP/SSE server still running. Press Ctrl-C to stop.");
    tokio::signal::ctrl_c().await?;

    eprintln!("MCP servers shut down");
    Ok(())
}