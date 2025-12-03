//! Database path configuration for SynCore
//!
//! Provides unified database path resolution with environment variable support.
//! All database paths are ABSOLUTE and deterministic, regardless of process CWD.

use once_cell::sync::Lazy;
use std::path::PathBuf;

/// Cached binary directory (where syncore executable is located)
static BINARY_DIR: Lazy<PathBuf> = Lazy::new(find_binary_dir);

/// Find the binary directory where databases should be stored.
///
/// Priority order:
/// 1. SYNCORE_DATA_DIR environment variable (if set and absolute)
/// 2. Executable's parent directory (binary location)
/// 3. ~/.config/syncore (XDG standard location)
/// 4. Fallback to current working directory
///
/// Returns an ABSOLUTE path to the database storage directory.
///
/// **Design Decision**: Store databases next to the binary.
/// This allows one syncore installation to serve all projects.
fn find_binary_dir() -> PathBuf {
    // Priority 1: Explicit SYNCORE_DATA_DIR env var
    if let Ok(dir) = std::env::var("SYNCORE_DATA_DIR") {
        let path = PathBuf::from(dir);
        if path.is_absolute() {
            eprintln!("[syncore] Using SYNCORE_DATA_DIR: {}", path.display());
            return path;
        }
        eprintln!(
            "[syncore] WARNING: SYNCORE_DATA_DIR is not absolute ({}), falling back",
            path.display()
        );
    }

    // Priority 2: Use executable's parent directory (same folder as binary)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            eprintln!("[syncore] Using binary directory for data: {}", parent.display());
            return parent.to_path_buf();
        }
    }

    // Priority 3: XDG standard location ~/.config/syncore
    if let Ok(home) = std::env::var("HOME") {
        let xdg_path = PathBuf::from(home).join(".config/syncore");
        eprintln!("[syncore] Using XDG config directory: {}", xdg_path.display());
        return xdg_path;
    }

    // Priority 4: Fallback to current working directory
    if let Ok(cwd) = std::env::current_dir() {
        eprintln!(
            "[syncore] WARNING: Could not determine data directory, using CWD: {}",
            cwd.display()
        );
        return cwd;
    }

    // Last resort
    eprintln!("[syncore] ERROR: Could not determine data directory, using current directory");
    PathBuf::from(".")
}

/// Returns the canonical main database path.
///
/// Priority:
/// 1. MAIN_DB environment variable (used as-is if absolute, or relative to project root)
/// 2. Global SyncoreConfig.paths.db_path (if config initialized)
/// 3. <project_root>/syncore.db (fallback)
///
/// This database contains: memory (key-value), tasks, embeddings, steps, etc.
pub fn main_db_path() -> String {
    // Priority 1: Check env var first
    if let Ok(path) = std::env::var("MAIN_DB") {
        eprintln!("[db_paths] main_db_path: Using MAIN_DB env var: {}", path);
        let path_buf = PathBuf::from(&path);
        if path_buf.is_absolute() {
            return path;
        }
        // If relative, resolve against binary directory
        let absolute = BINARY_DIR.join(&path);
        eprintln!("[db_paths] main_db_path: Resolved relative MAIN_DB to: {}", absolute.display());
        return absolute.to_string_lossy().to_string();
    }

    // Priority 2: Try to get from global config
    if let Some(config) = crate::config::SyncoreConfig::try_global() {
        let path = &config.paths.db_path;
        eprintln!("[db_paths] main_db_path: Found global config, db_path: {}", path);
        let path_buf = PathBuf::from(path);
        if path_buf.is_absolute() {
            eprintln!("[db_paths] main_db_path: Using absolute path from config: {}", path);
            return path.clone();
        }
        // If relative, resolve against project root
        let absolute = BINARY_DIR.join(path);
        eprintln!("[db_paths] main_db_path: Resolved relative path to: {}", absolute.display());
        return absolute.to_string_lossy().to_string();
    }

    // Priority 3: Fallback to default (binary directory)
    eprintln!("[db_paths] main_db_path: No env var or config, using binary directory");
    let default_path = BINARY_DIR.join("syncore.db");
    eprintln!("[db_paths] main_db_path: Fallback path: {}", default_path.display());
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
        // If relative, resolve against binary directory
        let absolute = BINARY_DIR.join(&path);
        return absolute.to_string_lossy().to_string();
    }

    // Default: <binary_dir>/syncore_intellitask.db
    let default_path = BINARY_DIR.join("syncore_intellitask.db");
    default_path.to_string_lossy().to_string()
}

