//! Code Relationship MCP Tools
//!
//! Provides MCP tool handlers for code relationship indexing, querying, and similarity search.
//! Integrates CodeDependencyExtractor (Tree-sitter) and CodeRelationshipStore (SQLite + Neo4j + FAISS).

use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use std::pin::Pin;
use std::future::Future;

use crate::portfolio::{CodeDependencyExtractor, CodeRelationshipStore};

type EventCallback = Arc<dyn Fn(String, Value) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Code Relationship Tools for MCP integration
pub struct CodeRelationshipTools {
    extractor: Arc<Mutex<CodeDependencyExtractor>>,
    store: Arc<CodeRelationshipStore>,
    event_callbacks: Arc<RwLock<Vec<EventCallback>>>,
}

impl CodeRelationshipTools {
    /// Create new CodeRelationshipTools with custom database path
    pub async fn new(db_path: &Path) -> Result<Self> {
        let extractor = CodeDependencyExtractor::new();
        let store = CodeRelationshipStore::new(db_path).await?;

        Ok(Self {
            extractor: Arc::new(Mutex::new(extractor)),
            store: Arc::new(store),
            event_callbacks: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Subscribe to relationship events
    pub async fn subscribe_to_events<F>(&self, callback: F)
    where
        F: Fn(&str, &Value) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static,
    {
        let wrapped = Arc::new(move |name: String, data: Value| {
            callback(&name, &data)
        });
        self.event_callbacks.write().await.push(wrapped);
    }

    /// Broadcast an event to all subscribers
    async fn broadcast_event(&self, event_name: &str, data: Value) {
        let callbacks = self.event_callbacks.read().await;
        for callback in callbacks.iter() {
            callback(event_name.to_string(), data.clone()).await;
        }
    }

    /// Index a Rust source file, extracting imports, calls, impls, and functions
    pub async fn handle_code_relationship_index(&self, params: Value) -> Result<Value> {
        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing file_path parameter"))?;

        let path = PathBuf::from(file_path);
        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", file_path));
        }

        let source = std::fs::read_to_string(&path)?;

        // Extract dependencies using Tree-sitter
        let deps = {
            let mut extractor = self.extractor.lock().await;
            extractor.extract_from_source(&source, file_path)?
        };

        // Store imports
        for import in &deps.imports {
            self.store.store_import(file_path, import).await?;
        }

        // Store function calls
        for (caller, callee) in &deps.calls {
            self.store.store_call(file_path, caller, callee).await?;
        }

        // Store trait implementations
        for (struct_name, trait_name) in &deps.implements {
            self.store.store_impl(file_path, struct_name, trait_name).await?;
        }

        // Index function bodies for semantic search
        // We need to re-parse to get function bodies
        self.index_function_bodies(file_path, &source).await?;

        let result = json!({
            "success": true,
            "file": file_path,
            "imports_found": deps.imports.len(),
            "functions_found": deps.function_defs.len(),
            "structs_found": deps.struct_defs.len(),
            "calls_found": deps.calls.len(),
            "trait_impls_found": deps.implements.len(),
        });

        // Broadcast event
        self.broadcast_event("relationship_indexed", json!({
            "file": file_path,
            "imports": deps.imports.len(),
            "functions": deps.function_defs.len(),
        })).await;

        Ok(result)
    }

    /// Index function bodies for semantic similarity search
    async fn index_function_bodies(&self, file_path: &str, source: &str) -> Result<()> {
        use tree_sitter::{Parser, Query, QueryCursor};

        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language())
            .map_err(|e| anyhow!("Failed to set language: {}", e))?;

        let tree = parser.parse(source, None)
            .ok_or_else(|| anyhow!("Failed to parse source"))?;

        let query_str = r#"
            (function_item
                name: (identifier) @fn_name
                body: (block) @fn_body)
        "#;

        let query = Query::new(tree_sitter_rust::language(), query_str)
            .map_err(|e| anyhow!("Query error: {}", e))?;
        let mut cursor = QueryCursor::new();
        let source_bytes = source.as_bytes();
        let matches: Vec<_> = cursor.matches(&query, tree.root_node(), source_bytes).collect();

        for m in matches {
            let mut fn_name = String::new();
            let mut fn_body = String::new();

            for capture in m.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                let text = capture.node.utf8_text(source_bytes)
                    .map_err(|e| anyhow!("UTF8 error: {}", e))?;

                if capture_name == "fn_name" {
                    fn_name = text.to_string();
                } else if capture_name == "fn_body" {
                    fn_body = text.to_string();
                }
            }

            if !fn_name.is_empty() && !fn_body.is_empty() {
                self.store.index_function(file_path, &fn_name, &fn_body).await?;
            }
        }

        Ok(())
    }

    /// Query code relationships (imports, calls, implementors)
    pub async fn handle_code_relationship_query(&self, params: Value) -> Result<Value> {
        let query_type = params["query_type"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing query_type parameter"))?;

        match query_type {
            "imports" => {
                let file = params["file"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing file parameter for imports query"))?;

                let imports = self.store.get_imports(file).await?;
                Ok(json!({ "imports": imports }))
            }
            "calls" => {
                let function = params["function"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing function parameter for calls query"))?;

                // Get all calls from this function across all files
                let calls = self.get_function_calls(function).await?;
                Ok(json!({ "calls": calls }))
            }
            "implementors" => {
                let trait_name = params["trait_name"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing trait_name parameter for implementors query"))?;

                let implementors = self.get_trait_implementors(trait_name).await?;
                Ok(json!({ "implementors": implementors }))
            }
            _ => Err(anyhow!("Unknown query_type: {}", query_type)),
        }
    }

    /// Get all calls from a function (searching across all files)
    async fn get_function_calls(&self, function: &str) -> Result<Vec<String>> {
        // We need to query the database for all files that have this function as a caller
        // For now, we'll use a direct SQL query approach
        let db = self.store.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT DISTINCT callee FROM code_calls WHERE caller = ?1"
        )?;
        let calls = stmt.query_map([function], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(calls)
    }

    /// Get all structs that implement a given trait
    async fn get_trait_implementors(&self, trait_name: &str) -> Result<Vec<String>> {
        let db = self.store.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT DISTINCT struct_name FROM code_impls WHERE trait_name = ?1"
        )?;
        let implementors = stmt.query_map([trait_name], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(implementors)
    }

    /// Search for semantically similar functions
    pub async fn handle_code_similarity_search(&self, params: Value) -> Result<Value> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing query parameter"))?;

        let limit = params["limit"]
            .as_u64()
            .unwrap_or(10) as usize;

        let similar = self.store.find_similar_functions(query, limit).await?;

        let matches: Vec<Value> = similar.into_iter()
            .map(|(file, function, score)| {
                json!({
                    "file": file,
                    "function": function,
                    "score": score,
                })
            })
            .collect();

        Ok(json!({ "matches": matches }))
    }
}

