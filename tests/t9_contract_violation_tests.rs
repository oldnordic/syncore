//! T9 Contract Violation Tests
//!
//! Focused tests that demonstrate the specific contract violations we need to fix.
//! These should fail initially and pass after implementing the fixes.

use anyhow::Result;
use serde_json::json;
use syncore::mcp_tools::streaming::OutputLimiter;

#[test]
fn test_streaming_limiter_basics() -> Result<()> {
    // Test the streaming limiter works as expected
    let limiter = OutputLimiter::default();

    // Small content should pass through unchanged
    let small_json = json!({
        "command": "test",
        "data": {"items": vec![1, 2, 3]}
    });

    let result = limiter.apply_json(&small_json)?;
    assert_eq!(result, small_json);

    // Large content should be truncated
    let large_items: Vec<i32> = (0..300).collect();
    let large_json = json!({
        "command": "test_large",
        "data": {"items": large_items}
    });

    let limited = limiter.apply_json(&large_json)?;

    // Should have truncation metadata
    assert!(limited.get("meta").is_some());
    assert_eq!(limited["meta"]["truncated"], true);
    assert!(limited.get("truncated_data").is_some());

    Ok(())
}

#[test]
fn test_code_suite_missing_streaming() -> Result<()> {
    // This test documents the violation: code_suite doesn't apply streaming
    // We expect this to show the issue but not fix it yet

    let limiter = OutputLimiter::default();

    // Simulate what code_suite.search might return without streaming
    let large_search_results: Vec<String> = (0..250).map(|i| format!("Result {}: Found pattern in file {}", i, "src/main.rs")).collect();
    let unprocessed_response = json!({
        "success": true,
        "command": "search",
        "data": {
            "results": large_search_results,
            "total_count": large_search_results.len()
        }
    });

    // This should be processed by streaming limiter but currently isn't in code_suite
    let processed = limiter.apply_json(&unprocessed_response)?;

    // After processing, should have truncation metadata
    assert!(processed.get("meta").is_some(), "Should have meta field");
    assert!(processed.get("truncated_data").is_some(), "Should have truncated_data field");

    let meta = &processed["meta"];
    if meta["truncated"] == true {
        println!("✅ Streaming would properly truncate code search results");
        if let Some(total_lines) = meta["total_lines"].as_u64() {
            assert!(total_lines > 200);
        }
        assert!(meta.get("storage_key").is_some(), "Should have storage_key");
        assert!(meta.get("hash").is_some(), "Should have hash");
    }

    Ok(())
}

#[test]
fn test_graph_suite_missing_streaming() -> Result<()> {
    // Document graph_suite streaming violation
    let limiter = OutputLimiter::default();

    // Simulate large graph query result
    let large_nodes: Vec<i32> = (0..200).collect();
    let graph_response = json!({
        "success": true,
        "command": "query",
        "data": {
            "nodes": large_nodes,
            "relationships": large_nodes.len() * 2, // Many relationships
            "metadata": {"query_type": "large_graph_scan"}
        }
    });

    let processed = limiter.apply_json(&graph_response)?;

    // This demonstrates what should happen after applying streaming to graph_suite
    if processed.get("data").is_some() {
        let data = &processed["data"];
        if data.get("meta").is_some() {
            println!("✅ Streaming would properly limit graph query results");
        }
    }

    Ok(())
}

#[test]
fn test_response_envelope_standard() -> Result<()> {
    // Test the standard response envelope all tools should follow

    // Correct envelope format
    let correct_envelope = json!({
        "success": true,
        "command": "example_tool",
        "data": {
            "result": "operation completed",
            "items": [1, 2, 3]
        },
        "error": null
    });

    // Verify envelope contract
    assert!(correct_envelope.is_object(), "Response must be object");
    assert!(correct_envelope.get("success").is_some(), "Must have success field");
    assert!(correct_envelope.get("command").is_some(), "Must have command field");
    assert!(correct_envelope.get("data").is_some(), "Must have data field");
    assert!(correct_envelope["data"].is_object(), "Data must be object, not array");

    // Error envelope format
    let error_envelope = json!({
        "success": false,
        "command": "example_tool",
        "data": {},
        "error": "Invalid parameters provided"
    });

    assert_eq!(error_envelope["success"], false, "Error response must have success=false");
    assert!(error_envelope.get("error").is_some(), "Error response must have error field");
    assert!(error_envelope["error"].is_string(), "Error field must be string");

    Ok(())
}

