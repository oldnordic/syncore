use crate::polyglot::polyglot_model::{UnifiedEdge, UnifiedEntity};
use serde_json::{json, Value};

pub fn generate_architecture_overview(
    entities: Vec<UnifiedEntity>,
    edges: Vec<UnifiedEdge>,
) -> String {
    let nodes: Vec<Value> = entities
        .into_iter()
        .map(|e| json!({ "id": e.id, "label": e.name, "group": e.language.to_string() }))
        .collect();

    let edges: Vec<Value> =
        edges.into_iter().map(|e| json!({ "from": e.from_id, "to": e.to_id })).collect();

    json!({
        "nodes": nodes,
        "edges": edges,
    })
    .to_string()
}

// Add a to_string implementation for LanguageType
impl ToString for crate::polyglot::polyglot_model::LanguageType {
    fn to_string(&self) -> String {
        match self {
            crate::polyglot::polyglot_model::LanguageType::Rust => "Rust".to_string(),
            crate::polyglot::polyglot_model::LanguageType::Python => "Python".to_string(),
            crate::polyglot::polyglot_model::LanguageType::Java => "Java".to_string(),
            crate::polyglot::polyglot_model::LanguageType::TypeScript => "TypeScript".to_string(),
            crate::polyglot::polyglot_model::LanguageType::JavaScript => "JavaScript".to_string(),
            crate::polyglot::polyglot_model::LanguageType::Go => "Go".to_string(),
            crate::polyglot::polyglot_model::LanguageType::C => "C".to_string(),
            crate::polyglot::polyglot_model::LanguageType::Cpp => "C++".to_string(),
        }
    }
}
