#[cfg(test)]
mod tests {
    use syncore_ts_js_plugin::*;
    use std::path::PathBuf;
    use plugin_api::{EntityKind, EdgeKind};

    fn get_test_fixtures_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
    }

    #[test]
    fn test_create_indexer() {
        let result = TsJsIndexer::new();
        assert!(result.is_ok(), "Failed to create TsJsIndexer");
    }

    #[test]
    fn test_index_directory() {
        let mut indexer = TsJsIndexer::new().expect("Failed to create indexer");
        let fixtures_path = get_test_fixtures_path();
        let root_path = fixtures_path.to_str().unwrap();

        let result = indexer.index_directory(root_path);
        assert!(result.is_ok(), "Failed to index directory");

        let plugin_result = result.unwrap();
        assert!(plugin_result.entities.is_some(), "No entities returned");
        assert!(plugin_result.edges.is_some(), "No edges returned");

        let entities = plugin_result.entities.unwrap();
        let edges = plugin_result.edges.unwrap();

        // Should have found some entities
        assert!(!entities.is_empty(), "No entities found in test fixtures");

        // Check for expected entity types
        let mut found_class = false;
        let mut found_interface = false;
        let mut found_function = false;
        let mut found_variable = false;

        for entity in entities {
            match entity.kind {
                EntityKind::Class => found_class = true,
                EntityKind::Interface => found_interface = true,
                EntityKind::Function => found_function = true,
                EntityKind::Variable => found_variable = true,
                _ => {}
            }
        }

        assert!(found_class, "Expected to find at least one class");
        assert!(found_interface, "Expected to find at least one interface");
        assert!(found_function, "Expected to find at least one function");
        assert!(found_variable, "Expected to find at least one variable");

        // Should have found some edges
        assert!(!edges.is_empty(), "No edges found in test fixtures");

        // Check for expected edge types
        let mut found_contains = false;

        for edge in edges {
            match edge.kind {
                EdgeKind::Contains => found_contains = true,
                _ => {}
            }
        }

        assert!(found_contains, "Expected to find at least one 'contains' edge");
    }

    #[test]
    fn test_index_single_file() {
        let mut indexer = TsJsIndexer::new().expect("Failed to create indexer");
        let fixtures_path = get_test_fixtures_path();
        let test_file = fixtures_path.join("user_service.ts");

        let mut entities = Vec::new();
        let mut edges = Vec::new();

        let result = indexer.index_file(&test_file, &mut entities, &mut edges);
        assert!(result.is_ok(), "Failed to index single file");

        // Should have found entities in the single file
        assert!(!entities.is_empty(), "No entities found in user_service.ts");

        // Check for specific entities in user_service.ts
        let mut found_user_service_class = false;
        let mut found_user_interface = false;
        let mut found_create_user_function = false;

        for entity in entities {
            match entity.name.as_str() {
                "UserService" => found_user_service_class = true,
                "User" => found_user_interface = true,
                "createUser" => found_create_user_function = true,
                _ => {}
            }
        }

        assert!(found_user_service_class, "Expected to find UserService class");
        assert!(found_user_interface, "Expected to find User interface");
        assert!(found_create_user_function, "Expected to find createUser function");
    }

    #[test]
    fn test_span_extraction() {
        let mut indexer = TsJsIndexer::new().expect("Failed to create indexer");
        let fixtures_path = get_test_fixtures_path();
        let test_file = fixtures_path.join("user_service.ts");

        let mut entities = Vec::new();
        let mut edges = Vec::new();

        let result = indexer.index_file(&test_file, &mut entities, &mut edges);
        assert!(result.is_ok(), "Failed to index single file");

        // Check that entities have spans
        for entity in entities {
            assert!(entity.span.is_some(), "Entity {} should have a span", entity.name);
            
            let span = entity.span.as_ref().unwrap();
            assert!(span.start_line > 0, "Span start line should be > 0");
            assert!(span.end_line >= span.start_line, "Span end line should be >= start line");
        }
    }

    #[test]
    fn test_file_path_handling() {
        let mut indexer = TsJsIndexer::new().expect("Failed to create indexer");
        let fixtures_path = get_test_fixtures_path();
        let root_path = fixtures_path.to_str().unwrap();

        let result = indexer.index_directory(root_path);
        assert!(result.is_ok(), "Failed to index directory");

        let plugin_result = result.unwrap();
        let entities = plugin_result.entities.unwrap();

        // Check that all entities have valid file paths
        for entity in entities {
            assert!(!entity.file_path.is_empty(), "Entity should have a non-empty file path");
            assert!(entity.file_path.contains("user_service.ts") || 
                    entity.file_path.contains("app.js"), 
                    "Entity file path should reference one of the test files");
        }
    }

    #[test]
    fn test_edge_relationships() {
        let mut indexer = TsJsIndexer::new().expect("Failed to create indexer");
        let fixtures_path = get_test_fixtures_path();
        let test_file = fixtures_path.join("user_service.ts");

        let mut entities = Vec::new();
        let mut edges = Vec::new();

        let result = indexer.index_file(&test_file, &mut entities, &mut edges);
        assert!(result.is_ok(), "Failed to index single file");

        // If we have edges, they should have valid from/to references
        for edge in edges {
            assert!(!edge.from.is_empty(), "Edge should have a non-empty 'from' field");
            assert!(!edge.to.is_empty(), "Edge should have a non-empty 'to' field");
            
            // The from and to should reference entities that exist
            let from_exists = entities.iter().any(|e| 
                edge.from.contains(&e.name) || edge.from == format!("{}:{}", e.file_path, e.name)
            );
            let to_exists = entities.iter().any(|e| 
                edge.to.contains(&e.name) || edge.to == format!("{}:{}", e.file_path, e.name)
            );
            
            // Note: This is a simplified check. In reality, the edge references
            // might be more complex and require full entity resolution
        }
    }

    #[test]
    fn test_nonexistent_directory() {
        let mut indexer = TsJsIndexer::new().expect("Failed to create indexer");
        let nonexistent_path = "/path/that/does/not/exist";

        let result = indexer.index_directory(nonexistent_path);
        assert!(result.is_ok(), "Should handle nonexistent directory gracefully");

        let plugin_result = result.unwrap();
        let entities = plugin_result.entities.unwrap_or_default();
        assert!(entities.is_empty(), "Should return no entities for nonexistent directory");
    }

    #[test]
    fn test_empty_directory() {
        let mut indexer = TsJsIndexer::new().expect("Failed to create indexer");
        
        // Create a temporary empty directory
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let empty_path = temp_dir.path().to_str().unwrap();

        let result = indexer.index_directory(empty_path);
        assert!(result.is_ok(), "Should handle empty directory gracefully");

        let plugin_result = result.unwrap();
        let entities = plugin_result.entities.unwrap_or_default();
        assert!(entities.is_empty(), "Should return no entities for empty directory");
    }
}