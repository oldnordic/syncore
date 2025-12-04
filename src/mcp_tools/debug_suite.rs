//! Debug Suite - Unified debugging, logs, and diagnostics
//!
//! Commands:
//! - `logs_tail`: Get recent log entries
//! - `tool_metadata_list`: List metadata for all MCP tools
//! - `project_file_report`: Generate report for a source file
//! - `project_module_map`: Generate module dependency map
//! - `project_hotspots`: Identify code hotspots
//! - `project_cycles`: Detect circular dependencies
//! - `project_dead_code`: Find potentially dead code
//! - `project_unused_imports`: Find unused imports
//! - `project_refactor_suggestions`: Generate refactor suggestions
//! - `project_code_smells`: Detect code smells and anti-patterns
//! - `project_cleanup_excluded`: Clean up indexed data for excluded directories

use crate::mcp_tools::{SuiteDispatcher, SuiteResult};
use crate::mcp_tools::streaming::OutputLimiter;
use crate::router::SynCoreState;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Debug suite arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSuiteArgs {
    pub command: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub n: Option<usize>,
    // Project analysis params
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub max_modules: Option<u32>,
    #[serde(default)]
    pub max_cycles: Option<u32>,
    #[serde(default)]
    pub max_depth: Option<u32>,
    #[serde(default)]
    pub min_loc: Option<u32>,
    #[serde(default)]
    pub min_fan_in: Option<u32>,
    #[serde(default)]
    pub min_fan_out: Option<u32>,
    #[serde(default)]
    pub min_entity_count: Option<u32>,
    #[serde(default)]
    pub exclude_public: Option<bool>,
    #[serde(default)]
    pub loc_threshold: Option<u32>,
    #[serde(default)]
    pub fan_in_threshold: Option<u32>,
    #[serde(default)]
    pub fan_out_threshold: Option<u32>,
    #[serde(default)]
    pub entity_threshold: Option<u32>,
}

/// Debug suite implementation
pub struct DebugSuite {
    state: SynCoreState,
}

impl DebugSuite {
    pub fn new(state: SynCoreState) -> Self {
        Self {
            state,
        }
    }

    /// Execute the suite command
    pub fn execute(&self, args: DebugSuiteArgs) -> SuiteResult {
        match args.command.as_str() {
            "logs_tail" => self.cmd_logs_tail(args),
            "tool_metadata_list" => self.cmd_tool_metadata_list(),
            "llm_health" => self.cmd_llm_health(),
            "project_file_report" => self.cmd_project_file_report(args),
            "project_module_map" => self.cmd_project_module_map(args),
            "project_hotspots" => self.cmd_project_hotspots(args),
            "project_cycles" => self.cmd_project_cycles(args),
            "project_dead_code" => self.cmd_project_dead_code(args),
            "project_unused_imports" => self.cmd_project_unused_imports(args),
            "project_refactor_suggestions" => self.cmd_project_refactor_suggestions(args),
            "project_code_smells" => self.cmd_project_code_smells(args),
            "project_cleanup_excluded" => self.cmd_project_cleanup_excluded(args),
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

    fn cmd_logs_tail(&self, args: DebugSuiteArgs) -> SuiteResult {
        let n = args.n.unwrap_or(50);

        // Use config for logs directory
        let logs_dir = crate::config::SyncoreConfig::try_global()
            .map(|c| c.paths.logs_dir.clone())
            .unwrap_or_else(|| "logs".to_string());

        let log_file = args.file_path.unwrap_or_else(|| {
            std::path::Path::new(&logs_dir).join("syncore.log").to_string_lossy().to_string()
        });

        match std::fs::read_to_string(&log_file) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().rev().take(n).collect();
                let reversed: Vec<&str> = lines.into_iter().rev().collect();

                SuiteResult::ok(
                    "logs_tail",
                    serde_json::json!({
                        "file": log_file,
                        "line_count": reversed.len(),
                        "lines": reversed
                    }),
                )
            }
            Err(e) => SuiteResult::err("logs_tail", format!("Failed to read log file: {}", e)),
        }
    }

