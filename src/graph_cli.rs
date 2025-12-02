//! Graph CLI - Command line tools for graph rebuild and validation
//!
//! Commands:
//! - validate: Run graph health checks and print report
//! - rebuild: Clear and rebuild all edges from indexed files
//! - stats: Show graph statistics

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::graph::Neo4jClient;
use crate::graph_rebuilder::{BatchEdgePusher, GraphValidator, RelationshipExtractor};

/// CLI configuration from environment or arguments
pub struct GraphCliConfig {
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_pass: String,
    pub source_dir: Option<PathBuf>,
}

impl GraphCliConfig {
    /// Load from environment variables with defaults
    pub fn from_env() -> Self {
        Self {
            neo4j_uri: std::env::var("NEO4J_URI")
                .unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string()),
            neo4j_user: std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string()),
            neo4j_pass: std::env::var("NEO4J_PASS").unwrap_or_else(|_| "password".to_string()),
            source_dir: std::env::var("SOURCE_DIR").ok().map(PathBuf::from),
        }
    }
}

/// Run graph validation and print report
pub async fn run_validate(config: &GraphCliConfig) -> Result<bool> {
    println!("Connecting to Neo4j at {}...", config.neo4j_uri);

    let client = Neo4jClient::connect(&config.neo4j_uri, &config.neo4j_user, &config.neo4j_pass)
        .await
        .context("Failed to connect to Neo4j")?;

    let validator = GraphValidator::new(client);

    println!("\n=== Graph Validation Report ===\n");

    let report =
        validator.full_validation_report().await.context("Failed to generate validation report")?;

    // Print connectivity
    println!(
        "Node Connectivity: {}/{} ({:.1}%)",
        report.nodes_with_edges,
        report.total_nodes,
        report.connectivity_ratio * 100.0
    );
    println!(
        "  Status: {} (target: >=95%)",
        if report.connectivity_ok {
            "OK"
        } else {
            "FAIL"
        }
    );

    // Print duplicates
    println!("\nDuplicate Edges: {}", report.duplicate_edges);
    println!(
        "  Status: {} (target: 0)",
        if report.duplicates_ok {
            "OK"
        } else {
            "FAIL"
        }
    );

    // Print orphan clusters
    println!("\nOrphan Clusters: {}", report.orphan_clusters.len());
    if !report.orphan_clusters.is_empty() && report.orphan_clusters.len() <= 5 {
        for cluster in &report.orphan_clusters {
            println!("  - {}", cluster);
        }
    }
    println!(
        "  Status: {} (target: <=1)",
        if report.orphan_clusters.len() <= 1 {
            "OK"
        } else {
            "FAIL"
        }
    );

    // Print diffusion
    println!(
        "\nDiffusion Test: {}",
        if report.diffusion_ok {
            "OK"
        } else {
            "FAIL"
        }
    );

    // Overall status
    let all_ok = report.all_ok();
    println!(
        "\n=== Overall: {} ===",
        if all_ok {
            "PASS"
        } else {
            "FAIL"
        }
    );

    Ok(all_ok)
}

/// Run graph rebuild - extract edges from source and push to Neo4j
pub async fn run_rebuild(config: &GraphCliConfig) -> Result<()> {
    let source_dir = config.source_dir.clone().unwrap_or_else(|| PathBuf::from("src"));

    println!("Connecting to Neo4j at {}...", config.neo4j_uri);

    let client = Neo4jClient::connect(&config.neo4j_uri, &config.neo4j_user, &config.neo4j_pass)
        .await
        .context("Failed to connect to Neo4j")?;

    println!("Extracting edges from {}...", source_dir.display());

    // Extract edges from source files
    let mut extractor = RelationshipExtractor::new()?;
    let edges = extractor
        .extract_from_directory(&source_dir)
        .context("Failed to extract edges from source")?;

    println!("Extracted {} edges from source files", edges.len());

    // Show edge type distribution
    let mut type_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for edge in &edges {
        *type_counts.entry(&edge.edge_type).or_insert(0) += 1;
    }
    println!("\nEdge type distribution:");
    for (edge_type, count) in &type_counts {
        println!("  {}: {}", edge_type, count);
    }

    // Note: Full rebuild would require mapping edge names to node IDs
    // This is a simplified version that shows what would be pushed
    println!("\nNote: Full rebuild requires node ID resolution from SQLite.");
    println!("Use `syncore_cli graph sync` for full rebuild with node mapping.");

    // Create pusher for future use
    let _pusher = BatchEdgePusher::new(client);

    println!("\nRebuild preparation complete.");
    println!("Edges ready for push: {}", edges.len());

    Ok(())
}

/// Show graph statistics
pub async fn run_stats(config: &GraphCliConfig) -> Result<()> {
    println!("Connecting to Neo4j at {}...", config.neo4j_uri);

    let client = Neo4jClient::connect(&config.neo4j_uri, &config.neo4j_user, &config.neo4j_pass)
        .await
        .context("Failed to connect to Neo4j")?;

    // Get comprehensive graph statistics using canonical API
    use crate::databases::neo4j::validate_structure;
    let stats = validate_structure(&client).await?;

    println!("\n=== Graph Statistics ===\n");
    println!("Namespace: {}", client.namespace());
    println!("Total Nodes: {}", stats.total_nodes);
    println!("Total Edges: {}", stats.total_edges);
    println!("Orphan Nodes: {}", stats.orphan_count);

    if !stats.entity_types.is_empty() {
        println!("\nEntity Types:");
        for (entity_type, count) in &stats.entity_types {
            println!("  {}: {}", entity_type, count);
        }
    }

    if !stats.edge_types.is_empty() {
        println!("\nEdge Types:");
        for (rel_type, count) in &stats.edge_types {
            println!("  {}: {}", rel_type, count);
        }
    }

    Ok(())
}

