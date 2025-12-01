//! Code Suite - Unified code indexing, search, and analysis
//!
//! Commands:
//! - `index`: Index a single code file
//! - `index_directory`: Index all files in a directory matching a pattern
//! - `search`: Semantic code search
//! - `parse`: Parse code structure with tree-sitter
//! - `grep`: Search code patterns using ripgrep
//! - `doc_index`: Index documents from a directory
//! - `doc_search`: Semantic document search
//! - `explain`: Explain a function with callers/callees
//! - `sync_neo4j`: Sync code entities and relationships to Neo4j
//! - `enrich_temporal`: Enrich entities with git history and filesystem metadata
//! - `fusion_query`: Execute tri-mode fusion RAG query (Simple/Attention/Reasoning)
//! - `help`: Show available commands

use crate::mcp_tools::{SuiteDispatcher, SuiteResult};
use crate::query::{PipelineExecutor, QueryConstraints, QueryPlanner};
use crate::router::SynCoreState;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Code suite arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSuiteArgs {
    pub command: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub directory: Option<String>,
    #[serde(default)]
    pub context_lines: Option<usize>,
    #[serde(default)]
    pub function_name: Option<String>,
    // Fields for fusion_query
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub mode_hint: Option<String>,
    #[serde(default)]
    pub top_k: Option<usize>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub project_label: Option<String>,
    #[serde(default)]
    pub local_root: Option<String>,
    // Fields for enrich_temporal
    #[serde(default)]
    pub only_missing: Option<bool>,
}

/// Code suite implementation
pub struct CodeSuite {
    state: SynCoreState,
}

impl CodeSuite {
    pub fn new(state: SynCoreState) -> Self {
        Self { state }
    }

    /// Execute the suite command
    pub fn execute(&self, args: CodeSuiteArgs) -> SuiteResult {
        match args.command.as_str() {
            "index" => self.cmd_index(args),
            "index_directory" => self.cmd_index_directory(args),
            "search" => self.cmd_search(args),
            "parse" => self.cmd_parse(args),
            "grep" => self.cmd_grep(args),
            "doc_index" => self.cmd_doc_index(args),
            "doc_search" => self.cmd_doc_search(args),
            "explain" => self.cmd_explain(args),
            "sync_neo4j" => self.cmd_sync_neo4j(args),
            "enrich_temporal" => self.cmd_enrich_temporal(args),
            "fusion_query" => self.cmd_fusion_query(args),
            "help" => self.cmd_help(),
            _ => SuiteResult::err(
                &args.command,
                format!(
                    "Unknown command '{}'. Use 'help' for available commands.",
                    args.command
                ),
            ),
        }
    }

    fn cmd_index(&self, args: CodeSuiteArgs) -> SuiteResult {
        let file_path = match args.file_path {
            Some(f) => f,
            None => return SuiteResult::err("index", "Missing required parameter: file_path"),
        };

        use crate::code_graph::CodeGraph;

        let db_conn = self.state.db_manager.code_graph_conn();
        match CodeGraph::with_connection(db_conn, self.state.code_store.clone()) {
            Ok(mut code_graph) => match code_graph.index_file(Path::new(&file_path)) {
                Ok(entity_count) => SuiteResult::ok(
                    "index",
                    serde_json::json!({
                        "indexed": true,
                        "file_path": file_path,
                        "entity_count": entity_count
                    }),
                ),
                Err(e) => SuiteResult::err("index", e.to_string()),
            },
            Err(e) => SuiteResult::err("index", format!("Failed to initialize CodeGraph: {}", e)),
        }
    }

    fn cmd_search(&self, args: CodeSuiteArgs) -> SuiteResult {
        let query = match args.query {
            Some(q) => q,
            None => return SuiteResult::err("search", "Missing required parameter: query"),
        };

        let limit = args.limit.unwrap_or(10);

        use crate::code_graph::CodeGraph;

        let db_conn = self.state.db_manager.code_graph_conn();
        match CodeGraph::with_connection(db_conn, self.state.code_store.clone()) {
            Ok(code_graph) => match code_graph.search_code(&query, limit) {
                Ok(results) => {
                    let hits: Vec<serde_json::Value> = results
                        .iter()
                        .map(|m| {
                            serde_json::json!({
                                "file_path": m.entity.file_path,
                                "name": m.entity.name,
                                "entity_type": m.entity.entity_type,
                                "score": m.score,
                                "line_start": m.entity.line_start,
                                "line_end": m.entity.line_end
                            })
                        })
                        .collect();

                    SuiteResult::ok(
                        "search",
                        serde_json::json!({
                            "query": query,
                            "count": hits.len(),
                            "results": hits
                        }),
                    )
                }
                Err(e) => SuiteResult::err("search", e.to_string()),
            },
            Err(e) => SuiteResult::err("search", format!("Failed to initialize CodeGraph: {}", e)),
        }
    }

