//! Integration tests for PathResolver usage across tools (APEX v1.7 Phase 6.5)
//!
//! Tests that all tools using PathResolver correctly resolve workspace roots
//! and handle excluded directories.

use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Test that PathResolver correctly identifies workspace root from nested paths
#[test]
fn test_path_resolver_finds_workspace_root() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(&cargo_toml, "[package]\nname = \"test\"\n")?;

    // Create nested directory structure
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir)?;
    let nested_dir = src_dir.join("nested");
    fs::create_dir(&nested_dir)?;

    // PathResolver should find workspace root from nested path
    let mut resolver = syncore::path_resolver::PathResolver::new();
    let root = resolver.resolve_workspace_root(&nested_dir)?;

    assert!(root.is_some());
    assert_eq!(root.unwrap().canonicalize()?, temp_dir.path().canonicalize()?);
    Ok(())
}

/// Test that PathResolver correctly excludes directories
#[test]
fn test_path_resolver_excludes_build_dirs() -> Result<()> {
    let resolver = syncore::path_resolver::PathResolver::new();

    // Test excluded directories
    assert!(resolver.is_excluded(Path::new("target/debug/build")));
    assert!(resolver.is_excluded(Path::new("node_modules/package")));
    assert!(resolver.is_excluded(Path::new("src/.git/objects")));
    assert!(resolver.is_excluded(Path::new("__pycache__/compiled")));

    // Test non-excluded directories
    assert!(!resolver.is_excluded(Path::new("src/main.rs")));
    assert!(!resolver.is_excluded(Path::new("tests/integration.rs")));
    assert!(!resolver.is_excluded(Path::new("examples/demo.rs")));

    Ok(())
}

/// Test that PathResolver handles multiple workspace indicators
#[test]
fn test_path_resolver_multiple_indicators() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create multiple workspace indicators
    let git_dir = temp_dir.path().join(".git");
    fs::create_dir(&git_dir)?;

    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(&cargo_toml, "[workspace]\nmembers = []\n")?;

    let package_json = temp_dir.path().join("package.json");
    fs::write(&package_json, "{\"name\": \"test\"}\n")?;

    // PathResolver should find workspace root from any indicator
    let mut resolver = syncore::path_resolver::PathResolver::new();
    let root = resolver.resolve_workspace_root(temp_dir.path())?;

    assert!(root.is_some());
    assert_eq!(root.unwrap().canonicalize()?, temp_dir.path().canonicalize()?);
    Ok(())
}

/// Test that PathResolver correctly converts absolute to relative paths
#[test]
fn test_path_resolver_project_relative() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(&cargo_toml, "[package]\nname = \"test\"\n")?;

    let mut resolver = syncore::path_resolver::PathResolver::new();
    resolver.resolve_workspace_root(temp_dir.path())?;

    // Test absolute to relative conversion
    let abs_path = temp_dir.path().join("src/main.rs");
    let relative = resolver.resolve_project_relative(&abs_path);

    assert_eq!(relative, Path::new("src/main.rs"));
    Ok(())
}

/// Test PathResolver with custom exclusions
#[test]
fn test_path_resolver_custom_exclusions() -> Result<()> {
    let mut resolver =
        syncore::path_resolver::PathResolver::with_exclusions(vec!["custom".to_string()]);

    // Custom exclusion should work
    assert!(resolver.is_excluded(Path::new("custom/file.rs")));

    // Default exclusions should NOT be present
    assert!(!resolver.is_excluded(Path::new("target/debug/build")));

    // Add a new exclusion dynamically
    resolver.add_exclusion("temp".to_string());
    assert!(resolver.is_excluded(Path::new("temp/file.txt")));

    Ok(())
}

/// Test PathResolver handles missing workspace gracefully
#[test]
fn test_path_resolver_no_workspace() -> Result<()> {
    let temp_dir = TempDir::new()?;
    // No workspace indicators created

    let mut resolver = syncore::path_resolver::PathResolver::new();
    let root = resolver.resolve_workspace_root(temp_dir.path())?;

    // Should return None when no workspace found
    assert!(root.is_none());
    Ok(())
}
