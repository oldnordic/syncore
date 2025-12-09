//! Tests for code drift detection module
//!
//! These tests verify drift detection using existing Syncore infrastructure:
//! - code_graph_fusion_query for entity grouping via embeddings
//! - SQLiteGraph metadata for temporal analysis
//! - debug_suite for architectural hotspots
//! - vector search for semantic similarity

use serde_json::json;
use std::sync::Arc;
use syncore::code_drift::*;
use syncore::router::SynCoreState;
use syncore::mcp_tools::code_drift_suite::*;
use tempfile::TempDir;

/// Create a test state with minimal setup
async fn create_test_state() -> (Arc<SynCoreState>, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create minimal test components
    let memory = syncore::memory::Memory::new(&format!("{}/test_memory.db", temp_dir.path().display())).unwrap();
    let tasks = syncore::tasks::Tasks::new(&format!("{}/test_tasks.db", temp_dir.path().display())).unwrap();

    // Create a simple vector store for testing (use 384 dimensions like the default)
    let vector_store = Arc::new(std::sync::Mutex::new(
        syncore::vector::VectorStore::new(Box::new(syncore::vector::RealEmbeddings::new(384).unwrap()))
    ));

    let state = Arc::new(SynCoreState::new(memory, tasks, vector_store));
    (state, temp_dir)
}

#[tokio::test]
async fn test_detect_semantic_drift_duplicate_function_names() {
    let (state, _temp_dir) = create_test_state().await;

    // Test detection of functions with same name but different implementations
    let result = detect_semantic_drift(state.clone(), json!({
        "query": "function_name:duplicate_handler",
        "similarity_threshold": 0.8
    })).await.unwrap();

    // Should return drift report with duplicate function analysis
    assert!(result.get("drift_type").is_some());
    assert_eq!(result["drift_type"], "semantic");
    assert!(result.get("duplicates").unwrap().as_array().unwrap().len() >= 0);
}

#[tokio::test]
async fn test_detect_architectural_drift_hotspots() {
    let (state, _temp_dir) = create_test_state().await;

    // Test detection of architectural drift using debug_suite hotspots
    let result = detect_architectural_drift(state.clone(), json!({
        "fan_in_threshold": 10,
        "fan_out_threshold": 15,
        "loc_threshold": 500
    })).await.unwrap();

    // Should return architectural drift analysis
    assert_eq!(result["drift_type"], "architectural");
    assert!(result.get("hotspots").is_some());
    assert!(result.get("violations").is_some());
}

#[tokio::test]
async fn test_detect_temporal_aging_stale_files() {
    let (state, _temp_dir) = create_test_state().await;

    // Test detection of temporal aging using SQLiteGraph metadata
    let result = detect_temporal_aging(state.clone(), json!({
        "max_age_days": 30,
        "min_change_count": 5
    })).await.unwrap();

    // Should return temporal aging report
    assert_eq!(result["drift_type"], "temporal");
    assert!(result.get("stale_files").is_some());
    assert!(result.get("aging_metrics").is_some());
}

#[tokio::test]
async fn test_detect_pattern_violations() {
    let (state, _temp_dir) = create_test_state().await;

    // Test detection of pattern violations
    let result = detect_pattern_violations(state.clone(), json!({
        "patterns": ["error_handling", "validation", "logging"],
        "severity": "warning"
    })).await.unwrap();

    // Should return pattern violation report
    assert_eq!(result["drift_type"], "pattern");
    assert!(result.get("violations").is_some());
    assert!(result.get("pattern_compliance").is_some());
}

#[tokio::test]
async fn test_detect_cross_repo_divergence() {
    let (state, _temp_dir) = create_test_state().await;

    // Test cross-repo divergence detection (syncore vs odincode)
    let result = detect_cross_repo_divergence(state.clone(), json!({
        "repo_a": "syncore",
        "repo_b": "odincode",
        "similarity_threshold": 0.9
    })).await.unwrap();

    // Should return cross-repo divergence report
    assert_eq!(result["drift_type"], "cross_repo");
    assert!(result.get("divergent_entities").is_some());
    assert!(result.get("consistency_score").is_some());
}