    fn cmd_parse(&self, args: CodeSuiteArgs) -> SuiteResult {
        let file_path = match args.file_path {
            Some(f) => f,
            None => return SuiteResult::err("parse", "Missing required parameter: file_path"),
        };

        use crate::parser::Parser;

        match Parser::new() {
            Ok(parser) => match parser.parse_file(Path::new(&file_path)) {
                Ok(structure) => SuiteResult::ok(
                    "parse",
                    serde_json::json!({
                        "file_path": file_path,
                        "language": structure.language,
                        "functions": structure.functions.len(),
                        "classes": structure.classes.len(),
                        "imports": structure.imports.len(),
                        "details": {
                            "functions": structure.functions,
                            "classes": structure.classes,
                            "imports": structure.imports
                        }
                    }),
                ),
                Err(e) => SuiteResult::err("parse", e.to_string()),
            },
            Err(e) => SuiteResult::err("parse", format!("Failed to initialize parser: {}", e)),
        }
    }

    fn cmd_index_directory(&self, args: CodeSuiteArgs) -> SuiteResult {
        let directory = match args.directory {
            Some(d) => d,
            None => {
                return SuiteResult::err("index_directory", "Missing required parameter: directory")
            }
        };
        let pattern = args.pattern.unwrap_or_else(|| "**/*.rs".to_string());

        use crate::code_graph::CodeGraph;
        use glob::glob;

        let db_conn = self.state.db_manager.code_graph_conn();
        match CodeGraph::with_connection(db_conn, self.state.code_store.clone()) {
            Ok(mut code_graph) => {
                let full_pattern = format!("{}/{}", directory, pattern);
                let mut indexed_count = 0;
                let mut total_entities = 0;
                let mut errors = Vec::new();

                match glob(&full_pattern) {
                    Ok(paths) => {
                        // APEX 2.15: Acquire reindex mutex to serialize DELETE+INSERT operations
                        // This prevents UNIQUE constraint collisions with concurrent LiveIndexer updates
                        let _lock = self
                            .state
                            .reindex_mutex
                            .lock()
                            .expect("reindex mutex poisoned");

                        // BUGFIX: Load config to get excluded directories (same as bootstrap.rs)
                        // Use default config if load fails (will use default_excluded_dirs)
                        let config = crate::config::SyncoreConfig::default();

                        for entry in paths.flatten() {
                            // BUGFIX: Skip excluded directories (target/, node_modules/, etc.)
                            let entry_str = entry.to_string_lossy();
                            let should_skip = config
                                .indexing
                                .excluded_dirs
                                .iter()
                                .any(|excluded| entry_str.contains(excluded));

                            if should_skip {
                                continue; // Skip this file
                            }

                            match code_graph.index_file(&entry) {
                                Ok(count) => {
                                    indexed_count += 1;
                                    total_entities += count;
                                }
                                Err(e) => errors.push(format!("{}: {}", entry.display(), e)),
                            }
                        }
                        // Mutex automatically released when _lock goes out of scope

                        SuiteResult::ok(
                            "index_directory",
                            serde_json::json!({
                                "directory": directory,
                                "pattern": pattern,
                                "files_indexed": indexed_count,
                                "total_entities": total_entities,
                                "errors": errors
                            }),
                        )
                    }
                    Err(e) => {
                        SuiteResult::err("index_directory", format!("Invalid glob pattern: {}", e))
                    }
                }
            }
            Err(e) => SuiteResult::err(
                "index_directory",
                format!("Failed to initialize CodeGraph: {}", e),
            ),
        }
    }

    fn cmd_grep(&self, args: CodeSuiteArgs) -> SuiteResult {
        let pattern = match args.pattern {
            Some(p) => p,
            None => return SuiteResult::err("grep", "Missing required parameter: pattern"),
        };

        use crate::parser::RipgrepSearcher;

        let path = args.file_path.unwrap_or_else(|| ".".to_string());
        let context_lines = args.context_lines.unwrap_or(2);

        match RipgrepSearcher::search(&pattern, Path::new(&path), context_lines) {
            Ok(results) => {
                let matches: Vec<serde_json::Value> = results
                    .iter()
                    .map(|m| {
                        serde_json::json!({
                            "file": m.file_path,
                            "line": m.line_number,
                            "content": m.line_content,
                            "match_text": m.match_text
                        })
                    })
                    .collect();

                SuiteResult::ok(
                    "grep",
                    serde_json::json!({
                        "pattern": pattern,
                        "path": path,
                        "context_lines": context_lines,
                        "match_count": matches.len(),
                        "matches": matches
                    }),
                )
            }
            Err(e) => SuiteResult::err("grep", e.to_string()),
        }
    }

