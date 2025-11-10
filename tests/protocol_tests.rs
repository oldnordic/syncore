use syncore::protocol::{SynCoreMsg, SynCoreTool};

#[test]
fn test_protocol_serialization() {
    // Test creating a MemoryStore message
    let args = rmp_serde::to_vec(&("test_key".to_string(), "test_value".to_string())).unwrap();
    let msg = SynCoreMsg {
        tool: SynCoreTool::MemoryStore,
        args,
    };
    
    // Serialize to MessagePack
    let serialized = rmp_serde::to_vec(&msg).unwrap();
    
    // Deserialize back
    let deserialized: SynCoreMsg = rmp_serde::from_slice(&serialized).unwrap();
    
    // Verify the tool matches
    match deserialized.tool {
        SynCoreTool::MemoryStore => {}, // Expected
        _ => panic!("Expected MemoryStore tool"),
    }
    
    // Verify the args can be deserialized back
    let (key, value): (String, String) = rmp_serde::from_slice(&deserialized.args).unwrap();
    assert_eq!(key, "test_key");
    assert_eq!(value, "test_value");
}

#[test]
fn test_protocol_memory_query() {
    // Test creating a MemoryQuery message
    let args = rmp_serde::to_vec(&"test_key".to_string()).unwrap();
    let msg = SynCoreMsg {
        tool: SynCoreTool::MemoryQuery,
        args,
    };
    
    // Serialize to MessagePack
    let serialized = rmp_serde::to_vec(&msg).unwrap();
    
    // Deserialize back
    let deserialized: SynCoreMsg = rmp_serde::from_slice(&serialized).unwrap();
    
    // Verify the tool matches
    match deserialized.tool {
        SynCoreTool::MemoryQuery => {}, // Expected
        _ => panic!("Expected MemoryQuery tool"),
    }
    
    // Verify the args can be deserialized back
    let key: String = rmp_serde::from_slice(&deserialized.args).unwrap();
    assert_eq!(key, "test_key");
}