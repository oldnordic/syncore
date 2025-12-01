//! Bootstrap module for initial cold-start indexing
//!
//! APEX 2.15 - Phase 2: Bootstrap implementation
//! This module contains bootstrap logic for:
//! - Cold start: Full initial index when code_entities is empty
//! - Warm start: Skip bootstrap when entities already exist

use crate::code_graph::CodeGraph;
use crate::config::SyncoreConfig;
use crate::vector::{HuggingFaceEmbeddings, VectorStore};
use anyhow::{Context, Result};
use glob::glob;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Run startup bootstrap check and indexing if needed.
///
/// This function is called during MCP server startup to determine:
/// - COLD START: If code_entities table is empty → run full bootstrap
/// - WARM START: If code_entities has data → skip bootstrap
///
/// # Test-only API
/// This function is exposed for integration testing.
/// In production, it's called from mcp_stdio_main.rs startup.
///
/// # Arguments
/// * `config` - SynCore configuration (paths, indexing settings)
///
/// # Returns
/// * `Ok(())` - Bootstrap completed or skipped successfully
/// * `Err(_)` - Bootstrap failed
///
/// # Example (test usage)
/// ```ignore
/// use syncore::bootstrap::run_startup_bootstrap_for_tests;
/// use syncore::config::SyncoreConfig;
///
/// let config = SyncoreConfig::default();
/// run_startup_bootstrap_for_tests(&config).await?;
/// ```
pub async fn run_startup_bootstrap_for_tests(cfg: &SyncoreConfig) -> Result<()> {
    // Step 1: Check entity count in code_entities
    let conn =
        Connection::open(&cfg.paths.code_graph_db).context("Failed to open code_graph database")?;

    let entity_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM code_entities", [], |row| row.get(0))
        .context("Failed to query entity count")?;

    // Step 2: Warm start - skip if entities exist
    if entity_count > 0 {
        eprintln!(
            "[SynCore] Existing code entities found (count = {}), skipping bootstrap.",
            entity_count
        );
        return Ok(());
    }

    // Step 3: Cold start - run full bootstrap index
    eprintln!("[SynCore] No code entities found. Running initial bootstrap index...");

    // Get workspace root
    // For tests: Use database directory's parent (workspace-local .syncore/test.db → workspace/)
    // For production: Use current directory
    let db_path = std::path::Path::new(&cfg.paths.code_graph_db);
    let workspace_root = db_path
        .parent() // .syncore/
        .and_then(|p| p.parent()) // workspace/
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    // Initialize CodeGraph with vector store
    let embeddings =
        Box::new(HuggingFaceEmbeddings::new().context("Failed to initialize embeddings")?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    let mut code_graph = CodeGraph::new(&cfg.paths.code_graph_db, vector_store)
        .context("Failed to initialize CodeGraph")?;

    // Index all Rust files in workspace using glob pattern
    let pattern = workspace_root.join("**/*.rs");
    let pattern_str = pattern.to_string_lossy();

    let mut indexed_count = 0;
    let mut total_entities = 0;

    if let Ok(paths) = glob(&pattern_str) {
        for entry in paths.flatten() {
            // Skip excluded directories (target/, etc.)
            let entry_str = entry.to_string_lossy();
            let should_skip = cfg
                .indexing
                .excluded_dirs
                .iter()
                .any(|excluded| entry_str.contains(excluded));

            if should_skip {
                continue;
            }

            match code_graph.index_file(&entry) {
                Ok(count) => {
                    indexed_count += 1;
                    total_entities += count;
                }
                Err(e) => {
                    eprintln!(
                        "[SynCore] Bootstrap: Failed to index {}: {}",
                        entry.display(),
                        e
                    );
                }
            }
        }
    }

    eprintln!(
        "[SynCore] Bootstrap complete: {} files indexed, {} entities extracted",
        indexed_count, total_entities
    );

    Ok(())
}
