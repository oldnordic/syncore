//! Standardized response schemas for all SynCore MCP tools

use serde::{Deserialize, Serialize};

/// Base response wrapper for all tool responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
    #[serde(default)]
    pub dry_run: bool,
}

impl<T> ToolResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            dry_run: false,
        }
    }

    pub fn success_dry_run(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            dry_run: true,
        }
    }

    pub fn error(error: serde_json::Value) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            dry_run: false,
        }
    }
}

// ============================================================================
// Memory Tool Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStoreResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryResponse {
    pub value: Option<String>,
    pub found: bool,
}

// ============================================================================
// Task Tool Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCreateResponse {
    pub task_id: i64,
    pub message: String,
}

// ============================================================================
// Vector Tool Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorInsertResponse {
    pub vector_id: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResponse {
    pub results: Vec<VectorHit>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorHit {
    pub id: i64,
    pub text: String,
    pub score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// Parser Tool Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserAnalyzeResponse {
    pub file_path: String,
    pub entities: Vec<CodeEntity>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEntity {
    pub kind: String,
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserSearchResponse {
    pub matches: Vec<SearchMatch>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    pub file_path: String,
    pub line_number: usize,
    pub line_text: String,
    pub context: Option<Vec<String>>,
}

// ============================================================================
// Code Tool Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIndexResponse {
    pub file_path: String,
    pub entities_indexed: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchResponse {
    pub results: Vec<CodeSearchResult>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchResult {
    pub file_path: String,
    pub entity_kind: String,
    pub entity_name: String,
    pub score: f32,
}

// ============================================================================
// Document Tool Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentIndexResponse {
    pub directory: String,
    pub files_indexed: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSearchResponse {
    pub results: Vec<DocumentSearchResult>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSearchResult {
    pub file_path: String,
    pub excerpt: String,
    pub score: f32,
}

// ============================================================================
// Graph Tool Responses
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryResponse {
    pub results: Vec<serde_json::Value>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphInsertResponse {
    pub message: String,
    pub success: bool,
}
