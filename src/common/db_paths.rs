//! Database path configuration for SynCore
//!
//! Provides unified database path resolution with environment variable support.
//! All database paths are ABSOLUTE and deterministic, regardless of process CWD.

use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};

/// Cached project root directory
static PROJECT_ROOT: Lazy<PathBuf> = Lazy::new(find_project_root);

/// Find the project root directory by walking up from the executable location
/// until we find a Cargo.toml file.
///
/// Priority order:
/// 1. PROJECT_ROOT environment variable (if set)
/// 2. Walk up from current_exe() until Cargo.toml found
/// 3. Fallback to current_exe().parent().unwrap()
///
/// Returns an ABSOLUTE path to the project root.
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

    // Priority 2: Walk up from current_exe() to find Cargo.toml
    if let Ok(exe_path) = std::env::current_exe() {
        let mut current = exe_path.as_path();

        // Walk up maximum 10 levels to prevent infinite loops
        for _ in 0..10 {
            if let Some(parent) = current.parent() {
                let cargo_toml = parent.join("Cargo.toml");
                if cargo_toml.exists() {
                    // Special case: If parent ends with "syncore", go up one more level
                    // This handles the /home/user/Projects/SynCore/syncore/ structure
                    if let Some(dir_name) = parent.file_name() {
                        if dir_name == "syncore" {
                            if let Some(grandparent) = parent.parent() {
                                eprintln!(
                                    "[syncore] Detected project root (moved up from syncore/): {}",
                                    grandparent.display()
                                );
                                return grandparent.to_path_buf();
                            }
                        }
                    }

                    eprintln!(
                        "[syncore] Detected project root via Cargo.toml: {}",
                        parent.display()
                    );
                    return parent.to_path_buf();
                }
                current = parent;
            } else {
                break;
            }
        }

        // Priority 3: Fallback to executable's parent directory
        if let Some(parent) = exe_path.parent() {
            eprintln!(
                "[syncore] WARNING: Could not find Cargo.toml, using exe parent: {}",
                parent.display()
            );
            return parent.to_path_buf();
        }
    }

    // Last resort: current directory (should never happen in production)
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    eprintln!(
        "[syncore] WARNING: Could not determine exe path, using CWD: {}",
        cwd.display()
    );
    cwd
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
