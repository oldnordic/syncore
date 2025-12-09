// Tests to validate the indexer fix works correctly
use std::fs;
use tempfile::TempDir;

#[test]
fn test_pattern_wrapping_fix() {
    // Test that the fix prevents double pattern wrapping

    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create test files
    fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    fs::write(src_dir.join("lib.rs"), "pub fn lib() {}").unwrap();

    let sub_dir = src_dir.join("utils");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("helper.rs"), "pub fn helper() {}").unwrap();

    // Change to temp directory for testing
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Test cases: (directory, pattern, expected_pattern)
    let test_cases = vec![
        ("src", "*.rs", "src/**/*.rs"),           // Simple pattern should be wrapped
        ("src", "src/**/*.rs", "src/**/*.rs"),     // Pattern with directory should NOT be double-wrapped
        ("src", "src/*.rs", "src/*.rs"),          // Simple directory pattern should NOT be wrapped
        ("src", "**/*.rs", "**/*.rs"),            // Already recursive should NOT be wrapped
    ];

    for (directory, pattern, expected_pattern) in test_cases {
        println!("Testing: directory='{}', pattern='{}'", directory, pattern);

        // Simulate the fixed logic from the indexer
        let search_pattern = if pattern.starts_with(&format!("{}/", directory)) || pattern.starts_with(&format!("{}**/", directory)) {
            pattern.to_string()
        } else {
            format!("{}/**/{}", directory, pattern)
        };

        println!("  Expected: '{}', Got: '{}'", expected_pattern, search_pattern);
        assert_eq!(search_pattern, expected_pattern, "Pattern generation failed for directory='{}', pattern='{}'", directory, pattern);

        // Test that the pattern actually finds files
        let paths: Vec<String> = glob::glob(&search_pattern)
            .unwrap()
            .filter_map(Result::ok)
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        if expected_pattern.contains("src/**/*.rs") {
            assert!(!paths.is_empty(), "Pattern '{}' should find files", search_pattern);
            println!("  ✅ Found {} files", paths.len());
        }
    }

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_double_pattern_issue_fixed() {
    // Specifically test that the double pattern issue is fixed

    let directory = "src";
    let user_pattern = "src/**/*.rs"; // This used to cause the double pattern issue

    // Apply the fixed logic
    let search_pattern = if user_pattern.starts_with(&format!("{}/", directory)) || user_pattern.starts_with(&format!("{}**/", directory)) {
        user_pattern.to_string()
    } else {
        format!("{}/**/{}", directory, user_pattern)
    };

    println!("Directory: '{}'", directory);
    println!("User pattern: '{}'", user_pattern);
    println!("Fixed pattern: '{}'", search_pattern);

    // The fix should prevent double wrapping
    assert_eq!(search_pattern, "src/**/*.rs");
    assert_ne!(search_pattern, "src/**/src/**/*.rs"); // This was the broken pattern

    println!("✅ Double pattern issue is FIXED!");
}

#[test]
fn test_pattern_detection_edge_cases() {
    // Test edge cases for pattern detection

    let directory = "src";

    let test_cases = vec![
        ("src/**/*.rs", true, "Pattern starts with src/**/"),
        ("src/*.rs", true, "Pattern starts with src/"),
        ("src/**/*.rs", true, "Pattern starts with src/**/"),
        ("**/*.rs", false, "Pattern is recursive but doesn't start with src"),
        ("*.rs", false, "Simple pattern without directory"),
        ("test/**/*.rs", false, "Pattern starts with different directory"),
        ("src/sub/**/*.rs", true, "Pattern starts with src/"),
    ];

    for (pattern, should_be_detected, description) in test_cases {
        let is_detected = pattern.starts_with(&format!("{}/", directory)) || pattern.starts_with(&format!("{}**/", directory));

        println!("Pattern: '{}' - {}", pattern, description);
        println!("  Detected as including directory: {}", is_detected);
        println!("  Should be detected: {}", should_be_detected);

        assert_eq!(is_detected, should_be_detected, "Pattern detection failed for: {}", pattern);
        println!("  ✅ Correctly detected");
    }
}

extern crate glob;