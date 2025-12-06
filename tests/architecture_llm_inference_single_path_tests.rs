//! Architecture LLM Inference Single-Path Tests
//!
//! Enforces that ALL Candle inference in the entire workspace flows
//! through candle_cache.rs exclusively. No alternative inference paths
//! are allowed. This is a critical security and stability guardrail.
//!
//! Tests MUST FAIL FIRST and MUST CHECK:
//! 1. No file outside candle_cache.rs contains forbidden Candle patterns
//! 2. All inference calls import from candle_cache
//! 3. No manual model/tokenizer initialization exists
//! 4. No hidden inference code paths exist

use std::fs;
use std::path::Path;
use std::process::Command;

/// Test: Ensure no forbidden Candle patterns exist outside candle_cache.rs
#[test]
fn test_no_forbidden_candle_patterns_outside_cache() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing No Forbidden Candle Patterns Outside candle_cache.rs ===");

    let forbidden_patterns = vec![
        "new_from_gguf",
        "load_gguf",
        "VarBuilder",
        "Device::Cpu",
        "CandleModel",
        "Tokenizer::new",
    ];

    // Search all Rust files except candle_cache.rs
    let rust_files = Command::new("find")
        .args(["src", "-name", "*.rs", "-not", "-path", "*/candle_cache.rs"])
        .output()?;

    let file_paths = String::from_utf8_lossy(&rust_files.stdout);

    for file_path in file_paths.lines() {
        if file_path.trim().is_empty() {
            continue;
        }

        let content = fs::read_to_string(file_path)?;

        for pattern in &forbidden_patterns {
            if content.contains(pattern) {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Forbidden pattern '{}' found in {}", pattern, file_path),
                )));
            }
        }

        println!("✅ {} contains no forbidden patterns", file_path);
    }

    println!("✅ No forbidden Candle patterns found outside candle_cache.rs");
    Ok(())
}

/// Test: Ensure all inference files import from candle_cache
#[test]
fn test_all_inference_imports_use_candle_cache() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing All Inference Imports Use candle_cache ===");

    // Find files that call .complete() method (inference calls)
    let complete_files = Command::new("rg").args(["-l", r#"\.complete\("#, "src/"]).output()?;

    let file_paths = String::from_utf8_lossy(&complete_files.stdout);

    for file_path in file_paths.lines() {
        if file_path.trim().is_empty() {
            continue;
        }

        // Skip test files, candle_cache.rs itself, and binary main files
        if file_path.contains("test")
            || file_path.contains("candle_cache.rs")
            || file_path.contains("main.rs")
        {
            continue;
        }

        let content = fs::read_to_string(file_path)?;

        // Check if file directly creates models (should import from candle_cache)
        // vs using LanguageModel interface (allowed without candle_cache import)
        let directly_creates_models = content.contains("GGUFEngine::new")
            || content.contains("GGUFEngine::new_test")
            || content.contains("Arc::new(GGUFEngine")
            || content.contains("get_or_init_model")
            || content.contains("load_qwen_model")
            || content.contains("Model::new");

        if directly_creates_models
            && !content.contains("use crate::llm::candle_cache")
            && !content.contains("use super::candle_cache")
        {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("File {} creates models but doesn't import from candle_cache", file_path),
            )));
        }

        if directly_creates_models {
            println!("✅ {} properly imports from candle_cache", file_path);
        } else {
            println!("✅ {} uses LanguageModel interface (no direct model creation)", file_path);
        }
    }

    println!("✅ All inference files import from candle_cache");
    Ok(())
}

/// Test: Ensure no manual model/tokenizer initialization exists
#[test]
fn test_no_manual_model_or_tokenizer_initialization() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing No Manual Model/Tokenizer Initialization ===");

    let forbidden_patterns = vec![
        "GGUFEngine::new",
        "GGUFEngine::new_test",
        "Tokenizer::new",
        "GgufTokenizer::new",
        "Arc::new(GGUFEngine",
        "Arc::new(GgufTokenizer",
    ];

    // Search all Rust files
    let rust_files = Command::new("find")
        .args(["src", "-name", "*.rs", "-not", "-path", "*/candle_cache.rs"])
        .output()?;

    let file_paths = String::from_utf8_lossy(&rust_files.stdout);

    for file_path in file_paths.lines() {
        if file_path.trim().is_empty()
            || file_path.contains("test")
            || file_path.contains("candle_cache.rs")
            || file_path.contains("main.rs")
        {
            continue;
        }

        let content = fs::read_to_string(file_path)?;

        for pattern in &forbidden_patterns {
            if content.contains(pattern) {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Manual model/tokenizer initialization '{}' found in {}",
                        pattern, file_path
                    ),
                )));
            }
        }

        println!("✅ {} contains no manual model/tokenizer initialization", file_path);
    }

    println!("✅ No manual model/tokenizer initialization found");
    Ok(())
}

/// Test: Ensure candle_cache.rs contains the allowed patterns (sanity check)
#[test]
fn test_candle_cache_contains_allowed_patterns() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing candle_cache.rs Contains Allowed Patterns ===");

    let cache_file = "src/llm/candle_cache.rs";

    if !Path::new(cache_file).exists() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} does not exist", cache_file),
        )));
    }

    let content = fs::read_to_string(cache_file)?;

    // Should contain these allowed patterns
    let allowed_patterns =
        vec!["get_or_init_model", "get_or_init_tokenizer", "CandleConfig", "OnceCell"];

    for pattern in &allowed_patterns {
        if !content.contains(pattern) {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Expected pattern '{}' not found in {}", pattern, cache_file),
            )));
        }
    }

    println!("✅ candle_cache.rs contains all expected patterns");
    Ok(())
}

/// Test: Verify no hidden inference code paths exist
#[test]
fn test_no_hidden_inference_code_paths() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing No Hidden Inference Code Paths ===");

    // Look for suspicious patterns that might indicate hidden inference
    let suspicious_patterns = vec![
        "candle-core::",
        "candle-nn::",
        "candle-transformers::",
        "hf_hub::",
        " use Model::new",
        " load_from_file(",
        "FromFile(",
    ];

    // Search all Rust files except candle_cache.rs and test files
    let rust_files = Command::new("find")
        .args([
            "src",
            "-name",
            "*.rs",
            "-not",
            "-path",
            "*/candle_cache.rs",
            "-not",
            "-path",
            "*/test*",
            "-not",
            "-path",
            "*/tests/*",
        ])
        .output()?;

    let file_paths = String::from_utf8_lossy(&rust_files.stdout);

    for file_path in file_paths.lines() {
        if file_path.trim().is_empty() {
            continue;
        }

        let content = fs::read_to_string(file_path)?;

        for pattern in &suspicious_patterns {
            if content.contains(pattern) {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Suspicious inference pattern '{}' found in {}", pattern, file_path),
                )));
            }
        }

        println!("✅ {} contains no suspicious inference patterns", file_path);
    }

    println!("✅ No hidden inference code paths found");
    Ok(())
}
