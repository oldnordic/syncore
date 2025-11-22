// Test program to verify schema auto-migration works with real old databases
// This program:
// 1. Copies an old database to a test location
// 2. Opens it with the new auto-migration code
// 3. Verifies the schema was upgraded correctly
// 4. Shows before/after comparison

use rusqlite::Connection;
use std::fs;

fn main() -> anyhow::Result<()> {
    println!("=== Schema Migration Test ===\n");

    // Test database paths
    let old_db = "/home/feanor/Projects/SynCore/syncore.db";
    let test_db = "./test_migration.db";

    // Clean up any previous test
    let _ = fs::remove_file(test_db);

    // Copy old database to test location
    println!("1. Copying old database...");
    fs::copy(old_db, test_db)?;
    println!("   ✓ Copied {} to {}\n", old_db, test_db);

    // Show BEFORE state
    println!("2. BEFORE migration:");
    show_database_state(test_db, "BEFORE")?;

    // Open database with auto-migration (this triggers the migration)
    println!("\n3. Opening database (triggers auto-migration)...");
    let db = syncore::db::open_db_with_wal(test_db)?;
    println!("   ✓ Database opened successfully\n");

    // Show AFTER state
    println!("4. AFTER migration:");
    show_database_state_connection(&db, "AFTER")?;

    // Verify migration worked
    println!("\n5. Verification:");
    verify_migration(&db)?;

    println!("\n=== Migration Test Complete ✓ ===");

    // Clean up
    drop(db);
    let _ = fs::remove_file(test_db);

    Ok(())
}

fn show_database_state(db_path: &str, label: &str) -> anyhow::Result<()> {
    let conn = Connection::open(db_path)?;
    show_database_state_connection(&conn, label)
}

fn show_database_state_connection(conn: &Connection, label: &str) -> anyhow::Result<()> {
    // Check for version table
    let has_version_table: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='_syncore_schema_version'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    println!("   Version table exists: {}", has_version_table);

    if has_version_table {
        let version: i32 = conn
            .query_row(
                "SELECT MAX(version) FROM _syncore_schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        println!("   Current schema version: {}", version);
    }

    // Check tasks table columns
    let mut stmt = conn.prepare("PRAGMA table_info(tasks)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();

    println!("   Tasks table columns ({}):", columns.len());

    // Check for IntelliTask fields
    let intellitask_fields = vec![
        "task_id",
        "prd_title",
        "complexity",
        "estimated_hours",
        "acceptance_criteria",
        "files_to_modify",
    ];

    for field in &intellitask_fields {
        let exists = columns.contains(&field.to_string());
        let symbol = if exists { "✓" } else { "✗" };
        println!("     {} {}", symbol, field);
    }

    Ok(())
}

fn verify_migration(conn: &Connection) -> anyhow::Result<()> {
    // Verify version is 2
    let version: i32 = conn.query_row(
        "SELECT MAX(version) FROM _syncore_schema_version",
        [],
        |row| row.get(0),
    )?;

    if version == 2 {
        println!("   ✓ Schema version is 2 (expected)");
    } else {
        println!("   ✗ Schema version is {} (expected 2)", version);
        anyhow::bail!("Wrong schema version");
    }

    // Verify all IntelliTask columns exist
    let result = conn.prepare(
        "SELECT task_id, prd_title, complexity, estimated_hours, acceptance_criteria, files_to_modify FROM tasks LIMIT 0"
    );

    if result.is_ok() {
        println!("   ✓ All IntelliTask columns exist");
    } else {
        println!("   ✗ IntelliTask columns missing");
        anyhow::bail!("IntelliTask columns not found");
    }

    // Verify indexes exist
    let mut stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='tasks'")?;
    let indexes: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    let has_task_id_idx = indexes.iter().any(|idx| idx.contains("task_id"));
    let has_prd_idx = indexes.iter().any(|idx| idx.contains("prd_title"));

    if has_task_id_idx {
        println!("   ✓ idx_tasks_task_id index exists");
    } else {
        println!("   ✗ idx_tasks_task_id index missing");
    }

    if has_prd_idx {
        println!("   ✓ idx_tasks_prd_title index exists");
    } else {
        println!("   ✗ idx_tasks_prd_title index missing");
    }

    Ok(())
}
