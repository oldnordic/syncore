use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use syncore::dlr::{DlrError, DlrManager, PluginCapability, PluginStatus};
use tempfile::TempDir;

#[cfg(test)]
mod dlr_lifecycle_tests {
    use super::*;

    fn create_test_plugin_binary(temp_dir: &TempDir, plugin_name: &str) -> String {
        let plugin_dir = temp_dir.path().join(plugin_name);
        fs::create_dir_all(&plugin_dir).unwrap();

        let cargo_toml = format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{}"
path = "src/main.rs"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
"#,
            plugin_name, plugin_name
        );

        let main_rs = r#"
use serde_json;
use std::io::{self, Write};

fn main() {
    let mut line = String::new();
    while io::stdin().read_line(&mut line).unwrap() > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() { line.clear(); continue; }
        
        let request: serde_json::Value = serde_json::from_str(trimmed).unwrap();
        
        if let Some(event) = request.get("event") {
            if event == "init" {
                let response = serde_json::json!({
                    "status": "ready"
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
            } else if event == "shutdown" {
                let response = serde_json::json!({
                    "status": "ok"
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
                break;
            }
        } else if let Some(task) = request.get("task") {
            if task == "capabilities" {
                let response = serde_json::json!({
                    "capabilities": ["index_directory", "lsp_ingest", "lint_ingest", "diagnostics_export"]
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
            } else {
                let response = serde_json::json!({
                    "result": {"task": task.as_str().unwrap_or("unknown")},
                    "diagnostics": [],
                    "entities": []
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
            }
        }
        
        line.clear();
    }
}
"#;

        let src_dir = plugin_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(plugin_dir.join("Cargo.toml"), cargo_toml).unwrap();
        fs::write(src_dir.join("main.rs"), main_rs).unwrap();

        let output = Command::new("cargo")
            .args(&["build", "--release"])
            .current_dir(&plugin_dir)
            .output()
            .expect("Failed to build test plugin");

        if !output.status.success() {
            panic!("Failed to build test plugin: {}", String::from_utf8_lossy(&output.stderr));
        }

        plugin_dir.join("target/release").join(plugin_name).to_string_lossy().to_string()
    }

    #[test]
    fn test_dlr_manager_discovers_and_registers_plugins() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut manager = DlrManager::new(temp_dir.path());

        let result = manager.discover_and_register_plugins();
        assert!(result.is_ok(), "Should discover and register plugins successfully");

        let count = result.unwrap();
        assert_eq!(count, 1, "Should discover and register 1 plugin");

        let plugins = manager.list_plugins();
        assert_eq!(plugins.len(), 1, "Should have 1 plugin in registry");

        let plugin = plugins.iter().find(|p| p.name == "test_plugin").unwrap();
        assert_eq!(plugin.name, "test_plugin");
        assert_eq!(plugin.status, PluginStatus::Unloaded);
    }

    #[test]
    fn test_dlr_manager_loads_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();

        let result = manager.load_plugin("test_plugin");
        assert!(result.is_ok(), "Should load plugin successfully");

        let plugin = manager.get_plugin("test_plugin").unwrap();
        assert_eq!(plugin.status, PluginStatus::Ready);
        assert!(plugin.process_id.is_some(), "Plugin should have a process ID");
        assert_eq!(plugin.capabilities.len(), 4, "Plugin should have 4 capabilities");
        assert!(plugin.capabilities.contains(&PluginCapability::IndexDirectory));
        assert!(plugin.capabilities.contains(&PluginCapability::LspIngest));
        assert!(plugin.capabilities.contains(&PluginCapability::LintIngest));
        assert!(plugin.capabilities.contains(&PluginCapability::DiagnosticsExport));
    }

    #[test]
    fn test_dlr_manager_loads_nonexistent_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = DlrManager::new(temp_dir.path());

        let result = manager.load_plugin("nonexistent_plugin");
        assert!(result.is_err(), "Should fail to load nonexistent plugin");
        assert!(matches!(result.unwrap_err(), DlrError::PluginNotFound(_)));
    }

    #[test]
    fn test_dlr_manager_unloads_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();
        manager.load_plugin("test_plugin").unwrap();

        let plugin_before = manager.get_plugin("test_plugin").unwrap();
        assert_eq!(plugin_before.status, PluginStatus::Ready);
        assert!(plugin_before.process_id.is_some());

        let result = manager.unload_plugin("test_plugin");
        assert!(result.is_ok(), "Should unload plugin successfully");

        let plugin_after = manager.get_plugin("test_plugin").unwrap();
        assert_eq!(plugin_after.status, PluginStatus::Shutdown);
        assert!(plugin_after.process_id.is_none());
    }

    #[test]
    fn test_dlr_manager_unloads_nonexistent_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = DlrManager::new(temp_dir.path());

        let result = manager.unload_plugin("nonexistent_plugin");
        assert!(result.is_err(), "Should fail to unload nonexistent plugin");
        assert!(matches!(result.unwrap_err(), DlrError::PluginNotFound(_)));
    }

    #[test]
    fn test_dlr_manager_shutdown_all() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin1_path = create_test_plugin_binary(&temp_dir, "plugin1");
        let _plugin2_path = create_test_plugin_binary(&temp_dir, "plugin2");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();
        manager.load_plugin("plugin1").unwrap();
        manager.load_plugin("plugin2").unwrap();

        let plugin1_before = manager.get_plugin("plugin1").unwrap();
        let plugin2_before = manager.get_plugin("plugin2").unwrap();
        assert_eq!(plugin1_before.status, PluginStatus::Ready);
        assert_eq!(plugin2_before.status, PluginStatus::Ready);

        let result = manager.shutdown_all();
        assert!(result.is_ok(), "Should shutdown all plugins successfully");

        let plugin1_after = manager.get_plugin("plugin1").unwrap();
        let plugin2_after = manager.get_plugin("plugin2").unwrap();
        assert_eq!(plugin1_after.status, PluginStatus::Shutdown);
        assert_eq!(plugin2_after.status, PluginStatus::Shutdown);
    }

