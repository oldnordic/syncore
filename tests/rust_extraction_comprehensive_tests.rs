//! Comprehensive Rust extraction tests
//! Tests based on actual tree-sitter node types and real parser implementation
//! These tests should FAIL with current implementation and PASS after fixes

use syncore::parser::{Parser, CodeStructure, ClassInfo, FunctionInfo, ImportInfo, VariableInfo};
use std::fs;
use std::path::Path;

#[test]
fn test_rust_structs_traits_enums_extraction() {
    let parser = Parser::new().expect("Failed to create parser");

    // Load the test fixture
    let fixture_path = Path::new("tests/fixtures/rust_extraction/structs_traits.rs");
    assert!(fixture_path.exists(), "Fixture file should exist");

    let structure = parser.parse_file(fixture_path)
        .expect("Failed to parse fixture file");

    println!("=== PARSING RESULTS ===");
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
        println!("{}: {} ({}) at line {}", i+1, class.name, class.class_type, class.line_number);
    }

    // ASSERTIONS - These should FAIL with current implementation

    // Expected structs from fixture: TestStruct, InnerStruct, PublicStruct, GenericStruct, TupleStruct
    let structs: Vec<&ClassInfo> = structure.classes.iter()
        .filter(|c| c.class_type == "struct")
        .collect();

    println!("Found {} structs", structs.len());
    for s in &structs {
        println!("  - {} at line {}", s.name, s.line_number);
    }

    // Expected traits from fixture: TestTrait, PublicTrait
    let traits: Vec<&ClassInfo> = structure.classes.iter()
        .filter(|c| c.class_type == "trait")
        .collect();

    println!("Found {} traits", traits.len());
    for t in &traits {
        println!("  - {} at line {}", t.name, t.line_number);
    }

    // Expected enums from fixture: TestEnum, PublicEnum
    let enums: Vec<&ClassInfo> = structure.classes.iter()
        .filter(|c| c.class_type == "enum")
        .collect();

    println!("Found {} enums", enums.len());
    for e in &enums {
        println!("  - {} at line {}", e.name, e.line_number);
    }

    // Expected functions: All methods from impl blocks
    println!("Found {} functions", structure.functions.len());

    // These assertions should FAIL initially, indicating the parser needs fixes
    assert_eq!(structs.len(), 5, "Expected 5 structs, got {}", structs.len());
    assert_eq!(traits.len(), 2, "Expected 2 traits, got {}", traits.len());
    assert_eq!(enums.len(), 2, "Expected 2 enums, got {}", enums.len());

    // Verify specific entities exist
    let struct_names: Vec<&str> = structs.iter().map(|s| s.name.as_str()).collect();
    assert!(struct_names.contains(&"TestStruct"), "Missing TestStruct");
    assert!(struct_names.contains(&"InnerStruct"), "Missing InnerStruct");
    assert!(struct_names.contains(&"PublicStruct"), "Missing PublicStruct");
    assert!(struct_names.contains(&"GenericStruct"), "Missing GenericStruct");
    assert!(struct_names.contains(&"TupleStruct"), "Missing TupleStruct");

    let trait_names: Vec<&str> = traits.iter().map(|t| t.name.as_str()).collect();
    assert!(trait_names.contains(&"TestTrait"), "Missing TestTrait");
    assert!(trait_names.contains(&"PublicTrait"), "Missing PublicTrait");

    let enum_names: Vec<&str> = enums.iter().map(|e| e.name.as_str()).collect();
    assert!(enum_names.contains(&"TestEnum"), "Missing TestEnum");
    assert!(enum_names.contains(&"PublicEnum"), "Missing PublicEnum");
}

#[test]
fn test_rust_functions_extraction() {
    let parser = Parser::new().expect("Failed to create parser");

    let fixture_path = Path::new("tests/fixtures/rust_extraction/functions.rs");
    assert!(fixture_path.exists(), "Fixture file should exist");

    let structure = parser.parse_file(fixture_path)
        .expect("Failed to parse fixture file");

    println!("=== FUNCTIONS EXTRACTION RESULTS ===");
    println!("Total functions found: {}", structure.functions.len());

    for (i, func) in structure.functions.iter().enumerate() {
        println!("{}: {} (lines {}-{})", i+1, func.name, func.line_number, func.end_line);
        println!("   Parameters: {:?}", func.parameters);
        println!("   Return type: {:?}", func.return_type);
        println!("   Visibility: {:?}", func.visibility);
        println!("   Docstring: {:?}", func.docstring);
        println!();
    }

    // Expected functions from fixture:
    // Free functions: free_function, async_function, generic_function, private_function, another_free (5)
    // Methods from impls: new, get_value, internal_logic, create_default, add_item, count, trait_method, default_trait_method (8)
    // Total expected: 13 functions

    // This should FAIL initially - current parser may not capture all functions
    assert!(structure.functions.len() >= 10,
           "Expected at least 10 functions, got {}", structure.functions.len());

    // Verify specific function types exist
    let function_names: Vec<&str> = structure.functions.iter().map(|f| f.name.as_str()).collect();

    // Free functions
    assert!(function_names.contains(&"free_function"), "Missing free_function");
    assert!(function_names.contains(&"async_function"), "Missing async_function");
    assert!(function_names.contains(&"generic_function"), "Missing generic_function");
    assert!(function_names.contains(&"another_free"), "Missing another_free");

    // Methods (these should be extracted from impl blocks)
    assert!(function_names.contains(&"new"), "Missing new method");
    assert!(function_names.contains(&"get_value"), "Missing get_value method");
    assert!(function_names.contains(&"create_default"), "Missing create_default method");
    assert!(function_names.contains(&"trait_method"), "Missing trait_method");
}

