//! SQLite Connection Manager (DbManager)
//!
//! Provides centralized, long-lived SQLite connections for all databases used by SynCore.
//! This eliminates the "open connection per call" anti-pattern that caused persistence failures
//! with WAL mode.
//!
//! ## Architecture
//!
//! - **Two long-lived connections**:
//!   - `main_db`: syncore.db (memory, tasks, embeddings, steps, etc.)
//!   - `code_graph_db`: syncore_code_graph.db (code entities, edges)
//!
//! - **Journal mode: WAL**:
//!   - WAL requires long-lived connections to function correctly
//!   - Short-lived connections cause "commit succeeds but data vanishes" bug
//!   - With long-lived connections, WAL checkpoints happen naturally
//!
//! - **Thread safety**:
//!   - Connections wrapped in `Arc<Mutex<Connection>>`
//!   - SQLite serializes writers automatically
//!   - Multiple readers allowed concurrently in WAL mode
//!
//! ## Usage
//!
//! ```rust
//! let db_manager = DbManager::new("syncore.db", "syncore_code_graph.db")?;
//!
//! // Get main database connection
//! let main_conn = db_manager.main_conn();
//! let conn_lock = main_conn.lock().unwrap();
//! conn_lock.execute("INSERT INTO memory (k, v, ts) VALUES (?, ?, ?)", params![...])?;
//!
//! // Get code graph database connection
//! let code_graph_conn = db_manager.code_graph_conn();
//! // Pass to CodeGraph::with_connection(...)
//! ```

use crate::schema_migration;
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// SQLite connection manager for SynCore databases
///
/// Manages long-lived connections to both main and code_graph databases.
/// Ensures proper WAL configuration and schema initialization.
pub struct DbManager {
    /// Main database connection (syncore.db)
    /// Contains: memory, tasks, task_links, steps, embeddings, etc.
    main_db: Arc<Mutex<Connection>>,

    /// Code graph database connection (syncore_code_graph.db)
    /// Contains: code_entities, code_edges, etc.
    code_graph_db: Arc<Mutex<Connection>>,
}

impl DbManager {
    /// Create a new DbManager with initialized connections
    ///
    /// # Arguments
    ///
    /// * `main_db_path` - Path to main database file (e.g., "syncore.db")
    /// * `code_graph_db_path` - Path to code graph database file (e.g., "syncore_code_graph.db")
    ///
    /// # Returns
    ///
    /// `Result<DbManager>` - Initialized manager with both connections ready
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Database files cannot be opened
    /// - WAL mode configuration fails
    /// - Schema migrations fail
    pub fn new(main_db_path: &str, code_graph_db_path: &str) -> Result<Self> {
        // Initialize main database
        let main_db =
            Self::init_connection(main_db_path).context("Failed to initialize main database")?;

        // Initialize code graph database
        let code_graph_db = Self::init_connection(code_graph_db_path)
            .context("Failed to initialize code graph database")?;

        Ok(Self {
            main_db: Arc::new(Mutex::new(main_db)),
            code_graph_db: Arc::new(Mutex::new(code_graph_db)),
        })
    }

    /// Initialize a single database connection with proper configuration
    ///
    /// This function:
    /// 1. Opens SQLite connection
    /// 2. Configures WAL mode (critical for long-lived connections)
    /// 3. Sets synchronous=NORMAL (balance between safety and performance)
    /// 4. Enables foreign keys
    /// 5. Runs schema migrations
    ///
    /// # WAL Mode Design Decision
    ///
    /// We use WAL (Write-Ahead Logging) because:
    /// - Better concurrency: readers don't block writers
    /// - Better performance for write-heavy workloads
    /// - Natural checkpoint behavior with long-lived connections
    ///
    /// Previous bug: Creating new connections per-call caused WAL frames to be
    /// discarded on connection close, leading to "commit succeeds but data vanishes".
    ///
    /// Solution: Long-lived connections (this DbManager) ensure WAL checkpoints
    /// happen properly, and committed data persists to disk.
    fn init_connection(db_path: &str) -> Result<Connection> {
        // Open connection
        let db = Connection::open(db_path)
            .with_context(|| format!("Failed to open database: {}", db_path))?;

        // Configure WAL mode (must be done BEFORE any writes)
        // Skip WAL for in-memory databases (they don't support it)
        if db_path != ":memory:" {
            db.pragma_update(None, "journal_mode", &"WAL")
                .context("Failed to set journal_mode=WAL")?;

            // Verify WAL mode was set successfully
            let journal_mode: String = db
                .pragma_query_value(None, "journal_mode", |row| row.get(0))
                .context("Failed to query journal_mode")?;

            if journal_mode.to_lowercase() != "wal" {
                anyhow::bail!("Failed to enable WAL mode, got: {}", journal_mode);
            }
        }

        // Configure other pragmas
        db.pragma_update(None, "synchronous", &"NORMAL")
            .context("Failed to set synchronous=NORMAL")?;

        db.pragma_update(None, "cache_size", &1000)
            .context("Failed to set cache_size")?;

        db.pragma_update(None, "foreign_keys", &"ON")
            .context("Failed to enable foreign keys")?;

        // Run schema migrations (critical: schema must match code expectations)
        schema_migration::run_migrations(&db).context("Failed to run schema migrations")?;

        Ok(db)
    }