    #[test]
    fn test_dlr_manager_find_plugins_by_capability() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();
        manager.load_plugin("test_plugin").unwrap();

        let index_plugins = manager.find_plugins_by_capability(&PluginCapability::IndexDirectory);
        assert_eq!(index_plugins.len(), 1, "Should find 1 plugin with IndexDirectory capability");

        let lsp_plugins = manager.find_plugins_by_capability(&PluginCapability::LspIngest);
        assert_eq!(lsp_plugins.len(), 1, "Should find 1 plugin with LspIngest capability");

        let lint_plugins = manager.find_plugins_by_capability(&PluginCapability::LintIngest);
        assert_eq!(lint_plugins.len(), 1, "Should find 1 plugin with LintIngest capability");

        let diag_plugins = manager.find_plugins_by_capability(&PluginCapability::DiagnosticsExport);
        assert_eq!(diag_plugins.len(), 1, "Should find 1 plugin with DiagnosticsExport capability");
    }

    #[test]
    fn test_dlr_manager_handles_plugin_crash_during_load() {
        let temp_dir = TempDir::new().unwrap();

        let cargo_toml = r#"
[package]
name = "crash_plugin"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "crash_plugin"
path = "src/main.rs"
"#;

        let main_rs = r#"
fn main() {
    std::process::exit(1);
}
"#;

        let plugin_dir = temp_dir.path().join("crash_plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        let src_dir = plugin_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(plugin_dir.join("Cargo.toml"), cargo_toml).unwrap();
        fs::write(src_dir.join("main.rs"), main_rs).unwrap();

        let output = Command::new("cargo")
            .args(&["build", "--release"])
            .current_dir(&plugin_dir)
            .output()
            .expect("Failed to build crash plugin");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();

        let result = manager.load_plugin("crash_plugin");
        assert!(result.is_err(), "Should fail to load crashed plugin");
        assert!(matches!(
            result.unwrap_err(),
            DlrError::PluginStartFailed(_) | DlrError::IpcFailed(_)
        ));
    }

    #[test]
    fn test_dlr_manager_drop_shuts_down_plugins() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        {
            let mut manager = DlrManager::new(temp_dir.path());
            manager.discover_and_register_plugins().unwrap();
            manager.load_plugin("test_plugin").unwrap();

            let plugin = manager.get_plugin("test_plugin").unwrap();
            assert_eq!(plugin.status, PluginStatus::Ready);
            assert!(plugin.process_id.is_some());
        }

        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();

        let plugin = manager.get_plugin("test_plugin").unwrap();
        assert_eq!(plugin.status, PluginStatus::Unloaded);
        assert!(plugin.process_id.is_none());
    }
}
