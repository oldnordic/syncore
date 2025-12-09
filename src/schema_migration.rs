// Schema Migration System for SynCore
// Ensures backward compatibility and automatic schema upgrades

use anyhow::{anyhow, Context, Result};
use rusqlite::{Connection, OptionalExtension};

/// Current schema version
pub const CURRENT_SCHEMA_VERSION: i32 = 8;

/// Get the current schema version from the database
/// Returns 0 if version table doesn't exist (brand new database)
pub fn get_schema_version(conn: &Connection) -> Result<i32> {
    // Check if version table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='_syncore_schema_version'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !table_exists {
        return Ok(0); // Brand new database
    }

    // Get the highest version number
    let version = conn
        .query_row("SELECT MAX(version) FROM _syncore_schema_version", [], |row| {
            row.get::<_, Option<i32>>(0)
        })
        .context("Failed to query schema version")?
        .unwrap_or(0);

    Ok(version)
}

/// Set the schema version in the database
pub fn set_schema_version(conn: &Connection, version: i32, description: &str) -> Result<()> {
    // Ensure version table exists
    init_version_table(conn)?;

    // Insert or replace the version record
    let timestamp =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;

    conn.execute(
        "INSERT OR REPLACE INTO _syncore_schema_version (version, applied_at, description) VALUES (?1, ?2, ?3)",
        (version, timestamp, description),
    )?;

    Ok(())
}

/// Initialize the schema version table
pub fn init_version_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS _syncore_schema_version (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL,
            description TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

/// Run all necessary migrations to bring database to current schema version
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Initialize version table if it doesn't exist
    init_version_table(conn)?;

    // Get current schema version
    let current_version = get_schema_version(conn)?;

    // Apply migrations in order, skipping already-applied ones
    if current_version < 1 {
        migration_001_initial_schema(conn).context("Failed to run migration 001")?;
    }

    if current_version < 2 {
        migration_002_intellitask_fields(conn).context("Failed to run migration 002")?;
    }

    if current_version < 3 {
        migration_003_code_diagnostics(conn).context("Failed to run migration 003")?;
    }

    if current_version < 4 {
        migration_004_code_graph(conn).context("Failed to run migration 004")?;
    }

    if current_version < 5 {
        migration_005_memory_extended_fields(conn).context("Failed to run migration 005")?;
    }

    if current_version < 6 {
        migration_006_code_graph_nullable_lines(conn).context("Failed to run migration 006")?;
    }

    if current_version < 7 {
        migration_007_code_entities_temporal_fields(conn).context("Failed to run migration 007")?;
    }

    if current_version < 8 {
        migration_008_timestamp_standardization(conn).context("Failed to run migration 008")?;
    }

    // Verify we're at the expected version
    let final_version = get_schema_version(conn)?;
    if final_version != CURRENT_SCHEMA_VERSION {
        return Err(anyhow!(
            "Migration completed but version mismatch: expected {}, got {}",
            CURRENT_SCHEMA_VERSION,
            final_version
        ));
    }

    Ok(())
}

/// Migration 001: Initial core schema (tasks, task_links, memories, etc.)
fn migration_001_initial_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("../migrations/01_core.sql"))
        .context("Failed to apply migration 001: Initial core schema")?;

    set_schema_version(conn, 1, "Initial core schema (tasks, memories, steps, embeddings)")?;
    Ok(())
}

/// Migration 002: Add IntelliTask fields to tasks table
fn migration_002_intellitask_fields(conn: &Connection) -> Result<()> {
    // Helper function to check if a column exists
    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        let query = format!("PRAGMA table_info({})", table);
        let mut stmt = conn.prepare(&query).unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        columns.contains(&column.to_string())
    }

    // Add new columns for IntelliTask support if they don't exist
    let columns_to_add = vec![
        ("task_id", "TEXT"),
        ("prd_title", "TEXT"),
        ("complexity", "TEXT"),
        ("estimated_hours", "REAL"),
        ("acceptance_criteria", "TEXT"), // JSON array
        ("files_to_modify", "TEXT"),     // JSON array
    ];

    for (col_name, col_type) in columns_to_add {
        if !column_exists(conn, "tasks", col_name) {
            let sql = format!("ALTER TABLE tasks ADD COLUMN {} {}", col_name, col_type);
            conn.execute(&sql, [])?;
        }
    }

    // Add indexes for new columns
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_tasks_task_id ON tasks(task_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_prd_title ON tasks(prd_title);
        "#,
    )?;

    set_schema_version(
        conn,
        2,
        "Add IntelliTask fields (task_id, complexity, estimated_hours, etc.)",
    )?;
    Ok(())
}

