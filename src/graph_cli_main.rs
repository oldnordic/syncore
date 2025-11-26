//! SynCore Graph CLI - Binary entry point
//!
//! Usage:
//!   syncore_graph_cli validate
//!   syncore_graph_cli rebuild
//!   syncore_graph_cli stats
//!
//! Environment:
//!   NEO4J_URI, NEO4J_USER, NEO4J_PASS, SOURCE_DIR, SYNCORE_CONFIG

use anyhow::Result;
use std::env;
use syncore::config::SyncoreConfig;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration from file (or use defaults)
    let config_path =
        env::var("SYNCORE_CONFIG").unwrap_or_else(|_| "config/syncore.toml".to_string());

    let config = if std::path::Path::new(&config_path).exists() {
        SyncoreConfig::load(&config_path).unwrap_or_default()
    } else {
        SyncoreConfig::default()
    };

    // Initialize global config for path filtering
    SyncoreConfig::init_global(config);

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
