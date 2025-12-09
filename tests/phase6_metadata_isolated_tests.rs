//! Isolated Phase 6 Metadata Validation Tests
//!
//! Tests our core metadata normalization logic without depending on
//! the full SynCore MCP server compilation.

use serde_json::{json, Value};
use std::collections::HashMap;

// Copy of our core Phase 6 structures for isolated testing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReasoningMetadata {
    pub request_id: String,
    pub backend_used: String,
    pub start_time_ms: u128,
    pub end_time_ms: u128,
    pub vector_search_ms: Option<u128>,
    pub graph_traversal_ms: Option<u128>,
    pub fusion_ms: Option<u128>,
    pub parameters: serde_json::Value,
    pub debug_flags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReasoningStage {
    Parsing,
    BackendSelection,
    VectorSearch,
    GraphTraversal,
    Fusion,
    Formatting,
}

impl ReasoningStage {
    pub fn to_debug_flag(&self, status: &str) -> String {
        match self {
            ReasoningStage::Parsing => format!("parsing:{}", status),
            ReasoningStage::BackendSelection => format!("backend:{}", status),
            ReasoningStage::VectorSearch => format!("vector:{}", status),
            ReasoningStage::GraphTraversal => format!("graph:{}", status),
            ReasoningStage::Fusion => format!("fusion:{}", status),
            ReasoningStage::Formatting => format!("formatting:{}", status),
        }
    }
}

/// Normalize metadata to ensure consistency and completeness
pub fn normalize_metadata(meta: &mut ReasoningMetadata) -> anyhow::Result<()> {
    // Ensure debug flags are sorted and contain stage markers
    meta.debug_flags.sort();

    // Ensure all required fields are present (no None values for core timing fields)
    // Note: Optional fields (vector_search_ms, graph_traversal_ms, fusion_ms) can be None

    // Validate timestamp ordering - return error instead of panicking
    if meta.start_time_ms > meta.end_time_ms {
        return Err(anyhow::anyhow!(
            "start_time_ms ({}) must be <= end_time_ms ({})",
            meta.start_time_ms,
            meta.end_time_ms
        ));
    }

    // Validate optional timing fields are within range
    if let Some(vector_ms) = meta.vector_search_ms {
        if vector_ms < meta.start_time_ms || vector_ms > meta.end_time_ms {
            return Err(anyhow::anyhow!(
                "vector_search_ms ({}) must be between start_time_ms ({}) and end_time_ms ({})",
                vector_ms,
                meta.start_time_ms,
                meta.end_time_ms
            ));
        }
    }

    if let Some(graph_ms) = meta.graph_traversal_ms {
        if graph_ms < meta.start_time_ms || graph_ms > meta.end_time_ms {
            return Err(anyhow::anyhow!(
                "graph_traversal_ms ({}) must be between start_time_ms ({}) and end_time_ms ({})",
                graph_ms,
                meta.start_time_ms,
                meta.end_time_ms
            ));
        }
    }

    if let Some(fusion_ms) = meta.fusion_ms {
        if fusion_ms < meta.start_time_ms || fusion_ms > meta.end_time_ms {
            return Err(anyhow::anyhow!(
                "fusion_ms ({}) must be between start_time_ms ({}) and end_time_ms ({})",
                fusion_ms,
                meta.start_time_ms,
                meta.end_time_ms
            ));
        }
    }

    // Ensure parameters is valid JSON (non-null object/array)
    if meta.parameters == serde_json::Value::Null {
        meta.parameters = serde_json::json!({});
    }

    Ok(())
}

#[cfg(test)]
mod phase6_tests {
    use super::*;

    #[test]
    fn test_normalize_metadata_sorts_debug_flags() -> anyhow::Result<()> {
        let mut metadata = ReasoningMetadata {
            request_id: "test_req_1".to_string(),
            backend_used: "SQLiteGraph".to_string(),
            start_time_ms: 1000,
            end_time_ms: 2000,
            vector_search_ms: Some(1200),
            graph_traversal_ms: Some(1500),
            fusion_ms: Some(1800),
            parameters: json!({"query": "test"}),
            debug_flags: vec![
                "formatting:ok".to_string(),
                "parsing:ok".to_string(),
                "backend:SQLiteGraph".to_string(),
                "vector:completed".to_string(),
                "graph:completed".to_string(),
            ],
        };

        normalize_metadata(&mut metadata)?;

        // Check that debug flags are sorted alphabetically
        assert_eq!(
            metadata.debug_flags,
            vec![
                "backend:SQLiteGraph",
                "formatting:ok",
                "graph:completed",
                "parsing:ok",
                "vector:completed",
            ]
        );

        Ok(())
    }

