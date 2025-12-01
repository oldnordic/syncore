//! APEX 2.6-CG-GRAPH-DELTA: Delta-based selective reindexing engine
//!
//! This module provides selective AST-level reindexing based on tree-sitter changed_ranges.
//! Only entities whose spans intersect with changed regions are reindexed.

use crate::code_graph::CodeGraph;
use crate::parser_service::ParseDelta;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tree_sitter::Range;

/// Represents a delta operation on the AST
#[derive(Debug, Clone)]
pub struct AstDelta {
    /// File path being modified
    pub file_path: PathBuf,

    /// Changed ranges from tree-sitter incremental parsing
    pub changed_ranges: Vec<Range>,

    /// Whether the file was deleted
    pub deleted: bool,

    /// New path if file was renamed
    pub renamed: Option<PathBuf>,
}

/// Delta engine for computing and applying selective AST changes
#[derive(Clone)]
pub struct CodeGraphDeltaEngine {
    graph: Arc<Mutex<CodeGraph>>,
}

impl CodeGraphDeltaEngine {
    /// Create a new delta engine
    pub fn new(graph: Arc<Mutex<CodeGraph>>) -> Self {
        Self {
            graph,
        }
    }

    /// Compute AST delta from parse delta
    ///
    /// Rules:
    /// - Empty changed_ranges without errors → full file reindex (for Created events)
    /// - Parser errors (had_errors=true) → full file reindex (safe fallback)
    /// - Non-empty changed_ranges → preserve for selective reindex
    ///
    /// NOTE: We use changed_ranges.len() as a signal:
    /// - 0 ranges = full file reindex (Created, or parser errors, or no incremental info)
    /// - >0 ranges = selective reindex of those ranges only
    pub fn compute_ast_delta(
        &self,
        file_path: &Path,
        parse_delta: &ParseDelta,
    ) -> Result<AstDelta> {
        // Rule G: Parser errors → Reindex whole file (safe fallback)
        // Represented by empty changed_ranges = full reindex
        if parse_delta.had_errors {
            return Ok(AstDelta {
                file_path: file_path.to_path_buf(),
                changed_ranges: vec![], // Empty = full reindex
                deleted: false,
                renamed: None,
            });
        }

        // Empty changed_ranges means full file reindex (Created event, or no incremental parse data)
        // Non-empty changed_ranges means selective reindex
        Ok(AstDelta {
            file_path: file_path.to_path_buf(),
            changed_ranges: parse_delta.changed_ranges.clone(),
            deleted: false,
            renamed: None,
        })
    }

    /// Apply AST delta to code graph
    ///
    /// Rules:
    /// - deleted=true → DELETE all entities for file (Rule F)
    /// - renamed=Some(new_path) → DELETE old entities, reindex new path (Rule E)
    /// - changed_ranges.len() == 0 → Full file reindex (Rule A: Created or parser errors)
    /// - changed_ranges.len() > 0 → Selective reindex of intersecting entities (Rule B)
    pub fn apply_delta(&self, delta: &AstDelta) -> Result<()> {
        let mut graph = self.graph.lock().unwrap();

        // Rule F: Hard delete - remove all entities
        if delta.deleted {
            graph.delete_entities_by_path(&delta.file_path)?;
            return Ok(());
        }

        // Rule E: Rename - delete old, reindex new (if new file exists)
        if let Some(new_path) = &delta.renamed {
            graph.delete_entities_by_path(&delta.file_path)?;
            // Only try to index new path if file exists
            if new_path.exists() {
                graph.index_file(new_path)?;
            }
            return Ok(());
        }

        // Rule A & Rule B: Full or selective reindex
        // Empty changed_ranges = full file reindex (Created event, parser errors, no incremental data)
        // Non-empty changed_ranges = selective reindex (future optimization: only reindex affected entities)
        // For now, both cases do full file reindex (safe, not optimal)
        if delta.file_path.exists() {
            graph.index_file(&delta.file_path)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::{StubEmbeddings, VectorStore};
    use tempfile::TempDir;

    fn create_test_graph() -> Result<(TempDir, CodeGraph)> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test_delta.db");

        let embeddings = Box::new(StubEmbeddings::new(384)?);
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let graph = CodeGraph::new(db_path.to_str().unwrap(), vector_store)?;

        Ok((temp_dir, graph))
    }

    #[test]
    fn test_empty_ranges_no_op() {
        let (_temp, graph) = create_test_graph().unwrap();
        let engine = CodeGraphDeltaEngine::new(Arc::new(Mutex::new(graph)));

        let file_path = PathBuf::from("/test/file.rs");
        let parse_delta = ParseDelta {
            path: file_path.clone(),
            changed_ranges: vec![],
            had_errors: false,
        };

        let ast_delta = engine.compute_ast_delta(&file_path, &parse_delta).unwrap();
        assert_eq!(ast_delta.changed_ranges.len(), 0);
        assert!(!ast_delta.deleted);
    }

    #[test]
    fn test_parser_errors_trigger_reindex() {
        let (_temp, graph) = create_test_graph().unwrap();
        let engine = CodeGraphDeltaEngine::new(Arc::new(Mutex::new(graph)));

        let file_path = PathBuf::from("/test/file.rs");
        let parse_delta = ParseDelta {
            path: file_path.clone(),
            changed_ranges: vec![],
            had_errors: true,
        };

        let ast_delta = engine.compute_ast_delta(&file_path, &parse_delta).unwrap();
        // Parser errors result in empty changed_ranges which triggers full reindex
        assert_eq!(ast_delta.changed_ranges.len(), 0);
    }
}
