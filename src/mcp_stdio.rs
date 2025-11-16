use anyhow::Result;
use serde_json::json;
use std::io::{self, BufRead, BufReader, Write};
use crate::mcp::{handle_mcp_request, SynCoreState, MCPRequest};
use tokio::runtime::Runtime;

/// Run MCP stdio server
pub fn run_stdio_server(state: SynCoreState) -> Result<()> {
    let rt = Runtime::new()?;
    rt.block_on(async_stdio_server(state))
}

async fn async_stdio_server(state: SynCoreState) -> Result<()> {
    eprintln!("SynCore MCP Server starting...");
    
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    
    loop {
        line.clear();
        
        match reader.read_line(&mut line) {
            Ok(0) => {
                eprintln!("EOF received, shutting down");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                
                // Parse JSON-RPC request
                match serde_json::from_str::<MCPRequest>(trimmed) {
                    Ok(request) => {
                        let response = handle_mcp_request(request, &state).await;
                        let response_json = serde_json::to_string(&response)?;
                        println!("{}", response_json);
                        io::stdout().flush()?;
                    }
                    Err(e) => {
                        let error_response = json!({
                            "jsonrpc": "2.0",
                            "error": {
                                "code": -32700,
                                "message": format!("Parse error: {}", e)
                            },
                            "id": null
                        });
                        println!("{}", serde_json::to_string(&error_response)?);
                        io::stdout().flush()?;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use crate::vector::HuggingFaceEmbeddings;

    #[test]
    fn test_mcp_request_parsing() {
        let request_json = r#"{"jsonrpc":"2.0","method":"mcp.describe","params":null,"id":1}"#;
        let request: MCPRequest = serde_json::from_str(request_json).unwrap();
        
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "mcp.describe");
        assert!(request.params.is_none());
        assert_eq!(request.id, 1);
    }

    #[tokio::test]
    async fn test_mcp_describe_request() -> Result<()> {
        let temp_db = NamedTempFile::new()?;
        let db_path = temp_db.path().to_str().unwrap();
        
        let memory = crate::memory::Memory::new(db_path)?;
        let taskmaster = crate::tasks::Tasks::new(&format!("{}_tasks", db_path))?;
        let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let vector_store = std::sync::Arc::new(std::sync::Mutex::new(crate::vector::VectorStore::new(embeddings)));
        
        let state = SynCoreState::new(memory, taskmaster, vector_store);
        
        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            method: "mcp.describe".to_string(),
            params: None,
            id: json!(1),
        };
        
        let response = handle_mcp_request(request, &state).await;
        
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert_eq!(response.id, json!(1));
        
        let result = response.result.unwrap();
        assert_eq!(result["name"], "SynCore");
        assert!(result["capabilities"].is_object());
        
        Ok(())
    }
}