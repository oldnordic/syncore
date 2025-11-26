use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub version: String,
    pub path: String,
    pub status: PluginStatus,
    pub capabilities: Vec<PluginCapability>,
    pub process_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginStatus {
    Unloaded,
    Loading,
    Ready,
    Busy,
    Error(String),
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginCapability {
    IndexDirectory,
    LspIngest,
    LintIngest,
    DiagnosticsExport,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginInitRequest {
    pub event: String,
    pub plugin_name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginInitResponse {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginCapabilitiesResponse {
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginExecuteRequest {
    pub task: String,
    pub params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginExecuteResponse {
    pub result: serde_json::Value,
    pub diagnostics: Vec<serde_json::Value>,
    pub entities: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginShutdownRequest {
    pub event: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginShutdownResponse {
    pub status: String,
}
