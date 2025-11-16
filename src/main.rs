use tokio::{net::TcpListener, io::{AsyncReadExt, AsyncWriteExt}};
use anyhow::Result;
use std::env;

use syncore::protocol::SynCoreMsg;
use syncore::memory::Memory;
use syncore::tasks::Tasks;
use syncore::vector::{VectorStore, HuggingFaceEmbeddings};
use syncore::router::{route_tool, SynCoreState};
use syncore::mcp::{handle_mcp_request, MCPRequest};
use syncore::mcp_stdio::run_stdio_server;
use syncore::metrics::start_metrics_server;
use syncore::backup::create_daily_backup;
use syncore::autonomy::AutonomyManager;
use syncore::logger::MarkdownLogger;
use std::sync::{Arc, Mutex};
use tokio::time::{interval, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let addr = env::var("SOCKET_PATH").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let db_path = env::var("DB_PATH").unwrap_or_else(|_| "syncore.db".to_string());
    let mode = env::var("SYNCORE_MODE").unwrap_or_else(|_| "mcp".to_string());
    let transport = env::var("TRANSPORT").unwrap_or_else(|_| "http".to_string());

    // Initialize state
    let memory = Memory::new(&db_path)?;
    let tasks = Tasks::new(&format!("{}_tasks", db_path))?;
    let tasks_arc = Arc::new(tasks.clone());
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);

    // Run in stdio mode if requested
    if transport == "stdio" {
        println!("Starting SynCore MCP Server in stdio mode...");
        run_stdio_server(state)?;
        return Ok(());
    }

    let listener = TcpListener::bind(&addr).await?;

    // Start metrics server on different port
    let metrics_addr = env::var("METRICS_ADDR").unwrap_or_else(|_| "127.0.0.1:9090".to_string());
    let metrics_addr_clone = metrics_addr.clone();
    tokio::spawn(async move {
        if let Err(e) = start_metrics_server(&metrics_addr_clone).await {
            eprintln!("Metrics server error: {}", e);
        }
    });

    // Start daily backup task
    let db_path_clone = db_path.clone();
    let logs_dir = "logs".to_string();
    tokio::spawn(async move {
        let mut backup_interval = interval(Duration::from_secs(86400)); // Daily
        loop {
            backup_interval.tick().await;
            if let Err(e) = create_daily_backup(&db_path_clone, &logs_dir) {
                eprintln!("Backup failed: {}", e);
            }
        }
    });

    // Start autonomy features
    let logger = Arc::new(MarkdownLogger::new("./logs"));
    let autonomy_manager = AutonomyManager::new(Arc::new(Mutex::new(tasks_arc.as_ref().clone())), logger, &db_path);
    autonomy_manager.start().await;

    println!("SynCore {} server listening on {}", mode, addr);
    println!("Metrics available on {}", metrics_addr);
    println!("Daily backups enabled");
    println!("Autonomy features active (heartbeat, nudges)");

    loop {
        let (mut socket, peer_addr) = listener.accept().await?;
        println!("Client connected from: {}", peer_addr);
        let state_clone = state.clone();
        let mode_clone = mode.clone();

        tokio::spawn(async move {
            let mut buf = [0; 4096];
            let n = match socket.read(&mut buf).await {
                Ok(n) => {
                    println!("Received {} bytes from {}", n, peer_addr);
                    n
                },
                Err(e) => {
                    eprintln!("Failed to read from socket {}: {}", peer_addr, e);
                    return;
                }
            };

            println!("Request data: {:?}", &buf[..n]);

            let response = if mode_clone == "mcp" {
                // Handle MCP JSON-RPC requests
                let request: MCPRequest = match serde_json::from_slice(&buf[..n]) {
                    Ok(req) => {
                        println!("MCP request: {:?}", req);
                        req
                    },
                    Err(e) => {
                        eprintln!("Failed to deserialize MCP request from {}: {}", peer_addr, e);
                        return;
                    }
                };

                let resp = handle_mcp_request(request, &state_clone).await;
                println!("MCP response: {:?}", resp);
                serde_json::to_vec(&resp).unwrap()
            } else {
                // Handle legacy MessagePack-RPC requests
                let msg: SynCoreMsg = match rmp_serde::from_slice(&buf[..n]) {
                    Ok(msg) => msg,
                    Err(e) => {
                        eprintln!("Failed to deserialize message from {}: {}", peer_addr, e);
                        return;
                    }
                };

                // Convert legacy MessagePack-RPC to tool call
                match route_tool(&msg.tool.to_string(), &msg.args, &state_clone) {
                    Ok(resp) => resp,
                    Err(e) => {
                        let error_response = serde_json::json!({"error": e.to_string()});
                        rmp_serde::to_vec(&error_response).unwrap()
                    }
                }
            };

            if let Err(e) = socket.write_all(&response).await {
                eprintln!("Failed to write response to {}: {}", peer_addr, e);
            } else {
                println!("Response sent successfully to {}", peer_addr);
            }
        });
    }
}