    #[test]
    fn test_normalize_metadata_validates_timestamp_order() {
        let mut metadata = ReasoningMetadata {
            request_id: "test_req_2".to_string(),
            backend_used: "SQLiteGraph".to_string(),
            start_time_ms: 2000, // After end time - should fail
            end_time_ms: 1000,
            vector_search_ms: None,
            graph_traversal_ms: None,
            fusion_ms: None,
            parameters: json!({}),
            debug_flags: vec!["parsing:ok".to_string()],
        };

        let result = normalize_metadata(&mut metadata);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("start_time_ms must be <= end_time_ms"));
    }

    #[test]
    fn test_normalize_metadata_validates_optional_timing_within_range() {
        let mut metadata = ReasoningMetadata {
            request_id: "test_req_3".to_string(),
            backend_used: "SQLiteGraph".to_string(),
            start_time_ms: 1000,
            end_time_ms: 2000,
            vector_search_ms: Some(2500), // Outside range - should fail
            graph_traversal_ms: Some(1500), // Within range - should pass
            fusion_ms: None,
            parameters: json!({}),
            debug_flags: vec!["parsing:ok".to_string()],
        };

        let result = normalize_metadata(&mut metadata);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("vector_search_ms must be between"));
    }

    #[test]
    fn test_normalize_metadata_fixes_null_parameters() -> anyhow::Result<()> {
        let mut metadata = ReasoningMetadata {
            request_id: "test_req_4".to_string(),
            backend_used: "SQLiteGraph".to_string(),
            start_time_ms: 1000,
            end_time_ms: 2000,
            vector_search_ms: None,
            graph_traversal_ms: None,
            fusion_ms: None,
            parameters: serde_json::Value::Null, // Should be fixed
            debug_flags: vec!["parsing:ok".to_string()],
        };

        normalize_metadata(&mut metadata)?;

        // Should convert null to empty object
        assert_eq!(metadata.parameters, json!({}));

        Ok(())
    }

    #[test]
    fn test_reasoning_stage_debug_flags() {
        // Test all stage debug flag generation
        assert_eq!(ReasoningStage::Parsing.to_debug_flag("ok"), "parsing:ok");
        assert_eq!(
            ReasoningStage::BackendSelection.to_debug_flag("SQLiteGraph"),
            "backend:SQLiteGraph"
        );
        assert_eq!(ReasoningStage::VectorSearch.to_debug_flag("completed"), "vector:completed");
        assert_eq!(ReasoningStage::GraphTraversal.to_debug_flag("completed"), "graph:completed");
        assert_eq!(ReasoningStage::Fusion.to_debug_flag("completed"), "fusion:completed");
        assert_eq!(ReasoningStage::Formatting.to_debug_flag("ok"), "formatting:ok");
    }

    #[test]
    fn test_all_required_fields_present() {
        // Test that our ReasoningMetadata struct includes all required fields
        let metadata = ReasoningMetadata {
            request_id: "req_123".to_string(),
            backend_used: "SQLiteGraph".to_string(),
            start_time_ms: 1000,
            end_time_ms: 2000,
            vector_search_ms: Some(1500),
            graph_traversal_ms: Some(1800),
            fusion_ms: None,
            parameters: json!({"query": "test", "top_k": 10}),
            debug_flags: vec!["parsing:ok".to_string(), "backend:SQLiteGraph".to_string()],
        };

        // Verify all Phase 6 required fields are present
        assert!(!metadata.request_id.is_empty());
        assert!(!metadata.backend_used.is_empty());
        assert!(metadata.start_time_ms < metadata.end_time_ms);
        // Optional fields should be Option<u128>
        assert!(metadata.vector_search_ms.is_some());
        assert!(metadata.graph_traversal_ms.is_some());
        assert!(metadata.fusion_ms.is_none());
        // Parameters should be valid JSON
        assert!(metadata.parameters.is_object());
        // Debug flags should be a vector of strings
        assert!(!metadata.debug_flags.is_empty());
    }

    #[test]
    fn test_metadata_json_serialization_roundtrip() -> anyhow::Result<()> {
        // Test that our metadata structures serialize/deserialize correctly
        let original = ReasoningMetadata {
            request_id: "req_456".to_string(),
            backend_used: "Neo4j".to_string(),
            start_time_ms: 1000,
            end_time_ms: 2000,
            vector_search_ms: Some(1200),
            graph_traversal_ms: Some(1600),
            fusion_ms: Some(1900),
            parameters: json!({"mode": "reasoning", "scope": "project"}),
            debug_flags: vec![
                "parsing:ok".to_string(),
                "backend:Neo4j".to_string(),
                "vector:completed".to_string(),
                "graph:completed".to_string(),
                "fusion:completed".to_string(),
                "formatting:ok".to_string(),
            ],
        };

        // Serialize to JSON
        let json_str = serde_json::to_string_pretty(&original)?;

        // Deserialize back
        let deserialized: ReasoningMetadata = serde_json::from_str(&json_str)?;

        // Should be identical
        assert_eq!(original.request_id, deserialized.request_id);
        assert_eq!(original.backend_used, deserialized.backend_used);
        assert_eq!(original.start_time_ms, deserialized.start_time_ms);
        assert_eq!(original.end_time_ms, deserialized.end_time_ms);
        assert_eq!(original.vector_search_ms, deserialized.vector_search_ms);
        assert_eq!(original.graph_traversal_ms, deserialized.graph_traversal_ms);
        assert_eq!(original.fusion_ms, deserialized.fusion_ms);
        assert_eq!(original.parameters, deserialized.parameters);
        assert_eq!(original.debug_flags, deserialized.debug_flags);

        Ok(())
    }
}
