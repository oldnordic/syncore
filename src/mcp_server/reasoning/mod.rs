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
pub mod consistency;
pub mod evaluation;
pub mod execution;
pub mod metadata;
pub mod reflection;
pub mod request_parsing;
pub mod response_formatting;
pub mod trace;

// Re-export all submodules for public API
pub use backend_selection::{
    select_reasoning_backend, BackendMetadata, BackendSelection, BackendSelectionConfig,
    BackendType,
};
pub use consistency::{
    build_tool_snapshot_from_unified_response, validate_snapshots_consistency, ConsistencyReport,
    ConsistencyViolation, ToolReasoningSnapshot,
};
pub use evaluation::{evaluate_reasoning, normalize_evaluation, ReasoningEvaluation};
pub use execution::{execute_reasoning_request, get_backend_metrics, validate_reasoning_request};
pub use metadata::{normalize_metadata, ReasoningMetadata, ReasoningStage};
pub use reflection::{build_reflection, normalize_reflection, ReasoningReflection};
pub use request_parsing::converters::to_codegraph_fusion_request;
pub use request_parsing::{
    normalize_scope, parse_unified_request, validate_top_k, RequestParameters,
    RequestParsingConfig, RequestType, UnifiedReasoningRequest,
};
pub use response_formatting::{
    create_backend_info, create_request_metadata, format_error_response, format_success_response,
    format_error_simple, format_success_simple, from_mcp_error, to_mcp_call_tool_result, BackendInfo, DebugInfo, ErrorCategory, ErrorInfo,
    GraphExpansionInfo, ReasoningResult, RequestMetadata, ResponseFormattingConfig,
    ScoreComponents, UnifiedReasoningResponse, VectorSearchInfo,
};
pub use trace::{ReasoningTrace, ReasoningTraceBuilder, ReasoningTraceStage};
