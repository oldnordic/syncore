//! Mapping Suite - Unified application structure mapping
//!
//! Commands:
//! - `record`: Record a file node in the application map
//! - `get`: Get a file node by path
//! - `search`: Search for related files
//! - `deps`: Get all transitive dependencies for a file
//! - `app_record`: Record a code change
//! - `app_get`: Get changes for a task
//! - `app_history`: Get change history for a file
//! - `app_search`: Search changes by query
//! - `help`: Show available commands

use crate::mcp_tools::streaming::OutputLimiter;
use crate::mcp_tools::{SuiteDispatcher, SuiteResult};
use crate::router::SynCoreState;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Mapping suite arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingSuiteArgs {
    pub command: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub imports: Option<Vec<String>>,
    #[serde(default)]
    pub exports: Option<Vec<String>>,
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
    #[serde(default)]
    pub query: Option<String>,
    // Application change tracking fields
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub change_type: Option<String>,
    #[serde(default)]
    pub old_content: Option<String>,
    #[serde(default)]
    pub new_content: Option<String>,
    #[serde(default)]
    pub line_start: Option<i32>,
    #[serde(default)]
    pub line_end: Option<i32>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub task_id: Option<i64>,
}

/// Mapping suite implementation
pub struct MappingSuite {
    state: SynCoreState,
}

impl MappingSuite {
    pub fn new(state: SynCoreState) -> Self {
        Self {
            state,
        }
    }

    /// Execute the suite command
    pub fn execute(&self, args: MappingSuiteArgs) -> SuiteResult {
        match args.command.as_str() {
            "record" => self.cmd_record(args),
            "get" => self.cmd_get(args),
            "search" => self.cmd_search(args),
            "deps" => self.cmd_deps(args),
            "app_record" => self.cmd_app_record(args),
            "app_get" => self.cmd_app_get(args),
            "app_history" => self.cmd_app_history(args),
            "app_search" => self.cmd_app_search(args),
            "help" => self.cmd_help(),
            _ => SuiteResult::err(
                &args.command,
                format!(
                    "Unknown command '{}'. Use 'help' to see available commands.",
                    args.command
                ),
            ),
        }
    }

    fn cmd_record(&self, args: MappingSuiteArgs) -> SuiteResult {
        let path = match args.path {
            Some(p) => p,
            None => return SuiteResult::err("record", "Missing required parameter: path"),
        };

        let kind = args.kind.unwrap_or_else(|| "file".to_string());

        use crate::portfolio::mapping_tool::{FileNode, MappingTool};

        let node = FileNode {
            path: path.clone(),
            kind,
            language: args.language,
            imports: args.imports.unwrap_or_default(),
            exports: args.exports.unwrap_or_default(),
            dependencies: args.dependencies.unwrap_or_default(),
        };

        let mapping_tool = MappingTool::new(self.state.clone());
        match mapping_tool.record_file(&node) {
            Ok(_) => SuiteResult::ok(
                "record",
                serde_json::json!({
                    "recorded": true,
                    "path": path
                }),
            ),
            Err(e) => SuiteResult::err("record", e.to_string()),
        }
    }

    fn cmd_get(&self, args: MappingSuiteArgs) -> SuiteResult {
        let path = match args.path {
            Some(p) => p,
            None => return SuiteResult::err("get", "Missing required parameter: path"),
        };

        use crate::portfolio::mapping_tool::MappingTool;

        let mapping_tool = MappingTool::new(self.state.clone());
        match mapping_tool.get_file(&path) {
            Ok(Some(node)) => SuiteResult::ok(
                "get",
                serde_json::json!({
                    "found": true,
                    "node": {
                        "path": node.path,
                        "kind": node.kind,
                        "language": node.language,
                        "imports": node.imports,
                        "exports": node.exports,
                        "dependencies": node.dependencies
                    }
                }),
            ),
            Ok(None) => SuiteResult::ok(
                "get",
                serde_json::json!({
                    "found": false,
                    "path": path
                }),
            ),
            Err(e) => SuiteResult::err("get", e.to_string()),
        }
    }

