use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::c_cpp_indexer::CppIndexer;
use crate::c_cpp_diagnostics::CppDiagnostics;
use crate::c_cpp_include_graph::IncludeGraph;
use crate::c_cpp_macro_extractor::MacroExtractor;

#[derive(Debug, Serialize, Deserialize)]
pub struct DlrRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub workspace_root: Option<String>,
    pub task: Option<String>,
    pub file_path: Option<String>,
    pub directory_path: Option<String>,
    pub recursive: Option<bool>,
    // For mock testing
    #[serde(default)]
    pub mock_clangd_response: Option<Value>,
    #[serde(default)]
    pub mock_clang_tidy_response: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DlrResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edges: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_graph: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_entities: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_edges: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub struct DlrEntry {
    workspace_root: Option<String>,
    initialized: bool,
    should_shutdown: bool,
    indexer: CppIndexer,
    diagnostics: CppDiagnostics,
    include_graph: IncludeGraph,
    macro_extractor: MacroExtractor,
    // Store indexed data
    entities: Vec<Value>,
    edges: Vec<Value>,
    file_to_entities: HashMap<String, Vec<usize>>,
}

impl DlrEntry {
    pub fn new() -> Self {
        Self {
            workspace_root: None,
            initialized: false,
            should_shutdown: false,
            indexer: CppIndexer::new(),
            diagnostics: CppDiagnostics::new(),
            include_graph: IncludeGraph::new(),
            macro_extractor: MacroExtractor::new(),
            entities: Vec::new(),
            edges: Vec::new(),
            file_to_entities: HashMap::new(),
        }
    }

    pub fn should_shutdown(&self) -> bool {
        self.should_shutdown
    }

    pub fn process_command(&mut self, line: &str) -> String {
        let request = match serde_json::from_str::<DlrRequest>(line) {
            Ok(req) => req,
            Err(e) => {
                return serde_json::to_string(&DlrResponse {
                    ok: false,
                    error: Some("invalid_request".to_string()),
                    message: Some(format!("Invalid JSON: {}", e)),
                    ..Default::default()
                }).unwrap();
            }
        };

        let response = match request.request_type.as_str() {
            "init" => self.handle_init(request),
            "capabilities" => self.handle_capabilities(request),
            "execute" => self.handle_execute(request),
            "shutdown" => self.handle_shutdown(request),
            _ => DlrResponse {
                ok: false,
                error: Some("invalid_request".to_string()),
                message: Some(format!("Unknown request type: {}", request.request_type)),
                ..Default::default()
            }
        };

        serde_json::to_string(&response).unwrap()
    }

    fn handle_init(&mut self, request: DlrRequest) -> DlrResponse {
        if let Some(workspace) = request.workspace_root {
            self.workspace_root = Some(workspace.clone());
            self.initialized = true;

            DlrResponse {
                ok: true,
                workspace: Some(workspace),
                ..Default::default()
            }
        } else {
            DlrResponse {
                ok: false,
                error: Some("invalid_request".to_string()),
                message: Some("Missing workspace_root".to_string()),
                ..Default::default()
            }
        }
    }

    fn handle_capabilities(&self, _request: DlrRequest) -> DlrResponse {
        DlrResponse {
            ok: true,
            capabilities: Some(json!({
                "tasks": [
                    "c.index_file",
                    "c.index_directory",
                    "c.run_diagnostics",
                    "c.capabilities",
                    "c.shutdown"
                ]
            })),
            ..Default::default()
        }
    }

