//! SynCore Graph CLI - Binary entry point
//!
//! Usage:
//!   syncore_graph_cli validate
//!   syncore_graph_cli rebuild
//!   syncore_graph_cli stats
//!
//! Environment:
//!   NEO4J_URI, NEO4J_USER, NEO4J_PASS, SOURCE_DIR

use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Handle "graph" subcommand if present (for compatibility with "syncore_cli graph validate")
    let effective_args = if args.len() > 1 && args[1] == "graph" {
        // Skip "graph" and pass rest to run_cli
        let mut new_args = vec![args[0].clone()];
        new_args.extend(args[2..].iter().cloned());
        new_args
    } else {
        args
    };

    syncore::graph_cli::run_cli(&effective_args).await
}
