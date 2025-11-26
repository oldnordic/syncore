//! APEX 2.5-LSP: LSP Bridge Static Tests (TDD-First)
//!
//! Unit-level tests for JSON-RPC parsing, framing, and LSP status logic.
//! Expected to fail until src/lsp_bridge/mod.rs is implemented.

use std::path::Path;

// These imports will fail until implementation exists
use syncore::lsp_bridge::{LspBridge, LspDiagnosticEvent, LspStatus};

// ============================================================================
// TEST 1: Status Disabled When No Server Found
// ============================================================================

#[tokio::test]
async fn test_lsp_status_disabled_when_no_server_found() {
    // Simulate environment where rust-analyzer is not present
    // by using a nonexistent language_id
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    let result = LspBridge::try_start(root, "nonexistent-language").await;

    // Should gracefully return Disabled, not error
    assert!(result.is_ok(), "Should gracefully handle missing server");

    let bridge = result.unwrap();
    assert_eq!(
        bridge.status(),
        LspStatus::Disabled,
        "Status should be Disabled when server not found"
    );
}

// ============================================================================
// TEST 2: Status Ready When Server Present
// ============================================================================

#[tokio::test]
#[ignore = "Requires rust-analyzer installed on PATH"]
async fn test_lsp_status_ready_when_server_present() {
    // This test is conditionally run only when rust-analyzer is available
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Check if rust-analyzer is on PATH
    let which_result = which::which("rust-analyzer");
    if which_result.is_err() {
        // Skip test if rust-analyzer not available
        return;
    }

    let result = LspBridge::try_start(root, "rust").await;

    assert!(
        result.is_ok(),
        "Should successfully start rust-analyzer: {:?}",
        result.err()
    );

    let bridge = result.unwrap();
    assert_eq!(
        bridge.status(),
        LspStatus::Ready,
        "Status should be Ready when rust-analyzer available"
    );
}

// ============================================================================
// TEST 3: Parse publishDiagnostics Payload
// ============================================================================

#[test]
fn test_parse_publish_diagnostics_payload() {
    // Valid LSP publishDiagnostics notification
    let payload = r#"{
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///tmp/test.rs",
            "diagnostics": [
                {
                    "range": {
                        "start": {"line": 5, "character": 10},
                        "end": {"line": 5, "character": 20}
                    },
                    "severity": 1,
                    "code": "E0308",
                    "message": "mismatched types"
                }
            ]
        }
    }"#;

    // Helper function to parse (will be in lsp_bridge module)
    let events =
        syncore::lsp_bridge::parse_publish_diagnostics(payload).expect("Failed to parse");

    assert_eq!(events.len(), 1, "Should parse one diagnostic");

    let event = &events[0];
    assert!(event.path.to_str().unwrap().ends_with("test.rs"));
    assert_eq!(event.message, "mismatched types");
    assert_eq!(event.range, (5, 10, 5, 20)); // (start_line, start_col, end_line, end_col)
    assert_eq!(event.severity, Some(1));
    assert_eq!(event.code.as_deref(), Some("E0308"));
}

// ============================================================================
// TEST 4: JSON-RPC Message Framing Roundtrip
// ============================================================================

#[test]
fn test_jsonrpc_message_framing_roundtrip() {
    let json_payload = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;

    // Frame the message (Content-Length: N\r\n\r\n<json>)
    let framed = syncore::lsp_bridge::frame_jsonrpc_message(json_payload);

    assert!(framed.starts_with("Content-Length: "));
    assert!(framed.contains("\r\n\r\n"));
    assert!(framed.ends_with(json_payload));

    // Parse it back
    let parsed = syncore::lsp_bridge::parse_jsonrpc_message(&framed).expect("Failed to parse");

    assert_eq!(parsed, json_payload, "Roundtrip should preserve payload");
}