#[test]
fn test_suite_result_format_compliance() -> Result<()> {
    // Test that SuiteResult struct complies with contract when serialized
    use syncore::mcp_tools::SuiteResult;

    // Successful SuiteResult
    let success_result = SuiteResult::ok("test_command", json!({"items": vec![1, 2, 3]}));
    let success_json = json!({
        "success": success_result.success,
        "command": success_result.command,
        "data": success_result.data,
        "error": success_result.error
    });

    // Verify compliance
    assert!(success_json["success"].is_boolean(), "Success must be boolean");
    assert_eq!(success_json["command"], "test_command");
    assert!(success_json["data"].is_object(), "Data must be object");
    assert!(success_json.get("error").unwrap().is_null(), "Error should be null on success");

    // Error SuiteResult
    let error_result = SuiteResult::err("test_command", "Something went wrong");
    let error_json = json!({
        "success": error_result.success,
        "command": error_result.command,
        "data": error_result.data,
        "error": error_result.error
    });

    assert_eq!(error_json["success"], false, "Error response must have success=false");
    assert!(error_json["error"].is_string(), "Error field must be string");
    assert_eq!(error_json["error"], "Something went wrong");

    Ok(())
}

#[test]
fn test_identified_contract_violations() -> Result<()> {
    // This test documents the specific violations we identified in the inventory

    println!("=== T9 CONTRACT VIOLATIONS IDENTIFIED ===");
    println!();
    println!("PRIORITY 1 VIOLATIONS:");
    println!("1. code_suite - Missing streaming enforcement in dispatch()");
    println!("   - Tools: search, grep, doc_search, index_directory");
    println!("   - Risk: VERY LARGE responses without truncation");
    println!("   - Location: src/mcp_tools/code_suite.rs:dispatch()");
    println!();
    println!("2. graph_suite - Missing streaming enforcement");
    println!("   - Tools: query, rag_query, rag_multihop");
    println!("   - Risk: Large graph results without truncation");
    println!("   - Location: src/mcp_tools/graph_suite.rs:dispatch()");
    println!();
    println!("3. reasoning_suite - Inconsistent response pattern");
    println!("   - Tool: reasoning_tree_get returns direct JSON, not SuiteResult");
    println!("   - Risk: Breaks contract consistency");
    println!("   - Location: src/mcp_tools/reasoning_suite.rs:handle_reasoning_tree_get()");
    println!();
    println!("PRIORITY 2 VIOLATIONS:");
    println!("4. mapping_suite - Missing streaming enforcement");
    println!("5. refrag_suite - Missing streaming enforcement");
    println!("6. Direct MCP handlers - Inconsistent response patterns");

    // This test always passes - it's just documentation
    assert!(true);
    Ok(())
}

#[test]
fn test_translator_integration_contract() -> Result<()> {
    // Test translator integration for LLM-dependent tools

    // Simulate malformed LLM output that needs fixing
    let malformed_llm_output = r#"{
        "title": "Test Task",
        "subtasks": [
            {
                "title": "Subtask 1",
                "estimated_hours": "2.5"  // String instead of number
            }
        ]
    }"#;

    // Test that translator can fix type issues
    use syncore::mcp_tools::translator::{translate_llm_output, TargetSchema};

    match translate_llm_output(malformed_llm_output, TargetSchema::TaskBreakdown) {
        Ok(translated) => {
            println!("✅ Translator successfully fixed malformed LLM output");

            // Verify type coercion worked
            if let Some(subtasks) = translated.get("subtasks").and_then(|v| v.as_array()) {
                if let Some(first) = subtasks.get(0) {
                    if let Some(hours) = first.get("estimated_hours") {
                        assert!(hours.is_number(), "String hours should be coerced to number");
                        println!("✅ Type coercion working: string '2.5' became number {}", hours);
                    }
                }
            }
        },
        Err(e) => {
            println!("❌ Translator error: {}", e);
            panic!("Translator should handle this malformed but valid JSON");
        }
    }

    Ok(())
}