use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LanguageType {
    Rust,
    Python,
    Java,
    TypeScript,
    JavaScript,
    Go,
    C,
    Cpp,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Typedef,
    Macro,
    Header,
    Module,
    Namespace,
    File,
    Package,
    Interface,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedEntity {
    pub id: i64,
    pub language: LanguageType,
    pub kind: EntityKind,
    pub name: String,
    pub file_path: String,
    pub span: Span,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UnifiedEdge {
    pub from_id: i64,
    pub to_id: i64,
    pub edge_type: String,
}
