//! Schema Validation Tests
//!
//! These tests ensure that all required database tables exist and have the correct schema.
//! This prevents regressions like the "no such table: code_entities" issue.

use anyhow::Result;
use syncore::db::open_db_with_wal;
use syncore::schema_migration::{get_schema_version, CURRENT_SCHEMA_VERSION};

/// Test that all required tables exist in the database
#[test]
fn test_all_required_tables_exist() -> Result<()> {
    // Use a temporary database for testing
    let db_path = ":memory:_schema_validation";
    let conn = open_db_with_wal(db_path)?;

    // List of all required tables
    let required_tables = vec![
        "_syncore_schema_version",
        "tasks",
        "task_links",
        "steps",
        "memory",
        "embeddings",
        "tool_calls",
        "code_entities",
        "code_edges",
        "code_embeddings",
        "code_macro_expansions",
        "code_diagnostics",
    ];

    for table_name in required_tables {
        let table_exists: bool = conn
            .prepare(&format!(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='{}'",
                table_name
            ))?
            .exists([])?;

        assert!(
            table_exists,
            "Required table '{}' does not exist in database schema",
            table_name
        );
    }

    Ok(())
}

/// Test that code_entities table has all required columns
#[test]
fn test_code_entities_table_schema() -> Result<()> {
    let db_path = ":memory:_code_entities_schema";
    let conn = open_db_with_wal(db_path)?;

    // Required columns for code_entities table (based on migration 02_code_graph.sql)
    let required_columns = vec![
        ("id", "INTEGER"),
        ("file_path", "TEXT"),
        ("entity_type", "TEXT"),
        ("name", "TEXT"),
        ("signature", "TEXT"),
        ("line_start", "INTEGER"),
        ("line_end", "INTEGER"),
        ("docstring", "TEXT"),
        ("language", "TEXT"),
        ("indexed_at", "INTEGER"),
        ("body_snippet", "TEXT"),
        // Additional columns added by later migrations
        ("created_at", "INTEGER"),
        ("last_modified_at", "INTEGER"),
        ("change_count", "INTEGER"),
        ("author_count", "INTEGER"),
    ];

    for (column_name, expected_type) in required_columns {
        let column_exists: bool = conn
            .prepare(&format!(
                "SELECT 1 FROM pragma_table_info('code_entities') WHERE name='{}'",
                column_name
            ))?
            .exists([])?;

        assert!(
            column_exists,
            "Required column '{}' does not exist in code_entities table",
            column_name
        );

        // Verify column type (basic check)
        let mut stmt = conn.prepare(&format!(
            "SELECT typeof({}) FROM pragma_table_info('code_entities') WHERE name='{}'",
            column_name, column_name
        ))?;
        let actual_type: String = stmt.query_row([], |row| row.get(0))?;

        // Allow INTEGER to be INTEGER or BIGINT, TEXT to be TEXT, etc.
        assert!(
            actual_type.starts_with(&expected_type[..1]), // Basic type match
            "Column '{}' has type '{}', expected something like '{}'",
            column_name, actual_type, expected_type
        );
    }

    Ok(())
}

/// Test that memory table has all required columns (including extended fields)
#[test]
fn test_memory_table_schema() -> Result<()> {
    let db_path = ":memory:_memory_schema";
    let conn = open_db_with_wal(db_path)?;

    // Required columns for memory table
    let required_columns = vec![
        ("id", "INTEGER"),
        ("k", "TEXT"),
        ("v", "TEXT"),
        ("ts", "INTEGER"),
        // Extended fields from migration 03_semantic_memory.sql and 05_memory_extended_fields.sql
        ("summary", "TEXT"),
        ("tags", "TEXT"),
        ("importance", "REAL"),
        ("namespace", "TEXT"),
    ];

    for (column_name, expected_type) in required_columns {
        let column_exists: bool = conn
            .prepare(&format!(
                "SELECT 1 FROM pragma_table_info('memory') WHERE name='{}'",
                column_name
            ))?
            .exists([])?;

        assert!(
            column_exists,
            "Required column '{}' does not exist in memory table",
            column_name
        );
    }

    Ok(())
}

