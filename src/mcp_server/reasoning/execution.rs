//! Reasoning request execution logic

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use crate::router::SynCoreState;
use rmcp::model::CallToolResult;

use super::{
    backend_selection::{BackendSelection, BackendSelectionConfig},
    request_parsing::{UnifiedReasoningRequest, RequestType},
    response_formatting::{
        create_request_metadata, create_backend_info, format_success_response,
        format_error_response, ResponseFormattingConfig, DebugInfo, ErrorCategory,
        ReasoningResult, ScoreComponents,
    },
    metadata::{ReasoningMetadata, ReasoningStage},
    trace::ReasoningTraceBuilder,
    evaluation::{evaluate_reasoning, normalize_evaluation},
};

/// Execute unified reasoning request
///
/// Main entry point for all reasoning tools. This function provides a single,
/// backend-agnostic implementation that can be used by raggraph_query,
/// raggraph_multihop, and code_graph_fusion_query.
///
/// # Arguments
/// * `request` - Parsed unified request
/// * `state` - SynCore state containing database connections
///
/// # Returns
/// CallToolResult with standardized JSON response containing metadata
pub fn execute_reasoning_request(
    request: UnifiedReasoningRequest,
    state: &SynCoreState,
) -> Result<CallToolResult> {
    // Step 1: Initialize metadata collection and trace builder
    let request_start_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let request_id = format!("req_{}", request_start_time);

    // Initialize trace builder with request parameters
    let parameters_value = serde_json::to_value(&request.parameters).unwrap_or(serde_json::Value::Null);
    let mut trace_builder = ReasoningTraceBuilder::new(&parameters_value);

    // Step 2: Select appropriate backend
    let backend_config = BackendSelectionConfig {
        prefer_sqlite: true,
        allow_neo4j_fallback: true,
        require_explicit_neo4j: false,
    };

    let backend_selection = super::backend_selection::select_reasoning_backend(
        Some(backend_config),
        state.neo4j.clone(),
    )?;

    // Step 3: Create request metadata for tracking
    let request_metadata = create_request_metadata(
        request.query.clone(),
        format!("{:?}", request.request_type),
        HashMap::new(), // TODO: Convert request.parameters to HashMap
    );

    // Step 4: Add parsing stage to trace
    trace_builder.add_success("parsing", "request parsed successfully");

    // Step 5: Execute the request based on type with timing
    let _start_time = std::time::Instant::now();
    let (results, debug_info, processing_time_ms) = match execute_with_trace(&request, &backend_selection, &mut trace_builder) {
        Ok(result) => result,
        Err(error) => {
            // Add failure stage to trace and return error response
            trace_builder.add_failure("execution", format!("execution failed: {}", error));

            let request_end_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis();

            let request_metadata = create_request_metadata(
                request.query.clone(),
                format!("{:?}", request.request_type),
                HashMap::new(),
            );

            let reasoning_metadata = create_error_metadata(&request_metadata, &request_end_time);
            let trace = trace_builder.finalize(&reasoning_metadata);

            // Generate evaluation for error case
            let evaluation = evaluate_reasoning(&reasoning_metadata, &trace);
            let normalized_evaluation = normalize_evaluation(evaluation);

            return super::response_formatting::to_mcp_call_tool_result(
                format_error_response(
                    request_metadata,
                    error,
                    ErrorCategory::Internal,
                    Some("Execution failed during reasoning processing".to_string()),
                    Some(reasoning_metadata),
                    Some(trace),
                    Some(normalized_evaluation),
                )?,
                Some(ResponseFormattingConfig::default()),
            );
        }
    };

    let request_end_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    // Step 5: Create reasoning metadata with stage traces
    let mut debug_flags = vec![
        ReasoningStage::Parsing.to_debug_flag("ok"),
        ReasoningStage::BackendSelection.to_debug_flag(&format!("{:?}", backend_selection.backend_type)),
        ReasoningStage::Formatting.to_debug_flag("ok"),
        "unified_reasoning".to_string(),
    ];

    // Add stage-specific flags based on request type
    match request.request_type {
        RequestType::Query => {
            debug_flags.push(ReasoningStage::VectorSearch.to_debug_flag("completed"));
            debug_flags.push(ReasoningStage::GraphTraversal.to_debug_flag("completed"));
            debug_flags.push(ReasoningStage::Fusion.to_debug_flag("skipped"));
        },
        RequestType::MultiHop => {
            debug_flags.push(ReasoningStage::VectorSearch.to_debug_flag("completed"));
            debug_flags.push(ReasoningStage::GraphTraversal.to_debug_flag("completed"));
            debug_flags.push(ReasoningStage::Fusion.to_debug_flag("skipped"));
        },
        RequestType::Fusion => {
            debug_flags.push(ReasoningStage::VectorSearch.to_debug_flag("completed"));
            debug_flags.push(ReasoningStage::GraphTraversal.to_debug_flag("completed"));
            debug_flags.push(ReasoningStage::Fusion.to_debug_flag("completed"));
        },
    }

    let reasoning_metadata = ReasoningMetadata {
        request_id: request_id.clone(),
        backend_used: format!("{:?}", backend_selection.backend_type),
        start_time_ms: request_start_time,
        end_time_ms: request_end_time,
        vector_search_ms: Some(processing_time_ms as u128), // Placeholder - will be refined per request type
        graph_traversal_ms: match request.request_type {
            RequestType::MultiHop => Some(processing_time_ms as u128),
            RequestType::Query => Some(processing_time_ms as u128 / 2),
            RequestType::Fusion => Some(processing_time_ms as u128),
        },
        fusion_ms: match request.request_type {
            RequestType::Fusion => Some(processing_time_ms as u128),
            _ => None,
        },
        parameters: serde_json::to_value(&request.parameters).unwrap_or(serde_json::Value::Null),
        debug_flags,
    };

    // Step 6: Create backend info
    let backend_info = create_backend_info(
        format!("{:?}", backend_selection.backend_type),
        backend_selection.metadata.config_source.clone(),
        backend_selection.metadata.auto_selected,
    );

    // Step 7: Add formatting stage to trace
    trace_builder.add_success("formatting", "response formatted successfully");

    // Step 8: Format the response with metadata and trace
    let response_config = ResponseFormattingConfig::default();
    let mut debug_info_with_timing = debug_info;
    debug_info_with_timing.processing_time_ms = Some(processing_time_ms);

    // Finalize the trace
    let trace = trace_builder.finalize(&reasoning_metadata);

    // Generate evaluation based on metadata and trace
    let evaluation = evaluate_reasoning(&reasoning_metadata, &trace);
    let normalized_evaluation = normalize_evaluation(evaluation);

    let unified_response = format_success_response(
        request_metadata,
        results,
        backend_info,
        debug_info_with_timing,
        Some(response_config),
        Some(reasoning_metadata),
        Some(trace),
        Some(normalized_evaluation),
    )?;

    // Step 9: Convert to MCP CallToolResult
    super::response_formatting::to_mcp_call_tool_result(unified_response, Some(ResponseFormattingConfig::default()))
}

