//! Database path configuration for SynCore
//!
//! Provides unified database path resolution with environment variable support.
//! All database paths are ABSOLUTE and deterministic, regardless of process CWD.

use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};

use crate::path_resolver::PathResolver; // APEX v1.7 Phase 6

/// Cached project root directory
static PROJECT_ROOT: Lazy<PathBuf> = Lazy::new(find_project_root);

/// Find the database storage directory.
///
/// Priority order:
/// 1. PROJECT_ROOT environment variable (if set)
/// 2. Executable's parent directory (binary location)
/// 3. Fallback to current working directory
///
/// Returns an ABSOLUTE path to the database storage directory.
///
/// **Design Decision**: Store databases in binary's directory.
/// This enables shared knowledge across all projects - user indexes code once,
/// and all projects benefit from the same semantic search database.
fn find_project_root() -> PathBuf {
    // Priority 1: Explicit PROJECT_ROOT env var
    if let Ok(root) = std::env::var("PROJECT_ROOT") {
        let path = PathBuf::from(root);
        if path.is_absolute() {
            eprintln!("[syncore] Using PROJECT_ROOT env var: {}", path.display());
            return path;
        } else {
            eprintln!(
                "[syncore] WARNING: PROJECT_ROOT is not absolute ({}), falling back to detection",
                path.display()
            );
        }
    }

    // Priority 2: Use executable's parent directory
    // This ensures shared knowledge across all projects
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            eprintln!("[syncore] Using binary directory: {}", parent.display());
            return parent.to_path_buf();
        }
    }

    // Priority 3: Fallback to current working directory
    if let Ok(cwd) = std::env::current_dir() {
        eprintln!(
            "[syncore] WARNING: Could not get exe path, using CWD: {}",
            cwd.display()
        );
        return cwd;
    }

    // Priority 4: Last resort - use PathResolver to find workspace root
    let mut resolver = PathResolver::new();
    let fallback = resolver
        .resolve_workspace_root(Path::new("."))
        .ok()
        .flatten()
        .unwrap_or_else(|| PathBuf::from("."));

    eprintln!(
        "[syncore] WARNING: Could not determine database directory, using PathResolver fallback: {}",
        fallback.display()
    );
    fallback
}

/// Returns the canonical main database path.
///
/// Priority:
/// 1. MAIN_DB environment variable (used as-is if absolute, or relative to project root)
/// 2. <project_root>/syncore.db
///
/// This database contains: memory (key-value), tasks, embeddings, steps, etc.
pub fn main_db_path() -> String {
    // Check env var first
    if let Ok(path) = std::env::var("MAIN_DB") {
        let path_buf = PathBuf::from(&path);
        if path_buf.is_absolute() {
            return path;
        }
        // If relative, resolve against project root
        let absolute = PROJECT_ROOT.join(&path);
        return absolute.to_string_lossy().to_string();
    }

    // Default: <project_root>/syncore.db
    let default_path = PROJECT_ROOT.join("syncore.db");
    default_path.to_string_lossy().to_string()
}

/// Returns the canonical IntelliTask database path.
///
/// Priority:
/// 1. INTELLITASK_DB environment variable (used as-is if absolute, or relative to project root)
/// 2. <project_root>/syncore_intellitask.db
///
/// This database contains: IntelliTask tasks, PRDs, subtasks, etc.
pub fn intellitask_db_path() -> String {
    // Check env var first
    if let Ok(path) = std::env::var("INTELLITASK_DB") {
        let path_buf = PathBuf::from(&path);
        if path_buf.is_absolute() {
            return path;
        }
        // If relative, resolve against project root
        let absolute = PROJECT_ROOT.join(&path);
        return absolute.to_string_lossy().to_string();
    }

    // Default: <project_root>/syncore_intellitask.db
    let default_path = PROJECT_ROOT.join("syncore_intellitask.db");
    default_path.to_string_lossy().to_string()
}

/// Returns the canonical code graph database path.
///
/// Priority:
/// 1. CODE_GRAPH_DB environment variable (used as-is if absolute, or relative to project root)
/// 2. <project_root>/syncore_code_graph.db
///
/// This ensures all code graph operations (indexing, querying, mapping) use
/// the same SQLite database, preventing split-brain bugs caused by CWD drift.
pub fn code_graph_db_path() -> String {
    // Check env var first
    if let Ok(path) = std::env::var("CODE_GRAPH_DB") {
        let path_buf = PathBuf::from(&path);
        if path_buf.is_absolute() {
            return path;
        }
        // If relative, resolve against project root
        let absolute = PROJECT_ROOT.join(&path);
        return absolute.to_string_lossy().to_string();
    }

    // Default: <project_root>/syncore_code_graph.db
    let default_path = PROJECT_ROOT.join("syncore_code_graph.db");
    default_path.to_string_lossy().to_string()
}

