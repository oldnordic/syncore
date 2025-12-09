//! Phase 6 TDD Tests for Reasoning Metadata Validation and Consolidation
//!
//! These tests MUST FAIL before implementing Phase 6 validation features
//! and PASS after complete implementation.

use anyhow::Result;
use rmcp::model::CallToolResult;
use serde_json::{json, Value};
use std::sync::Arc;
use syncore::mcp_server::server::MCPServerHandler;
use syncore::mcp_server::types::{RagGraphMultihopRequest, RagGraphQueryInput};
use syncore::router::SynCoreState;

/// Test 3.1: test_metadata_fields_consistent_across_tools
#[tokio::test]
async fn test_metadata_fields_consistent_across_tools() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Test raggraph_query
    let query_request = RagGraphQueryInput {
        query_text: "test metadata consistency".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    let query_result =
        handler.raggraph_query(syncore::mcp_server::Parameters(query_request)).await?;
    let query_response: Value =
        serde_json::from_str(&query_result.content[0].text.as_ref().unwrap())?;

    // Test raggraph_multihop
    let multihop_request = RagGraphMultihopRequest {
        seed_nodes: vec![1, 2, 3],
    };

    let multihop_result =
        handler.raggraph_multihop(syncore::mcp_server::Parameters(multihop_request)).await?;
    let multihop_response: Value =
        serde_json::from_str(&multihop_result.content[0].text.as_ref().unwrap())?;

    // Test code_graph_fusion_query
    let fusion_request = RagGraphQueryInput {
        query_text: "test fusion metadata".to_string(),
        namespace: Some("src".to_string()),
        mode_hint: Some("reasoning".to_string()),
        top_k: Some(10),
        project_label: Some("syncore".to_string()),
        scope: Some("project".to_string()),
    };

    let fusion_result =
        handler.code_graph_fusion_query(syncore::mcp_server::Parameters(fusion_request)).await?;
    let fusion_response: Value =
        serde_json::from_str(&fusion_result.content[0].text.as_ref().unwrap())?;

    // Extract metadata from all responses
    let query_metadata = query_response.get("metadata").unwrap().as_object().unwrap();
    let multihop_metadata = multihop_response.get("metadata").unwrap().as_object().unwrap();
    let fusion_metadata = fusion_response.get("metadata").unwrap().as_object().unwrap();

    // This test will FAIL before Phase 6 implementation because:
    // 1. Fields might be missing in some responses
    // 2. Field names might be inconsistent
    // 3. Data types might vary between tools

    // Check that all required fields exist in all responses
    let required_fields = vec![
        "request_id",
        "backend_used",
        "start_time_ms",
        "end_time_ms",
        "vector_search_ms",
        "graph_traversal_ms",
        "fusion_ms",
        "parameters",
        "debug_flags",
    ];

    for field in required_fields {
        assert!(query_metadata.contains_key(field), "query metadata missing field: {}", field);
        assert!(
            multihop_metadata.contains_key(field),
            "multihop metadata missing field: {}",
            field
        );
        assert!(fusion_metadata.contains_key(field), "fusion metadata missing field: {}", field);
    }

    // Check that field types are consistent
    assert_eq!(
        query_metadata.get("request_id").unwrap().is_string(),
        multihop_metadata.get("request_id").unwrap().is_string(),
        "request_id type inconsistent"
    );

    assert_eq!(
        query_metadata.get("backend_used").unwrap().is_string(),
        multihop_metadata.get("backend_used").unwrap().is_string(),
        "backend_used type inconsistent"
    );

    assert_eq!(
        query_metadata.get("start_time_ms").unwrap().is_number(),
        multihop_metadata.get("start_time_ms").unwrap().is_number(),
        "start_time_ms type inconsistent"
    );

    assert_eq!(
        query_metadata.get("debug_flags").unwrap().is_array(),
        multihop_metadata.get("debug_flags").unwrap().is_array(),
        "debug_flags type inconsistent"
    );

    Ok(())
}

/// Test 3.2: test_metadata_optional_fields_present_even_if_none
#[tokio::test]
async fn test_metadata_optional_fields_present_even_if_none() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryInput {
        query_text: "test optional fields".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    let metadata = response.get("metadata").unwrap().as_object().unwrap();

    // This test will FAIL before Phase 6 because optional fields might be:
    // 1. Completely missing from JSON (not null)
    // 2. Different data types between tools
    // 3. Inconsistent naming

    // Check that optional fields exist as null or with values
    let optional_fields = vec!["vector_search_ms", "graph_traversal_ms", "fusion_ms"];

    for field in optional_fields {
        let field_value = metadata.get(field);
        assert!(
            field_value.is_some(),
            "optional field '{}' should be present (even if null)",
            field
        );

        // Should be either a number (Some(u128)) or null
        match field_value.unwrap() {
            Value::Number(_) => {} // Some timing value
            Value::Null => {}      // None represented as null
            _ => panic!(
                "optional field '{}' should be number or null, found: {:?}",
                field, field_value
            ),
        }
    }

    Ok(())
}