#[test]
fn test_rust_imports_and_constants_extraction() {
    let parser = Parser::new().expect("Failed to create parser");

    let fixture_path = Path::new("tests/fixtures/rust_extraction/imports_consts.rs");
    assert!(fixture_path.exists(), "Fixture file should exist");

    let structure = parser.parse_file(fixture_path)
        .expect("Failed to parse fixture file");

    println!("=== IMPORTS AND CONSTANTS RESULTS ===");
    println!("Imports found: {}", structure.imports.len());
    for (i, import) in structure.imports.iter().enumerate() {
        println!("{}: {} (line {})", i+1, import.module, import.line_number);
        if let Some(ref alias) = import.alias {
            println!("   Alias: {}", alias);
        }
    }

    println!("\nVariables found: {}", structure.variables.len());
    for (i, var) in structure.variables.iter().enumerate() {
        println!("{}: {} (line {})", i+1, var.name, var.line_number);
        println!("   Type: {:?}", var.var_type);
        println!("   Value: {:?}", var.value);
        println!("   Visibility: {:?}", var.visibility);
    }

    // Expected imports: 12 from fixture (see comments)
    // This should FAIL initially - parser may not extract all import formats correctly
    assert!(structure.imports.len() >= 8,
           "Expected at least 8 imports, got {}", structure.imports.len());

    // Verify specific imports exist
    let import_modules: Vec<&str> = structure.imports.iter().map(|i| i.module.as_str()).collect();
    assert!(import_modules.iter().any(|m| m.contains("HashMap")), "Missing HashMap import");
    assert!(import_modules.iter().any(|m| m.contains("Duration")), "Missing Duration import");

    // Expected variables: constants and static variables
    // The parser extracts const/static items as variables
    assert!(structure.variables.len() >= 4,
           "Expected at least 4 constants/statics, got {}", structure.variables.len());

    // Verify specific constants exist
    let variable_names: Vec<&str> = structure.variables.iter().map(|v| v.name.as_str()).collect();
    assert!(variable_names.contains(&"MAX_SIZE"), "Missing MAX_SIZE constant");
    assert!(variable_names.contains(&"GLOBAL_COUNTER"), "Missing GLOBAL_COUNTER static");
}

#[test]
fn test_rust_field_extraction() {
    let parser = Parser::new().expect("Failed to create parser");

    let fixture_path = Path::new("tests/fixtures/rust_extraction/structs_traits.rs");
    let structure = parser.parse_file(fixture_path)
        .expect("Failed to parse fixture file");

    println!("=== FIELD EXTRACTION RESULTS ===");

    // Check that structs have their fields extracted
    let structs: Vec<&ClassInfo> = structure.classes.iter()
        .filter(|c| c.class_type == "struct")
        .collect();

    for class_info in structs {
        println!("Struct: {} has {} fields", class_info.name, class_info.fields.len());
        for (i, field) in class_info.fields.iter().enumerate() {
            println!("  {}: {} -> {:?}", i+1, field.name, field.var_type);
        }
    }

    // TestStruct should have fields: field1, field2
    let test_struct = structs.iter().find(|s| s.name == "TestStruct");
    if let Some(ts) = test_struct {
        assert_eq!(ts.fields.len(), 2, "TestStruct should have 2 fields, got {}", ts.fields.len());

        let field_names: Vec<&str> = ts.fields.iter().map(|f| f.name.as_str()).collect();
        assert!(field_names.contains(&"field1"), "Missing field1");
        assert!(field_names.contains(&"field2"), "Missing field2");
    } else {
        panic!("TestStruct not found in parsed results");
    }

    // PublicStruct should have fields: items, count
    let public_struct = structs.iter().find(|s| s.name == "PublicStruct");
    if let Some(ps) = public_struct {
        assert_eq!(ps.fields.len(), 2, "PublicStruct should have 2 fields, got {}", ps.fields.len());

        let field_names: Vec<&str> = ps.fields.iter().map(|f| f.name.as_str()).collect();
        assert!(field_names.contains(&"items"), "Missing items field");
        assert!(field_names.contains(&"count"), "Missing count field");
    } else {
        panic!("PublicStruct not found in parsed results");
    }
}

#[test]
fn test_rust_visibility_extraction() {
    let parser = Parser::new().expect("Failed to create parser");

    let fixture_path = Path::new("tests/fixtures/rust_extraction/structs_traits.rs");
    let structure = parser.parse_file(fixture_path)
        .expect("Failed to parse fixture file");

    println!("=== VISIBILITY EXTRACTION RESULTS ===");

    // Check function visibility
    for func in &structure.functions {
        println!("Function: {} -> {:?}", func.name, func.visibility);
    }

    // Check struct/field visibility
    for class in &structure.classes {
        println!("Class: {} -> {:?}", class.name, class.visibility);
        for field in &class.fields {
            println!("  Field: {} -> {:?}", field.name, field.visibility);
        }
    }

    // Should find some pub functions and some private ones
    let pub_functions: Vec<&FunctionInfo> = structure.functions.iter()
        .filter(|f| f.visibility.as_ref().map_or(false, |v| v.contains("pub")))
        .collect();

    let private_functions: Vec<&FunctionInfo> = structure.functions.iter()
        .filter(|f| f.visibility.is_none() || f.visibility.as_ref().map_or(false, |v| !v.contains("pub")))
        .collect();

    println!("Public functions: {}", pub_functions.len());
    println!("Private functions: {}", private_functions.len());

    // Should have both public and private functions
    assert!(pub_functions.len() > 0, "Should have at least one public function");
    assert!(private_functions.len() > 0, "Should have at least one private function");
}