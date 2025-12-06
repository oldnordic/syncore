//! Architecture Regression Test - No Ollama References
//!
//! This test ensures that no "ollama" references can be accidentally re-introduced
//! into the codebase without breaking this test. This is a critical guardrail
//! that protects the architecture from regression.

use std::io;
use std::path::Path;
use std::process::Command;

/// Test: Verify no Ollama references exist anywhere in the codebase
///
/// This test uses ripgrep to search for any case-insensitive references to "ollama"
/// in the source code. If any are found, the test fails immediately, preventing
/// accidental re-introduction of Ollama dependencies.
///
/// This test covers:
/// - Source code files (src/**/*.rs)
/// - Test files (tests/**/*.rs)
/// - Documentation files (*.md, *.rst)
/// - Configuration files (*.toml, *.yaml, *.json)
/// - Build files (*.sh, *.yml, Dockerfile)
///
/// Any occurrence of "ollama" in these files will cause this test to fail.
#[test]
fn test_no_ollama_references_anywhere() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing No Ollama References in Codebase ===");

    // Directories to search for Ollama references
    let search_patterns = vec![
        "src/**/*.rs",
        "tests/**/*.rs",
        "docs/**/*.md",
        "*.md",
        "*.toml",
        "*.yaml",
        "*.yml",
        "*.json",
        "*.sh",
        "Dockerfile*",
        "Makefile*",
    ];

    let mut total_violations = 0;

    for pattern in search_patterns {
        println!("Checking pattern: {}", pattern);

        // Use ripgrep to search for case-insensitive "ollama"
        let output = Command::new("rg")
            .args([
                "-i",      // case-insensitive
                "--count", // only show count of matches
                "ollama", pattern,
            ])
            .output()?;

        let count_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if !count_str.is_empty() {
            let count: usize = count_str.parse().unwrap_or(0);
            if count > 0 {
                println!("❌ Found {} Ollama references in {}", count, pattern);
                total_violations += count;

                // Show the actual matches for debugging
                let detailed_output =
                    Command::new("rg").args(["-i", "-n", "ollama", pattern]).output()?;

                let details = String::from_utf8_lossy(&detailed_output.stdout);
                if !details.is_empty() {
                    println!("Details:\n{}", details);
                }
            } else {
                println!("✅ No Ollama references found in {}", pattern);
            }
        } else {
            println!("✅ No Ollama references found in {}", pattern);
        }
    }

    if total_violations > 0 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Found {} Ollama references in codebase. Architecture violation detected!",
                total_violations
            ),
        )));
    }

    println!("✅ No Ollama references found anywhere in the codebase");
    Ok(())
}

/// Test: Verify specific high-risk files are Ollama-free
///
/// This test specifically checks files that are most likely to accidentally
/// re-introduce Ollama dependencies:
/// - Configuration files
/// - Module declarations
/// - Build scripts
/// - Documentation
#[test]
fn test_high_risk_files_are_ollama_free() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing High-Risk Files Are Ollama-Free ===");

    let high_risk_files = vec![
        "src/lib.rs",
        "src/config.rs",
        "src/llm/factory.rs",
        "src/llm/mod.rs",
        "Cargo.toml",
        "README.md",
        "CLAUDE.md",
        "syncore.toml",
        ".gitignore",
    ];

    for file_path in high_risk_files {
        if Path::new(file_path).exists() {
            let content = std::fs::read_to_string(file_path)?;

            // Check for case-insensitive Ollama references
            let ollama_count = content.to_lowercase().matches("ollama").count();

            if ollama_count > 0 {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "High-risk file {} contains {} Ollama references",
                        file_path, ollama_count
                    ),
                )));
            }

            println!("✅ {} is Ollama-free", file_path);
        } else {
            println!("⚠️  File {} does not exist, skipping", file_path);
        }
    }

    println!("✅ All high-risk files are Ollama-free");
    Ok(())
}

/// Test: Verify module declarations don't contain Ollama modules
#[test]
fn test_no_ollama_module_declarations() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing No Ollama Module Declarations ===");

    let mod_files = vec!["src/lib.rs"];

    for mod_file in mod_files {
        if !Path::new(mod_file).exists() {
            continue;
        }

        let content = std::fs::read_to_string(mod_file)?;

        // Check for `pub mod ollama;` or similar
        let ollama_mod_patterns = vec!["pub mod ollama", "mod ollama", "use.*ollama"];

        for pattern in ollama_mod_patterns {
            if content.to_lowercase().contains(&pattern.replace(".*", "")) {
                return Err(Box::new(std::io::Error::new(
                    io::ErrorKind::Other,
                    format!("Found Ollama module declaration in {}", mod_file),
                )));
            }
        }

        println!("✅ {} has no Ollama module declarations", mod_file);
    }

    println!("✅ No Ollama module declarations found");
    Ok(())
}

/// Test: Verify environment variable references don't contain Ollama
#[test]
fn test_no_ollama_environment_variables() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing No Ollama Environment Variables ===");

    let config_files = vec!["src/config.rs", "src/llm/factory.rs"];

    for config_file in config_files {
        if !Path::new(config_file).exists() {
            continue;
        }

        let content = std::fs::read_to_string(config_file)?;

        // Check for Ollama-related environment variables
        let ollama_env_patterns = vec!["OLLAMA_", "ollama_"];

        for pattern in ollama_env_patterns {
            if content.contains(pattern) {
                return Err(Box::new(std::io::Error::new(
                    io::ErrorKind::Other,
                    format!("Found Ollama environment variable reference in {}", config_file),
                )));
            }
        }

        println!("✅ {} has no Ollama environment variable references", config_file);
    }

    println!("✅ No Ollama environment variable references found");
    Ok(())
}

/// Test: Verify comments don't contain Ollama references
#[test]
fn test_no_ollama_in_comments() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing No Ollama References in Comments ===");

    // Check that no comments accidentally reference Ollama functionality
    let comment_patterns = vec!["//.*ollama", "///.*ollama", "/*.*ollama", "#.*ollama"];

    for pattern in comment_patterns {
        let output = Command::new("rg").args(["-n", pattern, "src/"]).output()?;

        if !output.stdout.is_empty() {
            return Err(Box::new(std::io::Error::new(
                io::ErrorKind::Other,
                format!("Found Ollama reference in comments matching: {}", pattern),
            )));
        }
    }

    println!("✅ No Ollama references found in comments");
    Ok(())
}

/// Test: Verify no Ollama imports in the dependency graph
#[test]
fn test_no_ollama_imports() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing No Ollama Imports ===");

    // Check that no import statements reference Ollama
    let import_patterns = vec!["use.*ollama", "extern.*ollama", "mod.*ollama"];

    for pattern in import_patterns {
        let output = Command::new("rg").args(["-n", pattern, "src/"]).output()?;

        if !output.stdout.is_empty() {
            return Err(Box::new(std::io::Error::new(
                io::ErrorKind::Other,
                format!("Found Ollama import matching: {}", pattern),
            )));
        }
    }

    println!("✅ No Ollama imports found");
    Ok(())
}
