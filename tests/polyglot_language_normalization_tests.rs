use syncore::polyglot::polyglot_aggregator::*;
use syncore::polyglot::polyglot_model::*;

#[test]
fn test_normalize_rust_module() {
    let mut entity = UnifiedEntity {
        id: 1,
        language: LanguageType::Rust,
        kind: EntityKind::Module,
        name: "my_module".to_string(),
        file_path: "src/my_module/mod.rs".to_string(),
        span: Span { start: 0, end: 10 },
        attributes: Default::default(),
    };
    normalize_entity_namespace(&mut entity);
    assert_eq!(entity.attributes.get("namespace").unwrap(), "my_module");
}

#[test]
fn test_normalize_java_package() {
    let mut entity = UnifiedEntity {
        id: 1,
        language: LanguageType::Java,
        kind: EntityKind::Package,
        name: "my.package".to_string(),
        file_path: "src/my/package/MyClass.java".to_string(),
        span: Span { start: 0, end: 10 },
        attributes: Default::default(),
    };
    normalize_entity_namespace(&mut entity);
    assert_eq!(entity.attributes.get("namespace").unwrap(), "my.package");
}

#[test]
fn test_normalize_ts_import() {
    let mut entity = UnifiedEntity {
        id: 1,
        language: LanguageType::TypeScript,
        kind: EntityKind::Module,
        name: "my-module".to_string(),
        file_path: "src/my-module.ts".to_string(),
        span: Span { start: 0, end: 10 },
        attributes: Default::default(),
    };
    normalize_entity_namespace(&mut entity);
    assert_eq!(entity.attributes.get("namespace").unwrap(), "my-module");
}

#[test]
fn test_normalize_go_module() {
    let mut entity = UnifiedEntity {
        id: 1,
        language: LanguageType::Go,
        kind: EntityKind::Module,
        name: "github.com/my-org/my-repo".to_string(),
        file_path: "main.go".to_string(),
        span: Span { start: 0, end: 10 },
        attributes: Default::default(),
    };
    normalize_entity_namespace(&mut entity);
    assert_eq!(
        entity.attributes.get("namespace").unwrap(),
        "github.com/my-org/my-repo"
    );
}

#[test]
fn test_normalize_cpp_namespace() {
    let mut entity = UnifiedEntity {
        id: 1,
        language: LanguageType::Cpp,
        kind: EntityKind::Namespace,
        name: "my_namespace".to_string(),
        file_path: "src/my_namespace.cpp".to_string(),
        span: Span { start: 0, end: 10 },
        attributes: Default::default(),
    };
    normalize_entity_namespace(&mut entity);
    assert_eq!(entity.attributes.get("namespace").unwrap(), "my_namespace");
}

#[test]
fn test_normalize_python_package() {
    let mut entity = UnifiedEntity {
        id: 1,
        language: LanguageType::Python,
        kind: EntityKind::Package,
        name: "my_package".to_string(),
        file_path: "my_package/__init__.py".to_string(),
        span: Span { start: 0, end: 10 },
        attributes: Default::default(),
    };
    normalize_entity_namespace(&mut entity);
    assert_eq!(entity.attributes.get("namespace").unwrap(), "my_package");
}
