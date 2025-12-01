//! CLI commands for SynCore tool management
//!
//! Provides `syncore tools list` and `syncore tools test` functionality.

use crate::mcp::protocol::list_tools;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// List all registered MCP tools
pub async fn list_mcp_tools() {
    let tools = list_tools().await;

    println!("SynCore MCP Tools ({} registered)", tools.len());
    println!("{}", "=".repeat(60));

    for (i, tool) in tools.iter().enumerate() {
        println!("\n{}. {}", i + 1, tool.name);
        println!("   Description: {}", tool.description);
        println!("   Input Schema: {}", tool.input_schema);
        println!("   Output Schema: {}", tool.output_schema);
    }

    println!("\n{}", "=".repeat(60));
    println!("Total: {} tools available", tools.len());
}

/// Test that all tool schemas exist and are valid JSON
pub async fn test_mcp_tools() -> Result<()> {
    let tools = list_tools().await;
    let mut passed = 0;
    let mut failed = 0;
    let mut warnings = Vec::new();

    println!("Testing SynCore MCP Tools...");
    println!("{}", "=".repeat(60));

    for tool in &tools {
        print!("Testing {}... ", tool.name);

        // Check input schema
        let input_exists = Path::new(&tool.input_schema).exists();
        let input_valid = if input_exists {
            match fs::read_to_string(&tool.input_schema) {
                Ok(content) => serde_json::from_str::<serde_json::Value>(&content).is_ok(),
                Err(_) => false,
            }
        } else {
            false
        };

        // Check output schema
        let output_exists = Path::new(&tool.output_schema).exists();
        let output_valid = if output_exists {
            match fs::read_to_string(&tool.output_schema) {
                Ok(content) => serde_json::from_str::<serde_json::Value>(&content).is_ok(),
                Err(_) => false,
            }
        } else {
            false
        };

        if input_valid && output_valid {
            println!("PASS");
            passed += 1;
        } else {
            println!("FAIL");
            failed += 1;

            if !input_exists {
                warnings
                    .push(format!("  - {} input schema missing: {}", tool.name, tool.input_schema));
            } else if !input_valid {
                warnings.push(format!(
                    "  - {} input schema invalid JSON: {}",
                    tool.name, tool.input_schema
                ));
            }

            if !output_exists {
                warnings.push(format!(
                    "  - {} output schema missing: {}",
                    tool.name, tool.output_schema
                ));
            } else if !output_valid {
                warnings.push(format!(
                    "  - {} output schema invalid JSON: {}",
                    tool.name, tool.output_schema
                ));
            }
        }
    }

    println!("{}", "=".repeat(60));
    println!("Results: {} passed, {} failed", passed, failed);

    if !warnings.is_empty() {
        println!("\nWarnings:");
        for warning in &warnings {
            println!("{}", warning);
        }
    }

    if failed > 0 {
        Err(anyhow::anyhow!("{} tools failed validation", failed))
    } else {
        println!("\nAll tools validated successfully!");
        Ok(())
    }
}

/// Generate runtime manifest for MCP tools
pub async fn generate_mcp_manifest(output_path: &str) -> Result<()> {
    let tools = list_tools().await;

    let manifest = serde_json::json!({
        "name": "SynCore",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Cognitive micro-kernel with sequential thinking, memory, task management, and code intelligence",
        "protocol": "MCP",
        "transport": ["stdio", "http/sse"],
        "tools": tools.iter().map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
                "output_schema": t.output_schema
            })
        }).collect::<Vec<_>>()
    });

    let json_string = serde_json::to_string_pretty(&manifest)?;
    fs::write(output_path, json_string)?;

    println!("Generated MCP manifest: {}", output_path);
    Ok(())
}

/// Log all registered tools (for server startup)
pub async fn log_registered_tools() {
    let tools = list_tools().await;

    eprintln!("Registered Tools ({}):", tools.len());
    for tool in &tools {
        eprintln!("  - {}", tool.name);
    }
}
