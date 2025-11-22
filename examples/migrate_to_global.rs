// Migrate existing articles from local database to global knowledge store
// This ensures all your stored articles are available across all projects

use anyhow::Result;
use rusqlite::Connection;
use syncore::global_store::{get_global_db_path, GlobalDbPool};

fn main() -> Result<()> {
    println!("=== Migrating Articles to Global Knowledge Store ===\n");

    // Open local database (parent directory where articles were stored)
    let local_db_path = "/home/feanor/Projects/SynCore/syncore.db";
    println!("1. Opening local database: {}", local_db_path);
    let local_db = Connection::open(local_db_path)?;
    println!("   ✓ Local database opened\n");

    // Open global database
    println!(
        "2. Opening global database: {}",
        get_global_db_path().display()
    );
    let global_db_pool = GlobalDbPool::new()?;
    let global_db = global_db_pool.get();
    println!("   ✓ Global database opened\n");

    // Count all memory entries (articles are stored with various keys)
    let local_count: i64 =
        local_db.query_row("SELECT COUNT(*) FROM memory", [], |row| row.get(0))?;
    println!("3. Found {} memory entries in local database", local_count);

    if local_count == 0 {
        println!("   No articles to migrate.\n");
        return Ok(());
    }

    // Migrate all memory entries (articles and knowledge)
    println!("\n4. Migrating knowledge entries...");
    let mut stmt = local_db.prepare("SELECT k, v, ts FROM memory")?;

    let articles: Vec<(String, String, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut migrated = 0;
    for (key, value, ts) in articles {
        // Check if already exists in global
        let exists: bool = global_db
            .query_row("SELECT 1 FROM memory WHERE k = ?1", [&key], |_| Ok(true))
            .unwrap_or(false);

        if !exists {
            global_db.execute(
                "INSERT INTO memory (k, v, ts) VALUES (?1, ?2, ?3)",
                (&key, &value, ts),
            )?;
            migrated += 1;
            println!("   ✓ Migrated: {}", key);
        } else {
            println!("   ⊘ Skipped (already exists): {}", key);
        }
    }

    println!("\n5. Migrating vector embeddings...");

    // Count embeddings
    let embedding_count: i64 =
        local_db.query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))?;
    println!("   Found {} embeddings in local database", embedding_count);

    // Migrate embeddings (if any)
    if embedding_count > 0 {
        let mut stmt = local_db.prepare("SELECT id, text_id, vec_id FROM embeddings")?;

        let embeddings: Vec<(i64, String, i64)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut embedding_migrated = 0;
        for (id, text_id, vec_id) in embeddings {
            let exists: bool = global_db
                .query_row(
                    "SELECT 1 FROM embeddings WHERE text_id = ?1",
                    [&text_id],
                    |_| Ok(true),
                )
                .unwrap_or(false);

            if !exists {
                global_db.execute(
                    "INSERT INTO embeddings (id, text_id, vec_id) VALUES (?1, ?2, ?3)",
                    (id, &text_id, vec_id),
                )?;
                embedding_migrated += 1;
            }
        }
        println!("   ✓ Migrated {} embeddings", embedding_migrated);
    }

    println!("\n=== Migration Complete ===");
    println!("Summary:");
    println!(
        "  - Quick references migrated: {}/{}",
        migrated, local_count
    );
    println!("  - Embeddings migrated: {}", embedding_count);
    println!("  - Global database: {}", get_global_db_path().display());
    println!("\nAll articles are now available across all SynCore projects!");

    Ok(())
}
