// SPEC: SYNCORE-CODE-TOOLS-IMPROVEMENT-01 (APEX v1.2)
// STEP A: Build artifact exclusion tests for code_index_directory
//
// These tests verify that target/, node_modules/, .git/, and other
// build artifact directories are excluded from code indexing.

use syncore::macro_tools::path_filter::should_index_path;

/// Test: should_index_path returns true for normal source files
#[test]
fn test_should_index_path_allows_src_main_rs() {
    assert!(should_index_path("src/main.rs"));
}

#[test]
fn test_should_index_path_allows_src_lib_rs() {
    assert!(should_index_path("src/lib.rs"));
}

#[test]
fn test_should_index_path_allows_src_subdir_mod_rs() {
    assert!(should_index_path("src/memory/mod.rs"));
}

#[test]
fn test_should_index_path_allows_tests_dir() {
    assert!(should_index_path("tests/integration_test.rs"));
}

#[test]
fn test_should_index_path_allows_examples_dir() {
    assert!(should_index_path("examples/demo.rs"));
}

/// Test: should_index_path returns false for Rust build artifacts
#[test]
fn test_should_index_path_excludes_target_debug() {
    assert!(!should_index_path("target/debug/build/typenum/tests.rs"));
}

#[test]
fn test_should_index_path_excludes_target_release() {
    assert!(!should_index_path("target/release/deps/something.rs"));
}

#[test]
fn test_should_index_path_excludes_target_root() {
    assert!(!should_index_path("target/whatever.rs"));
}

/// Test: should_index_path returns false for absolute paths with target/
#[test]
fn test_should_index_path_excludes_absolute_target_path() {
    assert!(!should_index_path("/home/user/project/target/debug/build/foo.rs"));
}

/// Test: should_index_path returns false for Node.js artifacts
#[test]
fn test_should_index_path_excludes_node_modules() {
    assert!(!should_index_path("node_modules/lodash/index.js"));
}

#[test]
fn test_should_index_path_excludes_nested_node_modules() {
    assert!(!should_index_path("packages/app/node_modules/react/index.js"));
}

/// Test: should_index_path returns false for version control
#[test]
fn test_should_index_path_excludes_git_objects() {
    assert!(!should_index_path(".git/objects/ab/123456.pack"));
}

#[test]
fn test_should_index_path_excludes_git_hooks() {
    assert!(!should_index_path(".git/hooks/pre-commit"));
}

/// Test: should_index_path returns false for Python artifacts
#[test]
fn test_should_index_path_excludes_pycache() {
    assert!(!should_index_path("src/__pycache__/module.cpython-311.pyc"));
}

#[test]
fn test_should_index_path_excludes_venv() {
    assert!(!should_index_path(".venv/lib/python3.11/site-packages/module.py"));
}

#[test]
fn test_should_index_path_excludes_venv_dir() {
    assert!(!should_index_path("venv/bin/activate"));
}

/// Test: should_index_path returns false for Go artifacts
#[test]
fn test_should_index_path_excludes_go_vendor() {
    assert!(!should_index_path("vendor/github.com/pkg/errors/errors.go"));
}

/// Test: should_index_path returns false for IDE/editor directories
#[test]
fn test_should_index_path_excludes_vscode() {
    assert!(!should_index_path(".vscode/settings.json"));
}

#[test]
fn test_should_index_path_excludes_idea() {
    assert!(!should_index_path(".idea/workspace.xml"));
}

/// Test: should_index_path returns false for build output directories
#[test]
fn test_should_index_path_excludes_dist() {
    assert!(!should_index_path("dist/bundle.js"));
}

#[test]
fn test_should_index_path_excludes_build() {
    assert!(!should_index_path("build/output.js"));
}

#[test]
fn test_should_index_path_excludes_out() {
    assert!(!should_index_path("out/compiled.js"));
}

/// Test: Edge cases and path variations
#[test]
fn test_should_index_path_allows_target_in_filename() {
    // File named "target_parser.rs" in src/ should be allowed
    assert!(should_index_path("src/target_parser.rs"));
}

#[test]
fn test_should_index_path_allows_build_in_filename() {
    // File named "build_config.rs" in src/ should be allowed
    assert!(should_index_path("src/build_config.rs"));
}

#[test]
fn test_should_index_path_handles_empty_path() {
    // Empty path should be excluded
    assert!(!should_index_path(""));
}

#[test]
fn test_should_index_path_handles_relative_dots() {
    // Relative path with ./src/ should be allowed
    assert!(should_index_path("./src/main.rs"));
}

#[test]
fn test_should_index_path_excludes_cargo_registry() {
    // Cargo registry dependencies
    assert!(!should_index_path(
        "/home/user/.cargo/registry/src/index.crates.io-12345/serde-1.0.0/src/lib.rs"
    ));
}

/// Test: Java/JVM artifacts
#[test]
fn test_should_index_path_excludes_java_target() {
    assert!(!should_index_path("target/classes/com/example/Main.class"));
}

/// Test: C/C++ build artifacts
#[test]
fn test_should_index_path_excludes_cmake_build() {
    assert!(!should_index_path("cmake-build-debug/CMakeFiles/main.cpp.o"));
}

/// Test: Coverage and test output
#[test]
fn test_should_index_path_excludes_coverage() {
    assert!(!should_index_path("coverage/lcov.info"));
}

#[test]
fn test_should_index_path_excludes_htmlcov() {
    assert!(!should_index_path("htmlcov/index.html"));
}
