//! Language Parser Trait for Multilanguage Code Analysis
//!
//! Defines the unified interface for language-specific parsers
//! that extract CodeEntity and CodeEdge structures from source code.

use anyhow::Result;
use std::path::Path;

use super::types::{CodeEdge, CodeEntity};

/// Trait for language-specific code parsers
///
/// Implementations must extract entities and relationships
/// from source code using the exact CodeEntity and CodeEdge structures.
pub trait LanguageParser {
    /// Check if this parser supports the given file
    ///
    /// # Arguments
    /// * `file_path` - Path to the source file
    ///
    /// # Returns
    /// true if the parser can handle this file type
    fn supports(&self, file_path: &Path) -> bool;

    /// Extract code entities from source file
    ///
    /// # Arguments
    /// * `file_path` - Path to the source file
    ///
    /// # Returns
    /// Vector of CodeEntity structs extracted from the file
    fn parse_entities(&self, file_path: &Path) -> Result<Vec<CodeEntity>>;

    /// Extract code relationships (edges) from source file
    ///
    /// # Arguments
    /// * `file_path` - Path to the source file
    ///
    /// # Returns
    /// Vector of CodeEdge structs representing relationships between entities
    fn parse_edges(&self, file_path: &Path) -> Result<Vec<CodeEdge>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Mock parser for testing trait interface
    struct MockParser;

    impl LanguageParser for MockParser {
        fn supports(&self, file_path: &Path) -> bool {
            file_path.extension().map_or(false, |ext| ext == "mock")
        }

        fn parse_entities(&self, _file_path: &Path) -> Result<Vec<CodeEntity>> {
            Ok(vec![])
        }

        fn parse_edges(&self, _file_path: &Path) -> Result<Vec<CodeEdge>> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_trait_interface() {
        let parser = MockParser;

        // Test supports method
        assert!(parser.supports(&PathBuf::from("test.mock")));
        assert!(!parser.supports(&PathBuf::from("test.rs")));

        // Test parse methods return Result types
        let entities = parser.parse_entities(&PathBuf::from("test.mock")).unwrap();
        assert_eq!(entities.len(), 0);

        let edges = parser.parse_edges(&PathBuf::from("test.mock")).unwrap();
        assert_eq!(edges.len(), 0);
    }
}
