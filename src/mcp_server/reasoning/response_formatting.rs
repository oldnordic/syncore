//! Response formatting utilities for unified MCP reasoning tools
//!
//! Provides consistent response formatting, error handling, and JSON serialization
//! across all reasoning tools (raggraph_query, raggraph_multihop, code_graph_fusion_query).

use anyhow::Result;
use rmcp::model::{CallToolResult, Content};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unified response structure for all reasoning tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedReasoningResponse {
    /// Response type that generated this result
    pub response_type: String,
    /// Request that generated this response
    pub request_metadata: RequestMetadata,
    /// Results/entities returned by the reasoning tool
    pub results: Vec<ReasoningResult>,
    /// Backend information
    pub backend_info: BackendInfo,
    /// Performance and debugging information
    pub debug_info: DebugInfo,
    /// Success status
    pub success: bool,
    /// Error information if unsuccessful
    pub error: Option<ErrorInfo>,
    /// Reasoning execution metadata (diagnostic information)
    pub metadata: Option<crate::mcp_server::reasoning::ReasoningMetadata>,
    /// Deterministic reasoning trace (for introspection and analysis)
    pub trace: Option<crate::mcp_server::reasoning::ReasoningTrace>,
    /// Machine-auditable evaluation of reasoning execution
    pub evaluation: Option<crate::mcp_server::reasoning::ReasoningEvaluation>,
    /// Passive reflection analysis (improvement suggestions, risk assessment)
    pub reflection: Option<crate::mcp_server::reasoning::reflection::ReasoningReflection>,
}

/// Metadata about the original request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetadata {
    /// Original query text
    pub query: String,
    /// Request type (query, multihop, fusion)
    pub request_type: String,
    /// Parameters used
    pub parameters: HashMap<String, serde_json::Value>,
    /// Timestamp of request
    pub timestamp: u64,
}

/// Individual reasoning result/entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    /// Entity identifier
    pub id: String,
    /// Entity name
    pub name: String,
    /// Entity type (function, struct, etc.)
    pub entity_type: String,
    /// File path where entity is located
    pub file_path: String,
    /// Combined relevance score (0.0 to 1.0)
    pub relevance_score: f32,
    /// Individual score components
    pub scores: ScoreComponents,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Score components for detailed analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreComponents {
    /// Vector search similarity score
    pub vector_score: Option<f32>,
    /// Graph connectivity score
    pub graph_score: Option<f32>,
    /// Temporal relevance score (recency)
    pub temporal_score: Option<f32>,
    /// Graph embedding score (GraphBERT, etc.)
    pub graph_embedding_score: Option<f32>,
    /// Final combined score
    pub combined_score: f32,
}

/// Backend information for transparency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInfo {
    /// Backend type used (SQLiteGraph, Neo4j)
    pub backend_type: String,
    /// Backend configuration source
    pub config_source: String,
    /// Whether backend was auto-selected
    pub auto_selected: bool,
    /// Backend-specific metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Debug and performance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugInfo {
    /// Processing time in milliseconds
    pub processing_time_ms: Option<u64>,
    /// Number of entities examined
    pub entities_examined: Option<usize>,
    /// Graph depth reached
    pub graph_depth: Option<usize>,
    /// Vector search details
    pub vector_search_info: Option<VectorSearchInfo>,
    /// Graph expansion details
    pub graph_expansion_info: Option<GraphExpansionInfo>,
    /// Additional debug metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Vector search debug information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchInfo {
    /// Embedding model used
    pub model: Option<String>,
    /// Search method (exact, approximate)
    pub search_method: String,
    /// Total entities in vector store
    pub total_entities: Option<usize>,
    /// Candidates examined
    pub candidates_examined: Option<usize>,
}

/// Graph expansion debug information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExpansionInfo {
    /// Expansion algorithm used
    pub algorithm: String,
    /// Maximum depth allowed
    pub max_depth: Option<usize>,
    /// Actual depth reached
    pub depth_reached: Option<usize>,
    /// Nodes explored
    pub nodes_explored: Option<usize>,
    /// Edges traversed
    pub edges_traversed: Option<usize>,
}

/// Error information for failed requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// Error code for programmatic handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Error category
    pub category: ErrorCategory,
    /// Stack trace or additional context
    pub context: Option<String>,
}

/// Error categories for programmatic handling
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Invalid request parameters
    Validation,
    /// Backend connection or operation failure
    Backend,
    /// Resource limits exceeded
    Resource,
    /// Internal implementation error
    Internal,
    /// Configuration error
    Configuration,
}

/// Response formatting configuration
#[derive(Debug, Clone)]
pub struct ResponseFormattingConfig {
    /// Include detailed debug information
    pub include_debug_info: bool,
    /// Include backend information
    pub include_backend_info: bool,
    /// Include score components
    pub include_score_components: bool,
    /// Pretty-format JSON output
    pub pretty_format: bool,
    /// Maximum number of results to include
    pub max_results: Option<usize>,
}

