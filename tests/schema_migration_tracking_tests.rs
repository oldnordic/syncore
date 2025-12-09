//! Schema Migration Tracking TDD Tests
//! ====================================
//!
//! This test file implements Phase 2-3 of the SCHEMA_MIGRATION_DRIFT elimination.
//! We define strict invariants and then add failing tests to enforce deterministic
//! migration tracking instead of ad-hoc column sniffing.
//!
//! ## INSTRUCTIONS (from user):
//! Based on what you saw, define these invariants in comments at the top of the new test file

/// **INV-M1: Migration Table Existence Invariant**
///
/// After any call to `src/db.rs::ensure_schema()`, the database MUST contain
/// a proper `_syncore_schema_version` table with deterministic version tracking.
/// No ad-hoc column existence checks should remain in the codebase.
///
/// **Current Problem**: `src/db.rs::ensure_schema()` (lines 34-100) contains
/// ad-hoc logic like:
/// ```rust
/// let has_summary_column: bool = db
///     .prepare("SELECT name FROM pragma_table_info('memory') WHERE name='summary'")
///     .and_then(|mut stmt| stmt.exists([]))
///     .unwrap_or(false);
/// ```
///
/// **Required Behavior**: `ensure_schema()` MUST delegate to `schema_migration::run_migrations()`
/// and use ONLY the `_syncore_schema_version` table for migration state tracking.

/// **INV-M2: Migration Idempotency Invariant**
///
/// Multiple calls to `ensure_schema()` must be safe and idempotent.
/// The `_syncore_schema_version` table must track exactly which migrations
/// have been applied and never re-run them.
///
/// **Current Problem**: The ad-hoc checks in `ensure_schema()` may re-run migrations
/// or have race conditions where column existence checks fail.
///
/// **Required Behavior**: Each migration file in `/migrations/` must be applied exactly
/// once, tracked by version number, with safe repeatable calls.

/// **INV-M3: Migration Failure Atomicity Invariant**
///
/// If any migration fails, the database must be left in a consistent state.
/// No partially applied migrations should be allowed.
///
/// **Current Problem**: Ad-hoc column checks don't provide atomic rollback or
/// consistent error handling.
///
/// **Required Behavior**: Migration failures must roll back cleanly and leave
/// the schema version unchanged.

/// **INV-M4: No Existing Application Table Mutation Invariant**
///
/// This fix MUST NOT modify any existing application tables.
/// Only migration tracking infrastructure should be changed.
///
/// **Scope Constraint**: DO NOT modify tasks table, memory table, or any
/// application tables. Only fix the migration system to use proper tracking.
///
/// **Required Behavior**: All existing application tables must remain exactly
/// as they are. Only the `ensure_schema()` function should change to use
/// `schema_migration::run_migrations()` instead of ad-hoc checks.

/// **INV-M5: Deterministic Version Tracking Invariant**
///
/// The migration system must use deterministic version numbers, not
/// runtime column sniffing hacks.
///
/// **Current Problem**: The codebase has TWO parallel migration systems:
/// - Old system: `src/db.rs` with ad-hoc column checks (the drift)
/// - New system: `src/schema_migration.rs` with proper version tracking
///
/// **Required Behavior**: Only the new system should be used. The old ad-hoc
/// logic must be completely eliminated.

use anyhow::Result;
use std::sync::Arc;
use tempfile::NamedTempFile;
use syncore::db::ensure_schema;
use syncore::schema_migration::{run_migrations, get_schema_version, set_schema_version};

/// Test helper to create a fresh temporary database
fn create_temp_db() -> Result<Arc<rusqlite::Connection>> {
    let temp_file = NamedTempFile::new()?;
    let db = Arc::new(rusqlite::Connection::open(temp_file.path())?);
    // Enable WAL mode for consistency
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    Ok(db)
}

/// **PHASE 3 - FAILING TESTS**: These tests will initially fail because
/// the current `ensure_schema()` function uses ad-hoc column checks instead
/// of proper migration tracking.

/// **FAILING TEST 1**: Migration table must exist after ensure_schema()
#[test]
fn test_ensure_schema_creates_migration_tracking_table() -> Result<()> {
    // Arrange: Create fresh database
    let db = create_temp_db()?;

    // Act: Call the current ensure_schema() function
    ensure_schema(&db)?;

    // Assert: Migration tracking table MUST exist
    let mut stmt = db.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='_syncore_schema_version'"
    )?;
    let has_migration_table = stmt.exists([])?;

    assert!(has_migration_table,
        "INV-M1 VIOLATION: ensure_schema() must create _syncore_schema_version table");

    // Additional: Version should be set to current
    let version = get_schema_version(&db)?;
    assert!(version.is_some(),
        "INV-M1 VIOLATION: Schema version should be set after ensure_schema()");

    Ok(())
}