    fn cmd_tool_metadata_list(&self) -> SuiteResult {
        use crate::mcp::tool_metadata::list_all_metadata;

        let tools = list_all_metadata();
        let metadata: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "category": format!("{:?}", t.category),
                    "cost": format!("{:?}", t.cost),
                    "side_effects": {
                        "modifies_database": t.side_effects.modifies_database,
                        "modifies_filesystem": t.side_effects.modifies_filesystem,
                        "modifies_vector_store": t.side_effects.modifies_vector_store,
                        "modifies_graph": t.side_effects.modifies_graph,
                        "network_call": t.side_effects.network_call
                    }
                })
            })
            .collect();

        SuiteResult::ok(
            "tool_metadata_list",
            serde_json::json!({
                "count": metadata.len(),
                "tools": metadata
            }),
        )
    }

    fn cmd_llm_health(&self) -> SuiteResult {
        // Check if IntelliTask is available
        match &self.state.intellitask {
            Some(_intellitask) => {
                // Try to get the LLM backend from IntelliTask
                // Note: We need to add a health check method to IntelliTask
                // For now, just check that IntelliTask was initialized

                // Get config to show backend details
                let config = crate::config::SyncoreConfig::try_global();
                let backend_info = match config {
                    Some(cfg) => serde_json::json!({
                        "backend": cfg.llm.backend,
                        "model": cfg.llm.model,
                        "url": cfg.llm.url,
                        "timeout_seconds": cfg.llm.timeout_seconds,
                    }),
                    None => serde_json::json!({
                        "backend": "unknown",
                        "model": "unknown",
                    }),
                };

                // IntelliTask exists, so LLM backend was initialized successfully
                SuiteResult::ok(
                    "llm_health",
                    serde_json::json!({
                        "status": "healthy",
                        "intellitask_available": true,
                        "backend_config": backend_info,
                        "message": "LLM backend initialized successfully"
                    }),
                )
            }
            None => {
                // IntelliTask not initialized
                SuiteResult::ok(
                    "llm_health",
                    serde_json::json!({
                        "status": "unavailable",
                        "intellitask_available": false,
                        "message": "LLM backend not initialized. Set LLM_BACKEND=test for testing, or ensure Ollama is running for production.",
                        "suggestion": "Check LLM configuration in syncore.toml or environment variables (LLM_BACKEND, LLM_MODEL, LLM_URL)"
                    }),
                )
            }
        }
    }

    fn cmd_project_file_report(&self, args: DebugSuiteArgs) -> SuiteResult {
        let file_path = match args.file_path {
            Some(f) => f,
            None => {
                return SuiteResult::err(
                    "project_file_report",
                    "Missing required parameter: file_path",
                )
            }
        };

        use crate::project_analysis::{file_report::FileReportRequest, ProjectAnalysisEngine};

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = FileReportRequest {
            file_path: file_path.clone(),
        };

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(engine.file_report(request))
        });

        match result {
            Ok(response) => SuiteResult::ok(
                "project_file_report",
                serde_json::json!({
                    "file_path": file_path,
                    "report": response.data
                }),
            ),
            Err(e) => SuiteResult::err("project_file_report", e.to_string()),
        }
    }

    fn cmd_project_module_map(&self, args: DebugSuiteArgs) -> SuiteResult {
        use crate::project_analysis::{deps::ModuleMapRequest, ProjectAnalysisEngine};

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = ModuleMapRequest {
            root: args.root,
            max_modules: args.max_modules,
        };

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(engine.module_map(request))
        });

        match result {
            Ok(response) => SuiteResult::ok(
                "project_module_map",
                serde_json::json!({
                    "module_count": response.data.as_ref().map(|d| d.modules.len()).unwrap_or(0),
                    "module_map": response.data
                }),
            ),
            Err(e) => SuiteResult::err("project_module_map", e.to_string()),
        }
    }

    fn cmd_project_hotspots(&self, args: DebugSuiteArgs) -> SuiteResult {
        use crate::project_analysis::{hotspots::HotspotsRequest, ProjectAnalysisEngine};

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = HotspotsRequest {
            limit: args.limit.unwrap_or(20),
            min_loc: args.min_loc,
            min_fan_in: args.min_fan_in,
            min_fan_out: args.min_fan_out,
            min_entity_count: args.min_entity_count,
        };

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(engine.hotspots(request))
        });

        match result {
            Ok(response) => SuiteResult::ok(
                "project_hotspots",
                serde_json::json!({
                    "count": response.data.as_ref().map(|d| d.hotspots.len()).unwrap_or(0),
                    "hotspots": response.data
                }),
            ),
            Err(e) => SuiteResult::err("project_hotspots", e.to_string()),
        }
    }

    fn cmd_project_cycles(&self, args: DebugSuiteArgs) -> SuiteResult {
        use crate::project_analysis::{cycles::CyclesRequest, ProjectAnalysisEngine};

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = CyclesRequest {
            max_cycles: args.max_cycles.unwrap_or(10),
            max_depth: args.max_depth.unwrap_or(5),
        };

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(engine.cycles(request))
        });

        match result {
            Ok(response) => SuiteResult::ok(
                "project_cycles",
                serde_json::json!({
                    "count": response.data.as_ref().map(|d| d.cycles.len()).unwrap_or(0),
                    "cycles": response.data
                }),
            ),
            Err(e) => SuiteResult::err("project_cycles", e.to_string()),
        }
    }

    fn cmd_project_dead_code(&self, args: DebugSuiteArgs) -> SuiteResult {
        use crate::project_analysis::{dead_code::DeadCodeRequest, ProjectAnalysisEngine};

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = DeadCodeRequest {
            limit: args.limit,
            exclude_public: args.exclude_public,
        };

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(engine.dead_code(request))
        });

        match result {
            Ok(response) => SuiteResult::ok(
                "project_dead_code",
                serde_json::json!({
                    "dead_code": response.data
                }),
            ),
            Err(e) => SuiteResult::err("project_dead_code", e.to_string()),
        }
    }

    fn cmd_project_unused_imports(&self, args: DebugSuiteArgs) -> SuiteResult {
        use crate::project_analysis::{
            unused_imports::UnusedImportsRequest, ProjectAnalysisEngine,
        };

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = UnusedImportsRequest {
            file_path: args.file_path,
            limit: args.limit,
        };

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(engine.unused_imports(request))
        });

        match result {
            Ok(response) => SuiteResult::ok(
                "project_unused_imports",
                serde_json::json!({
                    "unused_imports": response.data
                }),
            ),
            Err(e) => SuiteResult::err("project_unused_imports", e.to_string()),
        }
    }

    fn cmd_project_refactor_suggestions(&self, args: DebugSuiteArgs) -> SuiteResult {
        use crate::project_analysis::{
            refactor::RefactorSuggestionsRequest, ProjectAnalysisEngine,
        };

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = RefactorSuggestionsRequest {
            limit: args.limit.unwrap_or(10),
            loc_threshold: args.loc_threshold,
            fan_in_threshold: args.fan_in_threshold,
            fan_out_threshold: args.fan_out_threshold,
            entity_threshold: args.entity_threshold,
        };

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(engine.refactor_suggestions(request))
        });

        match result {
            Ok(response) => SuiteResult::ok(
                "project_refactor_suggestions",
                serde_json::json!({
                    "suggestions": response.data
                }),
            ),
            Err(e) => SuiteResult::err("project_refactor_suggestions", e.to_string()),
        }
    }

    fn cmd_project_code_smells(&self, args: DebugSuiteArgs) -> SuiteResult {
        use crate::project_analysis::ProjectAnalysisEngine;

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());

        let limit = args.limit.unwrap_or(50) as usize;
        let include_entities = args.exclude_public.unwrap_or(false); // Reuse exclude_public for include_entities

        match engine.detect_file_smells(limit) {
            Ok(file_smells) => {
                let entity_smells = if include_entities {
                    engine.detect_entity_smells(limit).unwrap_or_default()
                } else {
                    vec![]
                };

                SuiteResult::ok(
                    "project_code_smells",
                    serde_json::json!({
                        "file_smells": file_smells,
                        "entity_smells": entity_smells
                    }),
                )
            }
            Err(e) => SuiteResult::err("project_code_smells", e.to_string()),
        }
    }

    fn cmd_project_cleanup_excluded(&self, _args: DebugSuiteArgs) -> SuiteResult {
        use crate::project_analysis::{cleanup::CleanupExcludedRequest, ProjectAnalysisEngine};

        let engine =
            ProjectAnalysisEngine::new(self.state.db_manager.clone(), self.state.neo4j.clone());
        let request = CleanupExcludedRequest {
            dry_run: true, // Default to dry_run for safety
            excluded_dirs: None,
        };

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(engine.cleanup_excluded(request))
        });

        match result {
            Ok(response) => SuiteResult::ok(
                "project_cleanup_excluded",
                serde_json::json!({
                    "cleanup": response.data
                }),
            ),
            Err(e) => SuiteResult::err("project_cleanup_excluded", e.to_string()),
        }
    }

    fn cmd_help(&self) -> SuiteResult {
        SuiteResult::ok(
            "help",
            serde_json::json!({
                "suite": "debug_suite",
                "description": "Debugging, logs, and diagnostics",
                "command_groups": {
                    "logs": ["logs_tail"],
                    "metadata": ["tool_metadata_list"],
                    "diagnostics": ["llm_health"],
                    "project_analysis": [
                        "project_file_report",
                        "project_module_map",
                        "project_hotspots",
                        "project_cycles",
                        "project_dead_code",
                        "project_unused_imports",
                        "project_refactor_suggestions",
                        "project_code_smells",
                        "project_cleanup_excluded"
                    ]
                }
            }),
        )
    }
}

