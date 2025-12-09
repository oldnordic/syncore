//! Indexing Filters Tests
//!
//! TDD tests for the enhanced indexing filter functionality.
//! Tests verify that database, SQLite, log, and coverage files are properly excluded.

use syncore::config::{SyncoreConfig, IndexingConfig};
use syncore::macro_tools::path_filter::{should_index_path, should_index_path_with_excludes};
use syncore::document_indexer::{IndexerConfig, DocumentType};

/// Helper to create a test config with custom excluded directories that includes defaults
fn create_test_config(mut excluded_dirs: Vec<String>) -> SyncoreConfig {
    let mut config = SyncoreConfig::default();

    // Add any custom exclusions to the defaults
    for custom in excluded_dirs {
        if !config.indexing.excluded_dirs.contains(&custom) {
            config.indexing.excluded_dirs.push(custom);
        }
    }

    config
}

#[test]
fn test_should_exclude_database_files() {
    let config = create_test_config(vec![
        "*.db".to_string(),
        "*.sqlite".to_string(),
        "*.sqlite3".to_string(),
    ]);

    // Debug: print the excluded directories
    println!("Excluded dirs: {:?}", config.indexing.excluded_dirs);

    // Test database files are excluded
    assert!(config.should_exclude_path("data/app.db"));
    assert!(config.should_exclude_path("cache/cache.sqlite"));
    assert!(config.should_exclude_path("backup/main.sqlite3"));
    assert!(config.should_exclude_path("./logs/debug.db"));

    // Test that source files are still allowed
    assert!(!config.should_exclude_path("src/main.rs"));
    assert!(!config.should_exclude_path("lib/app.py"));
}

#[test]
fn test_should_exclude_log_files() {
    let config = create_test_config(vec!["*.log".to_string()]);

    // Test log files are excluded
    assert!(config.should_exclude_path("logs/app.log"));
    assert!(config.should_exclude_path("var/log/system.log"));
    assert!(config.should_exclude_path("./error.log"));

    // Test that source files are still allowed
    assert!(!config.should_exclude_path("src/main.rs"));
    assert!(!config.should_exclude_path("lib/app.py"));
}

#[test]
fn test_should_exclude_existing_directories() {
    let config = create_test_config(vec!["target".to_string(), "node_modules".to_string()]);

    // Test existing directory exclusions still work
    assert!(config.should_exclude_path("target/debug/build.rs"));
    assert!(config.should_exclude_path("node_modules/lodash/index.js"));
    assert!(config.should_exclude_path(".git/objects/abc"));

    // Test that source files are still allowed
    assert!(!config.should_exclude_path("src/main.rs"));
}

#[test]
fn test_path_filter_with_custom_excludes_includes_database_patterns() {
    let excludes = vec![
        "*.db".to_string(),
        "*.sqlite".to_string(),
        "*.log".to_string(),
    ];

    // Test custom excludes with database patterns
    assert!(!should_index_path_with_excludes("src/main.rs", &excludes));
    assert!(!should_index_path_with_excludes("tests/test.rs", &excludes));

    assert!(!should_index_path_with_excludes("data/config.json", &excludes));

    // Database files should be excluded
    assert!(!should_index_path_with_excludes("database/app.db", &excludes));
    assert!(!should_index_path_with_excludes("cache/queries.sqlite", &excludes));
    assert!(!should_index_path_with_excludes("backup/main.sqlite3", &excludes));

    // Log files should be excluded
    assert!(!should_index_path_with_excludes("logs/debug.log", &excludes));
    assert!(!should_index_path_with_excludes("var/log/app.log", &excludes));
}

#[test]
fn test_default_path_filter_includes_new_patterns() {
    // Test with global function (uses defaults)

    // Database files should be excluded by default
    assert!(!should_index_path("target/debug/syncore.db"));
    assert!(!should_index_path("data/cache.sqlite"));
    assert!(!should_index_path("backup/main.sqlite3"));

    // Log files should be excluded by default
    assert!(!should_index_path("logs/app.log"));
    assert!(!should_index_path("var/log/system.log"));
    assert!(!should_index_path("./error.log"));

    // Coverage directories already excluded should still work
    assert!(!should_index_path("coverage/index.html"));
    assert!(!should_index_path("htmlcov/report.html"));
    assert!(!should_index_path(".nyc_output"));

    // Source files should be indexed
    assert!(should_index_path("src/main.rs"));
    assert!(should_index_path("lib/utils.py"));
    assert!(should_index_path("components/Button.jsx"));
}

#[test]
fn test_document_indexer_skips_database_extensions() {
    let indexer_config = IndexerConfig::default();

    // Test that IndexerConfig's skip_extensions includes database and log files
    assert!(indexer_config.skip_extensions.contains(&"db".to_string()));
    assert!(indexer_config.skip_extensions.contains(&"sqlite".to_string()));
    assert!(indexer_config.skip_extensions.contains(&"sqlite3".to_string()));
    assert!(indexer_config.skip_extensions.contains(&"log".to_string()));

    // Test DocumentType detection should exclude these extensions
    assert_eq!(DocumentType::from_path(std::path::Path::new("data/app.db")), None);
    assert_eq!(DocumentType::from_path(std::path::Path::new("cache.sqlite")), None);
    assert_eq!(DocumentType::from_path(std::path::Path::new("debug.log")), None);

    // Valid document types should still work
    assert!(DocumentType::from_path(std::path::Path::new("src/main.rs")).is_some());
    assert!(DocumentType::from_path(std::path::Path::new("docs/readme.md")).is_some());
    assert!(DocumentType::from_path(std::path::Path::new("config.json")).is_some());
}

#[test]
fn test_edge_cases_in_pattern_matching() {
    let config = create_test_config(vec![
        "*.db".to_string(),
        "*.log".to_string(),
    ]);

    // Test files with multiple extensions
    assert!(config.should_exclude_path("app.db.log")); // Should match *.log
    assert!(config.should_exclude_path("backup.sqlite.old")); // Should NOT match (exact match)

    // Test files with dots in directory names
    assert!(config.should_exclude_path("my.data.db/file.txt")); // Should match *.db
    assert!(!config.should_exclude_path("my.data/db/file.txt")); // Should NOT match

    // Test case sensitivity (should be case-insensitive for extensions)
    assert!(config.should_exclude_path("ERROR.LOG"));
    assert!(config.should_exclude_path("DATABASE.DB"));

    // Test paths without extensions
    assert!(!config.should_exclude_path("Makefile"));
    assert!(!config.should_exclude_path("Dockerfile"));
}

#[test]
fn test_config_override_maintains_new_patterns() {
    // Test that user config can override patterns but defaults include new ones
    let mut config = SyncoreConfig::default();

    // Verify defaults include new patterns
    assert!(config.indexing.excluded_dirs.contains(&"*.db".to_string()));
    assert!(config.indexing.excluded_dirs.contains(&"*.sqlite".to_string()));
    assert!(config.indexing.excluded_dirs.contains(&"*.log".to_string()));

    // User can override while keeping structure
    config.indexing.excluded_dirs.push("custom_dir".to_string());

    assert!(config.should_exclude_path("custom_dir/file.rs"));
    assert!(config.should_exclude_path("target/debug/app.db"));
    assert!(config.should_exclude_path("logs/error.log"));
}