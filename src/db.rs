use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::path::Path;

pub fn open_db_with_wal(path: &str) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.pragma_update(None, "journal_mode", &"WAL")?;
    db.pragma_update(None, "synchronous", &"NORMAL")?;
    db.pragma_update(None, "cache_size", &1000)?;
    db.pragma_update(None, "foreign_keys", &"ON")?;
    Ok(db)
}

pub fn run_migration(db: &Connection, migration_path: &str) -> anyhow::Result<()> {
    let migration_sql = std::fs::read_to_string(migration_path)?;
    db.execute_batch(&migration_sql)?;
    Ok(())
}

pub fn ensure_schema(db_path: &str) -> anyhow::Result<()> {
    let db = open_db_with_wal(db_path)?;

    // Check if tasks table exists to determine if we need to run migration
    let mut stmt = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='tasks'")?;
    let has_tasks = stmt.exists([])?;

    if !has_tasks {
        // Run the v1 migration
        let migration_path = "migrations/01_core.sql";
        if Path::new(migration_path).exists() {
            run_migration(&db, migration_path)?;
        } else {
            // Fallback: create basic schema inline
            db.execute_batch(include_str!("../migrations/01_core.sql"))?;
        }
    }

    Ok(())
}

#[derive(Clone)]
pub struct DbPool {
    conn: Arc<Mutex<Connection>>,
}

impl DbPool {
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        ensure_schema(db_path)?;
        let conn = open_db_with_wal(db_path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn get(&self) -> std::sync::MutexGuard<Connection> {
        self.conn.lock().unwrap()
    }
}