/// Migration 003: Add code_diagnostics table for static analysis results
fn migration_003_code_diagnostics(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("../migrations/03_code_diagnostics.sql"))
        .context("Failed to apply migration 003: Code diagnostics schema")?;

    set_schema_version(
        conn,
        3,
        "Add code_diagnostics table for static analysis results (Clippy, etc.)",
    )?;
    Ok(())
}

/// Migration 004: Add code graph tables (code_entities, code_edges, code_embeddings)
fn migration_004_code_graph(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("../migrations/02_code_graph.sql"))
        .context("Failed to apply migration 004: Code graph schema")?;

    set_schema_version(
        conn,
        4,
        "Add code graph tables (code_entities, code_edges, code_embeddings)",
    )?;
    Ok(())
}

/// Migration 005: Add extended memory fields for APEX 2.0-M-FIX (namespace isolation)
fn migration_005_memory_extended_fields(conn: &Connection) -> Result<()> {
    // APEX 2.4-CG-SCHEMA-FIX: Check if memory table exists AND if columns already exist
    // This prevents failures from duplicate migration attempts (migration 03 vs 05 conflict)

    // Check if memory table exists
    let has_memory_table: bool = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='memory'")
        .and_then(|mut stmt| stmt.exists([]))
        .unwrap_or(false);

    if !has_memory_table {
        // Skip this migration if memory table doesn't exist (CodeGraph-only databases)
        set_schema_version(conn, 5, "Skipped memory extended fields (memory table not present)")?;
        return Ok(());
    }

    // Check if 'summary' column already exists (added by migration 03_semantic_memory.sql)
    // Use PRAGMA table_info to check actual column existence
    let has_summary_column: bool = conn
        .prepare("SELECT name FROM pragma_table_info('memory') WHERE name='summary'")
        .and_then(|mut stmt| stmt.exists([]))
        .unwrap_or(false);

    if has_summary_column {
        // Columns already exist from migration 03, skip this migration
        set_schema_version(
            conn,
            5,
            "Skipped memory extended fields (columns already exist from migration 03)",
        )?;
        return Ok(());
    }

    // Memory table exists AND columns don't exist yet - apply the migration
    conn.execute_batch(include_str!("../migrations/05_memory_extended_fields.sql"))
        .context("Failed to apply migration 005: Memory extended fields")?;

    set_schema_version(
        conn,
        5,
        "Add extended memory fields (namespace, summary, importance, created_at, last_accessed, access_count)",
    )?;
    Ok(())
}

/// Migration 006: Allow NULL values for line_start and line_end in code_entities
fn migration_006_code_graph_nullable_lines(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("../migrations/06_code_graph_nullable_lines.sql"))
        .context("Failed to apply migration 006: Code graph nullable lines")?;

    set_schema_version(
        conn,
        6,
        "Allow NULL values for line_start and line_end in code_entities (File entities)",
    )?;
    Ok(())
}

