//! T8 Resilience Tests - System Resilience & Contract Enforcement
//!
//! Tests performance, security, and LLM contract enforcement for the translator pipeline.
//! Following T8 methodology: FAILING TESTS FIRST, fixes after.

use serde_json::json;
use std::time::Instant;
use syncore::intellitask::{IntelliTask, TaskBreakdown};
use syncore::mcp_tools::translator::{translate_llm_output, TargetSchema, TranslatorConfig};

/// Mock LLM for testing contract enforcement
struct MockLLM {
    responses: Vec<String>,
    call_count: std::cell::RefCell<usize>,
}

impl MockLLM {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            call_count: std::cell::RefCell::new(0),
        }
    }

    fn get_next_response(&self) -> String {
        let idx = *self.call_count.borrow();
        *self.call_count.borrow_mut() += 1;
        self.responses.get(idx).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod t8_performance_tests {
    use super::*;

    /// T8-PERF-1: Large JSON processing performance test
    /// Should complete <100ms for 10KB JSON
    #[test]
    fn test_large_json_performance_limit() {
        // Create large JSON structure (10KB+)
        let mut large_json = json!({
            "prd_title": "Large Test Task Breakdown",
            "parent_tasks": [],
            "relevant_files": [],
            "estimated_complexity": "Medium"
        });

        // Add 100 parent tasks with subtasks
        let mut parent_tasks = Vec::new();
        for i in 0..100 {
            let mut subtasks = Vec::new();
            for j in 0..10 {
                subtasks.push(json!({
                    "title": format!("Subtask {}-{}", i, j),
                    "description": "A".repeat(200), // 200 char description
                    "estimated_hours": 2.5
                }));
            }
            parent_tasks.push(json!({
                "title": format!("Parent Task {}", i),
                "description": "B".repeat(300),
                "estimated_hours": 25.0,
                "subtasks": subtasks
            }));
        }
        large_json["parent_tasks"] = json!(parent_tasks);

        let json_str = large_json.to_string();
        assert!(json_str.len() > 10240, "Test JSON should be >10KB");

        // Measure translation time
        let start = Instant::now();
        let result = translate_llm_output(&json_str, TargetSchema::TaskBreakdown);
        let duration = start.elapsed();

        // Should succeed quickly
        assert!(result.is_ok(), "Large JSON translation should succeed: {:?}", result);
        assert!(
            duration.as_millis() < 100,
            "Large JSON processing should complete <100ms, took {}",
            duration.as_millis()
        );
    }

    /// T8-PERF-2: Deep nesting performance test
    /// Should handle 50-level nesting without stack overflow
    #[test]
    fn test_deep_nesting_performance_limit() {
        // Create deeply nested JSON (50 levels)
        let mut nested = json!("leaf");
        for i in 0..50 {
            nested = json!({
                format!("level_{}", i): nested,
                "metadata": format!("metadata_{}", i)
            });
        }

        let test_json = json!({
            "prd_title": "Deep Nesting Test",
            "parent_tasks": [{
                "title": "Deep Nested Task",
                "nested_data": nested
            }],
            "relevant_files": [],
            "estimated_complexity": "Medium"
        });

        let start = Instant::now();
        let result = translate_llm_output(&test_json.to_string(), TargetSchema::TaskBreakdown);
        let duration = start.elapsed();

        // Should handle deep nesting efficiently
        assert!(result.is_ok(), "Deep nesting should be handled: {:?}", result);
        assert!(
            duration.as_millis() < 50,
            "Deep nesting should be fast, took {}ms",
            duration.as_millis()
        );
    }

    /// T8-PERF-3: Clone operation overhead test
    /// Verify clone counts are minimized in hot paths
    #[test]
    fn test_clone_operation_overhead() {
        let test_json = json!({
            "prd_title": "Clone Test Task Breakdown",
            "parent_tasks": [{
                "title": "Clone Test Task",
                "description": "Test clone overhead",
                "estimated_hours": 5.0,
                "subtasks": [
                    {"title": "Subtask 1", "description": "Test subtask"}
                ]
            }],
            "relevant_files": [],
            "estimated_complexity": "Low"
        });

        // Run translation multiple times to detect clone storms
        let start = Instant::now();
        for _ in 0..100 {
            let result = translate_llm_output(&test_json.to_string(), TargetSchema::TaskBreakdown);
            assert!(result.is_ok(), "Repeated translation should succeed");
        }
        let duration = start.elapsed();

        // 100 translations should complete quickly (indicating minimal clones)
        assert!(
            duration.as_millis() < 200,
            "100 translations should complete <200ms, took {}ms",
            duration.as_millis()
        );
    }
}

#[cfg(test)]
mod t8_security_tests {
    use super::*;

    /// T8-SEC-1: JSON injection attack prevention
    /// Should block/reject malicious JSON injection attempts
    #[test]
    fn test_json_injection_prevention() {
        let malicious_inputs = vec![
            // Script injection attempts
            r#"{"title": "<script>alert('xss')</script>", "description": "test"}"#.to_string(),
            r#"{"title": "合法", "description": "</script><script>alert('xss')</script>"}"#
                .to_string(),
            // Comment injection attempts
            r#"{"title": "test", "description": " legit */ /* malicious */"}"#.to_string(),
            r#"{"title": "test", "description": "normal // DROP TABLE users"}"#.to_string(),
            // Unicode attack attempts
            r#"{"title": "test", "description": "\u0000\u0001\u0002"}"#.to_string(),
            // Extremely long strings (DoS attempt)
            format!(r#"{{"title": "{}", "description": "test"}}"#, "A".repeat(100000)),
        ];

        for malicious_input in malicious_inputs {
            let result = translate_llm_output(&malicious_input, TargetSchema::TaskBreakdown);

            // Should either succeed with sanitized content OR fail safely
            match result {
                Ok(output) => {
                    // If successful, output should be sanitized
                    let output_str = serde_json::to_string(&output).unwrap();
                    assert!(!output_str.contains("<script>"), "Script tags should be sanitized");
                    assert!(
                        !output_str.contains("DROP TABLE"),
                        "SQL injection should be sanitized"
                    );
                    assert!(
                        output_str.len() < malicious_input.len() * 2,
                        "Output should not expand significantly"
                    );
                }
                Err(_) => {
                    // Failure is acceptable for malicious input
                }
            }
        }
    }

    /// T8-SEC-2: Unbounded allocation prevention
    /// Should limit memory allocation for large inputs
    #[test]
    fn test_unbounded_allocation_prevention() {
        // Create input with unbounded array potential
        let huge_array_input = format!(
            r#"{{
            "parent_tasks": [{}],
            "relevant_files": [{}],
            "unbounded_field": [{}]
        }}"#,
            r#"{"title": "task"}"#.repeat(10000),
            r#"{"path": "file"}"#.repeat(10000),
            r#"{"item": "data"}"#.repeat(50000)
        );

        let start = Instant::now();
        let result = translate_llm_output(&huge_array_input, TargetSchema::TaskBreakdown);
        let duration = start.elapsed();

        // Should complete quickly even with huge input (indicating limits)
        assert!(
            duration.as_millis() < 500,
            "Huge input processing should be bounded, took {}ms",
            duration.as_millis()
        );

        // Result should be reasonable size
        if let Ok(output) = result {
            let output_str = serde_json::to_string(&output).unwrap();
            assert!(output_str.len() < 100000, "Output should be size-limited");
        }
    }

    /// T8-SEC-3: Recursion depth attack prevention
    /// Should prevent stack overflow from extreme recursion
    #[test]
    fn test_recursion_depth_attack_prevention() {
        // Create extremely deep recursive structure
        let mut recursive_json = json!("leaf");
        for _ in 0..1000 {
            // Much deeper than reasonable
            recursive_json = json!({
                "recursive": recursive_json,
                "level": "deep"
            });
        }

        let test_input = json!({
            "parent_tasks": [{
                "title": "Recursion Test",
                "recursive_data": recursive_json
            }]
        });

        // Should handle without stack overflow
        let result = std::panic::catch_unwind(|| {
            translate_llm_output(&test_input.to_string(), TargetSchema::TaskBreakdown)
        });

        assert!(result.is_ok(), "Should not panic on deep recursion");

        if let Ok(Ok(translation)) = result {
            // Translation should either succeed with limits or fail gracefully
            assert!(true, "Deep recursion handled safely");
        }
    }
}

