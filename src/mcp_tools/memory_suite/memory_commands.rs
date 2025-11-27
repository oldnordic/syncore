//! Memory Commands Module
//!
//! Handles execution of memory storage and query commands.
//! Extracted from memory_suite.rs (lines 184-462).
//!
//! Commands:
//! - store: Store key-value pair with optional namespace
//! - query: Query value by key
//! - delete: Delete key-value pair
//! - list_keys: List all keys with optional limit
//! - memory_stats: Get memory statistics
//! - search_semantic: Semantic search in memory
//! - search_hybrid: Hybrid semantic+keyword search
//! - query_by_tags: Query entries by tags
//! - query_by_importance: Query entries by importance threshold
//! - query_recent: Get recent entries
//! - query_since: Get entries since timestamp
//! - consolidate_similar: Merge similar entries
//! - get_related_memories: Get related memories for key

use crate::mcp_tools::SuiteResult;
use super::{MemorySuite, MemorySuiteArgs};

/// Execute store command
pub fn cmd_store(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let key = match args.key {
        Some(k) => k,
        None => return SuiteResult::err("store", "Missing required parameter: key"),
    };
    let value = match args.value {
        Some(v) => v,
        None => return SuiteResult::err("store", "Missing required parameter: value"),
    };

    // APEX 2.0-M-FIX: Extract namespace parameter
    let namespace = args.namespace.as_deref();

    if args.dry_run.unwrap_or(false) {
        return SuiteResult::ok(
            "store",
            serde_json::json!({
                "dry_run": true,
                "would_store": { "key": key, "value": value, "namespace": namespace }
            }),
        );
    }

    // APEX 2.0-M-FIX: Use store_with_metadata() with namespace instead of store()
    let result = if let Some(ns) = namespace {
        // Explicit namespace provided
        suite.state.memory.store_with_metadata(&key, &value, ns, &[], 0.5)
    } else {
        // No namespace - use configured default via store()
        suite.state.memory.store(&key, &value).map(|_| 0)
    };

    match result {
        Ok(_) => SuiteResult::ok(
            "store",
            serde_json::json!({
                "stored": true,
                "key": key,
                "namespace": namespace.unwrap_or("default")
            }),
        ),
        Err(e) => SuiteResult::err("store", e.to_string()),
    }
}

/// Execute query command
pub fn cmd_query(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let key = match args.key {
        Some(k) => k,
        None => return SuiteResult::err("query", "Missing required parameter: key"),
    };

    match suite.state.memory.query(&key) {
        Ok(Some(value)) => SuiteResult::ok(
            "query",
            serde_json::json!({
                "found": true,
                "key": key,
                "value": value
            }),
        ),
        Ok(None) => SuiteResult::ok(
            "query",
            serde_json::json!({
                "found": false,
                "key": key
            }),
        ),
        Err(e) => SuiteResult::err("query", e.to_string()),
    }
}

/// Execute delete command
pub fn cmd_delete(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let key = match args.key {
        Some(k) => k,
        None => return SuiteResult::err("delete", "Missing required parameter: key"),
    };

    match suite.state.memory.delete(&key) {
        Ok(_) => SuiteResult::ok(
            "delete",
            serde_json::json!({
                "success": true,
                "key": key
            }),
        ),
        Err(e) => SuiteResult::err("delete", e.to_string()),
    }
}

/// Execute list_keys command
pub fn cmd_list_keys(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let limit = args.limit.map(|l| l as i64);

    match suite.state.memory.list_keys(limit) {
        Ok(keys) => SuiteResult::ok(
            "list_keys",
            serde_json::json!({
                "keys": keys,
                "count": keys.len()
            }),
        ),
        Err(e) => SuiteResult::err("list_keys", e.to_string()),
    }
}

/// Execute memory_stats command
pub fn cmd_memory_stats(suite: &MemorySuite, _args: MemorySuiteArgs) -> SuiteResult {
    match suite.state.memory.get_stats() {
        Ok((count, namespaces)) => SuiteResult::ok(
            "memory_stats",
            serde_json::json!({
                "count": count,
                "namespaces": namespaces,
                "size_bytes": count * 1024 // Rough estimate
            }),
        ),
        Err(e) => SuiteResult::err("memory_stats", e.to_string()),
    }
}

/// Execute search_semantic command
pub fn cmd_search_semantic(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let query = match args.query {
        Some(q) => q,
        None => return SuiteResult::err("search_semantic", "Missing required parameter: query"),
    };

    let limit = args.limit.unwrap_or(10);
    let namespace = args.namespace.as_deref();

    match suite.state.memory.search_semantic(&query, namespace, limit) {
        Ok(results) => SuiteResult::ok(
            "search_semantic",
            serde_json::json!({
                "results": results,
                "count": results.len()
            }),
        ),
        Err(e) => SuiteResult::err("search_semantic", e.to_string()),
    }
}

