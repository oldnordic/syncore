//! APEX 2.3-CG: Incremental Code Graph Update Service
//! APEX 2.6-CG-GRAPH-DELTA: Delta-based selective reindexing
//!
//! This module implements incremental code graph updates driven by FsEvent + ParseDelta.
//! It coordinates between FsWatcher, ParserService, and CodeGraph to efficiently update
//! only the changed files without full project reindexing.
//!
//! APEX 2.6 adds delta-based selective reindexing: only entities whose spans intersect
//! with changed_ranges are reindexed, not the entire file.

use anyhow::Result;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::code_graph::delta::CodeGraphDeltaEngine;
use crate::code_graph::{CodeEntity, CodeGraph, EntityType};
use crate::fs_watcher::FsEvent;
use crate::parser_service::ParseDelta;

// ============================================================================
// Public Types
// ============================================================================

/// Event representing a filesystem change with optional parse delta
#[derive(Debug, Clone)]
pub struct CodeGraphUpdateEvent {
    pub fs_event: FsEvent,
    pub parse_delta: Option<ParseDelta>,
}

/// Service that applies incremental code graph updates
#[derive(Clone)]
pub struct CodeGraphUpdateService {
    graph: Arc<Mutex<CodeGraph>>,
    delta_engine: CodeGraphDeltaEngine,
    /// APEX 2.15: Reindex mutex to serialize DELETE+INSERT operations
    reindex_mutex: Arc<std::sync::Mutex<()>>,
}

// ============================================================================
// Implementation
// ============================================================================

impl CodeGraphUpdateService {
    /// Create a new update service
    pub fn new(graph: CodeGraph, reindex_mutex: Arc<std::sync::Mutex<()>>) -> Result<Self> {
        // APEX 2.6-CG-GRAPH-DELTA: Wrap graph in Arc<Mutex<>> and initialize delta engine
        let graph_arc = Arc::new(Mutex::new(graph));
        let delta_engine = CodeGraphDeltaEngine::new(graph_arc.clone());

        Ok(Self {
            graph: graph_arc,
            delta_engine,
            reindex_mutex,
        })
    }

    /// Apply a code graph update event
    ///
    /// APEX 2.6-CG-GRAPH-DELTA: Now uses delta engine for selective reindexing.
    /// Returns the number of affected entities (inserted, updated, or deleted).
    pub fn apply_update(&mut self, event: CodeGraphUpdateEvent) -> Result<u64> {
        let file_path = event.fs_event.path();

        // Check if file extension is supported
        if !self.is_supported_file(file_path) {
            return Ok(0);
        }

        match event.fs_event {
            FsEvent::Created(_) | FsEvent::Modified(_) => {
                // APEX 2.6: Use delta engine if parse_delta is available
                if let Some(parse_delta) = event.parse_delta {
                    let ast_delta = self.delta_engine.compute_ast_delta(file_path, &parse_delta)?;
                    self.delta_engine.apply_delta(&ast_delta)?;
                    // Return approximate count (delta doesn't track exact count)
                    Ok(1)
                } else {
                    // Fallback: full file reindex (no delta available)
                    // APEX 2.15: Acquire reindex mutex to prevent UNIQUE constraint collisions
                    let _reindex_lock = self.reindex_mutex.lock().expect("reindex mutex poisoned");
                    let mut graph = self
                        .graph
                        .lock()
                        .map_err(|e| anyhow::anyhow!("Failed to lock graph: {}", e))?;
                    let count = graph.index_file(file_path)?;
                    Ok(count as u64)
                }
            }
            FsEvent::Removed(_) => {
                // For delete: remove entities for the file
                let deleted = self.delete_entities_for_file(file_path)?;
                Ok(deleted)
            }
        }
    }

    /// Query entities by file path (for test verification)
    pub fn query_entities_by_path(&self, path: &Path) -> Result<Vec<CodeEntity>> {
        let path_str = path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

        let graph =
            self.graph.lock().map_err(|e| anyhow::anyhow!("Failed to lock graph: {}", e))?;

        let db = graph.db.lock().map_err(|e| anyhow::anyhow!("Failed to lock db: {}", e))?;

        let mut stmt = db.prepare(
            "SELECT id, file_path, entity_type, name, signature, line_start, line_end, docstring, language,
                    created_at, last_modified_at, change_count, author_count
             FROM code_entities WHERE file_path = ?",
        )?;

        let entities = stmt
            .query_map([path_str], |row| {
                Ok(CodeEntity {
                    id: row.get(0)?,
                    file_path: row.get(1)?,
                    entity_type: EntityType::try_parse(&row.get::<_, String>(2)?)
                        .unwrap_or(EntityType::Function),
                    name: row.get(3)?,
                    signature: row.get(4)?,
                    line_start: row.get::<_, i64>(5)? as usize,
                    line_end: row.get::<_, i64>(6)? as usize,
                    docstring: row.get(7)?,
                    language: row.get(8)?,
                    body_snippet: None,
                    created_at: row.get(9)?,
                    last_modified_at: row.get(10)?,
                    change_count: row.get(11)?,
                    author_count: row.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entities)
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Check if file extension is supported
    fn is_supported_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext, "rs" | "js" | "py" | "json" | "toml" | "sh"))
            .unwrap_or(false)
    }

    /// Delete all entities for a file path
    fn delete_entities_for_file(&self, path: &Path) -> Result<u64> {
        let path_str = path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

        let graph =
            self.graph.lock().map_err(|e| anyhow::anyhow!("Failed to lock graph: {}", e))?;

        let db = graph.db.lock().map_err(|e| anyhow::anyhow!("Failed to lock db: {}", e))?;

        let deleted = db.execute("DELETE FROM code_entities WHERE file_path = ?", [path_str])?;

        Ok(deleted as u64)
    }
}

// ============================================================================
// Bridge Helper Function
// ============================================================================

/// Helper to bridge ParserService deltas to CodeGraphUpdateService
///
/// This function is called after ParserService.apply_fs_event() produces ParseDeltas.
pub fn on_parse_delta_update_graph(
    service: &mut CodeGraphUpdateService,
    fs_event: FsEvent,
    deltas: Vec<ParseDelta>,
) -> Result<u64> {
    // Extract the first ParseDelta if available
    let parse_delta = match fs_event {
        FsEvent::Created(_) | FsEvent::Modified(_) => deltas.first().cloned(),
        FsEvent::Removed(_) => None, // File no longer exists
    };

    // Create update event
    let event = CodeGraphUpdateEvent {
        fs_event,
        parse_delta,
    };

    // Apply the update
    service.apply_update(event)
}
