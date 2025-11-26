//! Central path resolution for SynCore tools (APEX v1.7 Phase 5)
//!
//! Provides consistent path handling across all tools to prevent:
//! - Relative path ambiguities (current_dir assumptions)
//! - Duplicate indexing of excluded directories (target/, node_modules/)
//! - Inconsistent workspace root detection
//!
//! All tools should use this module for path operations.

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Default excluded directories for code indexing
const DEFAULT_EXCLUDED_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    "dist",
    "build",
    ".venv",
    "venv",
    "env",
];

/// Path resolver for consistent path handling across tools
pub struct PathResolver {
    /// Detected workspace root (project root containing .git, Cargo.toml, etc.)
    workspace_root: Option<PathBuf>,
    /// List of excluded directory names
    excluded_dirs: Vec<String>,
}

impl PathResolver {
    /// Create a new PathResolver with default exclusions
    pub fn new() -> Self {
        Self {
            workspace_root: None,
            excluded_dirs: DEFAULT_EXCLUDED_DIRS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }

    /// Create a PathResolver with custom exclusions
    pub fn with_exclusions(exclusions: Vec<String>) -> Self {
        Self {
            workspace_root: None,
            excluded_dirs: exclusions,
        }
    }

    /// Resolve workspace root from a given path
    ///
    /// Searches upward from the given path for workspace indicators:
    /// - .git directory (Git repository)
    /// - Cargo.toml (Rust workspace)
    /// - package.json (Node.js project)
    /// - pyproject.toml (Python project)
    /// - go.mod (Go module)
    ///
    /// # Arguments
    /// * `start_path` - Path to start searching from (file or directory)
    ///
    /// # Returns
    /// Workspace root path if found, otherwise None
    pub fn resolve_workspace_root(&mut self, start_path: &Path) -> Result<Option<PathBuf>> {
        // Start from the given path (or its parent if it's a file)
        let mut current = if start_path.is_file() {
            start_path
                .parent()
                .ok_or_else(|| anyhow!("Cannot get parent of file: {:?}", start_path))?
        } else {
            start_path
        };

        // Search upward for workspace indicators
        loop {
            // Check for workspace indicators
            if current.join(".git").exists()
                || current.join("Cargo.toml").exists()
                || current.join("package.json").exists()
                || current.join("pyproject.toml").exists()
                || current.join("go.mod").exists()
            {
                let root = current.to_path_buf();
                self.workspace_root = Some(root.clone());
                return Ok(Some(root));
            }

            // Move to parent directory
            match current.parent() {
                Some(parent) => current = parent,
                None => {
                    // Reached filesystem root without finding workspace
                    return Ok(None);
                }
            }
        }
    }

    /// Get the cached workspace root (if previously resolved)
    pub fn workspace_root(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }

    /// Resolve a path relative to workspace root
    ///
    /// Converts absolute paths to workspace-relative paths for consistent
    /// storage and comparison.
    ///
    /// # Arguments
    /// * `path` - Absolute or relative path
    ///
    /// # Returns
    /// Path relative to workspace root, or original path if no workspace found
    pub fn resolve_project_relative(&self, path: &Path) -> PathBuf {
        if let Some(root) = &self.workspace_root {
            // Try to strip workspace prefix
            if let Ok(relative) = path.strip_prefix(root) {
                return relative.to_path_buf();
            }
        }

        // Return original path if:
        // - No workspace root detected
        // - Path is outside workspace
        path.to_path_buf()
    }

    /// Check if a path should be excluded from indexing
    ///
    /// Checks if any component of the path matches an excluded directory.
    ///
    /// # Arguments
    /// * `path` - Path to check
    ///
    /// # Returns
    /// true if the path should be excluded
    ///
    /// # Examples
    /// ```
    /// use syncore::path_resolver::PathResolver;
    /// use std::path::Path;
    ///
    /// let resolver = PathResolver::new();
    /// assert!(resolver.is_excluded(Path::new("src/target/debug/build.rs")));
    /// assert!(resolver.is_excluded(Path::new("node_modules/package/index.js")));
    /// assert!(!resolver.is_excluded(Path::new("src/main.rs")));
    /// ```
    pub fn is_excluded(&self, path: &Path) -> bool {
        // Check each component of the path
        for component in path.components() {
            if let Some(os_str) = component.as_os_str().to_str() {
                if self.excluded_dirs.contains(&os_str.to_string()) {
                    return true;
                }
            }
        }
        false
    }

    /// Add an additional excluded directory
    pub fn add_exclusion(&mut self, dir: String) {
        if !self.excluded_dirs.contains(&dir) {
            self.excluded_dirs.push(dir);
        }
    }

    /// Get list of excluded directories
    pub fn excluded_dirs(&self) -> &[String] {
        &self.excluded_dirs
    }

    /// Resolve an absolute path, ensuring it exists and is canonical
    ///
    /// # Arguments
    /// * `path` - Path to resolve
    ///
    /// # Returns
    /// Canonical absolute path
    pub fn resolve_absolute(&self, path: &Path) -> Result<PathBuf> {
        if !path.exists() {
            return Err(anyhow!("Path does not exist: {:?}", path));
        }

        path.canonicalize()
            .map_err(|e| anyhow!("Failed to canonicalize path {:?}: {}", path, e))
    }
}

impl Default for PathResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_exclusions() {
        let resolver = PathResolver::new();
        assert!(resolver.is_excluded(Path::new("target/debug/main")));
        assert!(resolver.is_excluded(Path::new("node_modules/package/index.js")));
        assert!(resolver.is_excluded(Path::new("src/.git/objects")));
        assert!(!resolver.is_excluded(Path::new("src/main.rs")));
    }

    #[test]
    fn test_custom_exclusions() {
        let resolver = PathResolver::with_exclusions(vec!["custom".to_string(), "build".to_string()]);
        assert!(resolver.is_excluded(Path::new("custom/file.rs")));
        assert!(resolver.is_excluded(Path::new("build/output")));
        assert!(!resolver.is_excluded(Path::new("target/debug"))); // Not in custom list
    }

    #[test]
    fn test_add_exclusion() {
        let mut resolver = PathResolver::new();
        resolver.add_exclusion("temp".to_string());
        assert!(resolver.is_excluded(Path::new("temp/file.txt")));
    }

    #[test]
    fn test_resolve_workspace_root_with_git() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir)?;

        let mut resolver = PathResolver::new();
        let root = resolver.resolve_workspace_root(temp_dir.path())?;

        assert!(root.is_some());
        assert_eq!(root.unwrap(), temp_dir.path().canonicalize()?);
        Ok(())
    }

    #[test]
    fn test_resolve_workspace_root_with_cargo() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"test\"\n")?;

        let mut resolver = PathResolver::new();
        let root = resolver.resolve_workspace_root(temp_dir.path())?;

        assert!(root.is_some());
        assert_eq!(root.unwrap(), temp_dir.path().canonicalize()?);
        Ok(())
    }

    #[test]
    fn test_resolve_project_relative() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"test\"\n")?;

        let mut resolver = PathResolver::new();
        resolver.resolve_workspace_root(temp_dir.path())?;

        let abs_path = temp_dir.path().join("src/main.rs");
        let relative = resolver.resolve_project_relative(&abs_path);

        assert_eq!(relative, Path::new("src/main.rs"));
        Ok(())
    }

    #[test]
    fn test_resolve_absolute() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "content")?;

        let resolver = PathResolver::new();
        let absolute = resolver.resolve_absolute(&test_file)?;

        assert!(absolute.is_absolute());
        assert!(absolute.exists());
        Ok(())
    }
}