/// Full sync: extract edges → push to Neo4j with CONTAINS edges
pub async fn run_sync(config: &GraphCliConfig) -> Result<()> {
    use crate::code_graph::EdgeType;

    let source_dir = config.source_dir.as_ref().cloned().unwrap_or_else(|| PathBuf::from("src"));

    println!("=== Full Graph Sync with CONTAINS Edges ===\n");

    // Step 1: Connect to Neo4j
    println!("1. Connecting to Neo4j at {}...", config.neo4j_uri);
    let client = Neo4jClient::connect(&config.neo4j_uri, &config.neo4j_user, &config.neo4j_pass)
        .await
        .context("Failed to connect to Neo4j")?;

    // Step 2: Extract edges including CONTAINS
    println!("2. Extracting edges (including CONTAINS/MODULE_CHILD)...");
    let mut extractor = RelationshipExtractor::new()?;
    let extracted_edges = extractor.extract_from_directory(&source_dir)?;
    println!("   Extracted {} edges", extracted_edges.len());

    // Show distribution
    let mut type_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for edge in &extracted_edges {
        *type_counts.entry(&edge.edge_type).or_insert(0) += 1;
    }
    println!("   Distribution:");
    for (edge_type, count) in &type_counts {
        println!("     {}: {}", edge_type, count);
    }

    // Step 3: Convert extracted edges to name-based format
    println!("3. Converting edges for name-based push...");
    let named_edges: Vec<(&str, &str, EdgeType)> = extracted_edges
        .iter()
        .filter_map(|edge| {
            let edge_type = match edge.edge_type.as_str() {
                "calls" => EdgeType::Calls,
                "imports" => EdgeType::Imports,
                "uses" => EdgeType::Uses,
                "inherits" => EdgeType::Inherits,
                "references" => EdgeType::References,
                "contains" => EdgeType::Contains,
                "implements" => EdgeType::Implements,
                "uses_field" => EdgeType::UsesField,
                "uses_type" => EdgeType::UsesType,
                "module_child" => EdgeType::ModuleChild,
                _ => return None,
            };
            Some((edge.src_name.as_str(), edge.dst_name.as_str(), edge_type))
        })
        .collect();
    println!("   Prepared {} edges for push", named_edges.len());

    // Step 4: Push edges to Neo4j by name (creates nodes if missing)
    println!("4. Pushing edges to Neo4j (creating nodes as needed)...");
    let pusher = BatchEdgePusher::new(client.clone());
    let pushed_count = pusher.push_edges_by_name(&named_edges).await?;
    println!("   Pushed {} edges", pushed_count);

    // Step 5: Validate result
    println!("\n5. Validating result...");
    let validator = GraphValidator::new(client);
    let stats = validator.validate_node_connectivity().await?;
    let connectivity = stats.nodes_with_edges as f64 / stats.total_nodes.max(1) as f64;

    println!("\n=== Sync Complete ===");
    println!(
        "Connectivity: {}/{} ({:.1}%)",
        stats.nodes_with_edges,
        stats.total_nodes,
        connectivity * 100.0
    );

    if connectivity >= 0.95 {
        println!("Status: PASS (target >= 95%)");
    } else {
        println!("Status: BELOW TARGET (got {:.1}%, need 95%)", connectivity * 100.0);
    }

    Ok(())
}

/// Parse CLI arguments and run appropriate command
pub async fn run_cli(args: &[String]) -> Result<()> {
    let config = GraphCliConfig::from_env();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "validate" => {
            run_validate(&config).await?;
        }
        "rebuild" => {
            run_rebuild(&config).await?;
        }
        "sync" => {
            run_sync(&config).await?;
        }
        "stats" => {
            run_stats(&config).await?;
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        cmd => {
            eprintln!("Unknown command: {}", cmd);
            print_usage();
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_usage() {
    println!("SynCore Graph CLI");
    println!();
    println!("Usage: syncore_cli graph <command>");
    println!();
    println!("Commands:");
    println!("  validate   Run graph health checks and print report");
    println!("  rebuild    Extract edges from source and prepare for push");
    println!("  sync       Full sync: index files → extract edges → push to Neo4j");
    println!("  stats      Show graph node/edge statistics");
    println!("  help       Show this help message");
    println!();
    println!("Environment Variables:");
    println!("  NEO4J_URI    Neo4j connection URI (default: bolt://127.0.0.1:7687)");
    println!("  NEO4J_USER   Neo4j username (default: neo4j)");
    println!("  NEO4J_PASS   Neo4j password (default: password)");
    println!("  SOURCE_DIR   Source directory for rebuild (default: src)");
    println!("  DB_PATH      SQLite database path (default: syncore.db)");
}
