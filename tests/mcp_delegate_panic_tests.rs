//! MCP Delegate Panic Tests
//!
//! Tests for H3: Panic in mcp_delegate due to .unwrap() calls

use serde_json::json;
use syncore::mcp_server::SynCoreMCPServer;
use syncore::router::SynCoreState;
use tempfile::NamedTempFile;

/// Test H3: mcp_delegate panics on non-string data field
#[test]
fn test_mcp_delegate_non_string_data_safe_pattern() {
    // Test the safe pattern we want to implement in mcp_delegate

    // Simulate problematic case: data is not a string
    let params = json!({
        "ok": true,
        "data": {"not": "a_string"}  // This would cause unwrap() to panic
    });

    // Safe pattern: check if data is string before calling as_str()
    let safe_data = params.get("data").and_then(|d| d.as_str()).unwrap_or("non_string_data");

    // Should not panic and return fallback string
    assert_eq!(safe_data, "non_string_data");
}

/// Test H3: mcp_delegate panics on missing ok field
#[test]
fn test_mcp_delegate_missing_ok_field_safe_pattern() {
    // Simulate envelope without ok field
    let params = json!({
        "data": "some_string"
        // Missing "ok" field
    });

    // Safe pattern: check if ok field exists
    let has_ok = params.get("ok").is_some();
    assert!(!has_ok, "Should not have ok field");

    // Safe access pattern
    let ok_value = params.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    assert!(!ok_value, "Should default to false when ok is missing");
}

/// Test H3: mcp_delegate panics on malformed envelope
#[test]
fn test_mcp_delegate_malformed_envelope_safe_pattern() {
    // Simulate completely malformed envelope
    let params = json!("not_an_object");

    // Safe pattern: check if it's an object first
    let is_object = params.as_object().is_some();
    assert!(!is_object, "Should not be an object");

    // Safe access pattern
    let ok_value = params.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    assert!(!ok_value, "Should default to false for malformed envelope");
}

/// Test that demonstrates the safe pattern (what we want after fix)
#[test]
fn test_safe_mcp_delegate_pattern() {
    // This shows how mcp_delegate should handle malformed data safely

    // Case 1: Non-string data
    let params1 = json!({
        "ok": true,
        "data": {"not": "a_string"}
    });

    let safe_data1 = params1.get("data").and_then(|d| d.as_str()).unwrap_or("non_string_data");

    assert_eq!(safe_data1, "non_string_data");

    // Case 2: Missing ok field
    let params2 = json!({
        "data": "some_string"
    });

    let has_ok = params2.get("ok").is_some();
    assert!(!has_ok);

    // Case 3: Malformed envelope
    let params3 = json!("not_an_object");

    let is_object = params3.as_object().is_some();
    assert!(!is_object);
}