/// Test 3.3: test_metadata_stage_traces_present
#[tokio::test]
async fn test_metadata_stage_traces_present() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryInput {
        query_text: "test stage traces".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    let metadata = response.get("metadata").unwrap().as_object().unwrap();
    let debug_flags = metadata.get("debug_flags").unwrap().as_array().unwrap();

    // This test will FAIL before Phase 6 because:
    // 1. Stage markers don't exist yet (ReasoningStage enum not implemented)
    // 2. Debug flags format is not standardized
    // 3. Stage naming is not consistent

    let debug_flag_strings: Vec<String> =
        debug_flags.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect();

    // Check for required stage markers
    let required_stages = vec!["parsing:ok", "backend:", "formatting:ok"];

    for stage in required_stages {
        let found = debug_flag_strings.iter().any(|flag| flag.contains(stage));
        assert!(
            found,
            "debug_flags missing stage marker containing '{}'. Found flags: {:?}",
            stage, debug_flag_strings
        );
    }

    Ok(())
}

/// Test 3.4: test_metadata_normalization_enforces_ordering
#[tokio::test]
async fn test_metadata_normalization_enforces_ordering() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryInput {
        query_text: "test normalization ordering".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    let metadata = response.get("metadata").unwrap().as_object().unwrap();
    let debug_flags = metadata.get("debug_flags").unwrap().as_array().unwrap();

    // This test will FAIL before Phase 6 because:
    // 1. normalize_metadata() function doesn't exist yet
    // 2. Debug flags are not sorted alphabetically
    // 3. Ordering is not enforced

    let debug_flag_strings: Vec<String> =
        debug_flags.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect();

    // Check that debug flags are sorted alphabetically
    let mut sorted_flags = debug_flag_strings.clone();
    sorted_flags.sort();

    assert_eq!(
        debug_flag_strings, sorted_flags,
        "debug_flags should be sorted alphabetically. Expected: {:?}, Found: {:?}",
        sorted_flags, debug_flag_strings
    );

    Ok(())
}

/// Test 3.5: test_timing_fields_always_increasing
#[tokio::test]
async fn test_timing_fields_always_increasing() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryInput {
        query_text: "test timing ordering".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    let metadata = response.get("metadata").unwrap().as_object().unwrap();

    let start_time = metadata.get("start_time_ms").unwrap().as_u64().unwrap() as u128;
    let end_time = metadata.get("end_time_ms").unwrap().as_u64().unwrap() as u128;

    // This test will FAIL before Phase 6 because:
    // 1. Timing fields might not be properly calculated
    // 2. Optional timing fields might be missing instead of null
    // 3. Chronological order is not enforced

    // Basic timing validation
    assert!(
        start_time < end_time,
        "start_time_ms ({}) should be less than end_time_ms ({})",
        start_time,
        end_time
    );

    // Check optional timing fields if they exist
    if let Some(vector_search) = metadata.get("vector_search_ms").and_then(|v| v.as_u64()) {
        let vector_search_ms = vector_search as u128;
        assert!(
            start_time <= vector_search_ms && vector_search_ms <= end_time,
            "vector_search_ms ({}) should be between start_time_ms ({}) and end_time_ms ({})",
            vector_search_ms,
            start_time,
            end_time
        );
    }

    if let Some(graph_traversal) = metadata.get("graph_traversal_ms").and_then(|v| v.as_u64()) {
        let graph_traversal_ms = graph_traversal as u128;
        assert!(
            start_time <= graph_traversal_ms && graph_traversal_ms <= end_time,
            "graph_traversal_ms ({}) should be between start_time_ms ({}) and end_time_ms ({})",
            graph_traversal_ms,
            start_time,
            end_time
        );
    }

    if let Some(fusion) = metadata.get("fusion_ms").and_then(|v| v.as_u64()) {
        let fusion_ms = fusion as u128;
        assert!(
            start_time <= fusion_ms && fusion_ms <= end_time,
            "fusion_ms ({}) should be between start_time_ms ({}) and end_time_ms ({})",
            fusion_ms,
            start_time,
            end_time
        );
    }

    Ok(())
}

/// Test 3.6: test_error_responses_include_full_metadata
#[tokio::test]
async fn test_error_responses_include_full_metadata() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    // Create a request that should cause an error (empty query)
    let request = RagGraphQueryInput {
        query_text: "".to_string(), // Empty query should cause error
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await;

    // Should return an error result, but still include metadata
    assert!(result.is_ok(), "Should return error result, not panic");

    let response_text = result.unwrap().content[0].text.as_ref().unwrap();
    let response: Value = serde_json::from_str(response_text)?;

    // This test will FAIL before Phase 6 because:
    // 1. Error responses might not include metadata
    // 2. Error metadata might be incomplete or inconsistent
    // 3. Error metadata structure might differ from success responses

    assert!(response.get("metadata").is_some(), "Error responses should contain 'metadata' field");

    // Should also have error information
    assert!(response.get("error").is_some(), "Error responses should contain 'error' field");

    let metadata = response.get("metadata").unwrap().as_object().unwrap();

    // Error metadata should have all required fields
    let required_fields = vec![
        "request_id",
        "backend_used",
        "start_time_ms",
        "end_time_ms",
        "vector_search_ms",
        "graph_traversal_ms",
        "fusion_ms",
        "parameters",
        "debug_flags",
    ];

    for field in required_fields {
        assert!(metadata.contains_key(field), "error metadata missing field: {}", field);
    }

    Ok(())
}