    fn handle_execute(&mut self, request: DlrRequest) -> DlrResponse {
        if !self.initialized {
            return DlrResponse {
                ok: false,
                error: Some("not_initialized".to_string()),
                message: Some("Plugin not initialized. Call init first.".to_string()),
                ..Default::default()
            };
        }

        let task = match request.task {
            Some(ref t) => t,
            None => {
                return DlrResponse {
                    ok: false,
                    error: Some("invalid_request".to_string()),
                    message: Some("Missing task".to_string()),
                    ..Default::default()
                };
            }
        };

        match task.as_str() {
            "c.index_file" => {
                if let Some(file_path) = request.file_path {
                    self.index_file(&file_path)
                } else {
                    DlrResponse {
                        ok: false,
                        error: Some("invalid_request".to_string()),
                        message: Some("Missing file_path for index_file".to_string()),
                        ..Default::default()
                    }
                }
            }
            "c.index_directory" => {
                if let Some(dir_path) = request.directory_path {
                    let recursive = request.recursive.unwrap_or(false);
                    self.index_directory(&dir_path, recursive)
                } else {
                    DlrResponse {
                        ok: false,
                        error: Some("invalid_request".to_string()),
                        message: Some("Missing directory_path for index_directory".to_string()),
                        ..Default::default()
                    }
                }
            }
            "c.run_diagnostics" => {
                if let Some(file_path) = request.file_path {
                    self.run_diagnostics(&file_path, request.mock_clangd_response, request.mock_clang_tidy_response)
                } else {
                    DlrResponse {
                        ok: false,
                        error: Some("invalid_request".to_string()),
                        message: Some("Missing file_path for run_diagnostics".to_string()),
                        ..Default::default()
                    }
                }
            }
            "c.capabilities" => self.handle_capabilities(request),
            "c.shutdown" => self.handle_shutdown(request),
            _ => DlrResponse {
                ok: false,
                error: Some("unsupported_task".to_string()),
                message: Some(format!("Unsupported task: {}", task)),
                ..Default::default()
            }
        }
    }

    fn handle_shutdown(&mut self, _request: DlrRequest) -> DlrResponse {
        self.should_shutdown = true;
        DlrResponse {
            ok: true,
            ..Default::default()
        }
    }

    fn index_file(&mut self, file_path: &str) -> DlrResponse {
        // Check if file exists
        if !std::path::Path::new(file_path).exists() {
            return DlrResponse {
                ok: false,
                error: Some("file_not_found".to_string()),
                message: Some(format!("File not found: {}", file_path)),
                ..Default::default()
            };
        }

        // Parse the file
        let parse_result = self.indexer.index_file(file_path);
        match parse_result {
            Ok((mut file_entities, mut file_edges)) => {
                // Add includes to include graph
                if let Ok(include_edges) = self.include_graph.process_file(file_path) {
                    file_edges.extend(include_edges);
                }

                // Extract macros
                if let Ok((macro_entities, macro_edges)) = self.macro_extractor.extract_from_file(file_path) {
                    file_entities.extend(macro_entities);
                    file_edges.extend(macro_edges);
                }

                // Store entity indices for this file
                let start_index = self.entities.len();
                let file_entity_indices: Vec<usize> = (start_index..start_index + file_entities.len()).collect();

                // Add file path to entities
                for entity in &mut file_entities {
                    entity.as_object_mut()
                        .unwrap()
                        .insert("file_path".to_string(), Value::String(file_path.to_string()));
                }

                // Store entities and edges
                self.entities.extend(file_entities);
                self.edges.extend(file_edges);
                self.file_to_entities.insert(file_path.to_string(), file_entity_indices);

                DlrResponse {
                    ok: true,
                    entities: Some(self.entities.clone()),
                    edges: Some(self.edges.clone()),
                    total_entities: Some(self.entities.len() as u64),
                    total_edges: Some(self.edges.len() as u64),
                    ..Default::default()
                }
            }
            Err(e) => {
                DlrResponse {
                    ok: false,
                    error: Some("indexing_error".to_string()),
                    message: Some(format!("Failed to index file {}: {}", file_path, e)),
                    ..Default::default()
                }
            }
        }
    }

