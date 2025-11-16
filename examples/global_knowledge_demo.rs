// Global Knowledge Store Demonstration
// Shows how articles and knowledge are shared across all SynCore projects

use anyhow::Result;
use syncore::global_store::{GlobalDbPool, GlobalVectorStore, get_global_dir, get_global_db_path};

fn main() -> Result<()> {
    println!("=== SynCore Global Knowledge Store ===\n");

    // Show global paths
    println!("1. Global Storage Locations:");
    let global_dir = get_global_dir();
    let db_path = get_global_db_path();
    println!("   Global directory: {}", global_dir.display());
    println!("   Global database:  {}", db_path.display());
    println!();

    // Create global database pool
    println!("2. Initializing Global Database...");
    let global_db = GlobalDbPool::new()?;
    println!("   ✓ Global database ready");
    println!();

    // Create global vector store
    println!("3. Initializing Global Vector Store...");
    let vector_store = GlobalVectorStore::new()?;
    println!("   ✓ Vector store ready");
    println!();

    // Demonstrate database access
    println!("4. Database Demo:");
    {
        let conn = global_db.get();

        // Store a sample knowledge item
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO memory (k, v, ts) VALUES (?1, ?2, ?3)",
            ("global_demo_key", "This knowledge is shared across all projects!", ts),
        )?;

        println!("   ✓ Stored knowledge: 'global_demo_key' → 'This knowledge is shared across all projects!'");
    }

    // Demonstrate vector storage paths
    println!("\n5. Vector Storage:");
    println!("   Articles index:       {}", vector_store.get_index_path("articles").display());
    println!("   Code patterns index:  {}", vector_store.get_index_path("code_patterns").display());
    println!("   Documentation index:  {}", vector_store.get_index_path("documentation").display());
    println!();

    println!("6. Benefits:");
    println!("   ✓ Articles stored once, available everywhere");
    println!("   ✓ Knowledge accumulates globally");
    println!("   ✓ Faster semantic search (larger corpus)");
    println!("   ✓ No duplication across projects");
    println!();

    println!("7. Per-Project vs Global:");
    println!("   Per-Project (./syncore.db):");
    println!("     - IntelliTask breakdowns");
    println!("     - Project-specific memories");
    println!("     - Local workflow state");
    println!();
    println!("   Global (~/.syncore/global.db):");
    println!("     - Technical articles");
    println!("     - Documentation snippets");
    println!("     - Reusable code patterns");
    println!("     - General programming knowledge");
    println!();

    println!("=== Demo Complete ===");

    Ok(())
}
