//! Vector Commands Module
//!
//! Handles execution of vector store operations.
//! Extracted from memory_suite.rs (lines 188-273).
//!
//! Commands:
//! - vector_insert: Insert text into vector store with namespace
//! - vector_search: Semantic search in vector store

use crate::mcp_tools::SuiteResult;
use super::{MemorySuite, MemorySuiteArgs};

/// Execute vector_insert command
pub fn cmd_vector_insert(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let text = match args.text {
        Some(t) => t,
        None => return SuiteResult::err("vector_insert", "Missing required parameter: text"),
    };

    if args.dry_run.unwrap_or(false) {
        return SuiteResult::ok(
            "vector_insert",
            serde_json::json!({
                "dry_run": true,
                "text_length": text.len()
            }),
        );
    }

    let namespace = args.namespace.as_deref().unwrap_or("default");

    // Use GENERAL domain store for memory/document operations
    match suite.state.general_store.lock() {
        Ok(mut store) => {
            // Generate a unique ID based on text hash
            let id = {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                text.hash(&mut hasher);
                (hasher.finish() & 0x7FFFFFFFFFFFFFFF) as i64
            };

            match store.insert_text(id, None, &text, namespace) {
                Ok(_) => SuiteResult::ok(
                    "vector_insert",
                    serde_json::json!({
                        "inserted": true,
                        "id": id,
                        "namespace": namespace
                    }),
                ),
                Err(e) => SuiteResult::err("vector_insert", e.to_string()),
            }
        }
        Err(e) => SuiteResult::err("vector_insert", format!("Lock error: {}", e)),
    }
}

/// Execute vector_search command
pub fn cmd_vector_search(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let query = match args.query {
        Some(q) => q,
        None => return SuiteResult::err("vector_search", "Missing required parameter: query"),
    };

    let limit = args.limit.unwrap_or(10);

    // Use GENERAL domain store for memory/document operations
    match suite.state.general_store.lock() {
        Ok(store) => {
            use crate::vector::SearchScope;
            match store.search(&query, limit, SearchScope::Global) {
                Ok(results) => {
                    let hits: Vec<serde_json::Value> = results
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
                        "vector_search",
                        serde_json::json!({
                            "query": query,
                            "count": hits.len(),
                            "results": hits
                        }),
                    )
                }
                Err(e) => SuiteResult::err("vector_search", e.to_string()),
            }
        }
        Err(e) => SuiteResult::err("vector_search", format!("Lock error: {}", e)),
    }
}
