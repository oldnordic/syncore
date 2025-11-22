//! Data model types for code graph entities and relationships

/// Query scope for fusion queries - controls search breadth across projects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum QueryScope {
    /// Restrict to current project + local file/directory focus
    Local,
    /// Restrict to current project only (default)
    #[default]
    Project,
    /// Search all projects in workspace
    Workspace,
    /// Search entire index without restriction
    Global,
    /// Engine/LLM decides based on heuristics (currently aliases to Project)
    Auto,
}

impl QueryScope {
    /// Parse scope from string (for MCP tool interface)
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "local" => QueryScope::Local,
            "project" => QueryScope::Project,
            "workspace" => QueryScope::Workspace,
            "global" => QueryScope::Global,
            "auto" => QueryScope::Auto,
            _ => QueryScope::Project, // Default to Project for unknown values
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            QueryScope::Local => "local",
            QueryScope::Project => "project",
            QueryScope::Workspace => "workspace",
            QueryScope::Global => "global",
            QueryScope::Auto => "auto",
        }
    }
}

/// Represents a code entity (function, class, import, etc.)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeEntity {
    pub id: Option<i64>,
    pub file_path: String,
    pub entity_type: EntityType,
    pub name: String,
    pub signature: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
    pub docstring: Option<String>,
    pub language: String,
    // PHASE 3: Temporal metadata fields
    pub created_at: Option<i64>,      // Unix timestamp from Git or filesystem
    pub last_modified_at: Option<i64>, // Unix timestamp from Git or filesystem
    pub change_count: Option<i32>,     // Number of commits touching this entity
    pub author_count: Option<i32>,     // Number of unique authors
}

impl CodeEntity {
    /// Create a new CodeEntity without temporal metadata (to be enriched later)
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        file_path: String,
        entity_type: EntityType,
        name: String,
        signature: Option<String>,
        line_start: usize,
        line_end: usize,
        docstring: Option<String>,
        language: String,
    ) -> Self {
        Self {
            id: None,
            file_path,
            entity_type,
            name,
            signature,
            line_start,
            line_end,
            docstring,
            language,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        }
    }
}

/// Types of code entities we can extract
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EntityType {
    Function,
    Class,
    Method,
    Import,
    Struct,
    Enum,
    Trait,
}

impl EntityType {
    pub fn as_str(&self) -> &str {
        match self {
            EntityType::Function => "function",
            EntityType::Class => "class",
            EntityType::Method => "method",
            EntityType::Import => "import",
            EntityType::Struct => "struct",
            EntityType::Enum => "enum",
            EntityType::Trait => "trait",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "function" => Some(EntityType::Function),
            "class" => Some(EntityType::Class),
            "method" => Some(EntityType::Method),
            "import" => Some(EntityType::Import),
            "struct" => Some(EntityType::Struct),
            "enum" => Some(EntityType::Enum),
            "trait" => Some(EntityType::Trait),
            _ => None,
        }
    }
}

/// Represents a relationship between code entities
#[derive(Debug, Clone)]
pub struct CodeEdge {
    pub src_entity_id: i64,
    pub dst_entity_id: i64,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeType {
    Calls,
    Imports,
    Inherits,
    References,
    Uses,
    Contains,
    // PHASE 1: New semantic edge types
    UsesField,    // Type/var uses struct field (foo.bar)
    Implements,   // impl Trait for Type
    UsesType,     // Function/struct uses type (Vec<MyType>)
    ModuleChild,  // Module/file hierarchy
}

impl EdgeType {
    pub fn as_str(&self) -> &str {
        match self {
            EdgeType::Calls => "calls",
            EdgeType::Imports => "imports",
            EdgeType::Inherits => "inherits",
            EdgeType::References => "references",
            EdgeType::Uses => "uses",
            EdgeType::Contains => "contains",
            EdgeType::UsesField => "uses_field",
            EdgeType::Implements => "implements",
            EdgeType::UsesType => "uses_type",
            EdgeType::ModuleChild => "module_child",
        }
    }
}

/// Result from code search combining semantic and structural information
#[derive(Debug, Clone)]
pub struct CodeMatch {
    pub entity: CodeEntity,
    pub score: f32,
    pub match_type: MatchType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchType {
    Semantic,   // Found via vector similarity
    Structural, // Found via graph traversal
    Combined,   // Found via both methods
}
