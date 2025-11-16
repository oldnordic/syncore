// Index Documents Example
// Recursively scans directories for documents, extracts text, chunks semantically,
// and stores in global knowledge database for efficient retrieval

use anyhow::Result;
use syncore::document_indexer::DocumentIndexer;
use syncore::global_store::get_global_db_path;
use std::env;

fn main() -> Result<()> {
    println!("=== SynCore Document Indexer ===\n");

    // Get directory path from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <directory_path>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} /home/user/research", args[0]);
        eprintln!("  {} ~/Projects/synapsedb/docs", args[0]);
        std::process::exit(1);
    }

    let dir_path = &args[1];
    println!("📂 Indexing directory: {}\n", dir_path);

    // Create indexer with default configuration
    let indexer = DocumentIndexer::with_defaults();

    println!("⚙️  Configuration:");
    println!("   - Max chunk size: 1000 characters");
    println!("   - Chunk overlap: 200 characters");
    println!("   - Skip hidden files: true");
    println!("   - Global database: {}\n", get_global_db_path().display());

    // Index the directory
    println!("🔍 Scanning for documents...");
    let chunk_count = indexer.index_directory(std::path::Path::new(dir_path))?;

    println!("\n✅ Indexing complete!");
    println!("   - Total chunks indexed: {}", chunk_count);
    println!("   - Stored in: {}", get_global_db_path().display());
    println!("\n💡 All documents are now searchable across all SynCore projects!");

    Ok(())
}