/// Migration 007: Add temporal fields to code_entities table
pub fn migration_007_code_entities_temporal_fields(conn: &Connection) -> Result<()> {
    // Enable foreign keys for this migration
    conn.execute("PRAGMA foreign_keys=ON", [])?;

    // Ensure code_entities table exists
    let table_exists: bool = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='code_entities' LIMIT 1")?
        .exists([])?;

    if !table_exists {
        anyhow::bail!("code_entities table not found when applying migration 007");
    }

    // Helper: returns true if column exists
    fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
        let sql = format!("SELECT 1 FROM pragma_table_info('{}') WHERE name = ?1 LIMIT 1", table);
        let mut stmt = conn.prepare(&sql)?;
        Ok(stmt.exists([column])?)
    }

    // For each temporal column, add it ONLY if it does not already exist
    if !column_exists(conn, "code_entities", "created_at")? {
        conn.execute("ALTER TABLE code_entities ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0", [])?;
        eprintln!("[schema] Added created_at column to code_entities");
    } else {
        eprintln!("[schema] created_at column already exists in code_entities");
    }

    if !column_exists(conn, "code_entities", "last_modified_at")? {
        conn.execute("ALTER TABLE code_entities ADD COLUMN last_modified_at INTEGER NOT NULL DEFAULT 0", [])?;
        eprintln!("[schema] Added last_modified_at column to code_entities");
    } else {
        eprintln!("[schema] last_modified_at column already exists in code_entities");
    }

    if !column_exists(conn, "code_entities", "change_count")? {
        conn.execute("ALTER TABLE code_entities ADD COLUMN change_count INTEGER NOT NULL DEFAULT 0", [])?;
        eprintln!("[schema] Added change_count column to code_entities");
    } else {
        eprintln!("[schema] change_count column already exists in code_entities");
    }

    if !column_exists(conn, "code_entities", "author_count")? {
        conn.execute("ALTER TABLE code_entities ADD COLUMN author_count INTEGER NOT NULL DEFAULT 0", [])?;
        eprintln!("[schema] Added author_count column to code_entities");
    } else {
        eprintln!("[schema] author_count column already exists in code_entities");
    }

    // Create indexes IF NOT EXISTS
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_code_entities_created_at ON code_entities(created_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_code_entities_last_modified_at ON code_entities(last_modified_at)",
        [],
    )?;

    eprintln!("[schema] Created temporal indexes for code_entities");

    set_schema_version(
        conn,
        7,
        "Add temporal fields to code_entities: created_at, last_modified_at, change_count, author_count (idempotent)",
    )?;
    Ok(())
}

/// Migration 008: Standardize all timestamp fields to INTEGER type (Unix epoch)
pub fn migration_008_timestamp_standardization(conn: &Connection) -> Result<()> {
    // Enable foreign keys for this migration
    conn.execute("PRAGMA foreign_keys=ON", [])?;

    // Always standardize code_diagnostics timestamps
    standardize_code_diagnostics_timestamps(conn)?;

    // Check if code_macro_expansions table exists before attempting to standardize it
    let macro_table_exists: bool = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='code_macro_expansions' LIMIT 1")?
        .exists([])?;

    if macro_table_exists {
        standardize_code_macro_expansions_timestamps(conn)?;
    } else {
        // Log informational message about skipping macro expansions table
        eprintln!("[schema] Skipping timestamp standardization for code_macro_expansions (table not found)");
    }

    set_schema_version(
        conn,
        8,
        "Standardize timestamp fields to INTEGER type: fix code_diagnostics and code_macro_expansions created_at fields",
    )?;
    Ok(())
}

/// Standardize code_diagnostics.created_at from TEXT to INTEGER
fn standardize_code_diagnostics_timestamps(conn: &Connection) -> Result<()> {
    // Check if code_diagnostics table exists
    let table_exists: bool = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='code_diagnostics' LIMIT 1")?
        .exists([])?;

    if !table_exists {
        eprintln!("[schema] Skipping timestamp standardization for code_diagnostics (table not found)");
        return Ok(());
    }

    // Check if the created_at column is still TEXT type
    let created_at_type: Option<String> = conn
        .prepare("SELECT type FROM pragma_table_info('code_diagnostics') WHERE name = 'created_at' LIMIT 1")?
        .query_row([], |row| row.get::<_, String>(0))
        .optional()?;

    // Handle missing column gracefully
    let created_at_is_text = match created_at_type {
        Some(col_type) => {
            let normalized_type = col_type.to_lowercase();
            normalized_type != "integer"
        }
        None => {
            eprintln!("[schema] Skipping timestamp standardization for code_diagnostics (created_at column not found)");
            return Ok(());
        }
    };

    if !created_at_is_text {
        eprintln!("[schema] Skipping timestamp standardization for code_diagnostics (created_at already INTEGER)");
        return Ok(());
    }

    // Apply the standardization: recreate table with INTEGER timestamp
    conn.execute_batch(
        r#"
        -- Add new INTEGER column with default current timestamp
        ALTER TABLE code_diagnostics ADD COLUMN created_at_new INTEGER NOT NULL DEFAULT (strftime('%s','now'));

        -- Copy existing data, setting current timestamp for all rows
        UPDATE code_diagnostics SET created_at_new = (strftime('%s','now')) WHERE created_at_new = (strftime('%s','now'));

        -- Recreate table with INTEGER timestamp
        CREATE TABLE code_diagnostics_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            line_start INTEGER NOT NULL,
            severity TEXT NOT NULL,
            diagnostic_type TEXT NOT NULL,
            message TEXT NOT NULL,
            tool TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );

        -- Copy data from old table to new table
        INSERT INTO code_diagnostics_new (id, file_path, line_start, severity, diagnostic_type, message, tool, created_at)
        SELECT id, file_path, line_start, severity, diagnostic_type, message, tool, created_at_new
        FROM code_diagnostics;

        -- Drop old table and rename new table
        DROP TABLE code_diagnostics;
        ALTER TABLE code_diagnostics_new RENAME TO code_diagnostics;

        -- Recreate indexes for code_diagnostics
        CREATE INDEX IF NOT EXISTS idx_diagnostics_file ON code_diagnostics(file_path);
        CREATE INDEX IF NOT EXISTS idx_diagnostics_tool ON code_diagnostics(tool);
        CREATE INDEX IF NOT EXISTS idx_diagnostics_type ON code_diagnostics(diagnostic_type);
        CREATE INDEX IF NOT EXISTS idx_diagnostics_severity ON code_diagnostics(severity);
        "#,
    ).context("Failed to standardize code_diagnostics timestamps")?;

    eprintln!("[schema] Successfully standardized code_diagnostics timestamps from TEXT to INTEGER");
    Ok(())
}

