//! Tests for ingestion ignore patterns and generated file detection

use std::path::PathBuf;
use syncore::ingestion::{BoundaryResult, GlobalIngestionCoordinator, IngestionConfig};
use tempfile::TempDir;

#[tokio::test]
async fn test_default_ignore_directories() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ignore_dirs: vec![
            "target".to_string(),
            "node_modules".to_string(),
            ".git".to_string(),
            ".idea".to_string(),
            ".vscode".to_string(),
            ".cache".to_string(),
            "dist".to_string(),
            "build".to_string(),
        ],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test default ignore directories
    let ignore_dirs = [
        "target/debug/main",
        "node_modules/package/index.js",
        ".git/objects/abc123",
        ".idea/modules.xml",
        ".vscode/settings.json",
        ".cache/tmp",
        "dist/bundle.js",
        "build/output.o",
    ];

    for ignore_path in &ignore_dirs {
        let full_path = temp_dir.path().join(ignore_path);
        // Create directories and files
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        if full_path.extension().is_none() {
            std::fs::create_dir_all(&full_path).unwrap();
        } else {
            std::fs::write(&full_path, "test").unwrap();
        }
        let result = gic.check_boundaries(&full_path);
        assert_eq!(result, BoundaryResult::Ignored, "Path {} should be ignored", ignore_path);
    }
}

#[tokio::test]
async fn test_custom_ignore_directories() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ignore_dirs: vec!["custom".to_string(), "build".to_string(), "output".to_string()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test custom ignore directories
    let custom_ignored = temp_dir.path().join("custom").join("file.txt");
    std::fs::create_dir_all(custom_ignored.parent().unwrap()).unwrap();
    std::fs::write(&custom_ignored, "test").unwrap();
    let result = gic.check_boundaries(&custom_ignored);
    assert_eq!(result, BoundaryResult::Ignored);

    // Test non-ignored directory
    let not_ignored = temp_dir.path().join("src").join("main.rs");
    std::fs::create_dir_all(not_ignored.parent().unwrap()).unwrap();
    std::fs::write(&not_ignored, "test").unwrap();
    let result = gic.check_boundaries(&not_ignored);
    assert_eq!(result, BoundaryResult::Allowed);
}

#[tokio::test]
async fn test_ignore_glob_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ignore_globs: vec!["*.log".to_string(), "*.tmp".to_string(), "test_*.rs".to_string()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test glob patterns
    let test_cases = [
        ("debug.log", true),
        ("temp.tmp", true),
        ("test_main.rs", true),
        ("main.rs", false),
        ("test.txt", false),
    ];

    for (file_name, should_ignore) in &test_cases {
        let full_path = temp_dir.path().join(file_name);
        std::fs::write(&full_path, "test content").unwrap();
        let result = gic.check_boundaries(&full_path);
        let expected = if *should_ignore {
            BoundaryResult::Ignored
        } else {
            BoundaryResult::Allowed
        };
        assert_eq!(result, expected, "File {} should be ignored: {}", file_name, should_ignore);
    }
}

#[tokio::test]
async fn test_generated_file_detection() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test generated file patterns
    let generated_files = [
        "data.hnsw.data",
        "data.hnsw.graph",
        "index.meta",
        "index.vectors",
        "other.db",
        "project.sqlite",
        "backup.sqlite3",
    ];

    for file_name in &generated_files {
        let full_path = temp_dir.path().join(file_name);
        std::fs::write(&full_path, "test content").unwrap();
        let result = gic.check_boundaries(&full_path);
        assert_eq!(
            result,
            BoundaryResult::GeneratedFile,
            "File {} should be detected as generated",
            file_name
        );
    }

    // Test that syncore.db is not considered generated
    let syncore_db = temp_dir.path().join("syncore.db");
    std::fs::write(&syncore_db, "test").unwrap();
    let result = gic.check_boundaries(&syncore_db);
    assert_eq!(result, BoundaryResult::Allowed);
}

#[tokio::test]
async fn test_nested_ignore_directories() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ignore_dirs: vec!["target".to_string()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test nested ignore directories
    let nested_ignored = temp_dir.path().join("target").join("debug").join("deps").join("lib.rlib");

    std::fs::create_dir_all(nested_ignored.parent().unwrap()).unwrap();
    std::fs::write(&nested_ignored, "test").unwrap();
    let result = gic.check_boundaries(&nested_ignored);
    assert_eq!(result, BoundaryResult::Ignored);
}

#[tokio::test]
async fn test_partial_directory_name_matching() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ignore_dirs: vec!["target".to_string()],
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test that partial matches don't trigger ignore
    let not_ignored = temp_dir.path().join("targeting").join("main.rs");
    std::fs::create_dir_all(not_ignored.parent().unwrap()).unwrap();
    std::fs::write(&not_ignored, "test").unwrap();
    let result = gic.check_boundaries(&not_ignored);
    assert_eq!(result, BoundaryResult::Allowed);

    // But exact matches should be ignored
    let ignored = temp_dir.path().join("target").join("main.rs");
    std::fs::create_dir_all(ignored.parent().unwrap()).unwrap();
    std::fs::write(&ignored, "test").unwrap();
    let result = gic.check_boundaries(&ignored);
    assert_eq!(result, BoundaryResult::Ignored);
}

#[tokio::test]
async fn test_case_sensitivity() {
    let temp_dir = TempDir::new().unwrap();
    let config = IngestionConfig {
        allowed_roots: vec![temp_dir.path().to_path_buf()],
        ignore_dirs: vec!["Target".to_string()], // Capital T
        ..Default::default()
    };
    let (gic, _main_rx, _low_prio_rx) = GlobalIngestionCoordinator::with_config(config);

    // Test case sensitivity (should be case-sensitive)
    let lowercase = temp_dir.path().join("target").join("main.rs");
    std::fs::create_dir_all(lowercase.parent().unwrap()).unwrap();
    std::fs::write(&lowercase, "test").unwrap();
    let result = gic.check_boundaries(&lowercase);
    assert_eq!(result, BoundaryResult::Allowed); // Not ignored due to case difference

    let uppercase = temp_dir.path().join("Target").join("main.rs");
    std::fs::create_dir_all(uppercase.parent().unwrap()).unwrap();
    std::fs::write(&uppercase, "test").unwrap();
    let result = gic.check_boundaries(&uppercase);
    assert_eq!(result, BoundaryResult::Ignored); // Ignored due to exact match
}