    fn cmd_search(&self, args: MappingSuiteArgs) -> SuiteResult {
        let query = match args.query {
            Some(q) => q,
            None => return SuiteResult::err("search", "Missing required parameter: query"),
        };

        use crate::portfolio::mapping_tool::MappingTool;

        let mapping_tool = MappingTool::new(self.state.clone());
        match mapping_tool.search_related(&query) {
            Ok(nodes) => {
                let results: Vec<serde_json::Value> = nodes
                    .iter()
                    .map(|n| {
                        serde_json::json!({
                            "path": n.path,
                            "kind": n.kind,
                            "language": n.language
                        })
                    })
                    .collect();

                SuiteResult::ok(
                    "search",
                    serde_json::json!({
                        "query": query,
                        "count": results.len(),
                        "results": results
                    }),
                )
            }
            Err(e) => SuiteResult::err("search", e.to_string()),
        }
    }

    fn cmd_deps(&self, args: MappingSuiteArgs) -> SuiteResult {
        let path = match args.path {
            Some(p) => p,
            None => return SuiteResult::err("deps", "Missing required parameter: path"),
        };

        use crate::portfolio::mapping_tool::MappingTool;

        let mapping_tool = MappingTool::new(self.state.clone());
        match mapping_tool.get_all_dependencies(&path) {
            Ok(deps) => SuiteResult::ok(
                "deps",
                serde_json::json!({
                    "path": path,
                    "count": deps.len(),
                    "dependencies": deps
                }),
            ),
            Err(e) => SuiteResult::err("deps", e.to_string()),
        }
    }

    fn cmd_app_record(&self, args: MappingSuiteArgs) -> SuiteResult {
        let file_path = match args.file_path {
            Some(f) => f,
            None => return SuiteResult::err("app_record", "Missing required parameter: file_path"),
        };

        let change_type = match args.change_type {
            Some(c) => c,
            None => {
                return SuiteResult::err("app_record", "Missing required parameter: change_type")
            }
        };

        let line_start = args.line_start.unwrap_or(0);
        let line_end = args.line_end.unwrap_or(0);
        let description = args.description.unwrap_or_default();

        use crate::portfolio::application_tool::{ApplicationTool, CodeChange};

        let change = CodeChange {
            file_path: file_path.clone(),
            change_type,
            old_content: args.old_content,
            new_content: args.new_content,
            line_start,
            line_end,
            description: description.clone(),
            task_id: args.task_id,
        };

        let app_tool = ApplicationTool::new(self.state.clone());
        match app_tool.record_change(&change) {
            Ok(change_id) => SuiteResult::ok(
                "app_record",
                serde_json::json!({
                    "recorded": true,
                    "change_id": change_id,
                    "file_path": file_path
                }),
            ),
            Err(e) => SuiteResult::err("app_record", e.to_string()),
        }
    }

