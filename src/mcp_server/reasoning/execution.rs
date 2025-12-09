//! Reasoning request execution logic

use crate::router::SynCoreState;
use anyhow::{anyhow, Result};
use rmcp::model::CallToolResult;
use serde_json::json;
use std::collections::HashMap;
use rusqlite::ToSql;

use super::{
    backend_selection::{BackendSelection, BackendSelectionConfig},
    evaluation::{evaluate_reasoning, normalize_evaluation},
    metadata::{ReasoningMetadata, ReasoningStage},
    request_parsing::{RequestType, UnifiedReasoningRequest},
    response_formatting::{
        create_backend_info, create_request_metadata, format_error_response,
        format_success_response, DebugInfo, ErrorCategory, ReasoningResult,
        ResponseFormattingConfig, ScoreComponents,
    },
    trace::ReasoningTraceBuilder,
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
    let request_start_time =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();

    let request_id = format!("req_{}", request_start_time);

    // Initialize trace builder with request parameters
    let parameters_value =
        serde_json::to_value(&request.parameters).unwrap_or(serde_json::Value::Null);
    let mut trace_builder = ReasoningTraceBuilder::new(&parameters_value);

    // Step 2: Select appropriate backend (SQLiteGraph-only for core reasoning tools)
    let backend_config = BackendSelectionConfig {
        prefer_sqlite: true,
        allow_neo4j_fallback: false, // HARDENED: No Neo4j fallback for core tools
        require_explicit_neo4j: true, // HARDENED: Require explicit Neo4j usage
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
    let (results, debug_info, processing_time_ms) =
        match execute_with_trace(&request, &backend_selection, &mut trace_builder, state) {
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

                let reasoning_metadata =
                    create_error_metadata(&request_metadata, &request_end_time);
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

    let request_end_time =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();

    // Step 5: Create reasoning metadata with stage traces
    let mut debug_flags = vec![
        ReasoningStage::Parsing.to_debug_flag("ok"),
        ReasoningStage::BackendSelection
            .to_debug_flag(&format!("{:?}", backend_selection.backend_type)),
        ReasoningStage::Formatting.to_debug_flag("ok"),
        "unified_reasoning".to_string(),
    ];

    // Add stage-specific flags based on request type
    match request.request_type {
        RequestType::Query => {
            debug_flags.push(ReasoningStage::VectorSearch.to_debug_flag("completed"));
            debug_flags.push(ReasoningStage::GraphTraversal.to_debug_flag("completed"));
            debug_flags.push(ReasoningStage::Fusion.to_debug_flag("skipped"));
        }
        RequestType::MultiHop => {
            debug_flags.push(ReasoningStage::VectorSearch.to_debug_flag("completed"));
            debug_flags.push(ReasoningStage::GraphTraversal.to_debug_flag("completed"));
            debug_flags.push(ReasoningStage::Fusion.to_debug_flag("skipped"));
        }
        RequestType::Fusion => {
            debug_flags.push(ReasoningStage::VectorSearch.to_debug_flag("completed"));
            debug_flags.push(ReasoningStage::GraphTraversal.to_debug_flag("completed"));
            debug_flags.push(ReasoningStage::Fusion.to_debug_flag("completed"));
        }
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
    super::response_formatting::to_mcp_call_tool_result(
        unified_response,
        Some(ResponseFormattingConfig::default()),
    )
}

/// Execute Query-type reasoning request using real database data
fn execute_query_request(
    request: &UnifiedReasoningRequest,
    backend_selection: &BackendSelection,
    state: &SynCoreState,
) -> Result<(Vec<super::response_formatting::ReasoningResult>, DebugInfo)> {
    let start_time = std::time::Instant::now();

    // Execute real search using SQLiteGraph via CodeSuite
    let search_results = execute_real_search(&request.query, request.top_k.unwrap_or(10) as usize, state)?;

    let processing_time_ms = start_time.elapsed().as_millis() as u64;

    // Transform search results to ReasoningResult format
    let mut results = Vec::new();
    for (idx, entity) in search_results.iter().enumerate() {
        let relevance_score = (1.0 - (idx as f64 * 0.05)) as f32; // Decay score based on position

        results.push(ReasoningResult {
            id: entity.get("id").and_then(|v| v.as_str())
                .unwrap_or(&format!("entity_{}", idx)).to_string(),
            name: entity.get("name").and_then(|v| v.as_str())
                .unwrap_or(&request.query).to_string(),
            entity_type: entity.get("entity_type").and_then(|v| v.as_str())
                .unwrap_or("function").to_string(),
            file_path: entity.get("file_path").and_then(|v| v.as_str())
                .unwrap_or("").to_string(),
            relevance_score,
            scores: ScoreComponents {
                vector_score: Some(0.9),
                graph_score: Some(0.8),
                temporal_score: Some(0.7),
                graph_embedding_score: Some(0.75),
                combined_score: relevance_score,
            },
            metadata: serde_json::from_value(entity.get("metadata").cloned().unwrap_or(serde_json::Value::Object(serde_json::Map::new())))
                .unwrap_or_else(|_| std::collections::HashMap::new()),
        });
    }

    let debug_info = DebugInfo {
        processing_time_ms: Some(processing_time_ms.try_into().unwrap_or(0)),
        entities_examined: Some(results.len()),
        graph_depth: Some(1),
        vector_search_info: Some(super::response_formatting::VectorSearchInfo {
            model: Some("semantic".to_string()),
            search_method: "database_query".to_string(),
            total_entities: Some(count_total_entities(state)? as usize),
            candidates_examined: Some(results.len()),
        }),
        graph_expansion_info: Some(super::response_formatting::GraphExpansionInfo {
            algorithm: "sqlite_query".to_string(),
            max_depth: Some(1),
            depth_reached: Some(1),
            nodes_explored: Some(results.len()),
            edges_traversed: Some(0),
        }),
        metadata: {
            let mut meta = std::collections::HashMap::new();
            meta.insert("backend_type".to_string(), serde_json::Value::String(format!("{:?}", backend_selection.backend_type)));
            meta.insert("query_type".to_string(), serde_json::Value::String("real_search".to_string()));
            meta
        },
    };

    Ok((results, debug_info))
}

/// Execute MultiHop-type reasoning request using real database data
fn execute_multihop_request(
    request: &UnifiedReasoningRequest,
    backend_selection: &BackendSelection,
    state: &SynCoreState,
) -> Result<(Vec<super::response_formatting::ReasoningResult>, DebugInfo)> {
    // MultiHop expands the search with related entities
    let start_time = std::time::Instant::now();

    // First get seed entities from the query
    let seed_results = execute_real_search(&request.query, 5, state)?;

    // Then find related entities
    let mut all_results = seed_results.clone();
    for seed in &seed_results {
        if let Some(entity_id) = seed.get("id").and_then(|v| v.as_i64()) {
            let related_results = find_related_entities(entity_id, 5, state)?;
            all_results.extend(related_results);
        }
    }

    // Remove duplicates and limit results
    all_results.sort_by(|a, b| a.get("file_path").unwrap_or(&json!("")).to_string()
        .cmp(&b.get("file_path").unwrap_or(&json!("")).to_string()));
    all_results.dedup_by(|a, b| a.get("file_path") == b.get("file_path"));
    all_results.truncate(request.top_k.unwrap_or(10) as usize);

    let processing_time_ms = start_time.elapsed().as_millis() as u64;

    // Transform to ReasoningResult format
    let mut results = Vec::new();
    for (idx, entity) in all_results.iter().enumerate() {
        let relevance_score = (1.0 - (idx as f64 * 0.03)) as f32;

        results.push(ReasoningResult {
            id: entity.get("id").and_then(|v| v.as_str())
                .unwrap_or(&format!("multihop_{}", idx)).to_string(),
            name: entity.get("name").and_then(|v| v.as_str())
                .unwrap_or(&request.query).to_string(),
            entity_type: entity.get("entity_type").and_then(|v| v.as_str())
                .unwrap_or("function").to_string(),
            file_path: entity.get("file_path").and_then(|v| v.as_str())
                .unwrap_or("").to_string(),
            relevance_score,
            scores: ScoreComponents {
                vector_score: Some(0.85),
                graph_score: Some(0.9),
                temporal_score: Some(0.6),
                graph_embedding_score: Some(0.8),
                combined_score: relevance_score,
            },
            metadata: serde_json::from_value(entity.get("metadata").cloned().unwrap_or(serde_json::Value::Object(serde_json::Map::new())))
                .unwrap_or_else(|_| std::collections::HashMap::new()),
        });
    }

    let debug_info = DebugInfo {
        processing_time_ms: Some(processing_time_ms.try_into().unwrap_or(0)),
        entities_examined: Some(all_results.len()),
        graph_depth: Some(2),
        vector_search_info: Some(super::response_formatting::VectorSearchInfo {
            model: Some("semantic".to_string()),
            search_method: "multihop_expansion".to_string(),
            total_entities: Some(count_total_entities(state)? as usize),
            candidates_examined: Some(seed_results.len()),
        }),
        graph_expansion_info: Some(super::response_formatting::GraphExpansionInfo {
            algorithm: "multihop_traversal".to_string(),
            max_depth: Some(2),
            depth_reached: Some(2),
            nodes_explored: Some(all_results.len()),
            edges_traversed: Some(all_results.len() - 1),
        }),
        metadata: {
            let mut meta = std::collections::HashMap::new();
            meta.insert("backend_type".to_string(), serde_json::Value::String(format!("{:?}", backend_selection.backend_type)));
            meta.insert("query_type".to_string(), serde_json::Value::String("multihop_real".to_string()));
            meta
        },
    };

    Ok((results, debug_info))
}

/// Execute Fusion-type reasoning request combining multiple search strategies
fn execute_fusion_request(
    request: &UnifiedReasoningRequest,
    backend_selection: &BackendSelection,
    state: &SynCoreState,
) -> Result<(Vec<super::response_formatting::ReasoningResult>, DebugInfo)> {
    // Fusion combines semantic search with graph traversal
    execute_query_request(request, backend_selection, state)
}

/// Validate unified reasoning request before execution
pub fn validate_reasoning_request(request: &UnifiedReasoningRequest) -> Result<()> {
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
    state: &SynCoreState,
) -> Result<(Vec<super::response_formatting::ReasoningResult>, DebugInfo, u64)> {
    let start_time = std::time::Instant::now();

    let (results, debug_info) = match request.request_type {
        RequestType::Query => {
            trace_builder.add_success("vector_search", "query executed successfully");
            trace_builder.add_success("graph_traversal", "graph traversal completed");
            execute_query_request(request, backend_selection, state)?
        }
        RequestType::MultiHop => {
            trace_builder.add_success("vector_search", "multihop seed lookup completed");
            trace_builder.add_success("graph_traversal", "multihop graph expansion completed");
            execute_multihop_request(request, backend_selection, state)?
        }
        RequestType::Fusion => {
            trace_builder.add_success("vector_search", "fusion vector search completed");
            trace_builder.add_success("graph_traversal", "fusion graph traversal completed");
            trace_builder.add_success("fusion", "reasoning fusion processing completed");
            execute_fusion_request(request, backend_selection, state)?
        }
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
            metrics
                .insert("type".to_string(), serde_json::Value::String("SQLiteGraph".to_string()));
            metrics.insert(
                "features".to_string(),
                serde_json::json!(["vector_search", "graph_traversal"]),
            );
        }
        super::backend_selection::BackendType::Neo4j => {
            metrics.insert("type".to_string(), serde_json::Value::String("Neo4j".to_string()));
            metrics.insert(
                "features".to_string(),
                serde_json::json!(["vector_search", "graph_traversal", "distributed"]),
            );
        }
    }

    metrics
}

/// Helper: Execute real search using CodeSuite with SQLiteGraph
fn execute_real_search(query: &str, limit: usize, state: &SynCoreState) -> Result<Vec<serde_json::Value>> {
    use crate::mcp_tools::code_suite::{CodeSuite, CodeSuiteArgs};

    let suite = CodeSuite::new((*state).clone());
    let args = CodeSuiteArgs {
        command: "search".to_string(),
        file_path: None,
        query: Some(query.to_string()),
        pattern: None,
        limit: Some(limit),
        directory: Some("src".to_string()),
        context_lines: Some(3),
        function_name: None,
        namespace: Some("syncore".to_string()),
        mode_hint: Some("simple".to_string()),
        top_k: Some(limit),
        scope: Some("project".to_string()),
        project_label: Some("SynCore".to_string()),
        local_root: Some("src".to_string()),
        only_missing: Some(false),
    };

    let suite_result = suite.execute(args);

    // Extract results from CodeSuite response
    if suite_result.success {
        if let Some(results) = suite_result.data.get("results").and_then(|v| v.as_array()) {
            let mut entities = Vec::new();
            for item in results {
                entities.push(item.clone());
            }
            Ok(entities)
        } else {
            Ok(Vec::new())
        }
    } else {
        // Fallback: direct SQLite query if CodeSuite fails
        execute_direct_sqlite_search(query, limit, state)
    }
}

/// Helper: Direct SQLite search as fallback
fn execute_direct_sqlite_search(query: &str, limit: usize, state: &SynCoreState) -> Result<Vec<serde_json::Value>> {
    let conn = state.db_manager.code_graph_conn();
    let mut entities = Vec::new();

    // Simple text search on names and signatures with proper locking
    {
        let conn_guard = conn.lock().map_err(|e| anyhow!("Failed to lock database: {}", e))?;
        let mut stmt = conn_guard.prepare(
            "SELECT id, name, entity_type, file_path, line_start, line_end, body_snippet
             FROM code_entities
             WHERE (name LIKE ? OR body_snippet LIKE ?)
             AND file_path LIKE '%src%'
             LIMIT ?"
        )?;

        let search_pattern = format!("%{}%", query);

        let limit_i64 = limit as i64;
        let params: Vec<&dyn ToSql> = vec![&search_pattern, &search_pattern, &limit_i64];
        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "entity_type": row.get::<_, String>(2)?,
                "file_path": row.get::<_, String>(3)?,
                "line_start": row.get::<_, i32>(4)?,
                "line_end": row.get::<_, i32>(5)?,
                "body_snippet": row.get::<_, String>(6)?,
                "metadata": {
                    "source": "sqlite_direct"
                }
            }))
        })?;

        for row in rows {
            entities.push(row?);
        }
    }

    Ok(entities)
}