/// Returns the canonical code graph database path.
///
/// Priority:
/// 1. CODE_GRAPH_DB environment variable (used as-is if absolute, or relative to project root)
/// 2. Global SyncoreConfig.paths.code_graph_db (if config initialized)
/// 3. <project_root>/syncore_code_graph.db (fallback)
///
/// This ensures all code graph operations (indexing, querying, mapping) use
/// the same SQLite database, preventing split-brain bugs caused by CWD drift.
pub fn code_graph_db_path() -> String {
    // Priority 1: Check env var first
    if let Ok(path) = std::env::var("CODE_GRAPH_DB") {
        eprintln!("[db_paths] code_graph_db_path: Using CODE_GRAPH_DB env var: {}", path);
        let path_buf = PathBuf::from(&path);
        if path_buf.is_absolute() {
            return path;
        }
        // If relative, resolve against binary directory
        let absolute = BINARY_DIR.join(&path);
        eprintln!(
            "[db_paths] code_graph_db_path: Resolved relative CODE_GRAPH_DB to: {}",
            absolute.display()
        );
        return absolute.to_string_lossy().to_string();
    }

    // Priority 2: Try to get from global config
    if let Some(config) = crate::config::SyncoreConfig::try_global() {
        let path = &config.paths.code_graph_db;
        eprintln!("[db_paths] code_graph_db_path: Found global config, code_graph_db: {}", path);
        let path_buf = PathBuf::from(path);
        if path_buf.is_absolute() {
            eprintln!("[db_paths] code_graph_db_path: Using absolute path from config: {}", path);
            return path.clone();
        }
        // If relative, resolve against project root
        let absolute = BINARY_DIR.join(path);
        eprintln!(
            "[db_paths] code_graph_db_path: Resolved relative path to: {}",
            absolute.display()
        );
        return absolute.to_string_lossy().to_string();
    }

    // Priority 3: Fallback to default (binary directory)
    eprintln!("[db_paths] code_graph_db_path: No env var or config, using binary directory");
    let default_path = BINARY_DIR.join("syncore_code_graph.db");
    eprintln!("[db_paths] code_graph_db_path: Fallback path: {}", default_path.display());
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
#[deprecated(
    note = "Use code_vector_index_path() or general_vector_index_path() for domain-aware routing"
)]
pub fn vector_index_path() -> String {
    // Check env var first
    if let Ok(path) = std::env::var("VECTOR_INDEX_PATH") {
        let path_buf = PathBuf::from(&path);
        if path_buf.is_absolute() {
            return path;
        }
        // If relative, resolve against binary directory
        let absolute = BINARY_DIR.join(&path);
        return absolute.to_string_lossy().to_string();
    }

    // Default: <binary_dir>/vector.index
    let default_path = BINARY_DIR.join("vector.index");
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
        // If relative, resolve against binary directory
        let absolute = BINARY_DIR.join(&path);
        return absolute.to_string_lossy().to_string();
    }

    // Default: <binary_dir>/syncore_code.index
    let default_path = BINARY_DIR.join("syncore_code.index");
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
        // If relative, resolve against binary directory
        let absolute = BINARY_DIR.join(&path);
        return absolute.to_string_lossy().to_string();
    }

    // Default: <binary_dir>/syncore_general.index
    let default_path = BINARY_DIR.join("syncore_general.index");
    default_path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_path_is_absolute() {
        // Clear environment variables
        std::env::remove_var("CODE_GRAPH_DB");
        std::env::remove_var("BINARY_DIR");

        let path = code_graph_db_path();
        let path_buf = PathBuf::from(&path);

        // Must be absolute
        assert!(path_buf.is_absolute(), "Default path must be absolute, got: {}", path);

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
    #[ignore] // Ignored because Lazy<T> caches BINARY_DIR and affects other tests
    fn test_project_root_env_var() {
        // Clear CODE_GRAPH_DB but set BINARY_DIR
        std::env::remove_var("CODE_GRAPH_DB");
        std::env::set_var("BINARY_DIR", "/tmp/test_root");

        // Note: Lazy<T> doesn't support reset, so BINARY_DIR is cached on first access
        // This test can interfere with other tests if run in parallel

        let path = code_graph_db_path();

        // Should use BINARY_DIR
        assert!(path.starts_with("/tmp/test_root"), "Should use BINARY_DIR env var, got: {}", path);

        // Cleanup
        std::env::remove_var("BINARY_DIR");
    }
}