    fn cmd_app_get(&self, args: MappingSuiteArgs) -> SuiteResult {
        let task_id = match args.task_id {
            Some(id) => id,
            None => return SuiteResult::err("app_get", "Missing required parameter: task_id"),
        };

        use crate::portfolio::application_tool::ApplicationTool;

        let app_tool = ApplicationTool::new(self.state.clone());
        match app_tool.get_changes_for_task(task_id) {
            Ok(changes) => {
                let changes_json: Vec<serde_json::Value> = changes
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "file_path": c.file_path,
                            "change_type": c.change_type,
                            "line_start": c.line_start,
                            "line_end": c.line_end,
                            "description": c.description
                        })
                    })
                    .collect();

                SuiteResult::ok(
                    "app_get",
                    serde_json::json!({
                        "task_id": task_id,
                        "count": changes.len(),
                        "changes": changes_json
                    }),
                )
            }
            Err(e) => SuiteResult::err("app_get", e.to_string()),
        }
    }

    fn cmd_app_history(&self, args: MappingSuiteArgs) -> SuiteResult {
        let file_path = match args.file_path {
            Some(f) => f,
            None => {
                return SuiteResult::err("app_history", "Missing required parameter: file_path")
            }
        };

        use crate::portfolio::application_tool::ApplicationTool;

        let app_tool = ApplicationTool::new(self.state.clone());
        match app_tool.get_file_history(&file_path) {
            Ok(changes) => {
                let changes_json: Vec<serde_json::Value> = changes
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "change_type": c.change_type,
                            "line_start": c.line_start,
                            "line_end": c.line_end,
                            "description": c.description,
                            "task_id": c.task_id
                        })
                    })
                    .collect();

                SuiteResult::ok(
                    "app_history",
                    serde_json::json!({
                        "file_path": file_path,
                        "count": changes.len(),
                        "history": changes_json
                    }),
                )
            }
            Err(e) => SuiteResult::err("app_history", e.to_string()),
        }
    }

    fn cmd_app_search(&self, args: MappingSuiteArgs) -> SuiteResult {
        let query = match args.query {
            Some(q) => q,
            None => return SuiteResult::err("app_search", "Missing required parameter: query"),
        };

        use crate::portfolio::application_tool::ApplicationTool;

        let app_tool = ApplicationTool::new(self.state.clone());
        match app_tool.search_changes(&query) {
            Ok(changes) => {
                let changes_json: Vec<serde_json::Value> = changes
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "file_path": c.file_path,
                            "change_type": c.change_type,
                            "line_start": c.line_start,
                            "line_end": c.line_end,
                            "description": c.description,
                            "task_id": c.task_id
                        })
                    })
                    .collect();

                SuiteResult::ok(
                    "app_search",
                    serde_json::json!({
                        "query": query,
                        "count": changes.len(),
                        "results": changes_json
                    }),
                )
            }
            Err(e) => SuiteResult::err("app_search", e.to_string()),
        }
    }

    fn cmd_help(&self) -> SuiteResult {
        SuiteResult::ok(
            "help",
            serde_json::json!({
                "suite": "mapping_suite",
                "description": "Application structure mapping and change tracking",
                "commands": {
                    "record": "Record a file node. Params: path, kind, language, imports, exports, dependencies",
                    "get": "Get a file node. Params: path",
                    "search": "Search related files. Params: query",
                    "deps": "Get transitive dependencies. Params: path",
                    "app_record": "Record a code change. Params: file_path, change_type, line_start, line_end, description, task_id",
                    "app_get": "Get changes for a task. Params: task_id",
                    "app_history": "Get file change history. Params: file_path",
                    "app_search": "Search changes. Params: query"
                }
            }),
        )
    }
}

impl SuiteDispatcher for MappingSuite {
    fn dispatch(&self, command: &str, args: serde_json::Value) -> SuiteResult {
        let mut suite_args: MappingSuiteArgs = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return SuiteResult::err(command, format!("Invalid arguments: {}", e)),
        };
        suite_args.command = command.to_string();

        let result = self.execute(suite_args);

        // Apply streaming contract enforcement to successful responses
        if result.success {
            let limiter = OutputLimiter::default();
            let result_json = json!({
                "command": command,
                "data": result.data
            });

            match limiter.apply_json(&result_json) {
                Ok(limited_json) => {
                    if let Some(limited_data) = limited_json.get("data") {
                        SuiteResult {
                            success: true,
                            command: command.to_string(),
                            data: limited_data.clone(),
                            error: None,
                        }
                    } else {
                        result
                    }
                }
                Err(_) => result, // Fallback to original on error
            }
        } else {
            result
        }
    }

    fn list_commands(&self) -> Vec<&'static str> {
        vec![
            "record",
            "get",
            "search",
            "deps",
            "app_record",
            "app_get",
            "app_history",
            "app_search",
            "help",
        ]
    }

    fn help(&self, command: &str) -> Option<&'static str> {
        match command {
            "record" => Some("Record file node. Params: path, kind, language, imports, exports, dependencies"),
            "get" => Some("Get file node. Params: path"),
            "search" => Some("Search files. Params: query"),
            "deps" => Some("Get dependencies. Params: path"),
            "app_record" => Some("Record code change. Params: file_path, change_type, line_start, line_end, description, task_id"),
            "app_get" => Some("Get task changes. Params: task_id"),
            "app_history" => Some("Get file history. Params: file_path"),
            "app_search" => Some("Search changes. Params: query"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mapping_suite_args_deserialization() {
        let json = serde_json::json!({
            "command": "record",
            "path": "/src/main.rs",
            "kind": "file",
            "language": "rust"
        });

        let args: MappingSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "record");
        assert_eq!(args.path, Some("/src/main.rs".to_string()));
        assert_eq!(args.kind, Some("file".to_string()));
        assert_eq!(args.language, Some("rust".to_string()));
    }
}