/// Standardize code_macro_expansions.created_at from TEXT to INTEGER
fn standardize_code_macro_expansions_timestamps(conn: &Connection) -> Result<()> {
    // Check if the created_at column is still TEXT type
    let created_at_type: Option<String> = conn
        .prepare("SELECT type FROM pragma_table_info('code_macro_expansions') WHERE name = 'created_at' LIMIT 1")?
        .query_row([], |row| row.get::<_, String>(0))
        .optional()?;

    // Handle missing column gracefully
    let created_at_is_text = match created_at_type {
        Some(col_type) => {
            let normalized_type = col_type.to_lowercase();
            normalized_type != "integer"
        }
        None => {
            eprintln!("[schema] Skipping timestamp standardization for code_macro_expansions (created_at column not found)");
            return Ok(());
        }
    };

    if !created_at_is_text {
        eprintln!("[schema] Skipping timestamp standardization for code_macro_expansions (created_at already INTEGER)");
        return Ok(());
    }

    // Apply the standardization: recreate table with INTEGER timestamp
    conn.execute_batch(
        r#"
        -- Add new INTEGER column with default current timestamp
        ALTER TABLE code_macro_expansions ADD COLUMN created_at_new INTEGER NOT NULL DEFAULT (strftime('%s','now'));

        -- Copy existing data, setting current timestamp for all rows
        UPDATE code_macro_expansions SET created_at_new = (strftime('%s','now')) WHERE created_at_new = (strftime('%s','now'));

        -- Recreate table with INTEGER timestamp
        CREATE TABLE code_macro_expansions_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            macro_name TEXT NOT NULL,
            span_start INTEGER NOT NULL,
            span_end INTEGER NOT NULL,
            original_code TEXT NOT NULL,
            expanded_code TEXT NOT NULL,
            expansion_type TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );

        -- Copy data from old table to new table
        INSERT INTO code_macro_expansions_new (id, file_path, macro_name, span_start, span_end, original_code, expanded_code, expansion_type, created_at)
        SELECT id, file_path, macro_name, span_start, span_end, original_code, expanded_code, expansion_type, created_at_new
        FROM code_macro_expansions;

        -- Drop old table and rename new table
        DROP TABLE code_macro_expansions;
        ALTER TABLE code_macro_expansions_new RENAME TO code_macro_expansions;

        -- Recreate indexes for code_macro_expansions
        CREATE INDEX IF NOT EXISTS idx_macro_expansions_file ON code_macro_expansions(file_path);
        CREATE INDEX IF NOT EXISTS idx_macro_expansions_name ON code_macro_expansions(macro_name);
        CREATE INDEX IF NOT EXISTS idx_macro_expansions_type ON code_macro_expansions(expansion_type);
        CREATE INDEX IF NOT EXISTS idx_macro_expansions_span ON code_macro_expansions(span_start, span_end);
        "#,
    ).context("Failed to standardize code_macro_expansions timestamps")?;

    eprintln!("[schema] Successfully standardized code_macro_expansions timestamps from TEXT to INTEGER");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    #[test]
    fn test_init_version_table_creates_table() {
        let conn = setup_test_db();

        // Should create version tracking table
        init_version_table(&conn).expect("Failed to init version table");

        // Verify table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='_syncore_schema_version'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);

        assert!(table_exists, "Version table should be created");
    }

    #[test]
    fn test_get_schema_version_returns_zero_for_new_db() {
        let conn = setup_test_db();

        // New database without version table should return 0
        let version = get_schema_version(&conn).expect("Should not fail on new DB");
        assert_eq!(version, 0, "New database should have version 0");
    }

    #[test]
    fn test_set_and_get_schema_version() {
        let conn = setup_test_db();
        init_version_table(&conn).unwrap();

        // Set version to 1
        set_schema_version(&conn, 1, "Initial schema").expect("Should set version");

        // Get it back
        let version = get_schema_version(&conn).expect("Should get version");
        assert_eq!(version, 1, "Should return the version we set");

        // Set version to 2
        set_schema_version(&conn, 2, "IntelliTask fields").expect("Should update version");

        // Get it back
        let version = get_schema_version(&conn).expect("Should get version");
        assert_eq!(version, 2, "Should return updated version");
    }

    #[test]
    fn test_migration_001_creates_core_tables() {
        let conn = setup_test_db();
        init_version_table(&conn).unwrap();

        // Run migration 001
        migration_001_initial_schema(&conn).expect("Migration 001 should succeed");

        // Verify tasks table exists
        let tasks_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='tasks'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(tasks_exists, "Tasks table should exist after migration 001");

        // Verify task_links table exists
        let links_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='task_links'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(links_exists, "Task links table should exist after migration 001");
    }

    #[test]
    fn test_migration_002_adds_intellitask_fields() {
        let conn = setup_test_db();
        init_version_table(&conn).unwrap();

        // Run migration 001 first (prerequisite)
        migration_001_initial_schema(&conn).expect("Migration 001 should succeed");

        // Run migration 002
        migration_002_intellitask_fields(&conn).expect("Migration 002 should succeed");

        // Verify new columns exist by preparing a statement (validates column names)
        let result = conn.prepare(
            "SELECT task_id, complexity, estimated_hours, acceptance_criteria, files_to_modify FROM tasks"
        );

        assert!(
            result.is_ok(),
            "IntelliTask columns should exist after migration 002: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_migration_001_is_idempotent() {
        let conn = setup_test_db();
        init_version_table(&conn).unwrap();

        // Run migration twice
        migration_001_initial_schema(&conn).expect("First run should succeed");
        let result = migration_001_initial_schema(&conn);

        // Should not fail on second run (idempotent)
        assert!(result.is_ok(), "Migration 001 should be idempotent (safe to run twice)");
    }

    #[test]
    fn test_migration_002_is_idempotent() {
        let conn = setup_test_db();
        init_version_table(&conn).unwrap();
        migration_001_initial_schema(&conn).unwrap();

        // Run migration twice
        migration_002_intellitask_fields(&conn).expect("First run should succeed");
        let result = migration_002_intellitask_fields(&conn);

        // Should not fail on second run (idempotent)
        assert!(result.is_ok(), "Migration 002 should be idempotent (safe to run twice)");
    }

    #[test]
    fn test_run_migrations_on_new_database() {
        let conn = setup_test_db();

        // Run migrations on brand new database
        run_migrations(&conn).expect("Migrations should succeed on new DB");

        // Verify we're at current version
        let version = get_schema_version(&conn).expect("Should have version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION, "Should be at current schema version");

        // Verify all tables exist
        let tasks_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='tasks'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(tasks_exists, "Tasks table should exist");
    }

    #[test]
    fn test_run_migrations_skips_already_applied() {
        let conn = setup_test_db();
        init_version_table(&conn).unwrap();

        // Manually run migration 001 and mark as applied
        migration_001_initial_schema(&conn).unwrap();
        set_schema_version(&conn, 1, "Initial schema").unwrap();

        // Run migrations - should only apply migration 002
        run_migrations(&conn).expect("Should apply only needed migrations");

        // Verify we're at version 2
        let version = get_schema_version(&conn).expect("Should have version");
        assert_eq!(version, 2, "Should have applied migration 002");
    }

    #[test]
    fn test_run_migrations_is_idempotent() {
        let conn = setup_test_db();

        // Run migrations twice
        run_migrations(&conn).expect("First run should succeed");
        run_migrations(&conn).expect("Second run should succeed (idempotent)");

        // Should still be at current version
        let version = get_schema_version(&conn).expect("Should have version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION, "Should still be at current version");
    }
}
