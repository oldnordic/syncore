use std::collections::HashMap;
use syncore::polyglot::polyglot_aggregator::*;
use syncore::polyglot::polyglot_model::*;

fn create_dummy_entity(id: i64, name: &str, language: LanguageType) -> UnifiedEntity {
    UnifiedEntity {
        id,
        language,
        kind: EntityKind::Function,
        name: name.to_string(),
        file_path: format!("src/{}.rs", name),
        span: Span { start: 0, end: 10 },
        attributes: HashMap::new(),
    }
}

#[test]
fn test_merge_entities_from_multiple_languages() {
    let rust_entity = create_dummy_entity(1, "my_function", LanguageType::Rust);
    let python_entity = create_dummy_entity(2, "my_function", LanguageType::Python);

    let merged_entities = merge_entities(vec![rust_entity, python_entity]);

    assert_eq!(
        merged_entities.len(),
        1,
        "Should merge entities with the same name"
    );
}

#[test]
fn test_resolve_naming_collisions() {
    let python_entity = create_dummy_entity(2, "my_function", LanguageType::Python);
    let rust_entity = create_dummy_entity(1, "my_function", LanguageType::Rust);

    let merged_entities = merge_entities(vec![python_entity, rust_entity]);
    let merged_entity = merged_entities.first().unwrap();

    assert_eq!(
        merged_entity.language,
        LanguageType::Rust,
        "Rust should be the default language for merged entities"
    );
}

#[test]
fn test_derive_stable_deterministic_ids() {
    let entity1 = create_dummy_entity(1, "my_function", LanguageType::Rust);
    let entity2 = create_dummy_entity(1, "my_function", LanguageType::Rust);

    let merged_entities1 = merge_entities(vec![entity1]);
    let merged_entities2 = merge_entities(vec![entity2]);

    assert_eq!(
        merged_entities1.first().unwrap().id,
        merged_entities2.first().unwrap().id,
        "Entities with same name and language should have same id"
    );
}

#[test]
fn test_normalize_namespaces() {
    let mut entity = create_dummy_entity(1, "my_function", LanguageType::Rust);
    entity
        .attributes
        .insert("namespace".to_string(), "crate::my_module".to_string());

    let merged_entities = merge_entities(vec![entity]);
    let merged_entity = merged_entities.first().unwrap();

    assert_eq!(
        merged_entity.attributes.get("namespace").unwrap(),
        "my_module",
        "Should normalize namespaces"
    );
}
