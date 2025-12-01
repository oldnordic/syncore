//! Dual-Domain Embedding Tests (TDD)
//!
//! Tests for existing CODE/GENERAL dual-domain architecture.
//! Tests the actual implementation with Code and General domains.
//!
//! Test Coverage:
//! 1. EmbeddingDomain::Code and EmbeddingDomain::General variants exist and behave correctly
//! 2. Code and General domains have distinct namespace mapping
//! 3. Code and General domains have distinct index paths
//! 4. EmbeddingConfig supports Code and General domains
//! 5. Domain routing works correctly

use syncore::vector::domain::{EmbeddingConfig, EmbeddingDomain};

// ============================================================================
// PHASE 1 TESTS: Domain Basics
// ============================================================================

#[test]
fn test_code_domain_variant_exists() {
    // CODE domain must be a valid enum variant
    let code = EmbeddingDomain::Code;
    assert_eq!(format!("{:?}", code), "Code");
}

#[test]
fn test_general_domain_variant_exists() {
    // GENERAL domain must be a valid enum variant
    let general = EmbeddingDomain::General;
    assert_eq!(format!("{:?}", general), "General");
}

#[test]
fn test_code_domain_display() {
    let code = EmbeddingDomain::Code;
    assert_eq!(format!("{}", code), "code");
}

#[test]
fn test_general_domain_display() {
    let general = EmbeddingDomain::General;
    assert_eq!(format!("{}", general), "general");
}

#[test]
fn test_code_domain_from_namespace() {
    // CODE domain namespaces
    assert_eq!(
        EmbeddingDomain::from_namespace("code_entity"),
        EmbeddingDomain::Code
    );
    assert_eq!(
        EmbeddingDomain::from_namespace("rust_code"),
        EmbeddingDomain::Code
    );
    assert_eq!(
        EmbeddingDomain::from_namespace("javascript_code"),
        EmbeddingDomain::Code
    );
}

#[test]
fn test_general_domain_from_namespace() {
    // GENERAL domain namespaces
    assert_eq!(
        EmbeddingDomain::from_namespace("documents"),
        EmbeddingDomain::General
    );
    assert_eq!(
        EmbeddingDomain::from_namespace("plan"),
        EmbeddingDomain::General
    );
    assert_eq!(
        EmbeddingDomain::from_namespace("sequential_cycle"),
        EmbeddingDomain::General
    );
}

#[test]
fn test_code_domain_equality() {
    assert_eq!(EmbeddingDomain::Code, EmbeddingDomain::Code);
    assert_ne!(EmbeddingDomain::Code, EmbeddingDomain::General);
}

#[test]
fn test_general_domain_equality() {
    assert_eq!(EmbeddingDomain::General, EmbeddingDomain::General);
    assert_ne!(EmbeddingDomain::General, EmbeddingDomain::Code);
}

#[test]
fn test_code_domain_serialization() {
    let code = EmbeddingDomain::Code;
    let json = serde_json::to_string(&code).unwrap();
    assert_eq!(json, r#""code""#);
}

#[test]
fn test_general_domain_serialization() {
    let general = EmbeddingDomain::General;
    let json = serde_json::to_string(&general).unwrap();
    assert_eq!(json, r#""general""#);
}

#[test]
fn test_code_domain_deserialization() {
    let code: EmbeddingDomain = serde_json::from_str(r#""code""#).unwrap();
    assert_eq!(code, EmbeddingDomain::Code);
}

#[test]
fn test_general_domain_deserialization() {
    let general: EmbeddingDomain = serde_json::from_str(r#""general""#).unwrap();
    assert_eq!(general, EmbeddingDomain::General);
}

#[test]
fn test_embedding_config_for_code() {
    let config = EmbeddingConfig::for_code();

    assert_eq!(config.domain, EmbeddingDomain::Code);
    assert!(!config.model_name.is_empty());
    assert!(config.index_path.contains("code"));
}

#[test]
fn test_embedding_config_for_general() {
    let config = EmbeddingConfig::for_general();

    assert_eq!(config.domain, EmbeddingDomain::General);
    assert!(!config.model_name.is_empty());
    assert!(config.index_path.contains("general"));
}

#[test]
fn test_code_domain_index_path_distinct() {
    let code_config = EmbeddingConfig::for_code();
    assert_eq!(code_config.index_path, "syncore_code.index");
    assert_ne!(code_config.index_path, "syncore_general.index");
}

#[test]
fn test_general_domain_index_path_distinct() {
    let general_config = EmbeddingConfig::for_general();
    assert_eq!(general_config.index_path, "syncore_general.index");
    assert_ne!(general_config.index_path, "syncore_code.index");
}

#[test]
fn test_dual_domain_index_paths_all_distinct() {
    let code_config = EmbeddingConfig::for_code();
    let general_config = EmbeddingConfig::for_general();

    // Both must have distinct index paths
    assert_ne!(code_config.index_path, general_config.index_path);
}

// ============================================================================
// BACKWARD COMPATIBILITY TESTS
// ============================================================================

#[test]
fn test_code_domain_unchanged() {
    // CODE domain behavior
    let code = EmbeddingDomain::Code;
    assert_eq!(format!("{}", code), "code");
    assert_eq!(
        EmbeddingDomain::from_namespace("code_entity"),
        EmbeddingDomain::Code
    );
    let config = EmbeddingConfig::for_code();
    assert_eq!(config.model_name, "BGE-small-en-v1.5");
}

#[test]
fn test_general_domain_unchanged() {
    // GENERAL domain behavior
    let general = EmbeddingDomain::General;
    assert_eq!(format!("{}", general), "general");
    assert_eq!(
        EmbeddingDomain::from_namespace("documents"),
        EmbeddingDomain::General
    );
    let config = EmbeddingConfig::for_general();
    assert_eq!(config.model_name, "all-MiniLM-L6-v2");
}

#[test]
fn test_unknown_namespace_defaults_to_general() {
    // Unknown namespaces should default to GENERAL
    assert_eq!(
        EmbeddingDomain::from_namespace("unknown_namespace"),
        EmbeddingDomain::General
    );
}
