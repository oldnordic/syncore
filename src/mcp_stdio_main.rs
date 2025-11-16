use anyhow::Result;
use std::env;

use syncore::memory::Memory;
use syncore::tasks::Tasks;
use syncore::vector::{VectorStore, HuggingFaceEmbeddings};
use syncore::router::SynCoreState;
use syncore::mcp_server::run_mcp_stdio_server;

#[tokio::main]
async fn main() -> Result<()> {
    // Get configuration from environment
    let db_path = env::var("DB_PATH").unwrap_or_else(|_| "syncore.db".to_string());
    
    eprintln!("Starting SynCore MCP stdio server...");
    eprintln!("Database path: {}", db_path);
    
    // Initialize state
    let memory = Memory::new(&db_path)?;
    let tasks = Tasks::new(&format!("{}_tasks", db_path))?;
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = std::sync::Arc::new(std::sync::Mutex::new(VectorStore::new(embeddings)));
    let state = SynCoreState::new(memory, tasks, vector_store);
    
    eprintln!("State initialized, starting MCP stdio server...");
    
    // Run rmcp-based stdio server
    run_mcp_stdio_server(state).await?;
    
    eprintln!("MCP stdio server shut down");
    Ok(())
}