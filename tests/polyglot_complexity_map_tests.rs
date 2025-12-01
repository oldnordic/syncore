use syncore::polyglot::polyglot_complexity::*;
use syncore::polyglot::polyglot_model::*;

#[test]
fn test_file_level_complexity() {
    let entities = vec![
        UnifiedEntity {
            id: 1,
            language: LanguageType::Rust,
            kind: EntityKind::Function,
            name: "my_function".to_string(),
            file_path: "src/main.rs".to_string(),
            span: Span {
                start: 0,
                end: 10,
            },
            attributes: Default::default(),
        },
        UnifiedEntity {
            id: 2,
            language: LanguageType::Rust,
            kind: EntityKind::Function,
            name: "my_other_function".to_string(),
            file_path: "src/main.rs".to_string(),
            span: Span {
                start: 11,
                end: 20,
            },
            attributes: Default::default(),
        },
    ];
    let complexity_map = calculate_complexity(entities);
    assert_eq!(complexity_map.get("src/main.rs").unwrap(), &2);
}