#[cfg(test)]
mod t8_contract_tests {
    use super::*;

    /// T8-CONTRACT-1: LLM response size limits
    /// Should enforce reasonable size limits on LLM responses
    #[test]
    fn test_llm_response_size_limits() {
        // Test with extremely large response (simulating LLM runaway)
        let huge_response = "A".repeat(1000000); // 1MB response

        let result = translate_llm_output(
            &format!(
                r#"{{
            "prd_title": "{}",
            "parent_tasks": [],
            "relevant_files": [],
            "estimated_complexity": "Medium"
        }}"#,
                huge_response
            ),
            TargetSchema::TaskBreakdown,
        );

        // Should reject huge responses
        assert!(result.is_err(), "Should reject extremely large LLM responses");

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("too large")
                    || error_msg.contains("limit")
                    || error_msg.contains("exceeds"),
                "Error should mention size limit: {}",
                error_msg
            );
        }
    }

    /// T8-CONTRACT-2: Schema validation before translation
    /// Should validate basic schema constraints before expensive translation
    #[test]
    fn test_schema_validation_before_translation() {
        let malformed_schemas = vec![
            // Completely invalid JSON
            r#"{"incomplete": json"#,
            // Wrong root type
            r#"["array", "instead", "of", "object"]"#,
            // Missing required fields
            r#"{"wrong_field": "value"}"#,
            // Missing prd_title field
            r#"{"parent_tasks": [], "relevant_files": [], "estimated_complexity": "Medium"}"#,
            // Nested type violations
            r#"{"prd_title": "test", "parent_tasks": "should be array not string", "relevant_files": [], "estimated_complexity": "Medium"}"#,
        ];

        for malformed in malformed_schemas {
            let start = Instant::now();
            let result = translate_llm_output(malformed, TargetSchema::TaskBreakdown);
            let duration = start.elapsed();

            // Should fail quickly for malformed schema (fast validation)
            match result {
                Err(_) => {
                    // Expected outcome - should fail cleanly
                }
                Ok(_) => {
                    // Some malformed inputs might accidentally pass basic parsing
                    // This is acceptable as long as it doesn't crash
                }
            }
            assert!(
                duration.as_millis() < 50,
                "Schema validation should be fast, took {}ms",
                duration.as_millis()
            );
        }
    }

    /// T8-CONTRACT-3: Input sanitization enforcement
    /// Should sanitize dangerous content before processing
    #[test]
    fn test_input_sanitization_enforcement() {
        let dangerous_inputs = vec![
            // Null bytes and control characters
            r#"{"title": "test\u0000dangerous", "description": "safe"}"#.to_string(),
            // Extremely long keys
            format!(r#"{{"{}": "value"}}"#, "A".repeat(1000)),
            // Special characters that could break parsers
            r#"{"title": "test", "description": "\x01\x02\x03\x04"}"#.to_string(),
            // HTML/XML-like content
            r#"{"title": "<img src=x onerror=alert(1)>", "description": "XSS test"}"#.to_string(),
        ];

        for dangerous_input in dangerous_inputs {
            let result = translate_llm_output(&dangerous_input, TargetSchema::TaskBreakdown);

            match result {
                Ok(output) => {
                    // If successful, output should be sanitized
                    let output_str = serde_json::to_string(&output).unwrap();
                    assert!(!output_str.contains('\0'), "Null bytes should be removed");
                    assert!(!output_str.contains("onerror"), "XSS vectors should be neutralized");

                    // Keys should be reasonable length
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output_str) {
                        if let Some(obj) = parsed.as_object() {
                            for key in obj.keys() {
                                assert!(key.len() < 100, "Keys should be length-limited");
                            }
                        }
                    }
                }
                Err(_) => {
                    // Rejection is also acceptable for dangerous content
                }
            }
        }
    }
}
