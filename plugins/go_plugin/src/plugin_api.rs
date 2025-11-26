use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum PluginRequest {
    #[serde(rename = "init")]
    Init {
        plugin_name: String,
        version: String,
    },
    #[serde(rename = "capabilities")]
    Capabilities,
    #[serde(rename = "execute")]
    Execute {
        task: String,
        params: HashMap<String, serde_json::Value>,
    },
    #[serde(rename = "shutdown")]
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_tasks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<PluginResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<Vec<Entity>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edges: Option<Vec<Edge>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<Diagnostic>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub file_path: String,
    pub name: String,
    pub kind: EntityKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EntityKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Interface,
    Function,
    Variable,
    Import,
    Export,
    TypeAlias,
    Enum,
    Method,
    Property,
    Parameter,
    Struct,
    Const,
    Var,
    Field,
    Constructor,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Event,
    Operator,
    TypeParameter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EdgeKind {
    Contains,
    Extends,
    Implements,
    Calls,
    Imports,
    Exports,
    References,
    TypeOf,
    Instantiates,
    Field,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub tool: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginError {
    ToolNotFound(String),
    MalformedLspOutput(String),
    LspTimeout,
    InvalidRequest(String),
    IoError(String),
}

impl PluginResponse {
    pub fn ready(plugin_name: String, supported_tasks: Vec<String>) -> Self {
        Self {
            status: "ready".to_string(),
            plugin_name: Some(plugin_name),
            supported_tasks: Some(supported_tasks),
            tasks: None,
            result: None,
            error: None,
        }
    }

    pub fn capabilities(tasks: Vec<String>) -> Self {
        Self {
            status: "ok".to_string(),
            plugin_name: None,
            supported_tasks: None,
            tasks: Some(tasks),
            result: None,
            error: None,
        }
    }

    pub fn success(result: PluginResult) -> Self {
        Self {
            status: "ok".to_string(),
            plugin_name: None,
            supported_tasks: None,
            tasks: None,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            status: "error".to_string(),
            plugin_name: None,
            supported_tasks: None,
            tasks: None,
            result: None,
            error: Some(error),
        }
    }

    pub fn shutdown() -> Self {
        Self {
            status: "ok".to_string(),
            plugin_name: None,
            supported_tasks: None,
            tasks: None,
            result: None,
            error: None,
        }
    }
}