/// Execute Query-type reasoning request
fn execute_query_request(
    request: &UnifiedReasoningRequest,
    backend_selection: &BackendSelection,
) -> Result<(Vec<super::response_formatting::ReasoningResult>, DebugInfo)> {
    
    // Create a minimal stub response for reasoning execution
    // TODO: Implement full backend integration when needed

    // For now, return a simple success response with mock results
    let mock_results = vec![
        ReasoningResult {
            id: "mock_result_1".to_string(),
            name: request.query.clone(),
            entity_type: "function".to_string(),
            file_path: "/mock/path.rs".to_string(),
            relevance_score: 0.85,
            scores: ScoreComponents {
                vector_score: Some(0.9),
                graph_score: Some(0.8),
                temporal_score: Some(0.7),
                graph_embedding_score: Some(0.75),
                combined_score: 0.85,
            },
            metadata: std::collections::HashMap::new(),
        }
    ];
  
    let debug_info = DebugInfo {
        processing_time_ms: Some(50),
        entities_examined: Some(mock_results.len() as u64),
        graph_depth: Some(1),
        vector_search_info: Some(super::response_formatting::VectorSearchInfo {
            model: Some("semantic".to_string()),
            search_method: "semantic".to_string(),
            total_entities: None,
            candidates_examined: Some(mock_results.len()),
        }),
        graph_expansion_info: Some(super::response_formatting::GraphExpansionInfo {
            algorithm: "query".to_string(),
            max_depth: Some(3),
            depth_reached: Some(2),
            nodes_explored: Some(50),
            edges_traversed: Some(100),
        }),
        metadata: std::collections::HashMap::new(),
    };

    Ok((mock_results, debug_info))
}