/// Test 3.7: test_metadata_consistency_with_sqlitegraph_and_neo4j
#[tokio::test]
async fn test_metadata_consistency_with_sqlitegraph_and_neo4j() -> Result<()> {
    // Test with SQLiteGraph
    std::env::set_var("GRAPH_BACKEND", "sqlite");

    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryInput {
        query_text: "test sqlitegraph backend".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    let result = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;
    let response: Value = serde_json::from_str(&result.content[0].text.as_ref().unwrap())?;

    let metadata = response.get("metadata").unwrap().as_object().unwrap();
    let backend_used = metadata.get("backend_used").unwrap().as_str().unwrap();

    // This test will FAIL before Phase 6 because:
    // 1. backend_used might not match the actual backend used
    // 2. Backend detection logic might be inconsistent
    // 3. Neo4j fallback might not be properly reflected

    assert_eq!(
        backend_used, "SQLiteGraph",
        "metadata.backend_used should be 'SQLiteGraph' when GRAPH_BACKEND=sqlite"
    );

    // Test with Neo4j (will fall back to SQLiteGraph if Neo4j not available)
    std::env::set_var("GRAPH_BACKEND", "neo4j");
    std::env::set_var("NEO4J_URI", "bolt://127.0.0.1:7687");

    let state2 = Arc::new(SynCoreState::new());
    let handler2 = MCPServerHandler::new(state2);

    let request2 = RagGraphQueryInput {
        query_text: "test neo4j backend".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    let result2 = handler2.raggraph_query(syncore::mcp_server::Parameters(request2)).await?;
    let response2: Value = serde_json::from_str(&result2.content[0].text.as_ref().unwrap())?;

    let metadata2 = response2.get("metadata").unwrap().as_object().unwrap();
    let backend_used2 = metadata2.get("backend_used").unwrap().as_str().unwrap();

    // Should be either Neo4j (if available) or SQLiteGraph (fallback)
    assert!(
        backend_used2 == "Neo4j" || backend_used2 == "SQLiteGraph",
        "metadata.backend_used should be 'Neo4j' or 'SQLiteGraph', got '{}'",
        backend_used2
    );

    // Cleanup
    std::env::remove_var("GRAPH_BACKEND");
    std::env::remove_var("NEO4J_URI");

    Ok(())
}

/// Test 3.8: test_metadata_formatting_stable_across_runs
#[tokio::test]
async fn test_metadata_formatting_stable_across_runs() -> Result<()> {
    let state = Arc::new(SynCoreState::new());
    let handler = MCPServerHandler::new(state);

    let request = RagGraphQueryInput {
        query_text: "test formatting stability".to_string(),
        namespace: None,
        mode_hint: None,
        top_k: Some(5),
        project_label: None,
        scope: None,
    };

    // Execute the same request twice
    let result1 = handler.raggraph_query(syncore::mcp_server::Parameters(request.clone())).await?;
    let result2 = handler.raggraph_query(syncore::mcp_server::Parameters(request)).await?;

    let response1: Value = serde_json::from_str(&result1.content[0].text.as_ref().unwrap())?;
    let response2: Value = serde_json::from_str(&result2.content[0].text.as_ref().unwrap())?;

    let metadata1 = response1.get("metadata").unwrap().as_object().unwrap();
    let metadata2 = response2.get("metadata").unwrap().as_object().unwrap();

    // This test will FAIL before Phase 6 because:
    // 1. Debug flags might not be normalized/sorted
    // 2. Parameters might not be consistently formatted
    // 3. Field ordering might vary between runs

    // All fields except timestamps should be identical
    for field in ["request_id", "backend_used", "parameters", "debug_flags"] {
        if field != "request_id" {
            // request_id will be different due to timestamp
            assert_eq!(
                metadata1.get(field),
                metadata2.get(field),
                "metadata field '{}' should be identical across runs: {} vs {}",
                field,
                metadata1.get(field),
                metadata2.get(field)
            );
        }
    }

    // Check that debug_flags are sorted and consistent
    let debug_flags1: Vec<String> = metadata1
        .get("debug_flags")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect();

    let debug_flags2: Vec<String> = metadata2
        .get("debug_flags")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect();

    assert_eq!(debug_flags1, debug_flags2, "debug_flags should be identical across runs");

    // Check that debug flags are sorted
    let mut sorted_flags1 = debug_flags1.clone();
    sorted_flags1.sort();

    assert_eq!(debug_flags1, sorted_flags1, "debug_flags should be sorted alphabetically");

    Ok(())
}