/// Returns the canonical vector index path.
///
/// Priority:
/// 1. VECTOR_INDEX_PATH environment variable (used as-is if absolute, or relative to project root)
/// 2. <project_root>/vector.index
///
/// This ensures HNSW vector index is co-located with databases for consistent,
/// shared knowledge across all projects using the same binary.
///
/// **DEPRECATED**: Use `code_vector_index_path()` or `general_vector_index_path()` for domain-aware routing (APEX 1.7).
#[deprecated(note = "Use code_vector_index_path() or general_vector_index_path() for domain-aware routing")]
pub fn vector_index_path() -> String {
    // Check env var first
    if let Ok(path) = std::env::var("VECTOR_INDEX_PATH") {
        let path_buf = PathBuf::from(&path);
        if path_buf.is_absolute() {
            return path;
        }
        // If relative, resolve against project root
        let absolute = PROJECT_ROOT.join(&path);
        return absolute.to_string_lossy().to_string();
    }

    // Default: <project_root>/vector.index
    let default_path = PROJECT_ROOT.join("vector.index");
    default_path.to_string_lossy().to_string()
}

/// Get path for CODE domain vector index (code entities).
///
/// Resolution priority:
/// 1. CODE_VECTOR_INDEX_PATH environment variable (absolute or relative to project root)
/// 2. <project_root>/syncore_code.index
///
/// APEX 1.7: CODE domain stores code entities with code-optimized embeddings.
pub fn code_vector_index_path() -> String {
    // Check env var first
    if let Ok(path) = std::env::var("CODE_VECTOR_INDEX_PATH") {
        let path_buf = PathBuf::from(&path);
        if path_buf.is_absolute() {
            return path;
        }
        // If relative, resolve against project root
        let absolute = PROJECT_ROOT.join(&path);
        return absolute.to_string_lossy().to_string();
    }

    // Default: <project_root>/syncore_code.index
    let default_path = PROJECT_ROOT.join("syncore_code.index");
    default_path.to_string_lossy().to_string()
}

/// Get path for GENERAL domain vector index (documents, tasks, notes).
///
/// Resolution priority:
/// 1. GENERAL_VECTOR_INDEX_PATH environment variable (absolute or relative to project root)
/// 2. <project_root>/syncore_general.index
///
/// APEX 1.7: GENERAL domain stores documents, tasks, and reasoning steps with general-purpose embeddings.
pub fn general_vector_index_path() -> String {
    // Check env var first
    if let Ok(path) = std::env::var("GENERAL_VECTOR_INDEX_PATH") {
        let path_buf = PathBuf::from(&path);
        if path_buf.is_absolute() {
            return path;
        }
        // If relative, resolve against project root
        let absolute = PROJECT_ROOT.join(&path);
        return absolute.to_string_lossy().to_string();
    }

    // Default: <project_root>/syncore_general.index
    let default_path = PROJECT_ROOT.join("syncore_general.index");
    default_path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_path_is_absolute() {
        // Clear environment variables
        std::env::remove_var("CODE_GRAPH_DB");
        std::env::remove_var("PROJECT_ROOT");

        let path = code_graph_db_path();
        let path_buf = PathBuf::from(&path);

        // Must be absolute
        assert!(
            path_buf.is_absolute(),
            "Default path must be absolute, got: {}",
            path
        );

        // Must end with syncore_code_graph.db
        assert!(
            path.ends_with("syncore_code_graph.db"),
            "Path must end with syncore_code_graph.db"
        );
    }

    #[test]
    fn test_env_override_absolute() {
        std::env::set_var("CODE_GRAPH_DB", "/custom/path/graph.db");

        let path = code_graph_db_path();
        assert_eq!(path, "/custom/path/graph.db");

        // Cleanup
        std::env::remove_var("CODE_GRAPH_DB");
    }

    #[test]
    #[ignore] // Ignored because Lazy<T> caches PROJECT_ROOT and affects other tests
    fn test_project_root_env_var() {
        // Clear CODE_GRAPH_DB but set PROJECT_ROOT
        std::env::remove_var("CODE_GRAPH_DB");
        std::env::set_var("PROJECT_ROOT", "/tmp/test_root");

        // Note: Lazy<T> doesn't support reset, so PROJECT_ROOT is cached on first access
        // This test can interfere with other tests if run in parallel

        let path = code_graph_db_path();

        // Should use PROJECT_ROOT
        assert!(
            path.starts_with("/tmp/test_root"),
            "Should use PROJECT_ROOT env var, got: {}",
            path
        );

        // Cleanup
        std::env::remove_var("PROJECT_ROOT");
    }
}
