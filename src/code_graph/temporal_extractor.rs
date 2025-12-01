//! PHASE 3: Temporal Metadata Extraction
//!
//! This module extracts temporal metadata for code entities from:
//! 1. Filesystem metadata (created_at, last_modified_at)
//! 2. Git history (change_count, author_count)
//!
//! No mocks - uses real std::fs and git2 operations.

use anyhow::{anyhow, Result};
use git2::Repository;
use std::collections::HashSet;
use std::path::Path;

/// Temporal metadata extracted from filesystem and git
#[derive(Debug, Clone)]
pub struct TemporalMetadata {
    pub created_at: i64,
    pub last_modified_at: i64,
    pub change_count: i32,
    pub author_count: i32,
}

/// Extract temporal metadata for a file
///
/// Combines filesystem metadata and git history into complete temporal data.
/// Falls back gracefully if git is not available.
///
/// # Arguments
/// * `file_path` - Absolute path to the file
pub fn extract_temporal_metadata(file_path: &str) -> Result<TemporalMetadata> {
    let path = Path::new(file_path);

    // Extract filesystem metadata
    let (created_at, last_modified_at) = extract_filesystem_temporal(path)?;

    // Try to extract git metadata, fall back to defaults if not in git repo
    let (change_count, author_count) = match extract_git_temporal(path) {
        Ok((changes, authors)) => (changes, authors),
        Err(_) => (1, 1), // Not in git repo or git error - use defaults
    };

    Ok(TemporalMetadata {
        created_at,
        last_modified_at,
        change_count,
        author_count,
    })
}

/// Extract filesystem temporal metadata
///
/// Returns (created_at, last_modified_at) as Unix timestamps.
///
/// # Platform Notes
/// - Linux: Uses ctime (change time) for created_at
/// - macOS: Uses birth time if available, falls back to ctime
/// - Windows: Uses creation time
fn extract_filesystem_temporal(path: &Path) -> Result<(i64, i64)> {
    let metadata = std::fs::metadata(path)?;

    // Get last modified time
    let modified = metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;

    // Get created time (platform-specific)
    let created = match metadata.created() {
        Ok(time) => time.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
        Err(_) => modified, // Fall back to modified if created unavailable
    };

    Ok((created, modified))
}

/// Extract git temporal metadata
///
/// Returns (change_count, author_count) by analyzing git history.
///
/// # Arguments
/// * `path` - Path to the file
///
/// # Returns
/// - change_count: Number of commits that touched this file
/// - author_count: Number of unique authors who committed to this file
fn extract_git_temporal(path: &Path) -> Result<(i32, i32)> {
    // Find the git repository containing this file
    let repo = discover_repository(path)?;

    // Get file path relative to repo root
    let repo_workdir =
        repo.workdir().ok_or_else(|| anyhow!("Repository has no working directory"))?;

    let relative_path = path
        .strip_prefix(repo_workdir)
        .map_err(|_| anyhow!("File not in repository working directory"))?;

    // Walk commit history and count changes
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let mut change_count = 0;
    let mut authors = HashSet::new();

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        // Check if this commit touches our file
        if commit_touches_file(&repo, &commit, relative_path)? {
            change_count += 1;

            // Record author
            let author_name = commit.author().name().unwrap_or("unknown").to_string();
            authors.insert(author_name);
        }
    }

    // If file has no commits yet (newly added), return 1/1
    if change_count == 0 {
        return Ok((1, 1));
    }

    Ok((change_count, authors.len() as i32))
}

/// Discover git repository containing the given path
///
/// Walks up the directory tree to find .git directory.
fn discover_repository(path: &Path) -> Result<Repository> {
    Repository::discover(path).map_err(|e| anyhow!("Not in git repository: {}", e))
}

/// Check if a commit touches a specific file
///
/// Compares the commit's tree with its parent's tree to detect changes.
fn commit_touches_file(repo: &Repository, commit: &git2::Commit, file_path: &Path) -> Result<bool> {
    let commit_tree = commit.tree()?;

    // If commit has no parents (initial commit), check if file exists
    if commit.parent_count() == 0 {
        return Ok(commit_tree.get_path(file_path).is_ok());
    }

    // Compare with parent commit
    let parent = commit.parent(0)?;
    let parent_tree = parent.tree()?;

    let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)?;

    // Check if any delta involves our file
    for delta in diff.deltas() {
        if let Some(old_file) = delta.old_file().path() {
            if old_file == file_path {
                return Ok(true);
            }
        }
        if let Some(new_file) = delta.new_file().path() {
            if new_file == file_path {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_filesystem_temporal() -> Result<()> {
        // Create a temporary file
        let temp_file = "/tmp/test_temporal_fs.txt";
        std::fs::write(temp_file, "test content")?;

        let path = Path::new(temp_file);
        let (created, modified) = extract_filesystem_temporal(path)?;

        // Both timestamps should be recent (within last minute)
        let now =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;

        assert!(created > 0, "Created timestamp should be positive");
        assert!(modified > 0, "Modified timestamp should be positive");
        assert!(now - created < 60, "Created timestamp should be recent (within 60s)");
        assert!(now - modified < 60, "Modified timestamp should be recent (within 60s)");

        std::fs::remove_file(temp_file)?;
        Ok(())
    }

    #[test]
    fn test_extract_temporal_metadata_no_git() -> Result<()> {
        // Create file outside git repo
        let temp_file = "/tmp/test_temporal_nogit.txt";
        std::fs::write(temp_file, "test content")?;

        let metadata = extract_temporal_metadata(temp_file)?;

        assert!(metadata.created_at > 0);
        assert!(metadata.last_modified_at > 0);
        assert_eq!(metadata.change_count, 1, "Should default to 1 without git");
        assert_eq!(metadata.author_count, 1, "Should default to 1 without git");

        std::fs::remove_file(temp_file)?;
        Ok(())
    }
}
