//! APEX 2.5-LSP: LSP Bridge Integration Tests (TDD-First)
//!
//! Runtime tests that require actual rust-analyzer installation.
//! Expected to fail until src/lsp_bridge/mod.rs is implemented.

use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

use syncore::lsp_bridge::{LspBridge, LspStatus};

// ============================================================================
// TEST 5: LSP Bridge Emits Diagnostic on Syntax Error
// ============================================================================

#[tokio::test]
#[ignore = "Requires rust-analyzer installed"]
async fn test_lsp_bridge_emits_diagnostic_on_syntax_error() {
    // Check if rust-analyzer is available
    if which::which("rust-analyzer").is_err() {
        return; // Skip if not available
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create Rust file with deliberate syntax error (missing closing brace)
    let test_file = root.join("error.rs");
    let bad_code = r#"
fn broken() {
    println!("missing closing brace");
"#;

    std::fs::write(&test_file, bad_code).expect("Failed to write test file");

    // Start LSP bridge
    let mut bridge = LspBridge::try_start(root, "rust")
        .await
        .expect("Failed to start bridge");

    assert_eq!(bridge.status(), LspStatus::Ready);

    // Send didOpen notification
    bridge
        .send_did_open(&test_file, bad_code)
        .await
        .expect("Failed to send didOpen");

    // Wait for diagnostic (with timeout)
    let diagnostic_result =
        timeout(Duration::from_secs(10), bridge.recv_diagnostic()).await;

    assert!(
        diagnostic_result.is_ok(),
        "Should receive diagnostic within timeout"
    );

    let diagnostic = diagnostic_result
        .unwrap()
        .expect("Should receive at least one diagnostic");

    // Verify diagnostic properties
    assert!(
        diagnostic.path.ends_with("error.rs"),
        "Diagnostic should be for error.rs, got: {:?}",
        diagnostic.path
    );

    assert!(
        !diagnostic.message.is_empty(),
        "Diagnostic message should not be empty"
    );

    // rust-analyzer typically reports errors as severity 1 (Error)
    assert!(
        diagnostic.severity.is_some(),
        "Diagnostic should have severity"
    );
}

// ============================================================================
// TEST 6: LSP Bridge Handles Multiple Files
// ============================================================================

#[tokio::test]
#[ignore = "Requires rust-analyzer installed"]
async fn test_lsp_bridge_handles_multiple_files() {
    if which::which("rust-analyzer").is_err() {
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // Create valid file
    let valid_file = root.join("valid.rs");
    let valid_code = r#"
fn main() {
    println!("Hello, world!");
}
"#;
    std::fs::write(&valid_file, valid_code).expect("Failed to write valid file");

    // Create invalid file
    let invalid_file = root.join("invalid.rs");
    let invalid_code = r#"
fn broken() {
    let x: u32 = "string"; // type mismatch
}
"#;
    std::fs::write(&invalid_file, invalid_code).expect("Failed to write invalid file");

    // Start LSP bridge
    let mut bridge = LspBridge::try_start(root, "rust")
        .await
        .expect("Failed to start bridge");

    assert_eq!(bridge.status(), LspStatus::Ready);

    // Open both files
    bridge
        .send_did_open(&valid_file, valid_code)
        .await
        .expect("Failed to send didOpen for valid file");

    bridge
        .send_did_open(&invalid_file, invalid_code)
        .await
        .expect("Failed to send didOpen for invalid file");

    // Collect diagnostics for a short time
    let mut received_diagnostics = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);

    while tokio::time::Instant::now() < deadline {
        match timeout(Duration::from_secs(1), bridge.recv_diagnostic()).await {
            Ok(Some(diag)) => {
                received_diagnostics.push(diag);
            }
            Ok(None) => break,
            Err(_) => break, // Timeout waiting for diagnostic
        }
    }

    // Should have at least one diagnostic for the invalid file
    assert!(
        !received_diagnostics.is_empty(),
        "Should receive diagnostics for invalid file"
    );

    // Check that diagnostics are associated with correct path
    let has_invalid_file_diagnostic = received_diagnostics
        .iter()
        .any(|d| d.path.ends_with("invalid.rs"));

    assert!(
        has_invalid_file_diagnostic,
        "Should have diagnostic for invalid.rs"
    );

    // Optionally: verify no diagnostics for valid file
    // (though rust-analyzer may send empty diagnostics to clear previous errors)
}
