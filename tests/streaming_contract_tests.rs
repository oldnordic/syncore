//! Tests for the Streaming Contract Enforcer
//!
//! TDD approach: Write failing tests first, then make them pass

use anyhow::Result;
use serde_json::{json, Value};
use syncore::mcp_tools::streaming::OutputLimiter;

#[test]
fn test_truncates_large_output() -> Result<()> {
    let limiter = OutputLimiter::default();

    // Create a large output with >200 lines
    let large_lines: Vec<String> = (0..300)
        .map(|i| format!("Line {}: This is test line content that should be truncated", i))
        .collect();
    let large_json = json!({
        "lines": large_lines,
        "metadata": {
            "total": 300,
            "source": "test"
        }
    });

    let result = limiter.apply_json(&large_json)?;

    // Should have truncation metadata
    let meta = &result["meta"];
    assert_eq!(meta["truncated"], true);
    assert!(meta["total_lines"].as_u64().unwrap() >= 300); // Should count actual lines
    assert_eq!(meta["lines_returned"], "truncated");

    // Should have truncated_data instead of preview/tail
    assert!(result.get("truncated_data").is_some());
    assert!(result.get("preview").is_none());
    assert!(result.get("tail").is_none());

    // The truncated data should contain the first 10 elements of the lines array
    let truncated_data = &result["truncated_data"];
    assert!(truncated_data.is_object());
    assert!(truncated_data.get("lines").is_some());

    let lines_array = truncated_data["lines"].as_array().unwrap();
    assert_eq!(lines_array.len(), 10); // Should be limited to PREVIEW_LINES
    assert!(lines_array[0].as_str().unwrap().contains("Line 0:"));

    Ok(())
}

#[test]
fn test_respects_byte_limit() -> Result<()> {
    let limiter = OutputLimiter::new(200, 50_000); // 200 lines, 50KB limit

    // Create output that's >50KB but <200 lines
    let large_content = "x".repeat(60_000); // 60KB of x's
    let json_with_large_content = json!({
        "content": large_content,
        "metadata": "test"
    });

    let result = limiter.apply_json(&json_with_large_content)?;

    // Should be truncated due to byte limit
    let meta = &result["meta"];
    assert_eq!(meta["truncated"], true);
    assert!(meta["total_bytes"].as_u64().unwrap() > 50_000);

    Ok(())
}

#[test]
fn test_paging_mode_basic() -> Result<()> {
    let limiter = OutputLimiter::default();

    // Create array with 300 items
    let large_array: Vec<i32> = (0..300).collect();
    let array_json = json!(large_array);

    let result = limiter.apply_paging(&array_json, Some(1), Some(100))?;

    assert_eq!(result["page"], 1);
    assert_eq!(result["page_size"], 100);
    assert_eq!(result["total_items"], 300);
    assert_eq!(result["total_pages"], 3);

    let items = result["items"].as_array().unwrap();
    assert_eq!(items.len(), 100);
    assert_eq!(items[0], 0);
    assert_eq!(items[99], 99);

    Ok(())
}

#[test]
fn test_chunked_mode_basic() -> Result<()> {
    let limiter = OutputLimiter::default();

    // Create string that needs chunking
    let long_string = "This is a test string that will be split into chunks. ".repeat(100);
    let result = limiter.apply_chunking(&long_string, Some(512))?;

    assert!(result["chunk_count"].as_u64().unwrap() > 1);
    assert_eq!(result["chunk_size"], 512);
    assert_eq!(result["total_bytes"], long_string.len());

    let chunks = result["chunks"].as_array().unwrap();

    // No chunk should exceed the size limit
    for chunk in chunks {
        assert!(chunk.as_str().unwrap().len() <= 512);
    }

    Ok(())
}

#[test]
fn test_no_truncation_when_small() -> Result<()> {
    let limiter = OutputLimiter::default();

    let small_json = json!({
        "message": "hello world",
        "data": vec![1, 2, 3],
        "metadata": {
            "size": "small"
        }
    });

    let result = limiter.apply_json(&small_json)?;

    // Should return identical JSON without truncation metadata
    assert_eq!(result, small_json);
    assert!(result.get("meta").is_none());
    assert!(result.get("preview").is_none());
    assert!(result.get("tail").is_none());

    Ok(())
}

#[test]
fn test_hash_stability() -> Result<()> {
    let limiter = OutputLimiter::default();

    let content = "deterministic test content for hashing";
    let storage_key1 = limiter.store_truncated(content)?;
    let storage_key2 = limiter.store_truncated(content)?;

    // Same content should produce same storage key
    assert_eq!(storage_key1, storage_key2);

    // Storage key should contain hash
    assert!(storage_key1.starts_with("trunc_"));

    Ok(())
}

#[test]
fn test_storage_and_retrieval() -> Result<()> {
    let limiter = OutputLimiter::default();

    let original_content = "This is original content that gets truncated and stored";
    let storage_key = limiter.store_truncated(original_content)?;

    // Should be able to retrieve the stored content
    let retrieved = limiter.retrieve_stored(&storage_key)?;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), original_content);

    // Non-existent key should return None
    let not_found = limiter.retrieve_stored("non_existent_key")?;
    assert!(not_found.is_none());

    Ok(())
}

#[test]
fn test_paging_edge_cases() -> Result<()> {
    let limiter = OutputLimiter::default();

    let small_array = json!([1, 2, 3]);

    // Test invalid page numbers
    let invalid_page = limiter.apply_paging(&small_array, Some(5), Some(10))?;
    assert!(invalid_page.get("error").is_some());
    assert_eq!(invalid_page["page"], 5);

    // Test zero page (should be treated as invalid)
    let zero_page = limiter.apply_paging(&small_array, Some(0), Some(10))?;
    assert!(zero_page.get("error").is_some());

    Ok(())
}

#[test]
fn test_integration_with_suite_result_pattern() -> Result<()> {
    // Test the pattern that will be used in actual MCP tools
    let limiter = OutputLimiter::default();

    // Simulate a large debug output (like logs_tail or project analysis)
    let debug_lines: Vec<String> = (0..250)
        .map(|i| format!("DEBUG: Log entry {} with lots of information that might be truncated", i))
        .collect();
    let debug_output = json!({
        "command": "debug_logs_tail",
        "data": {
            "file": "test.log",
            "line_count": debug_lines.len(),
            "lines": debug_lines
        }
    });

    let limited_output = limiter.apply_json(&debug_output)?;

    // The structure should maintain command wrapper
    assert_eq!(limited_output["command"], "debug_logs_tail");

    // Metadata should be at top level with T9 contract format
    assert!(limited_output.get("meta").is_some());
    assert_eq!(limited_output["meta"]["truncated"], true);
    assert!(limited_output.get("truncated_data").is_some());

    // Check required T9 contract fields
    let meta = &limited_output["meta"];
    assert!(meta.get("total_lines").is_some());
    assert!(meta.get("total_bytes").is_some());
    assert!(meta.get("storage_key").is_some());
    assert!(meta.get("hash").is_some());

    Ok(())
}
