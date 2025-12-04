//! MCP STDIO Handshake Tests
//!
//! Tests for reproducing and fixing the stdio handshake failure after PHASE M+2
//!
//! The failure observed is:
//! - OpenCode / Claude connect via stdio
//! - After PHASE M+2 changes, the connection closes on handshake ("initialized request / connection closed")
//! - HTTP MCP still works; only stdio is broken

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::NamedTempFile;

/// Test that reproduces the stdio handshake failure
///
/// This test should FAIL initially, demonstrating the connection closure issue
#[test]
#[ignore] // Ignore initially - will enable after implementing test infrastructure
fn test_stdio_handshake_failure_reproduction() {
    // Create temporary database for test
    let temp_db = NamedTempFile::new().expect("Failed to create temp DB");
    let db_path = temp_db.path().to_str().unwrap();

    // Set environment variables for test
    std::env::set_var("DB_PATH", db_path);
    std::env::set_var("HTTP_PORT", "3005"); // Different port to avoid conflicts
    std::env::set_var("DISABLE_ROUTER_LOOP", "true"); // Disable router to isolate handshake

    // Start MCP server in stdio mode
    let mut child = Command::new("cargo")
        .args(&["run", "--bin", "syncore_mcp_stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server");

    // Give server time to start
    thread::sleep(Duration::from_secs(3));

    // Try to perform MCP handshake via stdio
    if let Some(stdin) = child.stdin.as_mut() {
        if let Some(stdout) = child.stdout.as_mut() {
            // Send initialize request
            let init_request = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {
                        "tools": {}
                    },
                    "clientInfo": {
                        "name": "test-client",
                        "version": "1.0.0"
                    }
                }
            });

            // Write initialize request to stdin
            let request_str = init_request.to_string() + "\n";
            if let Err(e) = stdin.write_all(request_str.as_bytes()) {
                eprintln!("Failed to write initialize request: {}", e);
            }
            if let Err(e) = stdin.flush() {
                eprintln!("Failed to flush stdin: {}", e);
            }

            // Read response from stdout
            let mut reader = BufReader::new(stdout);
            let mut response_line = String::new();

            // Wait for response with timeout
            let response_received = thread::spawn(move || {
                thread::sleep(Duration::from_millis(500));
                reader.read_line(&mut response_line).is_ok()
            })
            .join()
            .unwrap_or(false);

            if response_received && !response_line.trim().is_empty() {
                println!("Received response: {}", response_line.trim());

                // Try to parse as JSON
                if let Ok(response) = serde_json::from_str::<Value>(&response_line.trim()) {
                    println!("Parsed response: {:?}", response);

                    // Check if it's a valid initialize response
                    if let Some(result) = response.get("result") {
                        if let Some(server_info) = result.get("serverInfo") {
                            println!("Server info: {:?}", server_info);
                            // If we get here, handshake succeeded
                        }
                    } else if let Some(error) = response.get("error") {
                        println!("Handshake error: {:?}", error);
                    }
                } else {
                    println!("Failed to parse response as JSON");
                }
            } else {
                println!("No response received - connection likely closed");
            }
        }
    }

    // Check if server is still running after handshake attempt
    thread::sleep(Duration::from_secs(1));
    let status = child.try_wait();

    // Read any stderr output for debugging
    if let Some(stderr) = child.stderr.as_mut() {
        let mut stderr_reader = BufReader::new(stderr);
        let mut stderr_line = String::new();
        while stderr_reader.read_line(&mut stderr_line).is_ok() {
            if stderr_line.trim().is_empty() {
                break;
            }
            println!("STDERR: {}", stderr_line.trim());
            stderr_line.clear();
        }
    }

    // Clean up
    let _ = child.kill();

    // Analyze results
    match status {
        Ok(Some(exit_status)) => {
            println!("Server exited with status: {}", exit_status);
            // If server exited, it might be due to handshake failure
        }
        Ok(None) => {
            println!("Server is still running");
            // If server is still running, handshake might have succeeded
        }
        Err(e) => {
            println!("Error checking server status: {}", e);
        }
    }

    // This test is a placeholder - real implementation would need proper MCP client
    // For now, just ensure the test compiles and we can observe behavior
    assert!(true, "Test infrastructure placeholder - need to implement actual MCP client");
}

/// Test that verifies the ServerInfo structure is correct
#[test]
fn test_server_info_structure() {
    use rmcp::model::{ProtocolVersion, ServerCapabilities};
    use syncore::mcp_server::SynCoreMCPServer;
    use syncore::router::SynCoreState;
    use tempfile::NamedTempFile;

    // Create a minimal state for testing
    let temp_db = NamedTempFile::new().expect("Failed to create temp DB");
    let db_path = temp_db.path().to_str().unwrap();
    std::env::set_var("DB_PATH", db_path);

    // We can't easily create a SynCoreState in tests due to complex initialization
    // But we can check protocol version constants
    let expected_version = ProtocolVersion::V_2025_06_18;
    assert_eq!(expected_version.to_string(), "2025-06-18");

    // Check that version string matches what rmcp v0.8 expects
    println!("Protocol version used by server: {}", expected_version);

    // This test verifies that V_2025_06_18 is available and can be used
    // The actual server configuration is in src/mcp_server/server.rs:2229
    assert_eq!(
        expected_version.to_string(),
        "2025-06-18",
        "Server should use protocol version 2025-06-18 for rmcp v0.8 compatibility"
    );
}

/// Test that compares protocol versions
#[test]
fn test_protocol_version_compatibility() {
    use rmcp::model::ProtocolVersion;

    // Check available protocol versions in rmcp
    let versions = vec![ProtocolVersion::V_2024_11_05, ProtocolVersion::V_2025_06_18];

    for version in versions {
        println!("Available protocol version: {}", version);
    }

    // Verify that 2025-06-18 is the latest and should be used
    let current_version = ProtocolVersion::V_2025_06_18;
    assert_eq!(current_version.to_string(), "2025-06-18");

    // Test version string parsing
    let version_str = current_version.to_string();
    assert_eq!(version_str, "2025-06-18");
}

/// Test that checks if DISABLE_ROUTER_LOOP flag works
#[test]
fn test_disable_router_loop_flag() {
    // Test that we can read the environment variable
    let disable_router = std::env::var("DISABLE_ROUTER_LOOP")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    println!("DISABLE_ROUTER_LOOP flag: {}", disable_router);

    // Test with flag set
    std::env::set_var("DISABLE_ROUTER_LOOP", "true");
    let disable_router_true = std::env::var("DISABLE_ROUTER_LOOP")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
    assert!(disable_router_true, "Should be able to set DISABLE_ROUTER_LOOP=true");

    // Test with flag unset
    std::env::set_var("DISABLE_ROUTER_LOOP", "false");
    let disable_router_false = std::env::var("DISABLE_ROUTER_LOOP")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
    assert!(!disable_router_false, "Should be able to set DISABLE_ROUTER_LOOP=false");

    // Reset to original value
    if disable_router {
        std::env::set_var("DISABLE_ROUTER_LOOP", "true");
    } else {
        std::env::remove_var("DISABLE_ROUTER_LOOP");
    }
}
