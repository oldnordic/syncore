//! MCP Tool Suites - Unified tool interface for SynCore
//!
//! Replaces 30+ scattered tools with 5 unified suite tools:
//! - `memory_suite`: Memory and vector operations
//! - `code_suite`: Code indexing, search, and analysis
//! - `graph_suite`: Neo4j graph operations
//! - `mapping_suite`: Application structure mapping
//! - `debug_suite`: Debugging, logs, and diagnostics

pub mod code_suite;
pub mod debug_suite;
pub mod graph_suite;
pub mod mapping_suite;
pub mod memory_suite;
pub mod refrag_suite;

use serde::{Deserialize, Serialize};

/// Common result type for suite operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteResult {
    pub success: bool,
    pub command: String,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SuiteResult {
    pub fn ok(command: &str, data: serde_json::Value) -> Self {
        Self {
            success: true,
            command: command.to_string(),
            data,
            error: None,
        }
    }

    pub fn err(command: &str, error: impl ToString) -> Self {
        Self {
            success: false,
            command: command.to_string(),
            data: serde_json::Value::Null,
            error: Some(error.to_string()),
        }
    }
}

/// Suite command dispatcher trait
pub trait SuiteDispatcher {
    /// Dispatch a command with arguments
    fn dispatch(&self, command: &str, args: serde_json::Value) -> SuiteResult;

    /// List available commands
    fn list_commands(&self) -> Vec<&'static str>;

    /// Get help for a specific command
    fn help(&self, command: &str) -> Option<&'static str>;
}

/// Emit deprecation warning for legacy tool usage
pub fn emit_deprecation_warning(legacy_tool: &str, suite: &str, command: &str) {
    eprintln!(
        "[DEPRECATED] Tool '{}' is deprecated. Use '{} {}' instead.",
        legacy_tool, suite, command
    );
}