impl Default for ResponseFormattingConfig {
    fn default() -> Self {
        Self {
            include_debug_info: true,
            include_backend_info: true,
            include_score_components: true,
            pretty_format: true,
            max_results: Some(50),
        }
    }
}

/// Format successful response from reasoning tool
///
/// Creates a standardized response structure for all reasoning tools,
/// ensuring consistent JSON serialization and metadata inclusion.
///
/// # Arguments
/// * `request_metadata` - Information about the original request
/// * `results` - Results/entities from the reasoning tool
/// * `backend_info` - Backend information
/// * `debug_info` - Debug and performance information
/// * `config` - Optional formatting configuration
/// * `metadata` - Optional reasoning metadata
/// * `trace` - Optional reasoning trace
/// * `evaluation` - Optional reasoning evaluation
///
/// # Returns
/// Result containing the formatted unified response
pub fn format_success_response(
    request_metadata: RequestMetadata,
    results: Vec<ReasoningResult>,
    backend_info: BackendInfo,
    debug_info: DebugInfo,
    config: Option<ResponseFormattingConfig>,
    metadata: Option<crate::mcp_server::reasoning::ReasoningMetadata>,
    trace: Option<crate::mcp_server::reasoning::ReasoningTrace>,
    evaluation: Option<crate::mcp_server::reasoning::ReasoningEvaluation>,
) -> Result<UnifiedReasoningResponse> {
    let config = config.unwrap_or_default();

    // Normalize metadata if provided
    let normalized_metadata = if let Some(mut meta) = metadata {
        if let Err(e) = crate::mcp_server::reasoning::normalize_metadata(&mut meta) {
            // Log normalization error but continue with response
            eprintln!("Warning: Metadata normalization failed: {}", e);
        }
        Some(meta)
    } else {
        None
    };

    // Build reflection if all required components are available
    let reflection = if let (Some(ref meta), Some(ref trace), Some(ref eval)) =
        (normalized_metadata.as_ref(), trace.as_ref(), evaluation.as_ref())
    {
        Some(crate::mcp_server::reasoning::normalize_reflection(
            crate::mcp_server::reasoning::build_reflection(meta, trace, eval),
        ))
    } else {
        None
    };

    // Apply result limits
    let limited_results = if let Some(max_results) = config.max_results {
        results.into_iter().take(max_results).collect()
    } else {
        results
    };

    // Optionally filter debug information
    let filtered_debug_info = if config.include_debug_info {
        debug_info
    } else {
        DebugInfo {
            processing_time_ms: debug_info.processing_time_ms,
            entities_examined: debug_info.entities_examined,
            graph_depth: None,
            vector_search_info: None,
            graph_expansion_info: None,
            metadata: HashMap::new(),
        }
    };

    Ok(UnifiedReasoningResponse {
        response_type: request_metadata.request_type.clone(),
        request_metadata,
        results: limited_results,
        backend_info: if config.include_backend_info {
            backend_info
        } else {
            BackendInfo {
                backend_type: "hidden".to_string(),
                config_source: "hidden".to_string(),
                auto_selected: false,
                metadata: HashMap::new(),
            }
        },
        debug_info: filtered_debug_info,
        success: true,
        error: None,
        metadata: normalized_metadata,
        trace,
        evaluation,
        reflection,
    })
}

/// Format error response for reasoning tool
///
/// Creates a standardized error response structure with detailed error information.
///
/// # Arguments
/// * `request_metadata` - Information about the original request
/// * `error` - Error details
/// * `category` - Error category
/// * `context` - Additional error context
/// * `metadata` - Optional reasoning metadata
/// * `trace` - Optional reasoning trace
/// * `evaluation` - Optional reasoning evaluation
///
/// # Returns
/// Result containing the formatted error response
pub fn format_error_response(
    request_metadata: RequestMetadata,
    error: anyhow::Error,
    category: ErrorCategory,
    context: Option<String>,
    metadata: Option<crate::mcp_server::reasoning::ReasoningMetadata>,
    trace: Option<crate::mcp_server::reasoning::ReasoningTrace>,
    evaluation: Option<crate::mcp_server::reasoning::ReasoningEvaluation>,
) -> Result<UnifiedReasoningResponse> {
    // Normalize metadata if provided
    let mut normalized_metadata = metadata;
    if let Some(ref mut meta) = normalized_metadata {
        if let Err(e) = crate::mcp_server::reasoning::normalize_metadata(meta) {
            // Log normalization error but continue with response
            eprintln!("Warning: Metadata normalization failed in error response: {}", e);
        }
    }

    // Build reflection if all required components are available (even in error cases)
    let reflection = if let (Some(ref meta), Some(ref trace), Some(ref eval)) =
        (normalized_metadata.as_ref(), trace.as_ref(), evaluation.as_ref())
    {
        Some(crate::mcp_server::reasoning::normalize_reflection(
            crate::mcp_server::reasoning::build_reflection(meta, trace, eval),
        ))
    } else {
        None
    };

    Ok(UnifiedReasoningResponse {
        response_type: request_metadata.request_type.clone(),
        request_metadata,
        results: Vec::new(),
        backend_info: BackendInfo {
            backend_type: "error".to_string(),
            config_source: "error".to_string(),
            auto_selected: false,
            metadata: HashMap::new(),
        },
        debug_info: DebugInfo {
            processing_time_ms: None,
            entities_examined: None,
            graph_depth: None,
            vector_search_info: None,
            graph_expansion_info: None,
            metadata: HashMap::new(),
        },
        success: false,
        error: Some(ErrorInfo {
            code: error_code_from_error(&error),
            message: error.to_string(),
            category,
            context,
        }),
        metadata: normalized_metadata,
        trace,
        evaluation,
        reflection,
    })
}

