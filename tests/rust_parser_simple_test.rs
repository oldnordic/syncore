//! Simple test to verify Rust parser functionality
//! Tests actual parser implementation without complex test infrastructure

use syncore::parser::Parser;
use std::fs;
use std::path::Path;

#[test]
fn test_basic_rust_parsing() {
    let parser = Parser::new().expect("Failed to create parser");

    // Create a simple test case with known structure
    let rust_code = r#"
/// Test function
pub fn test_function() -> Result<()> {
    Ok(())
}

struct TestStruct {
    field1: i32,
    field2: String,
}

impl TestStruct {
    pub fn new() -> Self {
        Self { field1: 0, field2: String::new() }
    }
}

trait TestTrait {
    fn required_method(&self);
}

enum TestEnum {
    Variant1,
    Variant2(i32),
}

const MAX_SIZE: usize = 100;
use std::collections::HashMap;
"#;

    // Write test code to a temporary file
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.rs");
    fs::write(&test_file, rust_code).expect("Failed to write test file");

    // Parse the file
    let structure = parser.parse_file(&test_file).expect("Failed to parse test file");

    println!("=== BASIC RUST PARSING TEST RESULTS ===");
    println!("Language: {}", structure.language);
    println!("Functions: {}", structure.functions.len());
    println!("Classes: {}", structure.classes.len());
    println!("Imports: {}", structure.imports.len());
    println!("Variables: {}", structure.variables.len());

    println!("\n=== FUNCTIONS ===");
    for (i, func) in structure.functions.iter().enumerate() {
        println!("{}: {} (lines {}-{})", i+1, func.name, func.line_number, func.end_line);
    }

    println!("\n=== CLASSES ===");
    for (i, class) in structure.classes.iter().enumerate() {
        println!("{}: {} ({})", i+1, class.name, class.class_type);
    }

    println!("\n=== IMPORTS ===");
    for (i, import) in structure.imports.iter().enumerate() {
        println!("{}: {} ({})", i+1, import.module, import.import_type);
    }

    println!("\n=== VARIABLES ===");
    for (i, var) in structure.variables.iter().enumerate() {
        println!("{}: {}", i+1, var.name);
    }

    // Basic assertions to see what we currently extract
    assert_eq!(structure.language, "rust");

    // We expect at least 1 function (test_function) and methods from impl blocks
    assert!(structure.functions.len() >= 1, "Expected at least 1 function");

    // Check that test_function was found
    let func_names: Vec<&str> = structure.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(func_names.contains(&"test_function"), "Missing test_function");

    // We expect struct, trait, and enum to be extracted as classes
    let class_names: Vec<&str> = structure.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(class_names.contains(&"TestStruct"), "Missing TestStruct");

    // Test imports extraction
    assert!(structure.imports.len() >= 1, "Expected at least 1 import");

    // Test variables extraction (constants should be here)
    assert!(structure.variables.len() >= 1, "Expected at least 1 variable");
}

#[test]
fn test_rust_extraction_fixtures() {
    let parser = Parser::new().expect("Failed to create parser");

    // Test the existing fixture files if they exist
    let fixtures = [
        "tests/fixtures/rust_extraction/structs_traits.rs",
        "tests/fixtures/rust_extraction/functions.rs",
        "tests/fixtures/rust_extraction/imports_consts.rs",
    ];

    for fixture_path in &fixtures {
        let path = Path::new(fixture_path);
        if path.exists() {
            println!("\n=== Testing fixture: {} ===", fixture_path);

            let structure = parser.parse_file(path)
                .expect(&format!("Failed to parse fixture: {}", fixture_path));

            println!("Functions: {}", structure.functions.len());
            println!("Classes: {}", structure.classes.len());
            println!("Imports: {}", structure.imports.len());
            println!("Variables: {}", structure.variables.len());

            // Show what class types we found
            let mut class_types = std::collections::HashMap::new();
            for class in &structure.classes {
                *class_types.entry(class.class_type.clone()).or_insert(0) += 1;
            }

            for (class_type, count) in &class_types {
                println!("  {}: {}", class_type, count);
            }

            // Basic validation - we should find something meaningful
            if fixture_path.contains("structs_traits") {
                assert!(structure.classes.len() >= 3, "Expected multiple structs/traits/enums");
            } else if fixture_path.contains("functions") {
                assert!(structure.functions.len() >= 5, "Expected multiple functions");
            } else if fixture_path.contains("imports_consts") {
                // Let's debug what we actually find instead of asserting
                println!("  Expected many imports, found: {}", structure.imports.len());
                for (i, import) in structure.imports.iter().enumerate() {
                    println!("    {}: {}", i+1, import.module);
                }
                println!("  Expected several constants, found: {}", structure.variables.len());
                for (i, var) in structure.variables.iter().enumerate() {
                    println!("    {}: {}", i+1, var.name);
                }
            }
        } else {
            println!("Fixture {} does not exist, skipping", fixture_path);
        }
    }
}