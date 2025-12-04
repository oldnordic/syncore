//! Standardized request schemas for all SynCore MCP tools
//!
//! Provides consistent field naming across all tool requests:
//! - file_path (NOT filepath)
//! - query (NOT search_query)
//! - limit (NOT top_k)
//! - directory (NOT dir)

use serde::{Deserialize, Serialize};

/// Base trait for all tool requests with dry_run support
pub trait ToolRequest {
    /// Check if this is a dry-run request (no side effects)
    fn is_dry_run(&self) -> bool {
        false
    }
}

// ============================================================================
// Memory Tool Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStoreRequest {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for MemoryStoreRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryRequest {
    pub key: String,
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for MemoryQueryRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

// ============================================================================
// Task Tool Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCreateRequest {
    pub goal: String,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for TaskCreateRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

// ============================================================================
// Vector Tool Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorInsertRequest {
    pub text: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for VectorInsertRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchRequest {
    pub query: String, // Standardized: NOT search_query
    #[serde(default = "default_limit")]
    pub limit: usize, // Standardized: NOT top_k
    #[serde(default)]
    pub dry_run: bool,
}

fn default_limit() -> usize {
    5
}

impl ToolRequest for VectorSearchRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

// ============================================================================
// Parser Tool Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserAnalyzeRequest {
    pub file_path: String, // Standardized: NOT filepath
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for ParserAnalyzeRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserSearchRequest {
    pub pattern: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub context_lines: Option<usize>,
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for ParserSearchRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

// ============================================================================
// Code Tool Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIndexRequest {
    pub file_path: String, // Standardized
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for CodeIndexRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchRequest {
    pub query: String, // Standardized
    #[serde(default = "default_limit")]
    pub limit: usize, // Standardized
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for CodeSearchRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CodeIndexDirectoryRequest {
    pub directory: String, // Standardized: NOT dir
    pub pattern: String,
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for CodeIndexDirectoryRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

// ============================================================================
// Document Tool Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentIndexRequest {
    pub directory: String, // Standardized
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for DocumentIndexRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSearchRequest {
    pub query: String, // Standardized
    #[serde(default = "default_limit")]
    pub limit: usize, // Standardized
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for DocumentSearchRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

// ============================================================================
// Graph Tool Requests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryRequest {
    pub cypher: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for GraphQueryRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphInsertRequest {
    pub cypher: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    #[serde(default)]
    pub dry_run: bool,
}

impl ToolRequest for GraphInsertRequest {
    fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}
