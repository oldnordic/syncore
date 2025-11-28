//! Main CodeGraph struct and constructors

use crate::graph::Neo4jClient;
use crate::parser::Parser;
use crate::vector::VectorStore;
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Type alias for entity query rows from code_embeddings join
type EntityQueryRow = (i64, String, String, Option<String>, Option<String>);

/// Main code graph structure for indexing and searching code
pub struct CodeGraph {
    pub(super) db: Arc<Mutex<Connection>>,
    pub(super) vector_store: Arc<Mutex<VectorStore>>,
    pub(super) parser: Parser,
    /// PHASE 2: Optional Neo4j client for dual-write persistence
    pub(super) neo4j: Option<Arc<Neo4jClient>>,
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
            neo4j: None,
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
            let db_file: String = conn_lock.query_row(
                "SELECT file FROM pragma_database_list() WHERE name='main'",
                [],
                |row| row.get(0),
            )?;

            if db_file.is_empty() {
                return Err(anyhow!(
                    "CodeGraph detected :memory: database connection. \
                     Use persistent file path instead to prevent data loss."
                ));
            }

            Self::ensure_code_graph_schema(&conn_lock)?;
        }

        Ok(Self {
            db,
            vector_store,
            parser: Parser::new()?,
            neo4j: None,
        })
    }

    /// Create a new CodeGraph instance with Neo4j support (PHASE 2)
    ///
    /// This constructor enables dual-write persistence to both SQLite and Neo4j.
    /// Semantic edges will be persisted to both stores automatically.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to SQLite database file (NOT :memory:)
    /// * `vector_store` - Arc<Mutex<VectorStore>> for embedding management
    /// * `neo4j` - Arc<Neo4jClient> for graph database operations
    pub fn new_with_neo4j(
        db_path: &str,
        vector_store: Arc<Mutex<VectorStore>>,
        neo4j: Arc<Neo4jClient>,
    ) -> Result<Self> {
        // Reject :memory: database
        if db_path == ":memory:" {
            return Err(anyhow!(
                "CodeGraph cannot use :memory: database. Use persistent file path instead."
            ));
        }

        // Ensure schema exists
        crate::db::ensure_schema(db_path)?;

        // Open database with WAL mode
        let db = crate::db::open_db_with_wal(db_path)?;

        // Ensure code_graph schema
        Self::ensure_code_graph_schema(&db)?;

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            vector_store,
            parser: Parser::new()?,
            neo4j: Some(neo4j),
        })
    }

    /// Get reference to Neo4j client (PHASE 2)
    ///
    /// Returns the Neo4j client if available, otherwise returns error.
    /// This is used by dual-write persistence methods.
    pub fn neo4j_client(&self) -> Result<&Arc<Neo4jClient>> {
        self.neo4j
            .as_ref()
            .ok_or_else(|| anyhow!("Neo4j client not available. Use new_with_neo4j() constructor."))
    }

    /// Get database connection for testing purposes
    ///
    /// This is a test-only helper to allow verification of schema state.
    /// Should only be used in test code.
    pub fn db_for_testing(&self) -> &Arc<Mutex<Connection>> {
        &self.db
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
                    created_at INTEGER,
                    last_modified_at INTEGER,
                    change_count INTEGER,
                    author_count INTEGER,
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

        // PHASE 3: Migrate existing databases to add temporal columns if missing
        let has_created_at: bool = db
            .prepare("SELECT created_at FROM code_entities LIMIT 1")
            .is_ok();

        if !has_created_at {
            db.execute_batch(
                r#"
                ALTER TABLE code_entities ADD COLUMN created_at INTEGER;
                ALTER TABLE code_entities ADD COLUMN last_modified_at INTEGER;
                ALTER TABLE code_entities ADD COLUMN change_count INTEGER;
                ALTER TABLE code_entities ADD COLUMN author_count INTEGER;
                "#,
            )?;
        }

        // APEX v1.7 Phase 3: Add body_snippet column for function body indexing
        let has_body_snippet: bool = db
            .prepare("SELECT body_snippet FROM code_entities LIMIT 1")
            .is_ok();

        if !has_body_snippet {
            db.execute_batch(
                r#"
                ALTER TABLE code_entities ADD COLUMN body_snippet TEXT;
                "#,
            )?;
        }

        // PHASE 4: Add file_index_state table for incremental indexing
        let has_file_index_state: bool =
            db.prepare("SELECT 1 FROM file_index_state LIMIT 1").is_ok();

        if !has_file_index_state {
            db.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS file_index_state (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    file_path TEXT NOT NULL UNIQUE,
                    sha256 TEXT NOT NULL,
                    mtime INTEGER NOT NULL,
                    last_indexed_at INTEGER NOT NULL,
                    status TEXT NOT NULL DEFAULT 'ok'
                );

                CREATE INDEX IF NOT EXISTS idx_file_index_state_path ON file_index_state(file_path);
                "#,
            )?;
        }

        // Add code_macro_expansions table for Rust macro tracking
        let has_macro_expansions: bool =
            db.prepare("SELECT 1 FROM code_macro_expansions LIMIT 1").is_ok();

        if !has_macro_expansions {
            db.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS code_macro_expansions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    file_path TEXT NOT NULL,
                    macro_name TEXT NOT NULL,
                    span_start INTEGER NOT NULL,
                    span_end INTEGER NOT NULL,
                    original_code TEXT,
                    expanded_code TEXT,
                    expansion_type TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_macro_expansions_file ON code_macro_expansions(file_path);
                "#,
            )?;
        }

        Ok(())
    }

    /// Get reference to database connection (for tests and neo4j sync)
    pub fn db_conn(&self) -> &Arc<Mutex<Connection>> {
        &self.db
    }

    /// Perform multi-hop BFS traversal from a starting entity (PHASE 4)
    ///
    /// Traverses the code graph using BFS with depth limiting, cycle detection,
    /// and branch limiting. If Neo4j is available, unions neighbors from both
    /// SQLite and Neo4j.
    ///
    /// # Arguments
    /// * `entity_id` - Starting entity ID from code_entities table
    /// * `max_depth` - Maximum traversal depth (0 = just start node)
    ///
    /// # Returns
    /// MultiHopResult with all discovered nodes, sorted by depth then by id
    ///
    /// # Example
    /// ```rust,no_run
    /// # use syncore::code_graph::CodeGraph;
    /// # use syncore::vector::VectorStore;
    /// # use std::sync::{Arc, Mutex};
    /// # fn main() -> anyhow::Result<()> {
    /// # let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(syncore::vector::HuggingFaceEmbeddings::new()?))));
    /// # let code_graph = CodeGraph::new("test.db", vector_store)?;
    /// let result = tokio::runtime::Runtime::new()?.block_on(async {
    ///     code_graph.multi_hop(123, 2).await
    /// })?;
    /// for node in &result.nodes {
    ///     println!("Entity {} at depth {}", node.id, node.depth);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn multi_hop(
        &self,
        entity_id: i64,
        max_depth: usize,
    ) -> Result<super::multi_hop::MultiHopResult> {
        let db = self
            .db
            .lock()
            .map_err(|e| anyhow!("Failed to lock database: {}", e))?;

        let neo4j_ref = self.neo4j.as_ref().map(|arc| arc.as_ref());

        super::multi_hop::multi_hop(&db, neo4j_ref, entity_id, max_depth).await
    }

    /// Enrich all entities with temporal metadata (TASK A)
    ///
    /// For all code_entities rows where temporal fields are NULL,
    /// extract metadata from filesystem + git and update both SQLite and Neo4j.
    ///
    /// # Returns
    /// Number of entities enriched
    pub async fn enrich_temporal_metadata_for_all(&self) -> Result<usize> {
        let db = self
            .db
            .lock()
            .map_err(|e| anyhow!("Failed to lock database: {}", e))?;

        // Find entities with null temporal metadata
        let mut stmt = db.prepare(
            "SELECT id, file_path FROM code_entities
             WHERE created_at IS NULL
                OR last_modified_at IS NULL
                OR change_count IS NULL
                OR author_count IS NULL",
        )?;

        let entities_to_enrich: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut enriched_count = 0;

        for (entity_id, file_path) in entities_to_enrich {
            // Extract temporal metadata using existing Phase 3 module
            let temporal = match super::temporal_extractor::extract_temporal_metadata(&file_path) {
                Ok(t) => t,
                Err(_) => {
                    // If extraction fails, use defaults
                    super::temporal_extractor::TemporalMetadata {
                        created_at: 0,
                        last_modified_at: 0,
                        change_count: 1,
                        author_count: 1,
                    }
                }
            };

            // Update SQLite
            db.execute(
                "UPDATE code_entities
                 SET created_at = ?1, last_modified_at = ?2, change_count = ?3, author_count = ?4
                 WHERE id = ?5",
                rusqlite::params![
                    temporal.created_at,
                    temporal.last_modified_at,
                    temporal.change_count,
                    temporal.author_count,
                    entity_id
                ],
            )?;

            // Update Neo4j if available - use canonical update function
            if let Some(ref neo4j) = self.neo4j {
                use crate::databases::neo4j::update_git_metadata;

                update_git_metadata(
                    neo4j,
                    entity_id,
                    Some(temporal.created_at.to_string()),
                    Some(temporal.last_modified_at.to_string()),
                    Some(temporal.change_count as i64),
                    Some(temporal.author_count as i64),
                )
                .await?;
            }

            enriched_count += 1;
        }

        Ok(enriched_count)
    }

    /// Rebuild HNSW index from all indexed entities in SQLite
    ///
    /// This method is called at startup to ensure the in-memory HNSW index
    /// is populated from persisted entity data. It:
    /// 1. Queries all entities that have embeddings
    /// 2. Regenerates embedding text from entity metadata
    /// 3. Inserts each embedding into the HNSW index
    /// 4. Saves snapshot ONCE at the end (not per insert!)
    ///
    /// Returns the number of vectors loaded into HNSW.
    pub fn rebuild_hnsw_from_entities(&self) -> Result<usize> {
        // Query entities - collect fully before releasing db lock
        let entities: Vec<EntityQueryRow> = {
            let db = self
                .db
                .lock()
                .map_err(|e| anyhow!("Failed to lock database: {}", e))?;

            // Query all entities that have embeddings
            let mut stmt = db.prepare(
                "SELECT ce.entity_id, e.entity_type, e.name, e.signature, e.docstring
                 FROM code_embeddings ce
                 JOIN code_entities e ON ce.entity_id = e.id
                 ORDER BY ce.entity_id",
            )?;

            let mut rows = stmt.query([])?;
            let mut results = Vec::new();
            while let Some(row) = rows.next()? {
                results.push((
                    row.get(0)?, // entity_id
                    row.get(1)?, // entity_type
                    row.get(2)?, // name
                    row.get(3)?, // signature
                    row.get(4)?, // docstring
                ));
            }
            results
        }; // db lock released here

        if entities.is_empty() {
            eprintln!("[SynCore] No entities found in code_embeddings, HNSW index empty");
            return Ok(0);
        }

        let count = entities.len();
        eprintln!("[SynCore] Rebuilding HNSW index ({} entities)...", count);

        // Lock vector store and insert all entities using NO-SNAPSHOT version
        let mut vector_store = self
            .vector_store
            .lock()
            .map_err(|e| anyhow!("Failed to lock vector store: {}", e))?;

        for (i, (entity_id, entity_type, name, signature, docstring)) in entities.iter().enumerate()
        {
            // Progress logging every 1000 entities
            if i > 0 && i % 1000 == 0 {
                eprintln!("[SynCore] HNSW rebuild progress: {}/{}", i, count);
            }

            // Reconstruct embedding text (same format as indexer)
            let mut parts = vec![entity_type.clone(), name.clone()];
            if let Some(sig) = signature {
                parts.push(sig.clone());
            }
            if let Some(doc) = docstring {
                parts.push(doc.clone());
            }
            let text = parts.join(" ");

            // Insert WITHOUT saving snapshot (critical for performance!)
            if let Err(e) =
                vector_store.insert_text_no_snapshot(*entity_id, None, &text, "code_entity")
            {
                eprintln!(
                    "[SynCore] Warning: Failed to insert entity {} into HNSW: {}",
                    entity_id, e
                );
            }
        }

        // Save snapshot ONCE after all inserts complete
        if let Err(e) = vector_store.save_snapshot() {
            eprintln!("[SynCore] Warning: Failed to save HNSW snapshot: {}", e);
        }

        eprintln!("[SynCore] Rebuilt HNSW index ({} vectors)", count);
        Ok(count)
    }

    /// Delete all entities for a given file path (APEX 2.6-CG-GRAPH-DELTA)
    ///
    /// This is used by the delta engine when a file is deleted or renamed.
    /// Cascading deletes will remove associated edges automatically.
    pub fn delete_entities_by_path(&self, file_path: &Path) -> Result<usize> {
        let path_str = file_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid UTF-8 in file path"))?;

        let db = self
            .db
            .lock()
            .map_err(|e| anyhow!("Failed to lock db: {}", e))?;

        let deleted = db.execute(
            "DELETE FROM code_entities WHERE file_path = ?",
            [path_str],
        )?;

        Ok(deleted)
    }

    /// Extract graph structural features for an entity (for GraphBERT/GRAPH embeddings)
    ///
    /// Queries the code_edges table to compute:
    /// - degree_in: Count of incoming edges (entities referencing this one)
    /// - degree_out: Count of outgoing edges (entities this one references)
    /// - edge_types: Distribution of edge types (CALLS, DEFINES, IMPORTS, USES)
    ///
    /// # Arguments
    /// * `entity_id` - The entity ID to extract features for
    ///
    /// # Returns
    /// GraphFeatures struct with degree counts and edge type distribution
    pub fn extract_graph_features(&self, entity_id: i64) -> Result<super::graph_embeddings::GraphFeatures> {
        let db = self
            .db
            .lock()
            .map_err(|e| anyhow!("Failed to lock db: {}", e))?;

        // Query incoming edges (degree_in)
        let degree_in: u32 = db.query_row(
            "SELECT COUNT(*) FROM code_edges WHERE dst_entity_id = ?",
            [entity_id],
            |row| row.get::<_, i64>(0),
        )? as u32;

        // Query outgoing edges (degree_out)
        let degree_out: u32 = db.query_row(
            "SELECT COUNT(*) FROM code_edges WHERE src_entity_id = ?",
            [entity_id],
            |row| row.get::<_, i64>(0),
        )? as u32;

        // Query edge type distribution for outgoing edges
        let mut edge_types = std::collections::HashMap::new();
        let mut stmt = db.prepare(
            "SELECT edge_type, COUNT(*) as count FROM code_edges
             WHERE src_entity_id = ?
             GROUP BY edge_type",
        )?;

        let rows = stmt.query_map([entity_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u32))
        })?;

        for result in rows {
            let (edge_type, count) = result?;
            edge_types.insert(edge_type, count);
        }

        Ok(super::graph_embeddings::GraphFeatures {
            degree_in,
            degree_out,
            edge_types,
        })
    }
}