    fn cmd_doc_index(&self, args: CodeSuiteArgs) -> SuiteResult {
        let directory = match args.directory {
            Some(d) => d,
            None => return SuiteResult::err("doc_index", "Missing required parameter: directory"),
        };

        use crate::document_indexer::DocumentIndexer;

        let indexer = DocumentIndexer::with_defaults();
        let dir_path = Path::new(&directory);

        match indexer.index_directory(dir_path) {
            Ok(chunk_count) => SuiteResult::ok(
                "doc_index",
                serde_json::json!({
                    "indexed": true,
                    "chunk_count": chunk_count,
                    "directory": directory
                }),
            ),
            Err(e) => SuiteResult::err("doc_index", e.to_string()),
        }
    }

    fn cmd_doc_search(&self, args: CodeSuiteArgs) -> SuiteResult {
        let query = match args.query {
            Some(q) => q,
            None => return SuiteResult::err("doc_search", "Missing required parameter: query"),
        };

        let limit = args.limit.unwrap_or(5);

        use crate::global_store::GlobalVectorStore;

        match GlobalVectorStore::new() {
            Ok(store) => match store.search(&query, limit, "documents") {
                Ok(hits) => {
                    let results: Vec<serde_json::Value> = hits
                        .iter()
                        .map(|hit| {
                            serde_json::json!({
                                "id": hit.id,
                                "score": hit.score,
                                "text": hit.text
                            })
                        })
                        .collect();

                    SuiteResult::ok(
                        "doc_search",
                        serde_json::json!({
                            "query": query,
                            "count": results.len(),
                            "results": results
                        }),
                    )
                }
                Err(e) => SuiteResult::err("doc_search", e.to_string()),
            },
            Err(e) => SuiteResult::err("doc_search", e.to_string()),
        }
    }

    fn cmd_explain(&self, args: CodeSuiteArgs) -> SuiteResult {
        let function_name = match args.function_name {
            Some(f) => f,
            None => {
                return SuiteResult::err("explain", "Missing required parameter: function_name")
            }
        };

        let file_path = match args.file_path {
            Some(f) => f,
            None => return SuiteResult::err("explain", "Missing required parameter: file_path"),
        };

        use crate::code_graph::explain::FunctionExplainer;
        use std::fs;

        // Read the source file
        let code = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => return SuiteResult::err("explain", format!("Failed to read file: {}", e)),
        };

        // Note: For simplicity, we're not fetching callers/callees from code graph
        // In a full implementation, this would query the code graph
        let callers = Vec::new();
        let callees = Vec::new();