/// Convert to MCP CallToolResult with JSON serialization
pub fn to_mcp_call_tool_result(
    response: UnifiedReasoningResponse,
    config: Option<ResponseFormattingConfig>,
) -> Result<CallToolResult> {
    let config = config.unwrap_or_default();

    let json_str = if config.pretty_format {
        serde_json::to_string_pretty(&response)?
    } else {
        serde_json::to_string(&response)?
    };

    Ok(CallToolResult::success(vec![Content::text(json_str)]))
}

/// Convert MCP CallToolResult error to unified format
pub fn from_mcp_error(
    error: anyhow::Error,
    request_metadata: RequestMetadata,
) -> Result<UnifiedReasoningResponse> {
    format_error_response(request_metadata, error, ErrorCategory::Internal, None, None, None, None)
}

/// Simple wrapper for format_error_response with minimal parameters
/// For use when full reasoning context is not available
pub fn format_error_simple(
    request_metadata: RequestMetadata,
    error: anyhow::Error,
    category: ErrorCategory,
) -> Result<UnifiedReasoningResponse> {
    format_error_response(request_metadata, error, category, None, None, None, None)
}

/// Simple wrapper for format_success_response with minimal parameters
/// For use when full reasoning context is not available
pub fn format_success_simple(
    request_metadata: RequestMetadata,
    results: Vec<ReasoningResult>,
    backend_info: BackendInfo,
) -> Result<UnifiedReasoningResponse> {
    let debug_info = DebugInfo {
        processing_time_ms: None,
        entities_examined: None,
        graph_depth: None,
        vector_search_info: None,
        graph_expansion_info: None,
        metadata: std::collections::HashMap::new(),
    };

    format_success_response(
        request_metadata,
        results,
        backend_info,
        debug_info,
        None,
        None,
        None,
        None,
    )
}

/// Generate error code from anyhow::Error
fn error_code_from_error(error: &anyhow::Error) -> String {
    let error_string = error.to_string().to_lowercase();

    if error_string.contains("backend") || error_string.contains("connection") {
        "BACKEND_ERROR".to_string()
    } else if error_string.contains("parameter") || error_string.contains("validation") {
        "VALIDATION_ERROR".to_string()
    } else if error_string.contains("timeout") || error_string.contains("resource") {
        "RESOURCE_ERROR".to_string()
    } else if error_string.contains("config") {
        "CONFIG_ERROR".to_string()
    } else {
        "INTERNAL_ERROR".to_string()
    }
}

