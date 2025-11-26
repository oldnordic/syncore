/*
use std::fs;
use syncore::memory::Memory;
use syncore::protocol::{SynCoreMsg, SynCoreTool};
use syncore::router::SynCoreState;

#[tokio::test]
async fn test_server_memory_store_and_query() {
    // Clean up any existing test files
    let _ = fs::remove_file("test_integration.db");
    let _ = fs::remove_dir_all("test_integration_cache");

    let memory = Memory::new("test_integration.db").unwrap();
    let tasks = syncore::tasks::Tasks::new("test_integration_tasks.db").unwrap();
    let embeddings = Box::new(syncore::vector::RealEmbeddings::new(384).unwrap());
    let vector_store = std::sync::Arc::new(std::sync::Mutex::new(
        syncore::vector::VectorStore::new(embeddings),
    ));
    let state = SynCoreState::new(memory, tasks, vector_store);

    // Test MemoryStore via router
    let args = rmp_serde::to_vec(&("integration_test_key", "integration_test_value")).unwrap();
    let msg = SynCoreMsg {
        tool: SynCoreTool::MemoryStore,
        args,
    };

    let response = syncore::router::handle_message(msg, &state).unwrap();
    let response_json: serde_json::Value = rmp_serde::from_slice(&response).unwrap();
    let result = response_json["success"].as_bool().unwrap();
    assert_eq!(result, true);

    // Test MemoryQuery via router
    let args = rmp_serde::to_vec(&"integration_test_key").unwrap();
    let msg = SynCoreMsg {
        tool: SynCoreTool::MemoryQuery,
        args,
    };

    let response = syncore::router::handle_message(msg, &state).unwrap();
    let response_json: serde_json::Value = rmp_serde::from_slice(&response).unwrap();
    let result = response_json["value"].as_str().map(|s| s.to_string());
    assert_eq!(result, Some("integration_test_value".to_string()));

    // Clean up
    let _ = fs::remove_file("test_integration.db");
    let _ = fs::remove_dir_all("test_integration_cache");
}
*/
