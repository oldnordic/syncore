// SPEC: SYNCORE-CODE-TOOLS-IMPROVEMENT-01 (APEX v1.2)
// STEP C: Fusion query entity type boosting tests
//
// Tests verify that Function/Struct/Impl entities are boosted
// and Import entities are de-boosted in fusion scoring.

use syncore::code_graph::entity_boost::{compute_entity_type_boost, EntityCategory};

/// Test: Function entities get high boost
#[test]
fn test_function_boost() {
    let boost = compute_entity_type_boost("Function");
    assert!(boost > 1.0, "Function should get positive boost, got {}", boost);
    assert!(boost >= 1.3, "Function boost should be >= 1.3, got {}", boost);
}

/// Test: Struct entities get high boost
#[test]
fn test_struct_boost() {
    let boost = compute_entity_type_boost("Struct");
    assert!(boost > 1.0, "Struct should get positive boost, got {}", boost);
    assert!(boost >= 1.3, "Struct boost should be >= 1.3, got {}", boost);
}

/// Test: Impl block entities get high boost
#[test]
fn test_impl_boost() {
    let boost = compute_entity_type_boost("Impl");
    assert!(boost > 1.0, "Impl should get positive boost, got {}", boost);
    assert!(boost >= 1.2, "Impl boost should be >= 1.2, got {}", boost);
}

/// Test: Class entities get high boost
#[test]
fn test_class_boost() {
    let boost = compute_entity_type_boost("Class");
    assert!(boost > 1.0, "Class should get positive boost, got {}", boost);
    assert!(boost >= 1.3, "Class boost should be >= 1.3, got {}", boost);
}

/// Test: Method entities get high boost
#[test]
fn test_method_boost() {
    let boost = compute_entity_type_boost("Method");
    assert!(boost > 1.0, "Method should get positive boost, got {}", boost);
    assert!(boost >= 1.2, "Method boost should be >= 1.2, got {}", boost);
}

/// Test: Trait entities get moderate boost
#[test]
fn test_trait_boost() {
    let boost = compute_entity_type_boost("Trait");
    assert!(boost > 1.0, "Trait should get positive boost, got {}", boost);
    assert!(boost >= 1.15, "Trait boost should be >= 1.15, got {}", boost);
}

/// Test: Enum entities get moderate boost
#[test]
fn test_enum_boost() {
    let boost = compute_entity_type_boost("Enum");
    assert!(boost > 1.0, "Enum should get positive boost, got {}", boost);
    assert!(boost >= 1.15, "Enum boost should be >= 1.15, got {}", boost);
}

/// Test: Import entities get de-boosted
#[test]
fn test_import_deboost() {
    let boost = compute_entity_type_boost("Import");
    assert!(boost < 1.0, "Import should get negative boost (de-boost), got {}", boost);
    assert!(boost <= 0.7, "Import de-boost should be <= 0.7, got {}", boost);
}

/// Test: Use statement entities get de-boosted
#[test]
fn test_use_deboost() {
    let boost = compute_entity_type_boost("Use");
    assert!(boost < 1.0, "Use should get negative boost (de-boost), got {}", boost);
    assert!(boost <= 0.7, "Use de-boost should be <= 0.7, got {}", boost);
}

/// Test: Module entities are neutral
#[test]
fn test_module_neutral() {
    let boost = compute_entity_type_boost("Module");
    assert!((boost - 1.0).abs() < 0.1, "Module should be neutral (1.0 ± 0.1), got {}", boost);
}

/// Test: Unknown entity types are neutral
#[test]
fn test_unknown_neutral() {
    let boost = compute_entity_type_boost("SomeUnknownType");
    assert!((boost - 1.0).abs() < 0.01, "Unknown types should be neutral (1.0), got {}", boost);
}

/// Test: Empty string is neutral
#[test]
fn test_empty_neutral() {
    let boost = compute_entity_type_boost("");
    assert!((boost - 1.0).abs() < 0.01, "Empty string should be neutral (1.0), got {}", boost);
}

/// Test: Case insensitivity
#[test]
fn test_case_insensitive() {
    let boost_lower = compute_entity_type_boost("function");
    let boost_upper = compute_entity_type_boost("FUNCTION");
    let boost_mixed = compute_entity_type_boost("Function");

    assert!((boost_lower - boost_upper).abs() < 0.01, "Should be case insensitive");
    assert!((boost_lower - boost_mixed).abs() < 0.01, "Should be case insensitive");
}

/// Test: EntityCategory classification
#[test]
fn test_entity_category_implementation() {
    assert_eq!(EntityCategory::from_kind("Function"), EntityCategory::Implementation);
    assert_eq!(EntityCategory::from_kind("Struct"), EntityCategory::Implementation);
    assert_eq!(EntityCategory::from_kind("Class"), EntityCategory::Implementation);
}

#[test]
fn test_entity_category_import() {
    assert_eq!(EntityCategory::from_kind("Import"), EntityCategory::Import);
    assert_eq!(EntityCategory::from_kind("Use"), EntityCategory::Import);
}

#[test]
fn test_entity_category_secondary() {
    assert_eq!(EntityCategory::from_kind("Trait"), EntityCategory::Secondary);
    assert_eq!(EntityCategory::from_kind("Enum"), EntityCategory::Secondary);
    assert_eq!(EntityCategory::from_kind("TypeAlias"), EntityCategory::Secondary);
}

#[test]
fn test_entity_category_neutral() {
    assert_eq!(EntityCategory::from_kind("Module"), EntityCategory::Neutral);
    assert_eq!(EntityCategory::from_kind("SomethingElse"), EntityCategory::Neutral);
}

/// Test: Boosted score is higher than base score for implementations
#[test]
fn test_boosted_function_ranks_higher() {
    let base_score = 0.7;
    let function_boost = compute_entity_type_boost("Function");
    let import_boost = compute_entity_type_boost("Import");

    let function_score = base_score * function_boost;
    let import_score = base_score * import_boost;

    assert!(
        function_score > import_score,
        "Function score ({}) should be higher than Import score ({})",
        function_score,
        import_score
    );
}
