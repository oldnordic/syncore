// SPEC: SYNCORE-CODE-TOOLS-IMPROVEMENT-01 (APEX v1.2) - STEP A
// SPEC: SYNCORE-CONFIG-AND-TOOLS-FIX-01 (APEX v1.2) - STEP A/D
//
// This module provides path filtering to exclude build artifacts,
// vendor dependencies, and other non-source directories from code indexing.
//
// Now uses SyncoreConfig for excluded directories instead of hardcoded list.

use crate::config::SyncoreConfig;

/// Default excluded directories (used when config is not available)
const DEFAULT_EXCLUDED_DIRS: &[&str] = &[
    // Rust
    "target",
    // JavaScript/Node.js
    "node_modules",
    // Version control
    ".git",
    // Python
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    ".eggs",
    // Go
    "vendor",
    // Java/JVM
    ".gradle",
    ".m2",
    // IDE/Editor
    ".vscode",
    ".idea",
    ".vs",
    // Generic build output
    "dist",
    "build",
    "out",
    // Coverage/Test output
    "coverage",
    "htmlcov",
    ".nyc_output",
    // C/C++
    "cmake-build-debug",
    "cmake-build-release",
    // Cargo registry (absolute path component)
    ".cargo",
    // Database files (by extension)
    "*.db",
    "*.sqlite",
    "*.sqlite3",
    // Log files (by extension)
    "*.log",
];

/// Check if a path should be indexed using the global config.
///
/// Returns `false` for:
/// - Empty paths
/// - Paths containing excluded directories from config (or defaults)
///
/// Returns `true` for normal source files.
///
/// # Examples
///
/// ```
/// use syncore::macro_tools::path_filter::should_index_path;
///
/// assert!(should_index_path("src/main.rs"));
/// assert!(!should_index_path("target/debug/build/foo.rs"));
/// assert!(!should_index_path("node_modules/lodash/index.js"));
/// ```
pub fn should_index_path(path: &str) -> bool {
    // Try to use global config, fall back to defaults
    if let Some(config) = SyncoreConfig::try_global() {
        !config.should_exclude_path(path)
    } else {
        should_index_path_with_defaults(path)
    }
}

/// Check if a path should be indexed using a specific config.
pub fn should_index_path_with_config(path: &str, config: &SyncoreConfig) -> bool {
    !config.should_exclude_path(path)
}

/// Check if a path should be indexed using a custom excluded dirs list.
pub fn should_index_path_with_excludes(path: &str, excluded_dirs: &[String]) -> bool {
    if path.is_empty() {
        return false;
    }

    let normalized = path.trim_start_matches("./");
    let components: Vec<&str> = normalized.split(['/', '\\']).filter(|s| !s.is_empty()).collect();

    for component in &components {
        for excluded in excluded_dirs {
            if *component == excluded {
                return false;
            }
            // Handle cmake-build-* variants
            if excluded.starts_with("cmake-build-") && component.starts_with("cmake-build-") {
                return false;
            }
            // Handle file pattern exclusions (e.g., "*.db", "*.sqlite", "*.log")
            if excluded.starts_with("*.") && component.contains('.') {
                let extension = component.split('.').last().unwrap_or("");
                let pattern_ext = excluded.trim_start_matches('*').trim_start_matches('.');
                if extension.to_lowercase() == pattern_ext.to_lowercase() {
                    return false;
                }
            }
        }
    }

    true
}

/// Internal function using hardcoded defaults (fallback when no config)
fn should_index_path_with_defaults(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }

    let normalized = path.trim_start_matches("./");
    let components: Vec<&str> = normalized.split(['/', '\\']).filter(|s| !s.is_empty()).collect();

    for component in &components {
        for excluded in DEFAULT_EXCLUDED_DIRS {
            if *component == *excluded {
                return false;
            }
            // Handle cmake-build-* variants
            if excluded.starts_with("cmake-build-") && component.starts_with("cmake-build-") {
                return false;
            }
            // Handle file pattern exclusions (e.g., "*.db", "*.sqlite", "*.log")
            if excluded.starts_with("*.") && component.contains('.') {
                let extension = component.split('.').last().unwrap_or("");
                let pattern_ext = excluded.trim_start_matches('*').trim_start_matches('.');
                if extension.to_lowercase() == pattern_ext.to_lowercase() {
                    return false;
                }
            }
        }
    }

    true
}

/// Get the list of excluded directories from config or defaults
pub fn get_excluded_dirs() -> Vec<String> {
    if let Some(config) = SyncoreConfig::try_global() {
        config.indexing.excluded_dirs.clone()
    } else {
        DEFAULT_EXCLUDED_DIRS.iter().map(|s| s.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_src_files() {
        assert!(should_index_path("src/main.rs"));
        assert!(should_index_path("src/lib.rs"));
        assert!(should_index_path("src/memory/mod.rs"));
    }

    #[test]
    fn test_excludes_target() {
        assert!(!should_index_path("target/debug/build/typenum/tests.rs"));
        assert!(!should_index_path("target/release/deps/something.rs"));
    }

    #[test]
    fn test_excludes_node_modules() {
        assert!(!should_index_path("node_modules/lodash/index.js"));
    }

    #[test]
    fn test_allows_target_in_filename() {
        assert!(should_index_path("src/target_parser.rs"));
    }

    #[test]
    fn test_with_custom_excludes() {
        let excludes = vec!["custom_dir".to_string(), "my_build".to_string()];

        assert!(!should_index_path_with_excludes("custom_dir/file.rs", &excludes));
        assert!(!should_index_path_with_excludes("src/my_build/output.js", &excludes));
        assert!(should_index_path_with_excludes("src/main.rs", &excludes));
        // Note: default excludes like target are NOT excluded with custom list
        assert!(should_index_path_with_excludes("target/debug/foo.rs", &excludes));
    }

    #[test]
    fn test_get_excluded_dirs_returns_defaults() {
        // Without global config initialized, should return defaults
        let dirs = get_excluded_dirs();
        assert!(dirs.contains(&"target".to_string()));
        assert!(dirs.contains(&"node_modules".to_string()));
    }
}