/// Execute search_hybrid command
pub fn cmd_search_hybrid(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let query = match args.query {
        Some(q) => q,
        None => return SuiteResult::err("search_hybrid", "Missing required parameter: query"),
    };

    let keywords: Vec<&str> = args.keywords
        .as_ref()
        .map(|kws| kws.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    let limit = args.limit.unwrap_or(10);
    let namespace = args.namespace.as_deref();

    match suite.state.memory.search_hybrid(&query, &keywords, namespace, limit) {
        Ok(results) => SuiteResult::ok(
            "search_hybrid",
            serde_json::json!({
                "results": results,
                "count": results.len()
            }),
        ),
        Err(e) => SuiteResult::err("search_hybrid", e.to_string()),
    }
}

/// Execute query_by_tags command
pub fn cmd_query_by_tags(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let tags: Vec<&str> = match args.tags {
        Some(ref t) => t.iter().map(|s| s.as_str()).collect(),
        None => return SuiteResult::err("query_by_tags", "Missing required parameter: tags"),
    };

    let namespace = args.namespace.as_deref();

    match suite.state.memory.query_by_tags(&tags, namespace) {
        Ok(entries) => SuiteResult::ok(
            "query_by_tags",
            serde_json::json!({
                "entries": entries,
                "count": entries.len()
            }),
        ),
        Err(e) => SuiteResult::err("query_by_tags", e.to_string()),
    }
}

/// Execute query_by_importance command
pub fn cmd_query_by_importance(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let min_importance = match args.min_importance {
        Some(imp) => imp,
        None => return SuiteResult::err("query_by_importance", "Missing required parameter: min_importance"),
    };

    let limit = args.limit.unwrap_or(10);

    match suite.state.memory.query_by_importance(min_importance, limit) {
        Ok(entries) => SuiteResult::ok(
            "query_by_importance",
            serde_json::json!({
                "entries": entries,
                "count": entries.len()
            }),
        ),
        Err(e) => SuiteResult::err("query_by_importance", e.to_string()),
    }
}

/// Execute query_recent command
pub fn cmd_query_recent(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let limit = args.limit.unwrap_or(10);
    let namespace = args.namespace.as_deref();

    match suite.state.memory.query_recent(limit, namespace) {
        Ok(entries) => SuiteResult::ok(
            "query_recent",
            serde_json::json!({
                "entries": entries,
                "count": entries.len()
            }),
        ),
        Err(e) => SuiteResult::err("query_recent", e.to_string()),
    }
}

/// Execute query_since command
pub fn cmd_query_since(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let timestamp = match args.unix_timestamp {
        Some(ts) => ts as i64,
        None => return SuiteResult::err("query_since", "Missing required parameter: unix_timestamp"),
    };

    let namespace = args.namespace.as_deref();

    match suite.state.memory.query_since(timestamp, namespace) {
        Ok(entries) => SuiteResult::ok(
            "query_since",
            serde_json::json!({
                "entries": entries,
                "count": entries.len()
            }),
        ),
        Err(e) => SuiteResult::err("query_since", e.to_string()),
    }
}

/// Execute consolidate_similar command
pub fn cmd_consolidate_similar(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let threshold = match args.threshold {
        Some(t) => t,
        None => return SuiteResult::err("consolidate_similar", "Missing required parameter: threshold"),
    };

    match suite.state.memory.consolidate_similar(threshold) {
        Ok(removed_ids) => SuiteResult::ok(
            "consolidate_similar",
            serde_json::json!({
                "merged": removed_ids.len(),
                "removed": removed_ids.len(),
                "removed_ids": removed_ids
            }),
        ),
        Err(e) => SuiteResult::err("consolidate_similar", e.to_string()),
    }
}

/// Execute get_related_memories command
pub fn cmd_get_related_memories(suite: &MemorySuite, args: MemorySuiteArgs) -> SuiteResult {
    let key = match args.key {
        Some(k) => k,
        None => return SuiteResult::err("get_related_memories", "Missing required parameter: key"),
    };

    let limit = args.limit.unwrap_or(10);

    match suite.state.memory.get_related_memories(&key, limit) {
        Ok(entries) => SuiteResult::ok(
            "get_related_memories",
            serde_json::json!({
                "entries": entries,
                "count": entries.len()
            }),
        ),
        Err(e) => SuiteResult::err("get_related_memories", e.to_string()),
    }
}
