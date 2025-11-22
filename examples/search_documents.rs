// Semantic Document Search Example
// Search indexed documents using vector embeddings for semantic similarity

use anyhow::Result;
use std::env;
use syncore::global_store::GlobalVectorStore;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <search_query> [limit]", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} \"HNSW implementation\"", args[0]);
        eprintln!("  {} \"graph database architecture\" 10", args[0]);
        std::process::exit(1);
    }

    let query = &args[1];
    let limit = if args.len() > 2 {
        args[2].parse().unwrap_or(5)
    } else {
        5
    };

    println!("=== SynCore Semantic Document Search ===\n");
    println!("Query: \"{}\"", query);
    println!("Limit: {} results\n", limit);

    let vector_store = GlobalVectorStore::new()?;

    match vector_store.search(query, limit, "documents") {
        Ok(results) => {
            if results.is_empty() {
                println!("No documents found matching your query.");
                println!("Try indexing documents first with:");
                println!("  cargo run --example index_documents <directory>");
            } else {
                println!("Found {} relevant chunks:\n", results.len());
                for (i, hit) in results.iter().enumerate() {
                    println!("{}. [Score: {:.4}]", i + 1, hit.score);
                    println!("─────────────────────────────────");
                    // Show first 500 chars of chunk
                    let text_preview = if hit.text.len() > 500 {
                        format!("{}...", &hit.text[..500])
                    } else {
                        hit.text.clone()
                    };
                    println!("{}", text_preview);
                    println!();
                }
            }
        }
        Err(e) => {
            eprintln!("Search failed: {}", e);
        }
    }

    Ok(())
}
