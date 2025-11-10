use std::fs;
use std::process::{Command, Child};
use std::thread;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use syncore::protocol::{SynCoreMsg, SynCoreTool};
use syncore::memory::Memory;
use syncore::router::{route, SynCoreState};

#[tokio::test]
async fn test_server_memory_store_and_query() {
    // Clean up any existing test files
    let _ = fs::remove_file("test_integration.db");
    let _ = fs::remove_dir_all("test_integration_cache");
    
    let memory = Memory::new("test_integration.db");
    let state = SynCoreState::new(memory, "test_integration.db").unwrap();
    
    // Test MemoryStore via router
    let args = rmp_serde::to_vec(&("integration_test_key", "integration_test_value")).unwrap();
    let msg = SynCoreMsg {
        tool: SynCoreTool::MemoryStore,
        args,
    };
    
    let response = route(msg, &state).await;
    let result: String = rmp_serde::from_slice(&response).unwrap();
    assert_eq!(result, "ok");
    
    // Test MemoryQuery via router
    let args = rmp_serde::to_vec(&"integration_test_key").unwrap();
    let msg = SynCoreMsg {
        tool: SynCoreTool::MemoryQuery,
        args,
    };
    
    let response = route(msg, &state).await;
    let result: Option<String> = rmp_serde::from_slice(&response).unwrap();
    assert_eq!(result, Some("integration_test_value".to_string()));
    
    // Clean up
    let _ = fs::remove_file("test_integration.db");
    let _ = fs::remove_dir_all("test_integration_cache");
}