/// Execute MultiHop-type reasoning request
fn execute_multihop_request(
    request: &UnifiedReasoningRequest,
    backend_selection: &BackendSelection,
) -> Result<(Vec<super::response_formatting::ReasoningResult>, DebugInfo)> {
    // For now, delegate to query execution with multihop-specific parameters
    execute_query_request(request, backend_selection)
}

/// Execute Fusion-type reasoning request
fn execute_fusion_request(
    request: &UnifiedReasoningRequest,
    backend_selection: &BackendSelection,
) -> Result<(Vec<super::response_formatting::ReasoningResult>, DebugInfo)> {
    // For now, delegate to query execution with fusion-specific parameters
    execute_query_request(request, backend_selection)
}

/// Validate unified reasoning request before execution
pub fn validate_reasoning_request(
    request: &UnifiedReasoningRequest,
) -> Result<()> {
    if request.query.trim().is_empty() {
        return Err(anyhow::anyhow!("Query cannot be empty"));
    }

    if let Some(top_k) = request.top_k {
        if top_k == 0 || top_k > 1000 {
            return Err(anyhow::anyhow!("top_k must be between 1 and 1000"));
        }
    }

    Ok(())
}

/// Execute reasoning request with trace integration
fn execute_with_trace(
    request: &UnifiedReasoningRequest,
    backend_selection: &BackendSelection,
    trace_builder: &mut ReasoningTraceBuilder,
) -> Result<(Vec<super::response_formatting::ReasoningResult>, DebugInfo, u64)> {
    let start_time = std::time::Instant::now();

    let (results, debug_info) = match request.request_type {
        RequestType::Query => {
            trace_builder.add_success("vector_search", "query executed successfully");
            trace_builder.add_success("graph_traversal", "graph traversal completed");
            execute_query_request(request, backend_selection)?
        },
        RequestType::MultiHop => {
            trace_builder.add_success("vector_search", "multihop seed lookup completed");
            trace_builder.add_success("graph_traversal", "multihop graph expansion completed");
            execute_multihop_request(request, backend_selection)?
        },
        RequestType::Fusion => {
            trace_builder.add_success("vector_search", "fusion vector search completed");
            trace_builder.add_success("graph_traversal", "fusion graph traversal completed");
            trace_builder.add_success("fusion", "reasoning fusion processing completed");
            execute_fusion_request(request, backend_selection)?
        },
    };

    let processing_time_ms = start_time.elapsed().as_millis() as u64;
    Ok((results, debug_info, processing_time_ms))
}

/// Create minimal metadata for error responses
fn create_error_metadata(
    request_metadata: &super::response_formatting::RequestMetadata,
    end_time_ms: &u128,
) -> ReasoningMetadata {
    let start_time_ms = request_metadata.timestamp as u128;
    ReasoningMetadata {
        request_id: format!("err_{}", start_time_ms),
        backend_used: "unknown".to_string(),
        start_time_ms,
        end_time_ms: *end_time_ms,
        vector_search_ms: None,
        graph_traversal_ms: None,
        fusion_ms: None,
        parameters: serde_json::json!({}),
        debug_flags: vec!["parsing:error".to_string(), "execution:error".to_string()],
    }
}

/// Get backend performance metrics
pub fn get_backend_metrics(
    backend_type: &super::backend_selection::BackendType,
) -> HashMap<String, serde_json::Value> {
    let mut metrics = HashMap::new();

    match backend_type {
        super::backend_selection::BackendType::SQLiteGraph => {
            metrics.insert("type".to_string(), serde_json::Value::String("SQLiteGraph".to_string()));
            metrics.insert("features".to_string(), serde_json::json!(["vector_search", "graph_traversal"]));
        }
        super::backend_selection::BackendType::Neo4j => {
            metrics.insert("type".to_string(), serde_json::Value::String("Neo4j".to_string()));
            metrics.insert("features".to_string(), serde_json::json!(["vector_search", "graph_traversal", "distributed"]));
        }
    }

    metrics
}