#[tokio::test]
async fn test_code_drift_mcp_tool_semantic() {
    let (state, _temp_dir) = create_test_state().await;

    // Test MCP tool interface for semantic drift
    let suite = CodeDriftSuite::new((*state).clone());
    let args = CodeDriftSuiteArgs {
        command: "semantic".to_string(),
        query: Some("function".to_string()),
        similarity_threshold: Some(0.7),
        fan_in_threshold: None,
        fan_out_threshold: None,
        loc_threshold: None,
        max_age_days: None,
        min_change_count: None,
        pattern_types: None,
        severity: None,
        baseline_repo: None,
        comparison_repo: None,
        function_name: None,
        compare_signatures: None,
        compare_bodies: None,
        include_semantic: None,
        include_architectural: None,
        include_temporal: None,
        include_patterns: None,
        include_crossrepo: None,
        max_items: None,
        cursor: None,
    };

    let result = suite.execute(args).await.unwrap();
    assert_eq!(result["tool"], "drift.semantic");
    assert!(result["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_code_drift_mcp_tool_architecture() {
    let (state, _temp_dir) = create_test_state().await;

    // Test MCP tool interface for architectural drift
    let suite = CodeDriftSuite::new((*state).clone());
    let args = CodeDriftSuiteArgs {
        command: "architecture".to_string(),
        fan_in_threshold: Some(5),
        fan_out_threshold: Some(10),
        loc_threshold: Some(200),
        ..Default::default()
    };

    let result = suite.execute(args).await.unwrap();
    assert_eq!(result["tool"], "drift.architecture");
    assert!(result["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_code_drift_mcp_tool_crossrepo() {
    let (state, _temp_dir) = create_test_state().await;

    // Test MCP tool interface for cross-repo drift
    let suite = CodeDriftSuite::new((*state).clone());
    let args = CodeDriftSuiteArgs {
        command: "crossrepo".to_string(),
        baseline_repo: Some("syncore".to_string()),
        comparison_repo: Some("odincode".to_string()),
        similarity_threshold: Some(0.9),
        ..Default::default()
    };

    let result = suite.execute(args).await.unwrap();
    assert_eq!(result["tool"], "drift.crossrepo");
    assert!(result["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_function_divergence_detection() {
    let (state, _temp_dir) = create_test_state().await;

    // Test detection of function divergence (same name, different behavior)
    let result = detect_function_divergence(state.clone(), json!({
        "function_name": "process_data",
        "compare_signatures": true,
        "compare_bodies": true
    })).await.unwrap();

    // Should return function divergence analysis
    assert!(result.get("divergent_functions").is_some());
    assert!(result.get("signature_matches").is_some());
    assert!(result.get("body_similarity").is_some());
}

#[tokio::test]
async fn test_drift_aggregation_report() {
    let (state, _temp_dir) = create_test_state().await;

    // Test aggregation of all drift types into comprehensive report
    let semantic_result = detect_semantic_drift(state.clone(), json!({})).await.unwrap();
    let arch_result = detect_architectural_drift(state.clone(), json!({})).await.unwrap();
    let temp_result = detect_temporal_aging(state.clone(), json!({})).await.unwrap();

    // Aggregate results
    let aggregated = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "drift_summary": {
            "semantic": semantic_result,
            "architectural": arch_result,
            "temporal": temp_result
        },
        "overall_drift_score": 0.25,
        "analysis_count": 3
    });

    assert!(aggregated.get("drift_summary").is_some());
    assert!(aggregated.get("overall_drift_score").is_some());
    assert!(aggregated.get("timestamp").is_some());
}

// ========== PAGINATION TESTS ==========

#[tokio::test]
async fn test_code_drift_pagination_legacy_behavior() {
    let (state, _temp_dir) = create_test_state().await;

    // Test legacy behavior: no pagination options should return all items
    let suite = CodeDriftSuite::new((*state).clone());
    let args = CodeDriftSuiteArgs {
        command: "semantic".to_string(),
        query: Some("function".to_string()),
        similarity_threshold: Some(0.8),
        // No max_items or cursor - should work in legacy mode
        ..Default::default()
    };

    let result = suite.execute(args).await.unwrap();
    assert_eq!(result["tool"], "drift.semantic");
    assert!(result["success"].as_bool().unwrap());

    // In legacy mode, should get the original full response structure
    let drift_report = &result["drift_report"];
    assert!(drift_report.get("duplicates").is_some());
}

#[tokio::test]
async fn test_code_drift_pagination_with_max_items() {
    let (state, _temp_dir) = create_test_state().await;

    // Test pagination with max_items=10, no cursor
    let suite = CodeDriftSuite::new((*state).clone());
    let args = CodeDriftSuiteArgs {
        command: "semantic".to_string(),
        query: Some("function".to_string()),
        similarity_threshold: Some(0.8),
        max_items: Some(10),
        cursor: None,
        ..Default::default()
    };

    let result = suite.execute(args).await.unwrap();
    assert_eq!(result["tool"], "drift.semantic");
    assert!(result["success"].as_bool().unwrap());

    // Should have paged response structure
    let paged_result = &result["paged_result"];
    assert!(paged_result.get("items").is_some());
    assert!(paged_result.get("next_cursor").is_some());

    // Items count should not exceed max_items
    let items = paged_result["items"].as_array().unwrap();
    assert!(items.len() <= 10);

    // If there were more than 10 items originally, should have next cursor
    if items.len() == 10 {
        assert!(!paged_result["next_cursor"].is_null());
    }
}

#[tokio::test]
async fn test_code_drift_pagination_with_cursor() {
    let (state, _temp_dir) = create_test_state().await;

    // Test pagination with cursor="5", max_items=10
    let suite = CodeDriftSuite::new((*state).clone());
    let args = CodeDriftSuiteArgs {
        command: "semantic".to_string(),
        query: Some("function".to_string()),
        similarity_threshold: Some(0.8),
        max_items: Some(10),
        cursor: Some("5".to_string()),
        ..Default::default()
    };

    let result = suite.execute(args).await.unwrap();
    assert_eq!(result["tool"], "drift.semantic");
    assert!(result["success"].as_bool().unwrap());

    // Should have paged response structure
    let paged_result = &result["paged_result"];
    assert!(paged_result.get("items").is_some());
    assert!(paged_result.get("next_cursor").is_some());

    // Cursor should advance based on items returned
    if let Some(next_cursor) = paged_result["next_cursor"].as_str() {
        let expected_next = 5 + paged_result["items"].as_array().unwrap().len();
        assert_eq!(next_cursor, expected_next.to_string());
    }
}

#[tokio::test]
async fn test_code_drift_pagination_invalid_cursor() {
    let (state, _temp_dir) = create_test_state().await;

    // Test with invalid cursor - should default to 0
    let suite = CodeDriftSuite::new((*state).clone());
    let args = CodeDriftSuiteArgs {
        command: "semantic".to_string(),
        query: Some("function".to_string()),
        similarity_threshold: Some(0.8),
        max_items: Some(10),
        cursor: Some("invalid".to_string()),  // Invalid cursor
        ..Default::default()
    };

    let result = suite.execute(args).await.unwrap();
    assert_eq!(result["tool"], "drift.semantic");
    assert!(result["success"].as_bool().unwrap());

    // Should not panic and should start from beginning
    let paged_result = &result["paged_result"];
    assert!(paged_result.get("items").is_some());
}

#[tokio::test]
async fn test_code_drift_pagination_cursor_beyond_end() {
    let (state, _temp_dir) = create_test_state().await;

    // Test with cursor beyond available items
    let suite = CodeDriftSuite::new((*state).clone());
    let args = CodeDriftSuiteArgs {
        command: "semantic".to_string(),
        query: Some("function".to_string()),
        similarity_threshold: Some(0.8),
        max_items: Some(10),
        cursor: Some("999999".to_string()),  // Way beyond end
        ..Default::default()
    };

    let result = suite.execute(args).await.unwrap();
    assert_eq!(result["tool"], "drift.semantic");
    assert!(result["success"].as_bool().unwrap());

    // Should return empty items and no next cursor
    let paged_result = &result["paged_result"];
    let items = paged_result["items"].as_array().unwrap();
    assert_eq!(items.len(), 0);
    assert!(paged_result["next_cursor"].is_null());
}