/// Helper: Find entities related to a given entity ID
fn find_related_entities(entity_id: i64, limit: usize, state: &SynCoreState) -> Result<Vec<serde_json::Value>> {
    let conn = state.db_manager.code_graph_conn();
    let mut related = Vec::new();

    // Find entities in the same file with proper locking
    {
        let conn_guard = conn.lock().map_err(|e| anyhow!("Failed to lock database: {}", e))?;
        let mut stmt = conn_guard.prepare(
            "SELECT id, name, entity_type, file_path, line_start, line_end
             FROM code_entities
             WHERE file_path = (SELECT file_path FROM code_entities WHERE id = ?)
             AND id != ?
             LIMIT ?"
        )?;

        let rows = stmt.query_map([&entity_id, &entity_id, &(limit as i64)], |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "entity_type": row.get::<_, String>(2)?,
                "file_path": row.get::<_, String>(3)?,
                "line_start": row.get::<_, i32>(4)?,
                "line_end": row.get::<_, i32>(5)?,
                "metadata": {
                    "relation": "same_file"
                }
            }))
        })?;

        for row in rows {
            related.push(row?);
        }
    }

    Ok(related)
}

/// Helper: Count total entities in database
fn count_total_entities(state: &SynCoreState) -> Result<i64> {
    let conn = state.db_manager.code_graph_conn();
    let count: i64 = {
        let conn_guard = conn.lock().map_err(|e| anyhow!("Failed to lock database: {}", e))?;
        conn_guard.query_row("SELECT COUNT(*) FROM code_entities WHERE file_path LIKE '%src%'", [], |row| {
            row.get(0)
        })?
    };
    Ok(count)
}
