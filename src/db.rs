use crate::schema_migration;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

// Export DbManager module (new centralized SQLite connection manager)
pub mod manager;
pub use manager::DbManager;

pub fn open_db_with_wal(path: &str) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.pragma_update(None, "journal_mode", "WAL")?;
    db.pragma_update(None, "synchronous", "NORMAL")?;
    db.pragma_update(None, "cache_size", 1000)?;
    db.pragma_update(None, "foreign_keys", "ON")?;

    // Auto-migrate schema to current version
    // Migration failure is critical - database would be in inconsistent state
    schema_migration::run_migrations(&db)
        .expect("Failed to run schema migrations - database may be corrupted");

    Ok(db)
}

pub fn run_migration(db: &Connection, migration_path: &str) -> anyhow::Result<()> {
    let migration_sql = std::fs::read_to_string(migration_path)?;
    db.execute_batch(&migration_sql)?;
    Ok(())
}

pub fn ensure_schema(db_path: &str) -> anyhow::Result<()> {
    let db = open_db_with_wal(db_path)?;

    // Use the proper migration system instead of ad-hoc column sniffing
    // This eliminates SCHEMA_MIGRATION_DRIFT by using deterministic version tracking
    crate::schema_migration::run_migrations(&db)?;

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

    pub fn get(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}
