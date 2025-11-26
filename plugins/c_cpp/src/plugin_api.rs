//! Plugin API definitions for the C/C++ plugin
//!
//! This module provides common types and traits for language plugins.
//! Some types are scaffolding for future use.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Common trait that all language plugins must implement
pub trait LanguagePlugin {
    /// Get the name of the language plugin
    fn name(&self) -> &str;

    /// Get the file extensions this plugin handles
    fn extensions(&self) -> Vec<&'static str>;

    /// Check if this plugin can handle the given file path
    fn can_handle(&self, file_path: &str) -> bool {
        let path = std::path::Path::new(file_path);
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            self.extensions().contains(&ext)
        } else {
            false
        }
    }
}

/// Types of entities that can be extracted from source code
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Function,
    Method,
    StaticMethod,
    Class,
    Struct,
    Enum,
    Typedef,
    Namespace,
    Macro,
    Header,
    Variable,
    Parameter,
    Field,
    Interface,
}

/// Types of relationships between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    Calls,
    Defines,
    Implements,
    Inherits,
    Includes,
    MemberOf,
    Uses,
    Declares,
    Overloads,
    Overrides,
    Instantiates,
    UsesType,
    BelongsToNamespace,
    DefinesMacro,
    UsesMacro,
    MethodOf,
}

/// Severity levels for diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Standard entity structure for all language plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// The type of entity
    #[serde(rename = "type")]
    pub entity_type: EntityType,

    /// The name of the entity
    pub name: String,

    /// The file containing the entity
    pub file_path: String,

    /// Line number (1-based)
    pub line: u32,

    /// Column number (1-based)
    pub column: u32,

    /// End line number (1-based, inclusive)
    pub end_line: u32,

    /// End column number (1-based, inclusive)
    pub end_column: u32,

    /// Additional metadata specific to the entity type
    #[serde(flatten)]
    pub metadata: Value,
}

/// Standard edge structure for all language plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// The type of relationship
    #[serde(rename = "type")]
    pub edge_type: EdgeType,

    /// The source entity (name or ID)
    pub source: String,

    /// The target entity (name or ID)
    pub target: String,

    /// Additional metadata specific to the relationship
    #[serde(flatten)]
    pub metadata: Value,
}

/// Standard diagnostic structure for all language plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// The file containing the diagnostic
    pub file_path: String,

    /// Line number (1-based)
    pub line: u32,

    /// Column number (1-based)
    pub column: u32,

    /// The diagnostic message
    pub message: String,

    /// The severity level
    pub severity: Severity,

    /// The rule or check that triggered this diagnostic
    pub rule: Option<String>,

    /// The source of the diagnostic (e.g., "clangd", "clang-tidy")
    pub source: String,

    /// Available fixes for this diagnostic
    pub fixes: Vec<Fix>,
}

/// A fix that can be applied to resolve a diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    /// The file to modify
    pub file_path: String,

    /// Byte offset from the start of the file
    pub offset: u32,

    /// Number of bytes to replace
    pub length: u32,

    /// The replacement text
    pub replacement: String,
}

/// Result of indexing a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    /// The entities found in the file
    pub entities: Vec<Entity>,

    /// The relationships between entities
    pub edges: Vec<Edge>,

    /// The files included by this file
    pub includes: Vec<String>,

    /// The macros defined in this file
    pub macros: Vec<Macro>,
}

/// A macro definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macro {
    /// The macro name
    pub name: String,

    /// The parameters (for function-like macros)
    pub parameters: Vec<String>,

    /// The replacement text
    pub replacement: String,

    /// Whether this is a function-like macro
    pub is_function_like: bool,

    /// Whether this is a conditional macro
    pub is_conditional: bool,

    /// The file containing the macro
    pub file_path: String,

    /// Line number (1-based)
    pub line: u32,

    /// Column number (1-based)
    pub column: u32,
}