        // Use FunctionExplainer to analyze the function
        let explainer = FunctionExplainer::new();
        match explainer.explain(&function_name, &file_path, &code, callers, callees) {
            Some(response) => SuiteResult::ok("explain", serde_json::to_value(response).unwrap()),
            None => SuiteResult::err(
                "explain",
                format!("Function '{}' not found in '{}'", function_name, file_path),
            ),
        }
    }

    fn cmd_fusion_query(&self, args: CodeSuiteArgs) -> SuiteResult {
        let query = match args.query {
            Some(q) => q,
            None => return SuiteResult::err("fusion_query", "Missing required parameter: query"),
        };

        // Create query planner
        let planner = QueryPlanner::new();

        // Build constraints from args
        let constraints = QueryConstraints {
            scope: args.scope.unwrap_or_else(|| "project".to_string()),
            max_results: args.top_k,
            project_label: args.project_label.clone(),
            local_root: args.local_root.clone(),
            graph_required: self.state.neo4j.is_some(),
            allow_hopgraph: true,
            allow_raggraph: true,
            allow_vector: true,
        };

        // Plan the query
        let plan = match planner.plan_with_constraints(&query, constraints) {
            Ok(plan) => plan,
            Err(e) => {
                return SuiteResult::err("fusion_query", format!("Query planning failed: {}", e))
            }
        };

        // Execute pipeline
        let executor = PipelineExecutor::new();
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { executor.execute(&plan, &query).await })
        });

        match result {
            Ok(fusion_output) => SuiteResult::ok(
                "fusion_query",
                serde_json::json!({
                    "entities": fusion_output.entities,
                    "metadata": fusion_output.metadata,
                    "scoring_weights": fusion_output.scoring_weights,
                    "plan": {
                        "steps": plan.steps,
                        "constraints": plan.constraints,
                        "planning_metadata": plan.metadata
                    },
                    "entity_count": fusion_output.entities.len()
                }),
            ),
            Err(e) => SuiteResult::err("fusion_query", format!("Pipeline execution failed: {}", e)),
        }
    }

    fn cmd_sync_neo4j(&self, args: CodeSuiteArgs) -> SuiteResult {
        use crate::code_graph::neo4j_sync;

        // Check if we have Neo4j available
        let neo4j = match &self.state.neo4j {
            Some(n) => n,
            None => return SuiteResult::err("sync_neo4j", "Neo4j connection required"),
        };

        // Get SQLite connection
        let code_graph_conn = self.state.db_manager.code_graph_conn();

        // Convert usize limit to u64 if provided
        let limit = args.limit.map(|l| l as u64);

        // STEP 1: Sync entities FIRST (required for edges to reference)
        let entity_summary = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                neo4j_sync::sync_entities_to_neo4j(
                    &code_graph_conn,
                    &**neo4j,
                    args.namespace.as_deref(),
                    limit,
                )
                .await
            })
        });

        let entity_summary = match entity_summary {
            Ok(summary) => summary,
            Err(e) => return SuiteResult::err("sync_neo4j", format!("Entity sync failed: {}", e)),
        };

        // STEP 2: Sync edges (relationships between entities)
        let edge_summary = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                neo4j_sync::sync_relationships_to_neo4j(
                    &code_graph_conn,
                    &**neo4j,
                    args.namespace.as_deref(),
                    limit,
                )
                .await
            })
        });

        match edge_summary {
            Ok(mut summary) => {
                // Combine entity summary into edge summary
                summary.entities_processed = entity_summary.entities_processed;
                summary.entities_created = entity_summary.entities_created;
                summary.entities_skipped = entity_summary.entities_skipped;

                SuiteResult::ok("sync_neo4j", serde_json::to_value(summary).unwrap())
            }
            Err(e) => SuiteResult::err("sync_neo4j", format!("Relationship sync failed: {}", e)),
        }
    }

    fn cmd_enrich_temporal(&self, args: CodeSuiteArgs) -> SuiteResult {
        use crate::code_graph::temporal_extractor::extract_temporal_metadata;

        // Get SQLite connection
        let conn = self.state.db_manager.code_graph_conn();

        let only_missing = args.only_missing.unwrap_or(true);

        // Build query to get entities
        let query = if only_missing {
            "SELECT id, file_path FROM code_entities WHERE created_at IS NULL LIMIT ?1"
        } else {
            "SELECT id, file_path FROM code_entities LIMIT ?1"
        };

        let limit = args.limit.unwrap_or(usize::MAX) as i64;

        // Get entities to enrich
        let entities: Vec<(i64, String)> = match conn.lock() {
            Ok(conn) => {
                let mut stmt = match conn.prepare(query) {
                    Ok(s) => s,
                    Err(e) => {
                        return SuiteResult::err(
                            "enrich_temporal",
                            format!("Failed to prepare query: {}", e),
                        )
                    }
                };

                let rows = match stmt.query_map([limit], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                }) {
                    Ok(r) => r,
                    Err(e) => {
                        return SuiteResult::err(
                            "enrich_temporal",
                            format!("Failed to query entities: {}", e),
                        )
                    }
                };

                match rows.collect::<Result<Vec<_>, _>>() {
                    Ok(v) => v,
                    Err(e) => {
                        return SuiteResult::err(
                            "enrich_temporal",
                            format!("Failed to collect entities: {}", e),
                        )
                    }
                }
            }
            Err(e) => {
                return SuiteResult::err(
                    "enrich_temporal",
                    format!("Failed to lock database: {}", e),
                )
            }
        };

        let total_entities = entities.len();
        let mut enriched_count = 0;
        let mut failed_count = 0;

        // Enrich each entity
        for (entity_id, file_path) in entities {
            match extract_temporal_metadata(&file_path) {
                Ok(metadata) => {
                    // Update entity with temporal metadata
                    if let Ok(conn) = conn.lock() {
                        match conn.execute(
                            "UPDATE code_entities SET created_at = ?1, last_modified_at = ?2, change_count = ?3, author_count = ?4 WHERE id = ?5",
                            rusqlite::params![
                                metadata.created_at,
                                metadata.last_modified_at,
                                metadata.change_count,
                                metadata.author_count,
                                entity_id
                            ],
                        ) {
                            Ok(_) => enriched_count += 1,
                            Err(_) => failed_count += 1,
                        }
                    }
                }
                Err(_) => failed_count += 1,
            }
        }

        SuiteResult::ok(
            "enrich_temporal",
            serde_json::json!({
                "total_entities": total_entities,
                "enriched_count": enriched_count,
                "failed_count": failed_count
            }),
        )
    }

    fn cmd_help(&self) -> SuiteResult {
        SuiteResult::ok(
            "help",
            serde_json::json!({
                "suite": "code_suite",
                "description": "Code indexing, search, parsing, and analysis",
                "commands": {
                    "index": {
                        "description": "Index a single code file",
                        "params": ["file_path (required)"]
                    },
                    "index_directory": {
                        "description": "Index all files matching pattern in directory",
                        "params": ["directory (required)", "pattern (optional, default: **/*.rs)"]
                    },
                    "search": {
                        "description": "Semantic code search",
                        "params": ["query (required)", "limit (optional, default: 10)"]
                    },
                    "parse": {
                        "description": "Parse code structure with tree-sitter",
                        "params": ["file_path (required)"]
                    },
                    "grep": {
                        "description": "Search code patterns using ripgrep",
                        "params": ["pattern (required)", "file_path (optional)", "context_lines (optional)"]
                    },
                    "doc_index": {
                        "description": "Index documents from a directory",
                        "params": ["directory (required)"]
                    },
                    "doc_search": {
                        "description": "Semantic document search",
                        "params": ["query (required)", "limit (optional, default: 5)"]
                    },
                    "explain": {
                        "description": "Explain a function with signature, docstring, and metrics",
                        "params": ["function_name (required)", "file_path (required)"]
                    },
                    "sync_neo4j": {
                        "description": "Sync code entities and relationships from SQLite to Neo4j",
                        "params": ["namespace (optional)", "limit (optional)"]
                    },
                    "enrich_temporal": {
                        "description": "Enrich code entities with git history and filesystem metadata",
                        "params": ["limit (optional)", "only_missing (optional, default: true)"]
                    },
                    "fusion_query": {
                        "description": "Execute tri-mode fusion RAG query (Simple/Attention/Reasoning)",
                        "params": ["query (required)", "namespace (optional)", "mode_hint (optional)", "top_k (optional)", "scope (optional)", "project_label (optional)", "local_root (optional)"]
                    }
                }
            }),
        )
    }
}

