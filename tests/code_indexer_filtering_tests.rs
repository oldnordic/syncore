use syncore::code_directory_indexer::{DirectoryIndexer, DirectoryIndexRequest};
use syncore::macro_tools::path_filter;
use syncore::vector::{RealEmbeddings, VectorStore};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn create_test_indexer(db_path: &Path) -> DirectoryIndexer {
    let embeddings = Box::new(RealEmbeddings::new(384).unwrap());
    let vector_store = std::sync::Arc::new(std::sync::Mutex::new(VectorStore::new(embeddings)));
    DirectoryIndexer::new(db_path.to_str().unwrap(), vector_store).unwrap()
}

#[test]
fn test_path_filtering_excludes_build_artifacts() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");

    // Create directory structure with both source and build artifacts
    let src_dir = temp_dir.path().join("src");
    let target_dir = temp_dir.path().join("target");
    let build_dir = temp_dir.path().join("build");

    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&target_dir.join("debug/build"))?;
    fs::create_dir_all(&build_dir)?;

    // Create source files (should be indexed)
    fs::write(src_dir.join("memory.rs"), r#"
pub struct Memory {
    data: Vec<u8>,
}
impl Memory {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }
}
"#)?;

    fs::write(src_dir.join("tasks.rs"), r#"
pub struct Tasks {
    items: Vec<String>,
}
"#)?;

    // Create build artifact files (should be excluded)
    fs::write(target_dir.join("debug/build/typenum/tests.rs"), "generated code")?;
    fs::write(target_dir.join("generated.rs"), "more generated code")?;
    fs::write(build_dir.join("bindgen.rs"), "bindgen generated code")?;
    let _ = fs::write(build_dir.join("output.o"), "binary object");

    let mut indexer = create_test_indexer(&db_path);

    // Test current behavior - should FAIL because no filtering is applied
    let request = DirectoryIndexRequest {
        directory: temp_dir.path().to_str().unwrap().to_string(),
        pattern: "**/*.rs".to_string(),
    };

    let response = indexer.index_directory(&request)?;

    // Document behavior
    println!("Files indexed: {}", response.files_indexed);
    println!("Total entities: {}", response.total_entities);
    println!("Success: {}", response.success);
    if let Some(ref error) = response.error {
        println!("Error: {}", error);
    }

    // Should succeed and index some files
    assert!(response.success, "Indexing should succeed");
    assert!(response.files_indexed > 0, "Should index some files");

    // After fix: should only include source files, not build artifacts
    let source_files_count = 2; // memory.rs + tasks.rs

    // With path filtering implemented, build artifacts should be excluded
    assert_eq!(response.files_indexed, source_files_count,
               "Build artifacts should be filtered out, only source files counted");

    // Verify path filtering works correctly in isolation
    assert!(path_filter::should_index_path("src/memory.rs"));
    assert!(path_filter::should_index_path("src/tasks.rs"));
    assert!(!path_filter::should_index_path("target/debug/build/typenum/tests.rs"));
    assert!(!path_filter::should_index_path("build/bindgen.rs"));

    Ok(())
}