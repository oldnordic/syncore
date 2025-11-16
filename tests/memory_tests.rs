use syncore::memory::Memory;
use std::fs;

#[test]
fn test_memory_store_and_query() {
    // Clean up any existing test database
    let _ = fs::remove_file("test_memory.db");
    let _ = fs::remove_dir_all("test_cache");
    
    // Create memory instance
    let memory = Memory::new("test_memory.db").unwrap();
    
    // Test storing a value
    memory.store("test_key", "test_value").unwrap();
    
    // Test querying stored value
    let result = memory.query("test_key").unwrap();
    assert_eq!(result, Some("test_value".to_string()));
    
    // Test querying non-existent key
    let result = memory.query("non_existent").unwrap();
    assert_eq!(result, None);
    
    // Test overwriting existing value
    memory.store("test_key", "new_value").unwrap();
    let result = memory.query("test_key").unwrap();
    assert_eq!(result, Some("new_value".to_string()));
    
    // Clean up
    let _ = fs::remove_file("test_memory.db");
    let _ = fs::remove_dir_all("test_cache");
}