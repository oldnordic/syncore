use tokio::{net::TcpListener, io::{AsyncReadExt, AsyncWriteExt}};
use anyhow::Result;
use std::env;

use syncore::protocol::SynCoreMsg;
use syncore::memory::Memory;
use syncore::router::{route, SynCoreState};
use syncore::mcp::{handle_mcp_request, MCPRequest};
use syncore::metrics::start_metrics_server;
use syncore::backup::create_daily_backup;
use syncore::autonomy::AutonomyManager;
use tokio::time::{interval, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let addr = env::var("SOCKET_PATH").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let db_path = env::var("DB_PATH").unwrap_or_else(|_| "syncore.db".to_string());
    let mode = env::var("SYNCORE_MODE").unwrap_or_else(|_| "mcp".to_string());
    
    let listener = TcpListener::bind(&addr).await?;
    let memory = Memory::new(&db_path);
    let state = SynCoreState::new(memory, &db_path)?;
    
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
    let autonomy_manager = AutonomyManager::new(state.taskmaster.clone(), state.logger.clone());
    autonomy_manager.start().await;
    
    println!("SynCore {} server listening on {}", mode, addr);
    println!("Metrics available on {}", metrics_addr);
    println!("Daily backups enabled");
    println!("Autonomy features active (heartbeat, nudges)");
    
    loop {
        let (mut socket, _) = listener.accept().await?;
        let state_clone = state.clone();
        let mode_clone = mode.clone();
        
        tokio::spawn(async move {
            let mut buf = [0; 4096];
            let n = match socket.read(&mut buf).await {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Failed to read from socket: {}", e);
                    return;
                }
            };
            
            let response = if mode_clone == "mcp" {
                // Handle MCP JSON-RPC requests
                let request: MCPRequest = match serde_json::from_slice(&buf[..n]) {
                    Ok(req) => req,
                    Err(e) => {
                        eprintln!("Failed to deserialize MCP request: {}", e);
                        return;
                    }
                };
                
                let resp = handle_mcp_request(request, &state_clone).await;
                serde_json::to_vec(&resp).unwrap()
            } else {
                // Handle legacy MessagePack-RPC requests
                let msg: SynCoreMsg = match rmp_serde::from_slice(&buf[..n]) {
                    Ok(msg) => msg,
                    Err(e) => {
                        eprintln!("Failed to deserialize message: {}", e);
                        return;
                    }
                };
                
                route(msg, &state_clone).await
            };
            
            if let Err(e) = socket.write_all(&response).await {
                eprintln!("Failed to write response: {}", e);
            }
        });
    }
}