    /// Get main database connection (syncore.db)
    ///
    /// Returns Arc<Mutex<Connection>> for thread-safe shared access.
    /// Multiple callers can hold references; Mutex ensures safe concurrent access.
    pub fn main_conn(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.main_db)
    }

    /// Get code graph database connection (syncore_code_graph.db)
    ///
    /// Returns Arc<Mutex<Connection>> for thread-safe shared access.
    /// Multiple callers can hold references; Mutex ensures safe concurrent access.
    pub fn code_graph_conn(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.code_graph_db)
    }

    /// Explicitly checkpoint both databases
    ///
    /// Normally, WAL checkpoints happen automatically. This method allows
    /// explicit checkpointing (useful for testing or shutdown).
    ///
    /// # Checkpoint Modes
    ///
    /// - PASSIVE: Checkpoint as much as possible without blocking
    /// - FULL: Block until checkpoint completes
    /// - RESTART: Checkpoint and reset WAL file
    /// - TRUNCATE: Checkpoint and truncate WAL to zero bytes
    ///
    /// We use RESTART by default (checkpoint + reset, but don't truncate file).
    pub fn checkpoint(&self) -> Result<()> {
        {
            let main_lock = self
                .main_db
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock main_db: {}", e))?;

            main_lock
                .execute_batch("PRAGMA wal_checkpoint(RESTART)")
                .context("Failed to checkpoint main_db")?;
        }

        {
            let code_graph_lock = self
                .code_graph_db
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock code_graph_db: {}", e))?;

            code_graph_lock
                .execute_batch("PRAGMA wal_checkpoint(RESTART)")
                .context("Failed to checkpoint code_graph_db")?;
        }

        Ok(())
    }
}

impl Drop for DbManager {
    /// Ensure clean shutdown with explicit checkpoint
    ///
    /// When DbManager is dropped (application shutdown), we explicitly
    /// checkpoint both databases to ensure all WAL frames are merged.
    fn drop(&mut self) {
        // Best-effort checkpoint on drop
        // Ignore errors since we're already shutting down
        let _ = self.checkpoint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_db_manager_basic_initialization() {
        let test_main = "/tmp/dbmanager_test_main.db";
        let test_code_graph = "/tmp/dbmanager_test_code_graph.db";

        // Cleanup
        let _ = std::fs::remove_file(test_main);
        let _ = std::fs::remove_file(test_code_graph);

        // Create DbManager
        let manager =
            DbManager::new(test_main, test_code_graph).expect("Failed to create DbManager");

        // Verify files exist
        assert!(Path::new(test_main).exists());
        assert!(Path::new(test_code_graph).exists());

        // Verify connections work
        {
            let main_conn = manager.main_conn();
            let lock = main_conn.lock().unwrap();
            let result: i32 = lock
                .query_row("SELECT 1", [], |row| row.get(0))
                .expect("Failed to query main_db");
            assert_eq!(result, 1);
        }

        {
            let code_graph_conn = manager.code_graph_conn();
            let lock = code_graph_conn.lock().unwrap();
            let result: i32 = lock
                .query_row("SELECT 1", [], |row| row.get(0))
                .expect("Failed to query code_graph_db");
            assert_eq!(result, 1);
        }

        // Cleanup
        drop(manager);
        let _ = std::fs::remove_file(test_main);
        let _ = std::fs::remove_file(test_code_graph);
    }

    #[test]
    fn test_db_manager_checkpoint() {
        let test_main = "/tmp/dbmanager_test_checkpoint.db";
        let test_code_graph = "/tmp/dbmanager_test_checkpoint_cg.db";

        let _ = std::fs::remove_file(test_main);
        let _ = std::fs::remove_file(test_code_graph);

        let manager =
            DbManager::new(test_main, test_code_graph).expect("Failed to create DbManager");

        // Write some data
        {
            let main_conn = manager.main_conn();
            let lock = main_conn.lock().unwrap();
            lock.execute(
                "INSERT INTO memory (k, v, ts) VALUES (?, ?, ?)",
                rusqlite::params!["test_key", "test_value", 123],
            )
            .expect("Failed to insert");
        }

        // Explicit checkpoint should succeed
        manager.checkpoint().expect("Checkpoint failed");

        // Data should persist
        {
            let main_conn = manager.main_conn();
            let lock = main_conn.lock().unwrap();
            let count: i64 = lock
                .query_row(
                    "SELECT COUNT(*) FROM memory WHERE k = ?",
                    ["test_key"],
                    |row| row.get(0),
                )
                .expect("Failed to count");
            assert_eq!(count, 1);
        }

        // Cleanup
        drop(manager);
        let _ = std::fs::remove_file(test_main);
        let _ = std::fs::remove_file(test_code_graph);
    }
}
