use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use syncore::dlr::{DlrError, IpcClient};
use tempfile::TempDir;

#[cfg(test)]
mod dlr_ipc_tests {
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
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() { continue; }
        
        let request: serde_json::Value = serde_json::from_str(trimmed_line).unwrap();
        
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
            } else if task == "test_task" {
                let response = serde_json::json!({
                    "result": {"output": "test_result"},
                    "diagnostics": [{"level": "info", "message": "test diagnostic"}],
                    "entities": [{"type": "function", "name": "test_func"}]
                });
                println!("{}", serde_json::to_string(&response).unwrap());
                io::stdout().flush().unwrap();
            } else {
                let response = serde_json::json!({
                    "result": {},
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
    fn test_ipc_client_init_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut child = std::process::Command::new(&plugin_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn test plugin");

        let mut ipc_client = IpcClient::new(child).unwrap();

        let result = ipc_client.init_plugin("test_plugin", "0.1.0");
        assert!(result.is_ok(), "Should initialize plugin successfully");

        let response = result.unwrap();
        assert_eq!(response.status, "ready");
    }

    #[test]
    fn test_ipc_client_get_capabilities() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut child = std::process::Command::new(&plugin_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn test plugin");

        let mut ipc_client = IpcClient::new(child).unwrap();

        let result = ipc_client.get_capabilities();
        assert!(result.is_ok(), "Should get capabilities successfully");

        let response = result.unwrap();
        assert_eq!(response.capabilities.len(), 4);
        assert!(response.capabilities.contains(&"index_directory".to_string()));
        assert!(response.capabilities.contains(&"lsp_ingest".to_string()));
        assert!(response.capabilities.contains(&"lint_ingest".to_string()));
        assert!(response.capabilities.contains(&"diagnostics_export".to_string()));
    }

    #[test]
    fn test_ipc_client_execute_task() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut child = std::process::Command::new(&plugin_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn test plugin");

        let mut ipc_client = IpcClient::new(child).unwrap();

        let mut params = HashMap::new();
        params
            .insert("test_param".to_string(), serde_json::Value::String("test_value".to_string()));

        let result = ipc_client.execute_task("test_task", params);
        assert!(result.is_ok(), "Should execute task successfully");

        let response = result.unwrap();
        assert!(response.result.get("output").is_some());
        assert_eq!(response.diagnostics.len(), 1);
        assert_eq!(response.entities.len(), 1);
    }

    #[test]
    fn test_ipc_client_shutdown_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = create_test_plugin_binary(&temp_dir, "test_plugin");

        let mut child = std::process::Command::new(&plugin_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn test plugin");

        let mut ipc_client = IpcClient::new(child).unwrap();

        let result = ipc_client.shutdown_plugin();
        assert!(result.is_ok(), "Should shutdown plugin successfully");

        let response = result.unwrap();
        assert_eq!(response.status, "ok");
    }

    #[test]
    fn test_ipc_client_handles_invalid_response() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("invalid_plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        let cargo_toml = r#"
[package]
name = "invalid_plugin"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "invalid_plugin"
path = "src/main.rs"
"#;

        let main_rs = r#"
use std::io::{self, Write};

fn main() {
    println!("invalid json response");
    io::stdout().flush().unwrap();
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
            .expect("Failed to build invalid plugin");

        let plugin_path = plugin_dir.join("target/release").join("invalid_plugin");

        let mut child = std::process::Command::new(&plugin_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn invalid plugin");

        let mut ipc_client = IpcClient::new(child).unwrap();

        let result = ipc_client.init_plugin("invalid_plugin", "0.1.0");
        assert!(result.is_err(), "Should fail with invalid response");
        assert!(matches!(result.unwrap_err(), DlrError::InvalidResponse(_)));
    }

    #[test]
    fn test_ipc_client_handles_crashed_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("crash_plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

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

        let src_dir = plugin_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(plugin_dir.join("Cargo.toml"), cargo_toml).unwrap();
        fs::write(src_dir.join("main.rs"), main_rs).unwrap();

        let output = Command::new("cargo")
            .args(&["build", "--release"])
            .current_dir(&plugin_dir)
            .output()
            .expect("Failed to build crash plugin");

        let plugin_path = plugin_dir.join("target/release").join("crash_plugin");

        let mut child = std::process::Command::new(&plugin_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn crash plugin");

        let mut ipc_client = IpcClient::new(child).unwrap();

        let result = ipc_client.init_plugin("crash_plugin", "0.1.0");
        assert!(result.is_err(), "Should fail when plugin crashes");
        assert!(matches!(result.unwrap_err(), DlrError::IpcFailed(_)));
    }
}
