use rusqlite::{Connection, OptionalExtension};
use sled::Db;
use std::sync::{Arc, Mutex};
use anyhow::Result;

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
    pub fn new(db_path: &str) -> Result<Self> {
        crate::db::ensure_schema(db_path)?;

        let conn = crate::db::open_db_with_wal(db_path)?;

        // Create unique cache directory based on the database path
        let cache_path = format!("{}_cache", db_path);

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            cache: Arc::new(sled::open(cache_path)?),
        })
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
        let value = db.query_row(
            "SELECT v FROM memory WHERE k=?1",
            [key],
            |r| r.get::<_, String>(0)
        ).optional()?;

        Ok(value)
    }

    pub fn query_with_timestamp(&self, key: &str) -> Result<Option<(String, i64)>> {
        let db = self.db.lock().unwrap();
        let result = db.query_row(
            "SELECT v, ts FROM memory WHERE k=?1",
            [key],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        ).optional()?;

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