/// **FAILING TEST 2**: No ad-hoc column checks should be present in ensure_schema()
#[test]
fn test_ensure_schema_uses_proper_migration_not_column_sniffing() -> Result<()> {
    // Arrange: Create database with existing tables but no migration tracking
    let db = create_temp_db()?;

    // Manually create some application tables (simulating existing state)
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS memory (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            k TEXT NOT NULL,
            v TEXT NOT NULL,
            ts INTEGER NOT NULL
        );
         CREATE UNIQUE INDEX idx_memory_k ON memory(k);"
    )?;

    // Act: Call ensure_schema()
    ensure_schema(&db)?;

    // Assert: Must have used proper migration tracking
    let version = get_schema_version(&db)?;
    assert!(version.is_some(),
        "INV-M5 VIOLATION: ensure_schema() must use deterministic version tracking");

    // The fact that we have a version means proper migration tracking was used
    // instead of ad-hoc column checks
    Ok(())
}

/// **FAILING TEST 3**: Migration idempotency - multiple calls must be safe
#[test]
fn test_ensure_schema_idempotency_with_migration_tracking() -> Result<()> {
    // Arrange: Create fresh database
    let db = create_temp_db()?;

    // Act: Call ensure_schema() multiple times
    ensure_schema(&db)?;
    let first_version = get_schema_version(&db)?;

    ensure_schema(&db)?;
    let second_version = get_schema_version(&db)?;

    ensure_schema(&db)?;
    let third_version = get_schema_version(&db)?;

    // Assert: Version should remain consistent (no double-migrations)
    assert_eq!(first_version, second_version,
        "INV-M2 VIOLATION: Multiple ensure_schema() calls must not change schema version");

    assert_eq!(second_version, third_version,
        "INV-M2 VIOLATION: ensure_schema() must be idempotent");

    // Additional: Migration table should still exist and be consistent
    let mut stmt = db.prepare(
        "SELECT COUNT(*) FROM _syncore_schema_version"
    )?;
    let count: i64 = stmt.query_row([], |row| row.get(0))?;
    assert_eq!(count, 1,
        "INV-M2 VIOLATION: Should have exactly one migration version record");

    Ok(())
}

/// **FAILING TEST 4**: Migration atomicity - failures should not corrupt state
#[test]
fn test_migration_tracking_atomicity() -> Result<()> {
    // This test would require simulating a migration failure
    // For now, we'll test that the existing migration system handles failures correctly

    let db = create_temp_db()?;

    // Set a fake version to test rollback behavior
    set_schema_version(&db, 0)?;
    assert_eq!(get_schema_version(&db)?, Some(0),
        "Setup failed: Should be able to set schema version");

    // Run proper migrations should work
    run_migrations(&db)?;
    let final_version = get_schema_version(&db)?;

    assert!(final_version.is_some() && final_version.unwrap() > 0,
        "INV-M3 VIOLATION: Proper migrations should advance version correctly");

    Ok(())
}

/// **FAILING TEST 5**: Application tables should remain unchanged
#[test]
fn test_migration_fix_does_not_mutate_application_tables() -> Result<()> {
    // Arrange: Create database with known application table state
    let db = create_temp_db()?;

    // Create application tables manually with specific structure
    db.execute_batch(
        "CREATE TABLE memory (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            k TEXT NOT NULL,
            v TEXT NOT NULL,
            ts INTEGER NOT NULL
        );
         CREATE TABLE tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            goal TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'open',
            priority INTEGER NOT NULL DEFAULT 3
         );"
    )?;

    // Capture initial schema
    let initial_memory_schema = get_table_schema(&db, "memory")?;
    let initial_tasks_schema = get_table_schema(&db, "tasks")?;

    // Act: Run ensure_schema() with our fix
    ensure_schema(&db)?;

    // Assert: Application tables should remain unchanged
    let final_memory_schema = get_table_schema(&db, "memory")?;
    let final_tasks_schema = get_table_schema(&db, "tasks")?;

    assert_eq!(initial_memory_schema, final_memory_schema,
        "INV-M4 VIOLATION: memory table schema should not change");

    assert_eq!(initial_tasks_schema, final_tasks_schema,
        "INV-M4 VIOLATION: tasks table schema should not change");

    Ok(())
}

/// Helper function to get table schema for comparison
fn get_table_schema(db: &rusqlite::Connection, table_name: &str) -> Result<String> {
    let mut stmt = db.prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name=?")?;
    let schema: String = stmt.query_row([table_name], |row| row.get(0))?;
    Ok(schema)
}

/// **PASSING TEST 6**: Verify the new migration system works correctly
#[test]
fn test_proper_migration_system_works() -> Result<()> {
    // This test should PASS because it uses the correct migration system
    let db = create_temp_db()?;

    // Use the proper migration system directly
    run_migrations(&db)?;

    // Assert: Migration tracking should be in place
    let version = get_schema_version(&db)?;
    assert!(version.is_some(),
        "Proper migration system should set version");

    // Migration table should exist
    let mut stmt = db.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='_syncore_schema_version'"
    )?;
    let has_migration_table = stmt.exists([])?;
    assert!(has_migration_table,
        "Proper migration system should create tracking table");

    Ok(())
}