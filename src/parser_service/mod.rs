//! APEX 2.2-FW: Incremental Parser Service
//!
//! Maintains per-file parse state and applies incremental edits using tree-sitter.
//! Produces parse deltas with changed ranges for downstream consumers.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Parser, Range, Tree};

use crate::fs_watcher::{FsEvent, FsEventKind};

// ============================================================================
// Public Types
// ============================================================================

/// Parse delta containing changed ranges from incremental parse
#[derive(Debug, Clone)]
pub struct ParseDelta {
    pub path: PathBuf,
    pub changed_ranges: Vec<Range>,
    pub had_errors: bool,
}

/// Parsed file state (tree + source)
struct ParsedFileState {
    tree: Tree,
    source: String,
}

/// Parser service for incremental parsing
pub struct ParserService {
    parser: Parser,
    language: Language,
    root: PathBuf,
    file_states: HashMap<PathBuf, ParsedFileState>,
}

// ============================================================================
// Implementation
// ============================================================================

impl ParserService {
    /// Create new parser service for given language and root directory
    pub fn new(language: Language, root: PathBuf) -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(language)
            .context("Failed to set parser language")?;

        Ok(Self {
            parser,
            language,
            root,
            file_states: HashMap::new(),
        })
    }

    /// Apply filesystem event and produce parse deltas
    pub fn apply_fs_event(&mut self, event: FsEvent) -> Result<Vec<ParseDelta>> {
        match event.kind {
            FsEventKind::Created | FsEventKind::Modified => {
                self.parse_or_reparse(event.path)
            }
            FsEventKind::Removed => {
                self.remove_file(event.path);
                Ok(vec![])
            }
            FsEventKind::Renamed(new_path) => {
                // Handle as remove old + create new
                self.remove_file(event.path);
                self.parse_or_reparse(new_path)
            }
        }
    }

    /// Parse or incrementally reparse a file
    fn parse_or_reparse(&mut self, path: PathBuf) -> Result<Vec<ParseDelta>> {
        // Check if file extension is supported (only .rs for now, expand as needed)
        if !self.is_supported_file(&path) {
            return Ok(vec![]); // Silently skip unsupported files
        }

        // Read file content
        let new_source = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => {
                // File might have been deleted between event and read
                self.remove_file(path);
                return Ok(vec![]);
            }
        };

        // Check if we have previous state for incremental parse
        let delta = if let Some(old_state) = self.file_states.get(&path) {
            // Clone necessary data to avoid borrow conflicts
            let old_tree = old_state.tree.clone();
            let old_source = old_state.source.clone();
            self.incremental_parse(&path, &old_tree, &old_source, &new_source)?
        } else {
            self.full_parse(&path, &new_source)?
        };

        Ok(vec![delta])
    }

    /// Full parse for new file
    fn full_parse(&mut self, path: &Path, source: &str) -> Result<ParseDelta> {
        let tree = self
            .parser
            .parse(source, None)
            .context("Failed to parse file")?;

        let had_errors = tree.root_node().has_error();

        // Store state
        self.file_states.insert(
            path.to_path_buf(),
            ParsedFileState {
                tree,
                source: source.to_string(),
            },
        );

        // Full parse: entire file is "changed"
        let changed_ranges = vec![Range {
            start_byte: 0,
            end_byte: source.len(),
            start_point: tree_sitter::Point { row: 0, column: 0 },
            end_point: tree_sitter::Point {
                row: source.lines().count().saturating_sub(1),
                column: source.lines().last().map(|l| l.len()).unwrap_or(0),
            },
        }];

        Ok(ParseDelta {
            path: path.to_path_buf(),
            changed_ranges,
            had_errors,
        })
    }

    /// Incremental parse with tree reuse
    fn incremental_parse(
        &mut self,
        path: &Path,
        old_tree: &Tree,
        old_source: &str,
        new_source: &str,
    ) -> Result<ParseDelta> {

        // Compute simple diff (byte-level)
        let edit_start = Self::common_prefix_len(old_source, new_source);
        let edit_old_end = old_source.len()
            - Self::common_suffix_len(
                &old_source[edit_start..],
                &new_source[edit_start..],
            );
        let edit_new_end = new_source.len()
            - Self::common_suffix_len(
                &old_source[edit_start..],
                &new_source[edit_start..],
            );

        // Apply edit to old tree
        let mut new_tree = old_tree.clone();
        if edit_start < edit_old_end || edit_start < edit_new_end {
            // Actual change detected
            let edit = tree_sitter::InputEdit {
                start_byte: edit_start,
                old_end_byte: edit_old_end,
                new_end_byte: edit_new_end,
                start_position: Self::byte_to_point(old_source, edit_start),
                old_end_position: Self::byte_to_point(old_source, edit_old_end),
                new_end_position: Self::byte_to_point(new_source, edit_new_end),
            };

            new_tree.edit(&edit);
        }

        // Incremental parse with old tree
        let new_tree = self
            .parser
            .parse(new_source, Some(&new_tree))
            .context("Failed to incremental parse")?;

        let had_errors = new_tree.root_node().has_error();

        // Compute changed ranges
        let changed_ranges = new_tree
            .changed_ranges(old_tree)
            .collect::<Vec<_>>();

        // Update state
        self.file_states.insert(
            path.to_path_buf(),
            ParsedFileState {
                tree: new_tree,
                source: new_source.to_string(),
            },
        );

        Ok(ParseDelta {
            path: path.to_path_buf(),
            changed_ranges,
            had_errors,
        })
    }

    /// Remove file from tracked state
    fn remove_file(&mut self, path: PathBuf) {
        self.file_states.remove(&path);
    }

    /// Check if file extension is supported (simple check for now)
    fn is_supported_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "rs")
            .unwrap_or(false)
    }

    /// Helper: Common prefix length
    fn common_prefix_len(a: &str, b: &str) -> usize {
        a.bytes()
            .zip(b.bytes())
            .take_while(|(x, y)| x == y)
            .count()
    }

    /// Helper: Common suffix length
    fn common_suffix_len(a: &str, b: &str) -> usize {
        a.bytes()
            .rev()
            .zip(b.bytes().rev())
            .take_while(|(x, y)| x == y)
            .count()
    }

    /// Helper: Convert byte offset to tree-sitter Point
    fn byte_to_point(source: &str, byte: usize) -> tree_sitter::Point {
        let mut row = 0;
        let mut col = 0;
        for (i, ch) in source.char_indices() {
            if i >= byte {
                break;
            }
            if ch == '\n' {
                row += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        tree_sitter::Point { row, column: col }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_prefix_len() {
        assert_eq!(ParserService::common_prefix_len("hello", "hello"), 5);
        assert_eq!(ParserService::common_prefix_len("hello", "help"), 3);
        assert_eq!(ParserService::common_prefix_len("abc", "xyz"), 0);
        assert_eq!(ParserService::common_prefix_len("", "abc"), 0);
    }

    #[test]
    fn test_common_suffix_len() {
        assert_eq!(ParserService::common_suffix_len("hello", "jello"), 4);
        assert_eq!(ParserService::common_suffix_len("abc", "xyz"), 0);
        assert_eq!(ParserService::common_suffix_len("test", "fest"), 3);
    }

    #[test]
    fn test_byte_to_point() {
        let source = "line1\nline2\nline3";
        let point = ParserService::byte_to_point(source, 0);
        assert_eq!(point.row, 0);
        assert_eq!(point.column, 0);

        let point = ParserService::byte_to_point(source, 6); // Start of line2
        assert_eq!(point.row, 1);
        assert_eq!(point.column, 0);
    }
}
