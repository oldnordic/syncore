use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};
use sled::Db;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Memory {
    db: Arc<Mutex<Connection>>,
    cache: Arc<Db>,
}

impl Clone for Memory {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            cache: Arc::clone(&self.cache),
        }
    }
}

impl Memory {
    /// Create Memory using an existing database connection from DbManager.
    ///
    /// This is the preferred constructor when using DbManager. It reuses long-lived
    /// connections instead of creating new ones per-call.
    ///
    /// # Arguments
    ///
    /// * `db` - Arc<Mutex<Connection>> from DbManager.main_conn()
    /// * `cache_path` - Path for sled cache (typically derived from DB path)
    ///
    /// # Example
    ///
    /// ```rust
    /// let db_manager = DbManager::new("syncore.db", "syncore_code_graph.db")?;
    /// let memory = Memory::with_connection(
    ///     db_manager.main_conn(),
    ///     "syncore.db_cache"
    /// )?;
    /// ```
    pub fn with_connection(db: Arc<Mutex<Connection>>, cache_path: &str) -> Result<Self> {
        // Try to open cache, if corrupted, clean it and try again
        let cache = match sled::open(cache_path) {
            Ok(cache) => cache,
            Err(e) => {
                // Check if it's a corruption error and clean up if needed
                if e.to_string().contains("corrupted") || e.to_string().contains("offset None") {
                    eprintln!("Warning: Cache corrupted, attempting cleanup: {}", e);
                    // Remove corrupted cache directory
                    if let Err(cleanup_err) = std::fs::remove_dir_all(cache_path) {
                        eprintln!("Failed to remove corrupted cache: {}", cleanup_err);
                    }
                    // Try to create fresh cache
                    sled::open(cache_path).map_err(|err| {
                        anyhow::anyhow!("Failed to create fresh cache after cleanup: {}", err)
                    })?
                } else {
                    return Err(anyhow::anyhow!("Failed to open cache: {}", e));
                }
            }
        };

        Ok(Self {
            db,
            cache: Arc::new(cache),
        })
    }

    /// Legacy constructor - opens its own connection (deprecated, use with_connection instead).
    ///
    /// This method is kept for backward compatibility with existing code that hasn't
    /// been refactored to use DbManager yet.
    pub fn new(db_path: &str) -> Result<Self> {
        crate::db::ensure_schema(db_path)?;

        let conn = crate::db::open_db_with_wal(db_path)?;

        // Create unique cache directory based on the database path
        let cache_path = format!("{}_cache", db_path);

        Self::with_connection(Arc::new(Mutex::new(conn)), &cache_path)
    }

    pub fn store(&self, key: &str, value: &str) -> Result<()> {
        let db = self.db.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        db.execute(
            "INSERT OR REPLACE INTO memory (k, v, ts) VALUES (?1, ?2, ?3)",
            (key, value, now),
        )?;

        drop(db); // Release lock before cache operation
        self.cache.insert(key, value.as_bytes())?;
        self.cache.flush()?;

        Ok(())
    }

    pub fn query(&self, key: &str) -> Result<Option<String>> {
        // Try cache first
        if let Some(v) = self.cache.get(key)? {
            return Ok(Some(String::from_utf8(v.to_vec())?));
        }

        // Fallback to database
        let db = self.db.lock().unwrap();
        let value = db
            .query_row("SELECT v FROM memory WHERE k=?1", [key], |r| {
                r.get::<_, String>(0)
            })
            .optional()?;

        Ok(value)
    }

    pub fn query_with_timestamp(&self, key: &str) -> Result<Option<(String, i64)>> {
        let db = self.db.lock().unwrap();
        let result = db
            .query_row("SELECT v, ts FROM memory WHERE k=?1", [key], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .optional()?;

        Ok(result)
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute("DELETE FROM memory WHERE k=?1", [key])?;

        drop(db);
        self.cache.remove(key)?;
        self.cache.flush()?;

        Ok(())
    }

    pub fn list_keys(&self, limit: Option<i64>) -> Result<Vec<String>> {
        let db = self.db.lock().unwrap();
        let mut query = "SELECT k FROM memory ORDER BY ts DESC".to_string();

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = db.prepare(&query)?;
        let keys = stmt.query_map([], |row| row.get(0))?;

        let mut result = Vec::new();
        for key in keys {
            result.push(key?);
        }

        Ok(result)
    }
}
