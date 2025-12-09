//! Reasoning metadata structures and utilities

use serde::{Deserialize, Serialize};

/// Metadata about reasoning execution for diagnostic purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningMetadata {
    /// Unique identifier for the request
    pub request_id: String,
    /// Backend that was used (sqlitegraph or neo4j)
    pub backend_used: String,
    /// Request start time in milliseconds since epoch
    pub start_time_ms: u128,
    /// Request end time in milliseconds since epoch
    pub end_time_ms: u128,
    /// Time spent on vector search in milliseconds (if performed)
    pub vector_search_ms: Option<u128>,
    /// Time spent on graph traversal in milliseconds (if performed)
    pub graph_traversal_ms: Option<u128>,
    /// Time spent on fusion processing in milliseconds (if performed)
    pub fusion_ms: Option<u128>,
    /// Original parameters as JSON
    pub parameters: serde_json::Value,
    /// Debug flags for the request
    pub debug_flags: Vec<String>,
}

/// Trace stages for reasoning execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReasoningStage {
    Parsing,
    BackendSelection,
    VectorSearch,
    GraphTraversal,
    Fusion,
    Formatting,
}

impl ReasoningStage {
    /// Convert stage to debug flag format
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
