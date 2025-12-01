//! Incremental indexing support for code graph
//!
//! Provides file-level change detection using SHA256 + mtime to minimize
//! re-indexing work. Only changed files are parsed and indexed.

use anyhow::{anyhow, Result};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// Classification of a file's change status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeStatus {
    /// File is new (not previously indexed)
    New,
    /// File has been modified since last index
    Modified,
    /// File is unchanged since last index
    Unchanged,
    /// File was indexed but no longer exists on disk
    Deleted,
}

/// State of an indexed file (stored in file_index_state table)
#[derive(Debug, Clone)]
pub struct FileIndexState {
    pub file_path: String,
    pub sha256: String,
    pub mtime: i64,
    pub last_indexed_at: i64,
    pub status: String,
}

/// Result of classifying files for incremental indexing
#[derive(Debug, Default)]
pub struct IncrementalClassification {
    pub new_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub unchanged_files: Vec<String>,
    pub deleted_files: Vec<String>,
}

/// Compute SHA256 hash of file contents
pub fn compute_file_sha256(path: &Path) -> Result<String> {
    let contents = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Get file mtime as Unix timestamp
pub fn get_file_mtime(path: &Path) -> Result<i64> {
    let metadata = fs::metadata(path)?;
    let mtime = metadata.modified()?;
    let duration = mtime.duration_since(SystemTime::UNIX_EPOCH)?;
    Ok(duration.as_secs() as i64)
}

/// Get stored file state from database
pub fn get_stored_file_state(db: &Connection, file_path: &str) -> Result<Option<FileIndexState>> {
    let result = db.query_row(
        "SELECT file_path, sha256, mtime, last_indexed_at, status FROM file_index_state WHERE file_path = ?",
        [file_path],
        |row| {
            Ok(FileIndexState {
                file_path: row.get(0)?,
                sha256: row.get(1)?,
                mtime: row.get(2)?,
                last_indexed_at: row.get(3)?,
                status: row.get(4)?,
            })
        },
    );
    match result {
        Ok(state) => Ok(Some(state)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Update file state in database (upsert)
pub fn update_file_state(db: &Connection, state: &FileIndexState) -> Result<()> {
    db.execute(
        "INSERT INTO file_index_state (file_path, sha256, mtime, last_indexed_at, status)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(file_path) DO UPDATE SET
             sha256 = excluded.sha256,
             mtime = excluded.mtime,
             last_indexed_at = excluded.last_indexed_at,
             status = excluded.status",
        rusqlite::params![
            &state.file_path,
            &state.sha256,
            state.mtime,
            state.last_indexed_at,
            &state.status
        ],
    )?;
    Ok(())
}

/// Mark file as deleted in database
pub fn mark_file_deleted(db: &Connection, file_path: &str) -> Result<()> {
    let now = std::time::SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs() as i64;
    db.execute(
        "UPDATE file_index_state SET status = 'deleted', last_indexed_at = ? WHERE file_path = ?",
        rusqlite::params![now, file_path],
    )?;
    Ok(())
}

/// Remove file state from database entirely
pub fn remove_file_state(db: &Connection, file_path: &str) -> Result<()> {
    db.execute("DELETE FROM file_index_state WHERE file_path = ?", [file_path])?;
    Ok(())
}

/// Classify a single file's change status
pub fn classify_file(db: &Connection, file_path: &Path) -> Result<FileChangeStatus> {
    let path_str = file_path.to_str().ok_or_else(|| anyhow!("Invalid file path"))?;

    // Check if file exists on disk
    if !file_path.exists() {
        // Check if we have a stored state for it
        if get_stored_file_state(db, path_str)?.is_some() {
            return Ok(FileChangeStatus::Deleted);
        }
        return Err(anyhow!("File does not exist: {}", path_str));
    }

    // Compute current file state
    let current_sha256 = compute_file_sha256(file_path)?;
    let current_mtime = get_file_mtime(file_path)?;

    // Check stored state
    match get_stored_file_state(db, path_str)? {
        None => Ok(FileChangeStatus::New),
        Some(stored) => {
            if stored.sha256 != current_sha256 || stored.mtime != current_mtime {
                Ok(FileChangeStatus::Modified)
            } else {
                Ok(FileChangeStatus::Unchanged)
            }
        }
    }
}

/// Classify multiple files for incremental indexing
pub fn classify_files(db: &Connection, file_paths: &[String]) -> Result<IncrementalClassification> {
    let mut result = IncrementalClassification::default();

    // Build set of files we're checking
    let file_set: std::collections::HashSet<_> = file_paths.iter().cloned().collect();

    // Get all stored file states
    let mut stored_files: HashMap<String, FileIndexState> = HashMap::new();
    let mut stmt = db.prepare(
        "SELECT file_path, sha256, mtime, last_indexed_at, status FROM file_index_state",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(FileIndexState {
            file_path: row.get(0)?,
            sha256: row.get(1)?,
            mtime: row.get(2)?,
            last_indexed_at: row.get(3)?,
            status: row.get(4)?,
        })
    })?;
    for row in rows {
        let state = row?;
        stored_files.insert(state.file_path.clone(), state);
    }

    // Classify each file we're checking
    for file_path in file_paths {
        let path = Path::new(file_path);
        if !path.exists() {
            // File in list but doesn't exist - shouldn't happen normally
            continue;
        }

        match classify_file(db, path)? {
            FileChangeStatus::New => result.new_files.push(file_path.clone()),
            FileChangeStatus::Modified => result.modified_files.push(file_path.clone()),
            FileChangeStatus::Unchanged => result.unchanged_files.push(file_path.clone()),
            FileChangeStatus::Deleted => result.deleted_files.push(file_path.clone()),
        }
    }

    // Find files that were indexed but are no longer in the file list
    for (stored_path, state) in stored_files {
        if state.status != "deleted" && !file_set.contains(&stored_path) {
            // Check if file actually exists on disk
            if !Path::new(&stored_path).exists() {
                result.deleted_files.push(stored_path);
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_db() -> Result<(TempDir, Connection)> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let db = Connection::open(&db_path)?;
        db.execute_batch(
            r#"
            CREATE TABLE file_index_state (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL UNIQUE,
                sha256 TEXT NOT NULL,
                mtime INTEGER NOT NULL,
                last_indexed_at INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'ok'
            );
            "#,
        )?;
        Ok((temp_dir, db))
    }

    #[test]
    fn test_compute_sha256() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "hello world")?;

        let hash = compute_file_sha256(&file_path)?;
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hex is 64 chars
        Ok(())
    }

    #[test]
    fn test_classify_new_file() -> Result<()> {
        let (temp_dir, db) = setup_test_db()?;
        let file_path = temp_dir.path().join("new.rs");
        fs::write(&file_path, "pub fn new() {}")?;

        let status = classify_file(&db, &file_path)?;
        assert_eq!(status, FileChangeStatus::New);
        Ok(())
    }

    #[test]
    fn test_classify_unchanged_file() -> Result<()> {
        let (temp_dir, db) = setup_test_db()?;
        let file_path = temp_dir.path().join("unchanged.rs");
        fs::write(&file_path, "pub fn unchanged() {}")?;

        // Store initial state
        let sha256 = compute_file_sha256(&file_path)?;
        let mtime = get_file_mtime(&file_path)?;
        let state = FileIndexState {
            file_path: file_path.to_str().unwrap().to_string(),
            sha256,
            mtime,
            last_indexed_at: 12345,
            status: "ok".to_string(),
        };
        update_file_state(&db, &state)?;

        // Classify should be unchanged
        let status = classify_file(&db, &file_path)?;
        assert_eq!(status, FileChangeStatus::Unchanged);
        Ok(())
    }

    #[test]
    fn test_classify_modified_file() -> Result<()> {
        let (temp_dir, db) = setup_test_db()?;
        let file_path = temp_dir.path().join("modified.rs");
        fs::write(&file_path, "pub fn original() {}")?;

        // Store initial state
        let sha256 = compute_file_sha256(&file_path)?;
        let mtime = get_file_mtime(&file_path)?;
        let state = FileIndexState {
            file_path: file_path.to_str().unwrap().to_string(),
            sha256,
            mtime,
            last_indexed_at: 12345,
            status: "ok".to_string(),
        };
        update_file_state(&db, &state)?;

        // Modify the file
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&file_path, "pub fn modified() {}")?;

        // Classify should be modified
        let status = classify_file(&db, &file_path)?;
        assert_eq!(status, FileChangeStatus::Modified);
        Ok(())
    }
}
