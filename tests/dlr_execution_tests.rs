use std::collections::HashMap;
use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;
use syncore::dlr::{DlrError, DlrManager};
use tempfile::TempDir;

#[cfg(test)]
mod dlr_execution_tests {
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
    let mut line_buffer = String::new();
    while io::stdin().read_line(&mut line_buffer).unwrap() > 0 {
        let line = line_buffer.trim();
        if line.is_empty() { continue; }
        
        let request: serde_json::Value = serde_json::from_str(line).unwrap();
        
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
            } else if task == "index_directory" {
                let params = request.get("params").unwrap().as_object().unwrap();
                let directory = params.get("directory").unwrap().as_str().unwrap();
                
                let response = serde_json::json!({
                    "result": {
                        "indexed_files": 42,
                        "directory": directory
                    },
                    "diagnostics": [
                        {"level": "info", "message": "Successfully indexed directory", "file": "test.rs"}
                    ],
                    "entities": [
                        {"type": "function", "name": "test_function", "file": "test.rs", "line": 10},
                        {"type": "class", "name": "TestClass", "file": "test.rs", "line": 20}
                    ]
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
            } else if task == "lsp_ingest" {
                let response = serde_json::json!({
                    "result": {
                        "symbols_processed": 15,
                        "completions_generated": 8
                    },
                    "diagnostics": [
                        {"level": "warning", "message": "Unused variable", "file": "test.rs", "line": 5}
                    ],
                    "entities": [
                        {"type": "variable", "name": "test_var", "file": "test.rs", "line": 5}
                    ]
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
            } else if task == "error_task" {
                eprintln!("Simulated plugin error");
                std::process::exit(1);
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
        
        line_buffer.clear();
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
    fn test_dlr_manager_executes_plugin_task() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();
        manager.load_plugin("test_plugin").unwrap();

        let mut params = HashMap::new();
        params.insert("directory".to_string(), serde_json::Value::String("/test/dir".to_string()));

        let result = manager.execute_plugin_task("test_plugin", "index_directory", params);
        assert!(result.is_ok(), "Should execute plugin task successfully");

        let response = result.unwrap();
        assert!(response.contains_key("result"));
        assert!(response.contains_key("diagnostics"));
        assert!(response.contains_key("entities"));

        let result_value = response.get("result").unwrap();
        assert!(result_value.get("indexed_files").is_some());
        assert!(result_value.get("directory").is_some());

        let diagnostics = response.get("diagnostics").unwrap().as_array().unwrap();
        assert_eq!(diagnostics.len(), 1);

        let entities = response.get("entities").unwrap().as_array().unwrap();
        assert_eq!(entities.len(), 2);
    }

    #[test]
    fn test_dlr_manager_executes_lsp_task() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();
        manager.load_plugin("test_plugin").unwrap();

        let params = HashMap::new();

        let result = manager.execute_plugin_task("test_plugin", "lsp_ingest", params);
        assert!(result.is_ok(), "Should execute LSP task successfully");

        let response = result.unwrap();
        let result_value = response.get("result").unwrap();
        assert!(result_value.get("symbols_processed").is_some());
        assert!(result_value.get("completions_generated").is_some());

        let diagnostics = response.get("diagnostics").unwrap().as_array().unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].get("level").unwrap(), "warning");
    }

    #[test]
    fn test_dlr_manager_executes_unknown_task() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();
        manager.load_plugin("test_plugin").unwrap();

        let params = HashMap::new();

        let result = manager.execute_plugin_task("test_plugin", "unknown_task", params);
        assert!(result.is_ok(), "Should execute unknown task successfully");

        let response = result.unwrap();
        let result_value = response.get("result").unwrap();
        assert_eq!(result_value.get("task").unwrap(), "unknown_task");

        let diagnostics = response.get("diagnostics").unwrap().as_array().unwrap();
        assert_eq!(diagnostics.len(), 0);

        let entities = response.get("entities").unwrap().as_array().unwrap();
        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn test_dlr_manager_executes_task_on_nonexistent_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = DlrManager::new(temp_dir.path());

        let params = HashMap::new();

        let result = manager.execute_plugin_task("nonexistent_plugin", "index_directory", params);
        assert!(result.is_err(), "Should fail to execute task on nonexistent plugin");
        assert!(matches!(result.unwrap_err(), DlrError::PluginNotFound(_)));
    }

    #[test]
    fn test_dlr_manager_executes_task_on_unloaded_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();

