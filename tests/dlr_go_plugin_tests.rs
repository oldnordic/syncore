/*
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;
use syncore::dlr::{DlrManager, PluginCapability};
use syncore::plugin_api::{PluginRequest, PluginResponse};

#[test]
fn test_dlr_discover_go_plugin() -> Result<()> {
    let plugins_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins");

    let mut dlr_manager = DlrManager::new(&plugins_dir);

    // Discover plugins
    let discovered_count = dlr_manager.discover_and_register_plugins()?;

    // Should find at least our go_plugin
    assert!(discovered_count > 0, "Should discover at least one plugin");

    // Check if go_plugin is discovered
    let plugins = dlr_manager.list_plugins();
    let go_plugin_found = plugins.iter().any(|p| p.name == "syncore_go_plugin");
    assert!(go_plugin_found, "Should discover syncore_go_plugin");

    Ok(())
}

#[test]
fn test_dlr_load_go_plugin() -> Result<()> {
    let plugins_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins");

    let mut dlr_manager = DlrManager::new(&plugins_dir);

    // Discover plugins first
    dlr_manager.discover_and_register_plugins()?;

    // Load the go_plugin
    dlr_manager.load_plugin("syncore_go_plugin")?;

    // Check if plugin is loaded
    let plugin = dlr_manager.get_plugin("syncore_go_plugin")
        .expect("Plugin should be loaded");

    assert_eq!(plugin.name, "syncore_go_plugin");
    assert_eq!(plugin.status, syncore::dlr::PluginStatus::Ready);
    assert!(!plugin.capabilities.is_empty(), "Plugin should have capabilities");

    // Check if plugin has expected capabilities
    let has_index_capability = plugin.capabilities.contains(&PluginCapability::IndexDirectory);
    assert!(has_index_capability, "Plugin should have IndexDirectory capability");

    Ok(())
}

#[test]
fn test_dlr_execute_go_plugin_index_file() -> Result<()> {
    let plugins_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins");

    let mut dlr_manager = DlrManager::new(&plugins_dir);

    // Discover and load the plugin
    dlr_manager.discover_and_register_plugins()?;
    dlr_manager.load_plugin("syncore_go_plugin")?;

    // Create a test Go file
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");
    std::fs::write(&file_path, r#"
package main

import "fmt"

func main() {
    fmt.Println("Hello, world!")
}
"#)?;

    // Execute go.index_file task
    let mut params = HashMap::new();
    params.insert("file_path".to_string(),
                 serde_json::Value::String(file_path.to_string_lossy().to_string()));

    let result = dlr_manager.execute_plugin_task("syncore_go_plugin", "go.index_file", params)?;

    // Check the result
    assert!(result.contains_key("result"));
    assert!(result.contains_key("diagnostics"));
    assert!(result.contains_key("entities"));

    // Check if we have entities
    if let Some(entities) = result.get("entities").and_then(|v| v.as_array()) {
        assert!(!entities.is_empty(), "Should have entities from Go file");
    }

    Ok(())
}

#[test]
fn test_dlr_execute_go_plugin_diagnostics() -> Result<()> {
    let plugins_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins");

    let mut dlr_manager = DlrManager::new(&plugins_dir);

    // Discover and load the plugin
    dlr_manager.discover_and_register_plugins()?;
    dlr_manager.load_plugin("syncore_go_plugin")?;

    // Create a test Go file with potential issues
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.go");
    std::fs::write(&file_path, r#"
package main

import (
    "fmt"
    _ "os"  // Import but not using
)

func main() {
    var unusedVar string  // Declared but not used
    fmt.Println("Hello, world!")
}
"#)?;

    // Execute go.run_diagnostics task
    let mut params = HashMap::new();
    params.insert("file_path".to_string(),
                 serde_json::Value::String(file_path.to_string_lossy().to_string()));

    let result = dlr_manager.execute_plugin_task("syncore_go_plugin", "go.run_diagnostics", params)?;

    // Check the result
    assert!(result.contains_key("result"));
    assert!(result.contains_key("diagnostics"));

    // Check if we have diagnostics (might be empty if tools aren't available)
    if let Some(diagnostics) = result.get("diagnostics").and_then(|v| v.as_array()) {
        // We might not get any diagnostics if the tools aren't available, but the structure should be correct
        println!("Diagnostics count: {}", diagnostics.len());
        for diag in diagnostics {
            println!("Diagnostic: {}", diag);
        }
    }

    Ok(())
}

#[test]
fn test_dlr_unload_go_plugin() -> Result<()> {
    let plugins_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins");

    let mut dlr_manager = DlrManager::new(&plugins_dir);

    // Discover and load the plugin
    dlr_manager.discover_and_register_plugins()?;
    dlr_manager.load_plugin("syncore_go_plugin")?;

    // Verify plugin is loaded
    let plugin = dlr_manager.get_plugin("syncore_go_plugin")
        .expect("Plugin should be loaded");
    assert_eq!(plugin.status, syncore::dlr::PluginStatus::Ready);

    // Unload the plugin
    dlr_manager.unload_plugin("syncore_go_plugin")?;

    // Verify plugin is unloaded
    let plugin = dlr_manager.get_plugin("syncore_go_plugin")
        .expect("Plugin should still be registered");
    assert_eq!(plugin.status, syncore::dlr::PluginStatus::Shutdown);

    Ok(())
}
*/