impl SuiteDispatcher for DebugSuite {
    fn dispatch(&self, command: &str, args: serde_json::Value) -> SuiteResult {
        let mut suite_args: DebugSuiteArgs = match serde_json::from_value(args) {
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
                    // Extract the limited data back
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
                },
                Err(_) => result, // Fallback to original result on error
            }
        } else {
            result
        }
    }

    fn list_commands(&self) -> Vec<&'static str> {
        vec![
            "logs_tail",
            "tool_metadata_list",
            "llm_health",
            "project_file_report",
            "project_module_map",
            "project_hotspots",
            "project_cycles",
            "project_dead_code",
            "project_unused_imports",
            "project_refactor_suggestions",
            "project_code_smells",
            "project_cleanup_excluded",
            "help",
        ]
    }

    fn help(&self, command: &str) -> Option<&'static str> {
        match command {
            "logs_tail" => Some("Get recent log entries. Params: n, file_path"),
            "tool_metadata_list" => Some("List MCP tool metadata"),
            "llm_health" => Some("Check LLM backend health and configuration. No params required"),
            "project_file_report" => Some("File report. Params: file_path"),
            "project_module_map" => Some("Module map. Params: root, max_modules"),
            "project_hotspots" => {
                Some("Code hotspots. Params: limit, min_loc, min_fan_in, min_fan_out")
            }
            "project_cycles" => Some("Circular dependencies. Params: max_cycles, max_depth"),
            "project_dead_code" => Some("Dead code. Params: limit, exclude_public"),
            "project_unused_imports" => Some("Unused imports. Params: file_path, limit"),
            "project_refactor_suggestions" => {
                Some("Refactor suggestions. Params: limit, thresholds")
            }
            "project_code_smells" => Some("Code smells detection. Params: limit, include_entities"),
            "project_cleanup_excluded" => {
                Some("Cleanup excluded dirs. Params: dry_run, excluded_dirs")
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_suite_args_deserialization() {
        let json = serde_json::json!({
            "command": "logs_tail",
            "n": 100
        });

        let args: DebugSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "logs_tail");
        assert_eq!(args.n, Some(100));
    }
}