impl SuiteDispatcher for CodeSuite {
    fn dispatch(&self, command: &str, args: serde_json::Value) -> SuiteResult {
        let mut suite_args: CodeSuiteArgs = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return SuiteResult::err(command, format!("Invalid arguments: {}", e)),
        };
        suite_args.command = command.to_string();
        self.execute(suite_args)
    }

    fn list_commands(&self) -> Vec<&'static str> {
        vec![
            "index",
            "index_directory",
            "search",
            "parse",
            "grep",
            "doc_index",
            "doc_search",
            "explain",
            "sync_neo4j",
            "enrich_temporal",
            "fusion_query",
            "help",
        ]
    }

    fn help(&self, command: &str) -> Option<&'static str> {
        match command {
            "index" => Some("Index a single code file. Params: file_path"),
            "index_directory" => Some("Index directory. Params: directory, pattern"),
            "search" => Some("Semantic code search. Params: query, limit"),
            "parse" => Some("Parse code structure. Params: file_path"),
            "grep" => Some("Ripgrep search. Params: pattern, file_path, context_lines"),
            "doc_index" => Some("Index documents. Params: directory"),
            "doc_search" => Some("Semantic document search. Params: query, limit"),
            "explain" => Some("Explain function. Params: function_name, file_path"),
            "sync_neo4j" => Some("Sync entities to Neo4j. Params: namespace, limit"),
            "enrich_temporal" => Some("Enrich with git metadata. Params: limit, only_missing"),
            "fusion_query" => Some("Tri-mode RAG query. Params: query, namespace, mode_hint, top_k, scope, project_label, local_root"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_code_suite_args_deserialization() {
        let json = serde_json::json!({
            "command": "search",
            "query": "find functions",
            "limit": 5
        });

        let args: CodeSuiteArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.command, "search");
        assert_eq!(args.query, Some("find functions".to_string()));
        assert_eq!(args.limit, Some(5));
    }
}
