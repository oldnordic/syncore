//! Main CodeGraph struct and constructors

use crate::parser::Parser;
use crate::vector::VectorStore;
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Main code graph structure for indexing and searching code
pub struct CodeGraph {
    pub(super) db: Arc<Mutex<Connection>>,
    pub(super) vector_store: Arc<Mutex<VectorStore>>,
    pub(super) parser: Parser,
}

impl CodeGraph {
    /// Create a new CodeGraph instance
    pub fn new(db_path: &str, vector_store: Arc<Mutex<VectorStore>>) -> Result<Self> {
        // FIX 1: Reject :memory: database to prevent accidental loss of persisted data
        if db_path == ":memory:" {
            return Err(anyhow!(
                "CodeGraph cannot use :memory: database. Use persistent file path instead. \
                 This prevents accidental data loss and ensures SQLite/Neo4j/VectorStore sync."
            ));
        }

        // Write trace to file since eprintln doesn't work in MCP handlers
        let _ = std::fs::write(
            "/tmp/code_graph_trace.log",
            format!(
                "[TRACE] CodeGraph::new - Opening database at: {}\n[TRACE] Absolute path: {:?}\n",
                db_path,
                std::fs::canonicalize(db_path)
                    .unwrap_or_else(|_| std::path::PathBuf::from("NOT_FOUND"))
            ),
        );

        // Ensure schema exists (both core and code_graph tables)
        crate::db::ensure_schema(db_path)?;

        // Open database with WAL mode
        let db = crate::db::open_db_with_wal(db_path)?;

        // STEP 1: Verify actual database file being used
        let db_list: Vec<(i32, String, String)> = db
            .prepare("SELECT seq, name, file FROM pragma_database_list()")
            .and_then(|mut stmt| {
                let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
                rows.collect()
            })?;

        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/code_graph_diagnostic.log")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "=== STEP 1: Database File Verification ===")?;
                writeln!(f, "Requested path: {}", db_path)?;
                for (seq, name, file) in &db_list {
                    writeln!(f, "  DB[{}] name='{}' file='{}'", seq, name, file)?;
                }
                Ok(())
            });

        eprintln!("[TRACE] CodeGraph::new - Database opened successfully");

        // Double-check that code_graph schema exists (for test environments)
        // This is a safety net in case include_str! paths don't work in tests
        Self::ensure_code_graph_schema(&db)?;

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            vector_store,
            parser: Parser::new()?,
        })
    }

    /// Create a CodeGraph instance using an existing database connection from DbManager
    ///
    /// This is the preferred constructor when using DbManager. It reuses long-lived
    /// connections instead of creating new ones per-call.
    ///
    /// # Arguments
    ///
    /// * `db` - Arc<Mutex<Connection>> from DbManager.code_graph_conn()
    /// * `vector_store` - Arc<Mutex<VectorStore>> for embedding management
    ///
    /// # Example
    ///
    /// ```rust
    /// let db_manager = DbManager::new("syncore.db", "syncore_code_graph.db")?;
    /// let code_graph_conn = db_manager.code_graph_conn();
    /// let code_graph = CodeGraph::with_connection(code_graph_conn, vector_store)?;
    /// ```
    pub fn with_connection(
        db: Arc<Mutex<Connection>>,
        vector_store: Arc<Mutex<VectorStore>>,
    ) -> Result<Self> {
        // FIX 1: Detect if connection is :memory: and reject it
        {
            let conn_lock = db
                .lock()
                .map_err(|e| anyhow!("Failed to lock database connection: {}", e))?;

            // Check database file path via pragma_database_list
            let db_file: String = conn_lock
                .query_row(
                    "SELECT file FROM pragma_database_list() WHERE name='main'",
                    [],
                    |row| row.get(0),
                )?;

            if db_file.is_empty() || db_file == "" {
                return Err(anyhow!(
                    "CodeGraph detected :memory: database connection. \
                     Use persistent file path instead to prevent data loss."
                ));
            }

            Self::ensure_code_graph_schema(&*conn_lock)?;
        }

        Ok(Self {
            db,
            vector_store,
            parser: Parser::new()?,
        })
    }

    /// Ensure code_graph schema exists (fallback for test environments)
    fn ensure_code_graph_schema(db: &Connection) -> Result<()> {
        // Check if code_entities table exists
        let mut stmt = db.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='code_entities'",
        )?;
        let has_table = stmt.exists([])?;

        if !has_table {
            // Create code_graph schema inline
            db.execute_batch(
                r#"
                PRAGMA foreign_keys=ON;

                CREATE TABLE IF NOT EXISTS code_entities (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    file_path TEXT NOT NULL,
                    entity_type TEXT NOT NULL,
                    name TEXT NOT NULL,
                    signature TEXT,
                    line_start INTEGER NOT NULL,
                    line_end INTEGER NOT NULL,
                    docstring TEXT,
                    language TEXT NOT NULL,
                    indexed_at INTEGER NOT NULL,
                    UNIQUE(file_path, entity_type, name, line_start)
                );

                CREATE INDEX IF NOT EXISTS idx_entities_name ON code_entities(name);
                CREATE INDEX IF NOT EXISTS idx_entities_file ON code_entities(file_path);
                CREATE INDEX IF NOT EXISTS idx_entities_type ON code_entities(entity_type);
                CREATE INDEX IF NOT EXISTS idx_entities_lang ON code_entities(language);

                CREATE TABLE IF NOT EXISTS code_edges (
                    src_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
                    dst_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
                    edge_type TEXT NOT NULL,
                    PRIMARY KEY (src_entity_id, dst_entity_id, edge_type)
                );

                CREATE INDEX IF NOT EXISTS idx_edges_src ON code_edges(src_entity_id);
                CREATE INDEX IF NOT EXISTS idx_edges_dst ON code_edges(dst_entity_id);
                CREATE INDEX IF NOT EXISTS idx_edges_type ON code_edges(edge_type);

                CREATE TABLE IF NOT EXISTS code_embeddings (
                    entity_id INTEGER PRIMARY KEY REFERENCES code_entities(id) ON DELETE CASCADE,
                    vector_id INTEGER NOT NULL,
                    model_version TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_code_embeddings_vector ON code_embeddings(vector_id);
            "#,
            )?;
        }

        Ok(())
    }

    /// Get reference to database connection (for tests and neo4j sync)
    pub fn db_conn(&self) -> &Arc<Mutex<Connection>> {
        &self.db
    }
}
