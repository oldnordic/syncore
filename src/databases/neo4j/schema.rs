//! Canonical Neo4j Schema - Single Source of Truth
//!
//! This is THE ONLY place where Neo4j labels, properties, and relationships are defined.
//! No other module should define schema. No improvisation. No guessing.

/// Node Labels - Exhaustive list of allowed labels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeLabel {
    File,
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    Import,
    Constant,
    TypeAlias,
}

impl NodeLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Function => "Function",
            Self::Struct => "Struct",
            Self::Enum => "Enum",
            Self::Trait => "Trait",
            Self::Impl => "Impl",
            Self::Module => "Module",
            Self::Import => "Import",
            Self::Constant => "Constant",
            Self::TypeAlias => "TypeAlias",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "File" | "file" => Some(Self::File),
            "Function" | "function" => Some(Self::Function),
            "Struct" | "struct" => Some(Self::Struct),
            "Enum" | "enum" => Some(Self::Enum),
            "Trait" | "trait" => Some(Self::Trait),
            "Impl" | "impl" => Some(Self::Impl),
            "Module" | "module" => Some(Self::Module),
            "Import" | "import" | "use" => Some(Self::Import),
            "Constant" | "constant" => Some(Self::Constant),
            "TypeAlias" | "type_alias" => Some(Self::TypeAlias),
            _ => None,
        }
    }
}

/// Relationship Types - Exhaustive list of allowed relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationType {
    Declares,      // File DECLARES Function
    Calls,         // Function CALLS Function
    HasMember,     // Struct HAS_MEMBER Function (method)
    Implements,    // Struct IMPLEMENTS Trait
    Imports,       // File IMPORTS Module
    Uses,          // Function USES Struct/Enum
    Owns,          // Module OWNS File
    References,    // Generic reference
    Inherits,      // Class INHERITS Class (inheritance)
    Contains,      // Module CONTAINS Entity (containment)
    UsesField,     // Entity USES_FIELD Field (field access)
    UsesType,      // Entity USES_TYPE Type (type usage)
    ModuleChild,   // Module MODULE_CHILD Module (module hierarchy)
    DependsOn,     // File DEPENDS_ON File (file-level dependencies)
}

impl RelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Declares => "DECLARES",
            Self::Calls => "CALLS",
            Self::HasMember => "HAS_MEMBER",
            Self::Implements => "IMPLEMENTS",
            Self::Imports => "IMPORTS",
            Self::Uses => "USES",
            Self::Owns => "OWNS",
            Self::References => "REFERENCES",
            Self::Inherits => "INHERITS",
            Self::Contains => "CONTAINS",
            Self::UsesField => "USES_FIELD",
            Self::UsesType => "USES_TYPE",
            Self::ModuleChild => "MODULE_CHILD",
            Self::DependsOn => "DEPENDS_ON",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "DECLARES" => Some(Self::Declares),
            "CALLS" => Some(Self::Calls),
            "HAS_MEMBER" => Some(Self::HasMember),
            "IMPLEMENTS" => Some(Self::Implements),
            "IMPORTS" => Some(Self::Imports),
            "USES" => Some(Self::Uses),
            "OWNS" => Some(Self::Owns),
            "REFERENCES" => Some(Self::References),
            "INHERITS" => Some(Self::Inherits),
            "CONTAINS" => Some(Self::Contains),
            "USES_FIELD" => Some(Self::UsesField),
            "USES_TYPE" => Some(Self::UsesType),
            "MODULE_CHILD" => Some(Self::ModuleChild),
            "DEPENDS_ON" => Some(Self::DependsOn),
            _ => None,
        }
    }
}

/// Canonical Node Properties - Exhaustive list
///
/// Every Neo4j node MUST use only these properties.
/// Adding a new property requires updating this struct.
#[derive(Debug, Clone)]
pub struct NodeProperties {
    // Identity (required for all nodes)
    pub id: i64,                      // SQLite entity ID
    pub name: String,                 // Entity name

    // Location (required for code entities)
    pub path: Option<String>,         // File path
    pub start_line: Option<i64>,      // Start line number
    pub end_line: Option<i64>,        // End line number

    // Content
    pub signature: Option<String>,    // Function/method signature
    pub body_snippet: Option<String>, // Body content (first 1000 chars)
    pub docstring: Option<String>,    // Documentation

    // Metadata
    pub hash: Option<String>,         // SHA256 of content
    pub language: Option<String>,     // rust, javascript, python, etc.

    // Timestamps (file-level)
    pub file_sha256: Option<String>,  // Whole file hash
    pub mtime: Option<i64>,           // File modification timestamp

    // Git/History metadata (from code_graph)
    pub created_at: Option<String>,       // First seen timestamp
    pub last_modified_at: Option<String>, // Last modified timestamp
    pub change_count: Option<i64>,        // Number of changes
    pub author_count: Option<i64>,        // Number of authors
}

impl NodeProperties {
    /// Create minimal properties (just ID and name)
    pub fn minimal(id: i64, name: String) -> Self {
        Self {
            id,
            name,
            path: None,
            start_line: None,
            end_line: None,
            signature: None,
            body_snippet: None,
            docstring: None,
            hash: None,
            language: None,
            file_sha256: None,
            mtime: None,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        }
    }

    /// Create full properties for code entity
    pub fn full(
        id: i64,
        name: String,
        path: String,
        start_line: i64,
        end_line: i64,
        language: String,
    ) -> Self {
        Self {
            id,
            name,
            path: Some(path),
            start_line: Some(start_line),
            end_line: Some(end_line),
            signature: None,
            body_snippet: None,
            docstring: None,
            hash: None,
            language: Some(language),
            file_sha256: None,
            mtime: None,
            created_at: None,
            last_modified_at: None,
            change_count: None,
            author_count: None,
        }
    }
}

/// Get the project namespace from Neo4jClient
///
/// Uses the namespace configured in Neo4jClient (from GRAPH_NAMESPACE env var).
/// This ensures consistency with existing code that uses client.namespace().
///
/// Default: "syncore_default" (if GRAPH_NAMESPACE not set)
pub fn project_namespace(client: &crate::graph::Neo4jClient) -> &str {
    client.namespace()
}

/// Project label applied to all entities
///
/// This is the second label in the double-label pattern: `:Function:SynCore`
pub const PROJECT_LABEL: &str = "SynCore";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_label_roundtrip() {
        assert_eq!(NodeLabel::from_str("Function"), Some(NodeLabel::Function));
        assert_eq!(NodeLabel::Function.as_str(), "Function");

        assert_eq!(NodeLabel::from_str("function"), Some(NodeLabel::Function));
        assert_eq!(NodeLabel::from_str("invalid"), None);
    }

    #[test]
    fn test_relation_type_roundtrip() {
        assert_eq!(RelationType::from_str("CALLS"), Some(RelationType::Calls));
        assert_eq!(RelationType::Calls.as_str(), "CALLS");

        assert_eq!(RelationType::from_str("calls"), Some(RelationType::Calls));
        assert_eq!(RelationType::from_str("invalid"), None);
    }

    #[test]
    fn test_project_label_is_fixed() {
        assert_eq!(PROJECT_LABEL, "SynCore");
    }
}
