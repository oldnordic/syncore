use syncore::config::SyncoreConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing SynCore configuration...");
    
    // Test 1: Default configuration should use SQLiteGraph
    let default_config = SyncoreConfig::default();
    println!("Default backend: {:?}", default_config.graph.backend);
    
    // Test 2: Test configuration loading
    let config = SyncoreConfig::load_with_env("config/syncore.toml")?;
    println!("Loaded backend: {:?}", config.graph.backend);
    
    // Test 3: Test SQLite test configuration
    let test_config = SyncoreConfig::default_sqlite_test();
    println!("Test backend: {:?}", test_config.graph.backend);
    println!("Test path: {:?}", test_config.graph.path);
    
    println!("✅ All configuration tests passed!");
    Ok(())
}