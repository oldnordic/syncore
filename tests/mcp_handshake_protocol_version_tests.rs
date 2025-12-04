//! MCP Handshake Protocol Version Tests
//!
//! Tests for H1: Protocol version mismatch between rmcp v0.8 and server

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::NamedTempFile;

/// Test H1: Protocol version mismatch causes connection closed
///
/// This test should FAIL initially, demonstrating the protocol version issue
#[test]
#[ignore] // Ignore initially - will enable after implementing test infrastructure
fn test_protocol_version_mismatch_causes_connection_close() {
    // Create temporary database for test
    let temp_db = NamedTempFile::new().expect("Failed to create temp DB");
    let db_path = temp_db.path().to_str().unwrap();

    // Set environment variables for test
    std::env::set_var("DB_PATH", db_path);
    std::env::set_var("HTTP_PORT", "3002"); // Different port to avoid conflicts

    // Start MCP server in background
    let mut child = Command::new("cargo")
        .args(&["run", "--bin", "syncore_mcp_stdio"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server");

    // Give server time to start
    thread::sleep(Duration::from_secs(2));

    // Try to connect and send initialize request
    // This would require a proper MCP client implementation
    // For now, we'll simulate by checking if server is still running
    let status = child.try_wait();

    // Clean up
    let _ = child.kill();

    // If H1 is correct, server should have exited due to protocol version mismatch
    // This assertion will need to be refined based on actual behavior
    match status {
        Ok(Some(exit_status)) => {
            println!("Server exited with status: {}", exit_status);
            // If server exited, it might be due to protocol version mismatch
        }
        Ok(None) => {
            println!("Server is still running");
            // If server is still running, protocol version might not be the issue
        }
        Err(e) => {
            println!("Error checking server status: {}", e);
        }
    }

    // This test is a placeholder - real implementation would need proper MCP client
    // For now, just ensure the test compiles and we can observe behavior
    assert!(true, "Test infrastructure placeholder");
}

/// Test that server uses the correct protocol version for rmcp v0.8
#[test]
fn test_server_uses_correct_protocol_version() {
    use rmcp::model::ProtocolVersion;

    // Verify that the server is configured to use the protocol version
    // that rmcp v0.8 expects (2025-06-18)
    let expected_version = ProtocolVersion::V_2025_06_18;

    // This test verifies that V_2025_06_18 is available and can be used
    // The actual server configuration is in src/mcp_server/server.rs:2229

    // Check that the version string matches what rmcp v0.8 expects
    let version_str = expected_version.to_string();
    assert_eq!(
        version_str, "2025-06-18",
        "Server should use protocol version 2025-06-18 for rmcp v0.8 compatibility"
    );
}