    fn index_directory(&mut self, dir_path: &str, recursive: bool) -> DlrResponse {
        let dir = std::path::Path::new(dir_path);
        if !dir.exists() || !dir.is_dir() {
            return DlrResponse {
                ok: false,
                error: Some("directory_not_found".to_string()),
                message: Some(format!("Directory not found: {}", dir_path)),
                ..Default::default()
            };
        }

        let mut indexed_files = Vec::new();
        let mut all_entities = Vec::new();
        let mut all_edges = Vec::new();

        // Find all C/C++ files
        let walk_dir = walkdir::WalkDir::new(dir);
        let iter = if recursive {
            walk_dir.into_iter()
        } else {
            walk_dir.max_depth(1).into_iter()
        };

        for entry in iter {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue, // Skip entries we can't access
            };

            let path = entry.path();
            if path.is_file() {
                let path_str = path.to_string_lossy();
                if path_str.ends_with(".cpp") || path_str.ends_with(".cxx") || path_str.ends_with(".cc") ||
                   path_str.ends_with(".c") || path_str.ends_with(".hpp") || path_str.ends_with(".hxx") ||
                   path_str.ends_with(".h") || path_str.ends_with(".hh") {

                    // Index this file
                    match self.indexer.index_file(&path_str) {
                        Ok((mut file_entities, mut file_edges)) => {
                            // Add includes to include graph
                            if let Ok(include_edges) = self.include_graph.process_file(&path_str) {
                                file_edges.extend(include_edges);
                            }

                            // Extract macros
                            if let Ok((macro_entities, macro_edges)) = self.macro_extractor.extract_from_file(&path_str) {
                                file_entities.extend(macro_entities);
                                file_edges.extend(macro_edges);
                            }

                            // Add file path to entities
                            for entity in &mut file_entities {
                                entity.as_object_mut()
                                    .unwrap()
                                    .insert("file_path".to_string(), Value::String(path_str.to_string()));
                            }

                            all_entities.extend(file_entities);
                            all_edges.extend(file_edges);
                            indexed_files.push(path_str.to_string());
                        }
                        Err(_) => {
                            // Continue with other files even if one fails
                        }
                    }
                }
            }
        }

        // Store all entities and edges
        self.entities.extend(all_entities);
        self.edges.extend(all_edges);

        DlrResponse {
            ok: true,
            indexed_files: Some(indexed_files),
            total_entities: Some(self.entities.len() as u64),
            total_edges: Some(self.edges.len() as u64),
            entities: Some(self.entities.clone()),
            edges: Some(self.edges.clone()),
            include_graph: Some(self.include_graph.get_graph()),
            ..Default::default()
        }
    }

    fn run_diagnostics(&mut self, file_path: &str, mock_clangd_response: Option<Value>, mock_clang_tidy_response: Option<String>) -> DlrResponse {
        // Check if file exists
        if !std::path::Path::new(file_path).exists() {
            return DlrResponse {
                ok: false,
                error: Some("file_not_found".to_string()),
                message: Some(format!("File not found: {}", file_path)),
                ..Default::default()
            };
        }

        let mut diagnostics = Vec::new();

        // Run clangd diagnostics
        match self.diagnostics.run_clangd(file_path, mock_clangd_response) {
            Ok(mut clangd_diags) => {
                diagnostics.append(&mut clangd_diags);
            }
            Err(e) => {
                eprintln!("Error running clangd diagnostics: {}", e);
            }
        }

        // Run clang-tidy diagnostics
        match self.diagnostics.run_clang_tidy(file_path, mock_clang_tidy_response) {
            Ok(mut tidy_diags) => {
                diagnostics.append(&mut tidy_diags);
            }
            Err(e) => {
                eprintln!("Error running clang-tidy diagnostics: {}", e);
            }
        }

        DlrResponse {
            ok: true,
            diagnostics: Some(
                diagnostics.into_iter()
                    .map(|d| json!({
                        "file_path": d.file_path,
                        "line": d.line,
                        "column": d.column,
                        "message": d.message,
                        "severity": d.severity,
                        "rule": d.rule,
                        "source": d.source,
                        "fixes": d.fixes.iter().map(|f| json!({
                            "file_path": f.file_path,
                            "offset": f.offset,
                            "length": f.length,
                            "replacement": f.replacement,
                        })).collect::<Vec<_>>(),
                    }))
                    .collect()
            ),
            include_graph: Some(self.include_graph.get_graph()),
            ..Default::default()
        }
    }
}

impl Default for DlrResponse {
    fn default() -> Self {
        DlrResponse {
            ok: false,
            workspace: None,
            capabilities: None,
            entities: None,
            edges: None,
            diagnostics: None,
            include_graph: None,
            indexed_files: None,
            total_entities: None,
            total_edges: None,
            error: None,
            message: None,
        }
    }
}
