//! Unified MCP reasoning module
//!
//! This module provides shared utilities and a unified interface for all reasoning-related
//! MCP tools (raggraph_query, raggraph_multihop, code_graph_fusion_query).
//!
//! Key Features:
//! - Consistent backend selection across all reasoning tools
//! - Unified request parameter parsing and validation
//! - Standardized response formatting and error handling
//! - Backend-agnostic tool execution with SQLiteGraph-first preference
//!
//! Module Structure:
//! - mod.rs: Main unified interface and orchestration
//! - backend_selection.rs: Backend selection logic and configuration
//! - request_parsing.rs: Request parameter handling and validation
//! - response_formatting.rs: Response formatting and JSON serialization
//! - metadata.rs: Metadata structures and normalization
//! - execution.rs: Request execution logic
//!
//! Usage:
//! ```rust
//! use crate::mcp_server::reasoning::{execute_reasoning_request, UnifiedReasoningRequest};
//!
//! // Parse incoming request with unified handling
//! let unified_request = parse_unified_request(raw_params, request_type, None)?;
//!
//! // Execute with backend-agnostic logic
//! let response = execute_reasoning_request(unified_request, &mcp_state)?;
//!
//! // Returns standardized CallToolResult for all reasoning tools
//! ```

// Public submodules for unified reasoning infrastructure
pub mod backend_selection;
pub mod request_parsing;
pub mod response_formatting;
pub mod metadata;
pub mod execution;
pub mod trace;
pub mod evaluation;
pub mod reflection;
pub mod consistency;

// Re-export all submodules for public API
pub use backend_selection::{
    select_reasoning_backend, BackendSelection, BackendSelectionConfig, BackendType,
    BackendMetadata,
};
pub use metadata::{ReasoningMetadata, ReasoningStage, normalize_metadata};
pub use request_parsing::{
    parse_unified_request, UnifiedReasoningRequest, RequestType, RequestParameters,
    RequestParsingConfig, normalize_scope, validate_top_k,
};
pub use request_parsing::converters::to_codegraph_fusion_request;
pub use response_formatting::{
    format_success_response, format_error_response, to_mcp_call_tool_result,
    from_mcp_error, UnifiedReasoningResponse, RequestMetadata, ReasoningResult, ScoreComponents,
    BackendInfo, DebugInfo, VectorSearchInfo, GraphExpansionInfo, ErrorInfo, ErrorCategory,
    ResponseFormattingConfig, create_request_metadata, create_backend_info,
};
pub use execution::{execute_reasoning_request, validate_reasoning_request, get_backend_metrics};
pub use trace::{
    ReasoningTrace, ReasoningTraceStage, ReasoningTraceBuilder,
};
pub use evaluation::{
    ReasoningEvaluation, evaluate_reasoning, normalize_evaluation,
};
pub use reflection::{
    ReasoningReflection, build_reflection, normalize_reflection,
};
pub use consistency::{
    ToolReasoningSnapshot, ConsistencyViolation, ConsistencyReport,
    build_tool_snapshot_from_unified_response, validate_snapshots_consistency,
};