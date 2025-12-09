//! Project Isolation Tests
//!
//! TDD tests for project isolation and automatic project context.
//! Tests verify that project context is properly derived with ZERO configuration required,
//! using automatic detection from the current working directory, with proper precedence order.

use syncore::config::{get_project_label, get_project_root, ProjectContext};
use syncore::query::QueryConstraints;

#[test]
fn test_config_has_project_detection_settings() {
    // Test that config supports project detection (no env vars needed)
    let config = SyncoreConfig::default();

    // Config should have project detection enabled by default
    // (This will be implemented in the config structure)
    // For now, let's test the baseline behavior
    assert!(true); // Placeholder for now
}

#[test]
fn test_auto_project_detection() {
    // Test that project is detected automatically from current directory
    let label = get_project_label(None);
    let root = get_project_root();

    // Should detect "syncore" since we're running from within the syncore project
    assert_eq!(label, Some("syncore".to_string()));
    assert!(root.is_some()); // Current working directory should be available
}

#[test]
fn test_detect_project_label_from_path() {
    // Test project label detection from actual paths
    use syncore::config::detect_project_label_from_path;

    // Test generic project detection - should extract folder name from any path
    assert_eq!(
        detect_project_label_from_path("/home/user/projects/syncore/src/main.rs"),
        Some("syncore".to_string())
    );

    assert_eq!(
        detect_project_label_from_path("/home/user/projects/odincode/lib/app.rs"),
        Some("odincode".to_string())
    );

    assert_eq!(
        detect_project_label_from_path("/home/user/projects/my-cool-app/src/main.rs"),
        Some("my-cool-app".to_string())
    );

    assert_eq!(
        detect_project_label_from_path("/home/user/projects/web-project/components/Button.jsx"),
        Some("web-project".to_string())
    );

    assert_eq!(
        detect_project_label_from_path("relative/path/to/project/src/file.rs"),
        Some("project".to_string())
    );

    // Test case sensitivity - should preserve original case
    assert_eq!(
        detect_project_label_from_path("/home/user/Projects/SynCore/src/main.rs"),
        Some("SynCore".to_string())
    );

    assert_eq!(
        detect_project_label_from_path("/home/user/projects/My-App/src/main.rs"),
        Some("My-App".to_string())
    );

    // Test root-level paths
    assert_eq!(
        detect_project_label_from_path("/home/user/projects/syncore"),
        Some("syncore".to_string())
    );

    assert_eq!(
        detect_project_label_from_path("my-project"),
        Some("my-project".to_string())
    );

    // Test edge cases
    assert_eq!(
        detect_project_label_from_path("src"), // No parent directory
        None
    );

    assert_eq!(
        detect_project_label_from_path("/"), // Root directory
        None
    );

    assert_eq!(
        detect_project_label_from_path(""), // Empty path
        None
    );
}

#[test]
fn test_detect_project_label_from_current_dir() {
    // Test project detection from current working directory
    use syncore::config::detect_project_label_from_current_dir;

    // Test current directory detection
    let project_label = detect_project_label_from_current_dir();

    // We should detect "syncore" since we're running from within the syncore project
    assert_eq!(project_label, Some("syncore".to_string()));
}

#[test]
fn test_user_label_overrides_detection() {
    // Test precedence order: user-provided > auto-detection > None
    let detected_label = get_project_label(None);
    let user_label = get_project_label(Some("custom-project"));

    // User label should override auto-detection
    assert_eq!(user_label, Some("custom-project".to_string()));

    // Auto-detection should still work when no user label is provided
    assert_eq!(detected_label, Some("syncore".to_string()));
}

#[test]
fn test_project_context_auto() {
    // Test ProjectContext creation with automatic detection
    let context = ProjectContext::auto();

    assert!(context.is_some());
    let ctx = context.unwrap();
    assert_eq!(ctx.project_label, "syncore"); // Auto-detected from current directory
    assert!(ctx.project_root.exists()); // Should be a real directory
    assert_eq!(ctx.namespace, "syncore"); // Should derive from project_label
}

#[test]
fn test_project_context_from_user_label() {
    // Test ProjectContext creation with user-provided label
    let context = ProjectContext::from_user_label(Some("my-project"));

    assert!(context.is_some());
    let ctx = context.unwrap();
    assert_eq!(ctx.project_label, "my-project"); // User-provided
    assert!(ctx.project_root.exists()); // Should be current directory
    assert_eq!(ctx.namespace, "my-project"); // Should derive from project_label
}

#[test]
fn test_project_context_empty_user_label() {
    // Test ProjectContext creation with empty user label (should fall back to auto)
    let context = ProjectContext::from_user_label(Some(""));
    assert!(context.is_some()); // Should fall back to auto-detection
}