        let params = HashMap::new();

        let result = manager.execute_plugin_task("test_plugin", "index_directory", params);
        assert!(result.is_err(), "Should fail to execute task on unloaded plugin");
        assert!(matches!(result.unwrap_err(), DlrError::ExecutionFailed(_)));
    }

    #[test]
    fn test_dlr_manager_handles_plugin_crash_during_execution() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();
        manager.load_plugin("test_plugin").unwrap();

        let params = HashMap::new();

        let result = manager.execute_plugin_task("test_plugin", "error_task", params);
        assert!(result.is_err(), "Should fail when plugin crashes during execution");
        assert!(matches!(
            result.unwrap_err(),
            DlrError::IpcFailed(_) | DlrError::ExecutionFailed(_) | DlrError::InvalidResponse(_)
        ));
    }

    #[test]
    fn test_dlr_manager_multiple_task_executions() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();
        manager.load_plugin("test_plugin").unwrap();

        let mut params = HashMap::new();
        params.insert("directory".to_string(), serde_json::Value::String("/test/dir1".to_string()));

        let result1 = manager.execute_plugin_task("test_plugin", "index_directory", params.clone());
        assert!(result1.is_ok(), "First execution should succeed");

        params.insert("directory".to_string(), serde_json::Value::String("/test/dir2".to_string()));

        let result2 = manager.execute_plugin_task("test_plugin", "index_directory", params);
        assert!(result2.is_ok(), "Second execution should succeed");

        let response1 = result1.unwrap();
        let response2 = result2.unwrap();

        let dir1 = response1.get("result").unwrap().get("directory").unwrap();
        let dir2 = response2.get("result").unwrap().get("directory").unwrap();

        assert_ne!(dir1, dir2, "Different executions should produce different results");
    }

    #[test]
    fn test_dlr_manager_task_execution_with_complex_params() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();
        manager.load_plugin("test_plugin").unwrap();

        let mut params = HashMap::new();
        params.insert(
            "directory".to_string(),
            serde_json::Value::String("/complex/path".to_string()),
        );
        params.insert("recursive".to_string(), serde_json::Value::Bool(true));
        params.insert("file_types".to_string(), serde_json::json!(["rs", "toml", "json"]));
        params.insert(
            "options".to_string(),
            serde_json::json!({
                "follow_symlinks": false,
                "max_depth": 10,
                "exclude_patterns": ["target/", ".git/"]
            }),
        );

        let result = manager.execute_plugin_task("test_plugin", "index_directory", params);
        assert!(result.is_ok(), "Should execute task with complex parameters successfully");

        let response = result.unwrap();
        assert!(response.contains_key("result"));
        assert!(response.contains_key("diagnostics"));
        assert!(response.contains_key("entities"));
    }

    #[test]
    fn test_dlr_manager_concurrent_task_execution() {
        let temp_dir = TempDir::new().unwrap();
        let _plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut manager = DlrManager::new(temp_dir.path());
        manager.discover_and_register_plugins().unwrap();
        manager.load_plugin("test_plugin").unwrap();

        let mut params1 = HashMap::new();
        params1
            .insert("directory".to_string(), serde_json::Value::String("/test/dir1".to_string()));

        let mut params2 = HashMap::new();
        params2
            .insert("directory".to_string(), serde_json::Value::String("/test/dir2".to_string()));

        let temp_dir_path1 = temp_dir.path().to_path_buf();
        let handle1 = std::thread::spawn(move || {
            let mut manager_clone = DlrManager::new(&temp_dir_path1);
            manager_clone.discover_and_register_plugins().unwrap();
            manager_clone.load_plugin("test_plugin").unwrap();
            manager_clone.execute_plugin_task("test_plugin", "index_directory", params1)
        });

        let temp_dir_path2 = temp_dir.path().to_path_buf();
        let handle2 = std::thread::spawn(move || {
            let mut manager_clone = DlrManager::new(&temp_dir_path2);
            manager_clone.discover_and_register_plugins().unwrap();
            manager_clone.load_plugin("test_plugin").unwrap();
            manager_clone.execute_plugin_task("test_plugin", "index_directory", params2)
        });

        let result1 = handle1.join().unwrap();
        let result2 = handle2.join().unwrap();

        assert!(result1.is_ok(), "First concurrent execution should succeed");
        assert!(result2.is_ok(), "Second concurrent execution should succeed");
    }
}
