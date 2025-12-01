//! TASK A: Temporal Enrichment TDD Tests

use anyhow::Result;
use std::sync::{Arc, Mutex};
use syncore::code_graph::CodeGraph;
use syncore::vector::VectorStore;

#[tokio::test]
async fn test_enrich_temporal_metadata_populates_missing_fields() -> Result<()> {
    // Setup: Create test database with entity missing temporal data
    let test_db = "/tmp/test_temporal_enrich.db";
    let _ = std::fs::remove_file(test_db);

    // Create CodeGraph with real database
    let vector_store = Arc::new(Mutex::new(VectorStore::new(Box::new(
        syncore::vector::RealEmbeddings::new(384)?,
    ))));
    let code_graph = CodeGraph::new(test_db, vector_store)?;

    // Insert entity with NULL temporal fields directly into SQLite
    {
        let db = code_graph.db_conn().lock().unwrap();
        db.execute(
            "INSERT INTO code_entities
             (file_path, entity_type, name, line_start, line_end, language, indexed_at,
              created_at, last_modified_at, change_count, author_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL, NULL, NULL)",
            rusqlite::params![
                "/home/feanor/Projects/SynCore/syncore/src/code_graph/graph.rs",
                "function",
                "test_function",
                1,
                10,
                "rust",
                0
            ],
        )?;
    }

    // Run enrichment
    let enriched_count = code_graph.enrich_temporal_metadata_for_all().await?;

    // Verify enrichment occurred
    assert!(enriched_count > 0, "Should have enriched at least one entity");

    // Verify temporal fields are now non-null
    {
        let db = code_graph.db_conn().lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT created_at, last_modified_at, change_count, author_count
             FROM code_entities WHERE name = 'test_function'",
        )?;

        let result = stmt.query_row([], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, Option<i32>>(2)?,
                row.get::<_, Option<i32>>(3)?,
            ))
        })?;

        let (created_at, last_modified_at, change_count, author_count) = result;

        assert!(created_at.is_some(), "created_at should be non-null");
        assert!(last_modified_at.is_some(), "last_modified_at should be non-null");
        assert!(change_count.is_some(), "change_count should be non-null");
        assert!(author_count.is_some(), "author_count should be non-null");

        // Values should be >= 0
        assert!(created_at.unwrap() >= 0);
        assert!(last_modified_at.unwrap() >= 0);
        assert!(change_count.unwrap() >= 1);
        assert!(author_count.unwrap() >= 1);
    }

    // Cleanup
    let _ = std::fs::remove_file(test_db);
    Ok(())
}