/// Create default request metadata
pub fn create_request_metadata(
    query: String,
    request_type: String,
    parameters: HashMap<String, serde_json::Value>,
) -> RequestMetadata {
    RequestMetadata {
        query,
        request_type,
        parameters,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}

/// Create default backend info
pub fn create_backend_info(
    backend_type: String,
    config_source: String,
    auto_selected: bool,
) -> BackendInfo {
    BackendInfo {
        backend_type,
        config_source,
        auto_selected,
        metadata: HashMap::new(),
    }
}

/// Convert tool-specific result types to unified format
pub mod converters {
    use super::*;
    use crate::code_graph::rag_graph::RankedEntity;

    /// Convert RagGraphAPI RankedEntity to unified ReasoningResult
    pub fn from_raggraph_ranked_entity(entity: RankedEntity) -> ReasoningResult {
        ReasoningResult {
            id: entity.entity_id.to_string(),
            name: entity.name,
            entity_type: entity.entity_type,
            file_path: entity.file_path,
            relevance_score: entity.relevance_score,
            scores: ScoreComponents {
                vector_score: None, // Not available in RankedEntity
                graph_score: entity.graph_score,
                temporal_score: entity.temporal_score,
                graph_embedding_score: entity.graph_embedding_score,
                combined_score: entity.relevance_score,
            },
            metadata: HashMap::new(),
        }
    }

    /// Convert vector of RankedEntity to unified ReasoningResult vector
    pub fn from_raggraph_results(entities: Vec<RankedEntity>) -> Vec<ReasoningResult> {
        entities.into_iter().map(from_raggraph_ranked_entity).collect()
    }

    /// Add vector score information to results (when available)
    pub fn add_vector_scores(
        results: Vec<ReasoningResult>,
        vector_scores: Vec<f32>,
    ) -> Vec<ReasoningResult> {
        results
            .into_iter()
            .zip(vector_scores.into_iter())
            .map(|(mut result, vector_score)| {
                result.scores.vector_score = Some(vector_score);
                result
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_graph::rag_graph::RankedEntity;
    use serde_json::json;

    #[test]
    fn test_format_success_response() {
        let request_metadata =
            create_request_metadata("test query".to_string(), "query".to_string(), HashMap::new());

        let backend_info = create_backend_info("SQLiteGraph".to_string(), "auto".to_string(), true);

        let debug_info = DebugInfo {
            processing_time_ms: Some(100),
            entities_examined: Some(50),
            graph_depth: Some(3),
            vector_search_info: None,
            graph_expansion_info: None,
            metadata: HashMap::new(),
        };

        let results = vec![ReasoningResult {
            id: "1".to_string(),
            name: "test_function".to_string(),
            entity_type: "function".to_string(),
            file_path: "/path/to/file.rs".to_string(),
            relevance_score: 0.85,
            scores: ScoreComponents {
                vector_score: Some(0.9),
                graph_score: Some(0.7),
                temporal_score: Some(0.5),
                graph_embedding_score: Some(0.8),
                combined_score: 0.85,
            },
            metadata: HashMap::new(),
        }];

        let response = format_success_response(
            request_metadata,
            results,
            backend_info,
            debug_info,
            Some(ResponseFormattingConfig::default()),
            None, // metadata
            None, // trace
            None, // evaluation
        );

        assert!(response.is_ok());
        let unified = response.unwrap();
        assert!(unified.success);
        assert_eq!(unified.results.len(), 1);
        assert_eq!(unified.backend_info.backend_type, "SQLiteGraph");
        assert_eq!(unified.debug_info.processing_time_ms, Some(100));
    }

    #[test]
    fn test_format_error_response() {
        let request_metadata = create_request_metadata(
            "invalid query".to_string(),
            "query".to_string(),
            HashMap::new(),
        );

        let error = anyhow::anyhow!("Invalid parameter: top_k too large");
        let response = format_error_response(
            request_metadata,
            error,
            ErrorCategory::Validation,
            Some("top_k=200 exceeds maximum of 100".to_string()),
            None, // metadata
            None, // trace
            None, // evaluation
        );

        assert!(response.is_ok());
        let unified = response.unwrap();
        assert!(!unified.success);
        assert!(unified.error.is_some());
        let error_info = unified.error.unwrap();
        assert_eq!(error_info.category, ErrorCategory::Validation);
        assert!(error_info.message.contains("Invalid parameter"));
        assert_eq!(error_info.context, Some("top_k=200 exceeds maximum of 100".to_string()));
    }

    #[test]
    fn test_error_code_generation() {
        let backend_error = anyhow::anyhow!("Backend connection failed");
        assert_eq!(error_code_from_error(&backend_error), "BACKEND_ERROR");

        let validation_error = anyhow::anyhow!("Invalid parameter: missing query");
        assert_eq!(error_code_from_error(&validation_error), "VALIDATION_ERROR");

        let internal_error = anyhow::anyhow!("Unexpected null pointer");
        assert_eq!(error_code_from_error(&internal_error), "INTERNAL_ERROR");
    }

    #[test]
    fn test_raggraph_conversion() {
        let ranked_entity = RankedEntity {
            entity_id: 42,
            relevance_score: 0.92,
            entity_type: "function".to_string(),
            file_path: "/src/main.rs".to_string(),
            name: "main".to_string(),
            signature: Some("fn main()".to_string()),
            temporal_score: Some(0.6),
            graph_score: Some(0.8),
            graph_embedding_score: Some(0.7),
        };

        let result = converters::from_raggraph_ranked_entity(ranked_entity);
        assert_eq!(result.id, "42");
        assert_eq!(result.name, "main");
        assert_eq!(result.entity_type, "function");
        assert_eq!(result.file_path, "/src/main.rs");
        assert_eq!(result.relevance_score, 0.92);
        assert_eq!(result.scores.graph_score, Some(0.8));
        assert_eq!(result.scores.temporal_score, Some(0.6));
        assert_eq!(result.scores.graph_embedding_score, Some(0.7));
    }
}
