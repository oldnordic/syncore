use anyhow::Result;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use tempfile::TempDir;
use syncore_go_plugin::plugin_api::{PluginRequest, PluginResponse};

// Helper function to start the plugin process
fn start_plugin_process() -> Result<std::process::Child> {
    // Use the debug binary from target/debug
    let plugin_path = format!("{}/target/debug/syncore_go_plugin", env!("CARGO_MANIFEST_DIR"));

    // Build the plugin first if it doesn't exist
    if !std::path::Path::new(&plugin_path).exists() {
        let status = Command::new("cargo")
            .args(&["build", "--bin", "syncore_go_plugin"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to build syncore_go_plugin binary"));
        }
    }

    // Start the plugin process
    let child = Command::new(&plugin_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    Ok(child)
}

// Helper function to send request to plugin and get response
fn send_request(plugin: &mut std::process::Child, request: &PluginRequest) -> Result<PluginResponse> {
    let request_json = serde_json::to_string(request)?;

    // Send the request with newline
    if let Some(stdin) = plugin.stdin.as_mut() {
        writeln!(stdin, "{}", request_json)?;
        stdin.flush()?;
    } else {
        return Err(anyhow::anyhow!("Failed to get stdin handle for plugin"));
    }

    // Read one line response using BufReader
    if let Some(stdout) = plugin.stdout.take() {
        let mut reader = BufReader::new(stdout);
        let mut response_line = String::new();
        let bytes_read = reader.read_line(&mut response_line)?;

        if bytes_read == 0 {
            return Err(anyhow::anyhow!("No response from plugin"));
        }

        let response: PluginResponse = serde_json::from_str(response_line.trim())?;
        Ok(response)
    } else {
        Err(anyhow::anyhow!("Failed to get stdout handle for plugin"))
    }
}

// Helper to send raw JSON and get response
fn send_raw_request(plugin: &mut std::process::Child, request_json: &str) -> Result<PluginResponse> {
    // Send the request with newline
    if let Some(stdin) = plugin.stdin.as_mut() {
        writeln!(stdin, "{}", request_json)?;
        stdin.flush()?;
    } else {
        return Err(anyhow::anyhow!("Failed to get stdin handle for plugin"));
    }

    // Read one line response using BufReader
    if let Some(stdout) = plugin.stdout.take() {
        let mut reader = BufReader::new(stdout);
        let mut response_line = String::new();
        let bytes_read = reader.read_line(&mut response_line)?;

        if bytes_read == 0 {
            return Err(anyhow::anyhow!("No response from plugin"));
        }

        let response: PluginResponse = serde_json::from_str(response_line.trim())?;
        Ok(response)
    } else {
        Err(anyhow::anyhow!("Failed to get stdout handle for plugin"))
    }
}

#[test]
fn test_init_event() -> Result<()> {
    let mut plugin = start_plugin_process()?;

    let request = PluginRequest::Init {
        plugin_name: "go_plugin".to_string(),
        version: "1.0.0".to_string(),
    };

    let response = send_request(&mut plugin, &request)?;

    // Check that the response has the expected status
    assert_eq!(response.status, "ready");
    assert_eq!(response.plugin_name, Some("go_plugin".to_string()));
    assert!(response.supported_tasks.is_some());

    let supported_tasks = response.supported_tasks.unwrap();
    assert!(!supported_tasks.is_empty());

    // Clean up
    let _ = plugin.kill();
    Ok(())
}

#[test]
fn test_capabilities_event() -> Result<()> {
    let mut plugin = start_plugin_process()?;

    let request = PluginRequest::Capabilities;

    let response = send_request(&mut plugin, &request)?;

    // Check that the response has the expected status
    assert_eq!(response.status, "ok");
    assert!(response.tasks.is_some());

    let tasks = response.tasks.unwrap();
    assert!(!tasks.is_empty());

    // Verify all expected tasks are present
    let expected_tasks = vec![
        "go.index_file",
        "go.index_directory",
        "go.run_diagnostics",
        "go.find_references",
        "go.symbol_graph",
    ];

    for task in expected_tasks {
        assert!(tasks.contains(&task.to_string()), "Missing expected task: {}", task);
    }

    // Clean up
    let _ = plugin.kill();
    Ok(())
}

#[test]
fn test_execute_index_file() -> Result<()> {
    let mut plugin = start_plugin_process()?;

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

    let mut params = HashMap::new();
    params.insert("file_path".to_string(),
                 serde_json::Value::String(file_path.to_string_lossy().to_string()));

    let request = PluginRequest::Execute {
        task: "go.index_file".to_string(),
        params,
    };

    let response = send_request(&mut plugin, &request)?;

    // Check that the response has the expected status
    assert_eq!(response.status, "ok");
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result.entities.is_some());
    assert!(result.edges.is_some());

    let entities = result.entities.unwrap();
    assert!(!entities.is_empty());

    // Verify we have at least a package entity
    let package_found = entities.iter().any(|e| e.name == "main");
    assert!(package_found, "Should find package main");

    // Clean up
    let _ = plugin.kill();
    Ok(())
}

#[test]
fn test_execute_run_diagnostics() -> Result<()> {
    let mut plugin = start_plugin_process()?;

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

    let mut params = HashMap::new();
    params.insert("file_path".to_string(),
                 serde_json::Value::String(file_path.to_string_lossy().to_string()));

    let request = PluginRequest::Execute {
        task: "go.run_diagnostics".to_string(),
        params,
    };

    let response = send_request(&mut plugin, &request)?;

    // Check that the response has the expected status
    assert_eq!(response.status, "ok");
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result.diagnostics.is_some());

    let _diagnostics = result.diagnostics.unwrap();
    // We might not get any diagnostics if the tools aren't available, but the structure should be correct

    // Clean up
    let _ = plugin.kill();
    Ok(())
}

#[test]
fn test_shutdown_event() -> Result<()> {
    let mut plugin = start_plugin_process()?;

    let request = PluginRequest::Shutdown;

    let response = send_request(&mut plugin, &request)?;

    // Check that the response has the expected status
    assert_eq!(response.status, "ok");

    // Clean up
    let _ = plugin.kill();
    Ok(())
}

#[test]
fn test_invalid_event() -> Result<()> {
    let mut plugin = start_plugin_process()?;

    // Create a custom invalid request - this will fail to parse
    let invalid_request = r#"{"event": "invalid_event", "data": "some data"}"#;

    // The plugin should return an error because it can't parse this as PluginRequest
    let result = send_raw_request(&mut plugin, invalid_request);

    // The plugin will exit with error since it can't parse invalid event
    // Check that we get some kind of error (either parse error or empty response)
    match result {
        Ok(response) => {
            // If we somehow get a response, it should be an error
            assert_eq!(response.status, "error");
        }
        Err(_) => {
            // Expected - the plugin exits with error on invalid input
        }
    }

    // Clean up
    let _ = plugin.kill();
    Ok(())
}

#[test]
fn test_missing_required_params() -> Result<()> {
    let mut plugin = start_plugin_process()?;

    // Request with missing required parameter
    let request = PluginRequest::Execute {
        task: "go.index_file".to_string(),
        params: HashMap::new(), // Missing file_path parameter
    };

    let response = send_request(&mut plugin, &request)?;

    // Should result in an error response
    assert_eq!(response.status, "error");
    assert!(response.error.is_some());

    let error_message = response.error.unwrap();
    assert!(error_message.contains("file_path"));

    // Clean up
    let _ = plugin.kill();
    Ok(())
}
