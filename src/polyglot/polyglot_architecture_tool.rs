use crate::polyglot::polyglot_model::{UnifiedEdge, UnifiedEntity};
use serde_json::{json, Value};
use std::fmt;

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

// Add a Display implementation for LanguageType
impl fmt::Display for crate::polyglot::polyglot_model::LanguageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            crate::polyglot::polyglot_model::LanguageType::Rust => write!(f, "Rust"),
            crate::polyglot::polyglot_model::LanguageType::Python => write!(f, "Python"),
            crate::polyglot::polyglot_model::LanguageType::Java => write!(f, "Java"),
            crate::polyglot::polyglot_model::LanguageType::TypeScript => write!(f, "TypeScript"),
            crate::polyglot::polyglot_model::LanguageType::JavaScript => write!(f, "JavaScript"),
            crate::polyglot::polyglot_model::LanguageType::Go => write!(f, "Go"),
            crate::polyglot::polyglot_model::LanguageType::C => write!(f, "C"),
            crate::polyglot::polyglot_model::LanguageType::Cpp => write!(f, "C++"),
        }
    }
}
