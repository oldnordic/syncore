//! Shared test helpers for RealExecutor tests
//!
//! This module provides common utilities for testing RealExecutor's envelope-wrapped responses.

use serde_json::{json, Value};

/// Helper to unwrap the 'data' field from a success envelope
///
/// Asserts that the envelope has ok=true and contains a data field.
/// Returns a reference to the data field for further assertions.
pub fn unwrap_data(v: &Value) -> &Value {
    assert_eq!(v.get("ok"), Some(&json!(true)), "Expected ok=true in envelope: {:?}", v);
    v.get("data").expect("Missing data field in success envelope")
}

/// Helper to unwrap the 'error' field from an error envelope
///
/// Asserts that the envelope has ok=false and contains an error field.
/// Returns a reference to the error field for further assertions.
pub fn unwrap_error(v: &Value) -> &Value {
    assert_eq!(v.get("ok"), Some(&json!(false)), "Expected ok=false in envelope: {:?}", v);
    v.get("error").expect("Missing error field in error envelope")
}

/// Assert that a response is a success envelope with the expected structure
pub fn assert_success_envelope(v: &Value) {
    assert_eq!(v.get("ok"), Some(&json!(true)), "Envelope should have ok=true");
    assert!(v.get("data").is_some(), "Success envelope must have 'data' field");
    assert!(v.get("error").is_none(), "Success envelope must not have 'error' field");
}

/// Assert that a response is an error envelope with the expected structure
pub fn assert_error_envelope(v: &Value) {
    assert_eq!(v.get("ok"), Some(&json!(false)), "Envelope should have ok=false");
    assert!(v.get("error").is_some(), "Error envelope must have 'error' field");
    assert!(v.get("data").is_none(), "Error envelope must not have 'data' field");
}

/// Assert that an error envelope has the required error fields
pub fn assert_error_fields(error: &Value) {
    assert!(error.get("type").is_some(), "Error must have 'type' field");
    assert!(error.get("message").is_some(), "Error must have 'message' field");
}