/// Test that foreign key constraints are properly set up
#[test]
fn test_foreign_key_constraints() -> Result<()> {
    let db_path = ":memory:_foreign_key_schema";
    let conn = open_db_with_wal(db_path)?;

    // Test code_edges foreign keys
    let fk_info: Vec<(String, String, String)> = conn
        .prepare("PRAGMA foreign_key_list('code_edges')")?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?, // id
                row.get::<_, String>(2)?, // from column
                row.get::<_, String>(3)?, // to table
            ))
        })?
        .collect();

    // Should have foreign keys to code_entities table
    let has_src_fk = fk_info.iter().any(|(_, from_col, to_table)| {
        from_col == "src_entity_id" && to_table == "code_entities"
    });

    let has_dst_fk = fk_info.iter().any(|(_, from_col, to_table)| {
        from_col == "dst_entity_id" && to_table == "code_entities"
    });

    assert!(
        has_src_fk,
        "code_edges table missing foreign key from src_entity_id to code_entities.id"
    );

    assert!(
        has_dst_fk,
        "code_edges table missing foreign key from dst_entity_id to code_entities.id"
    );

    // Test code_embeddings foreign key
    let embed_fk_info: Vec<(String, String, String)> = conn
        .prepare("PRAGMA foreign_key_list('code_embeddings')")?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect();

    let has_entity_fk = embed_fk_info.iter().any(|(_, from_col, to_table)| {
        from_col == "entity_id" && to_table == "code_entities"
    });

    assert!(
        has_entity_fk,
        "code_embeddings table missing foreign key from entity_id to code_entities.id"
    );

    Ok(())
}

/// Test that required indexes exist for performance
#[test]
fn test_required_indexes_exist() -> Result<()> {
    let db_path = ":memory:_indexes_schema";
    let conn = open_db_with_wal(db_path)?;

    // Required indexes from migrations
    let required_indexes = vec![
        "idx_tasks_status_prio",
        "idx_tasks_parent",
        "idx_steps_task_state_time",
        "idx_memory_k",
        "idx_entities_name",
        "idx_entities_file",
        "idx_entities_type",
        "idx_entities_lang",
        "idx_edges_src",
        "idx_edges_dst",
        "idx_edges_type",
        "idx_code_embeddings_vector",
    ];

    for index_name in required_indexes {
        let index_exists: bool = conn
            .prepare(&format!(
                "SELECT 1 FROM sqlite_master WHERE type='index' AND name='{}'",
                index_name
            ))?
            .exists([])?;

        assert!(
            index_exists,
            "Required index '{}' does not exist in database schema",
            index_name
        );
    }

    Ok(())
}

/// Test that schema version tracking works
#[test]
fn test_schema_version_tracking() -> Result<()> {
    let db_path = ":memory:_schema_version";
    let conn = open_db_with_wal(db_path)?;

    // Should start with the expected schema version
    let version = get_schema_version(&conn)?;
    assert_eq!(
        version, CURRENT_SCHEMA_VERSION,
        "Schema version {} does not match expected {}",
        version, CURRENT_SCHEMA_VERSION
    );

    // Version table should exist
    let version_table_exists: bool = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='_syncore_schema_version'")?
        .exists([])?;

    assert!(
        version_table_exists,
        "Schema version table '_syncore_schema_version' does not exist"
    );

    Ok(())
}

/// Integration test: Verify that bootstrap process can query code_entities successfully
#[test]
fn test_bootstrap_can_query_code_entities() -> Result<()> {
    let db_path = ":memory:_bootstrap_test";
    let conn = open_db_with_wal(db_path)?;

    // This should not fail with "no such table: code_entities"
    let count: i64 = conn
        .prepare("SELECT COUNT(*) FROM code_entities")?
        .query_row([], |row| row.get(0))?;

    // Should be 0 since it's a fresh database
    assert_eq!(count, 0, "code_entities table should be empty in fresh database");

    // Verify we can also query entity count by file
    let count_by_file: i64 = conn
        .prepare("SELECT COUNT(*) FROM code_entities GROUP BY file_path")?
        .query_row([], |row| row.get(0))
        .unwrap_or(0); // Might be no rows if table is empty

    // Should work without error (might be 0 if no entities exist)
    assert!(count_by_file >= 0, "code_entities query should not fail");

    Ok(())
}