//! PHASE 3 TDD Tests: Temporal Metadata Extraction
//!
//! These tests verify that temporal metadata (created_at, last_modified_at,
//! change_count, author_count) is correctly extracted from filesystem and git,
//! and persisted to both SQLite and Neo4j.
//!
//! All tests use REAL filesystem operations and REAL git2 operations (no mocks).

use anyhow::Result;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use syncore::code_graph::temporal_extractor::{extract_temporal_metadata, TemporalMetadata};
use syncore::code_graph::CodeGraph;
use syncore::graph::Neo4jClient;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

/// Test 1: Extract filesystem metadata from real files
///
/// Verifies that created_at and last_modified_at are correctly extracted
/// from filesystem metadata using std::fs operations.
#[test]
fn test_extract_filesystem_metadata() -> Result<()> {
    // Create temporary file with known timestamp
    let temp_file = "/tmp/test_phase3_fs_metadata.rs";
    std::fs::write(temp_file, "fn test() {}")?;

    // Extract metadata
    let metadata = extract_temporal_metadata(temp_file)?;

    // Verify timestamps are recent (within last 60 seconds)
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;

    assert!(metadata.created_at > 0, "created_at should be positive Unix timestamp");
    assert!(metadata.last_modified_at > 0, "last_modified_at should be positive Unix timestamp");

    assert!(
        now - metadata.created_at < 60,
        "created_at should be within last 60 seconds, got diff: {}",
        now - metadata.created_at
    );
    assert!(
        now - metadata.last_modified_at < 60,
        "last_modified_at should be within last 60 seconds, got diff: {}",
        now - metadata.last_modified_at
    );

    // Without git, should default to 1/1
    assert_eq!(metadata.change_count, 1, "change_count should default to 1 without git");
    assert_eq!(metadata.author_count, 1, "author_count should default to 1 without git");

    // Cleanup
    std::fs::remove_file(temp_file)?;

    Ok(())
}

/// Test 2: Extract git change count and author count
///
/// Creates a temporary git repository, makes multiple commits with different
/// authors, and verifies that change_count and author_count are correct.
#[test]
fn test_extract_git_change_count() -> Result<()> {
    use git2::{Repository, Signature};

    // Create temporary directory for git repo
    let temp_dir = "/tmp/test_phase3_git_repo";
    let _ = std::fs::remove_dir_all(temp_dir); // Clean up from previous runs
    std::fs::create_dir_all(temp_dir)?;

    // Initialize git repository
    let repo = Repository::init(temp_dir)?;

    // Configure git user
    let mut config = repo.config()?;
    config.set_str("user.name", "Test User 1")?;
    config.set_str("user.email", "user1@test.com")?;

    // Create test file
    let test_file_path = format!("{}/test.rs", temp_dir);
    std::fs::write(&test_file_path, "fn version1() {}")?;

    // Commit 1 by User 1
    let mut index = repo.index()?;
    index.add_path(Path::new("test.rs"))?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig1 = Signature::now("User One", "user1@test.com")?;
    repo.commit(Some("HEAD"), &sig1, &sig1, "First commit", &tree, &[])?;

    // Modify file
    std::fs::write(&test_file_path, "fn version2() {}")?;

    // Commit 2 by User 1
    let mut index = repo.index()?;
    index.add_path(Path::new("test.rs"))?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let parent = repo.head()?.peel_to_commit()?;
    repo.commit(Some("HEAD"), &sig1, &sig1, "Second commit", &tree, &[&parent])?;

    // Modify file again
    std::fs::write(&test_file_path, "fn version3() {}")?;

    // Commit 3 by User 2 (different author)
    let mut index = repo.index()?;
    index.add_path(Path::new("test.rs"))?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let parent = repo.head()?.peel_to_commit()?;
    let sig2 = Signature::now("User Two", "user2@test.com")?;
    repo.commit(Some("HEAD"), &sig2, &sig2, "Third commit by different author", &tree, &[&parent])?;

    // Extract metadata
    let metadata = extract_temporal_metadata(&test_file_path)?;

    // Verify git metadata
    assert_eq!(metadata.change_count, 3, "File should have 3 commits (change_count)");
    assert_eq!(metadata.author_count, 2, "File should have 2 unique authors (author_count)");

    // Cleanup
    std::fs::remove_dir_all(temp_dir)?;

    Ok(())
}

/// Test 3: Indexer populates temporal metadata in SQLite
///
/// Runs real code indexing over a temporary git repository and verifies
/// that CodeEntities written to SQLite have correct temporal fields.
#[test]
fn test_indexer_populates_temporal_metadata() -> Result<()> {
    use git2::{Repository, Signature};

    // Create temporary directory and git repo
    let temp_dir = "/tmp/test_phase3_indexer";
    let _ = std::fs::remove_dir_all(temp_dir);
    std::fs::create_dir_all(temp_dir)?;

    let repo = Repository::init(temp_dir)?;
    let mut config = repo.config()?;
    config.set_str("user.name", "Indexer Test")?;
    config.set_str("user.email", "indexer@test.com")?;

    // Create Rust source file
    let source_file = format!("{}/module.rs", temp_dir);
    std::fs::write(
        &source_file,
        r#"
pub fn compute() -> i32 {
    42
}

pub struct Config {
    value: i32,
}
"#,
    )?;

    // Commit the file
    let mut index = repo.index()?;
    index.add_path(Path::new("module.rs"))?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = Signature::now("Indexer Test", "indexer@test.com")?;
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

    // Create CodeGraph with temporary database
    let db_path = "/tmp/test_phase3_indexer.db";
    let _ = std::fs::remove_file(db_path);

    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
    let mut code_graph = CodeGraph::new(db_path, vector_store)?;

    // Index the file (this should populate temporal metadata)
    code_graph.index_file(Path::new(&source_file))?;

    // Query SQLite to verify temporal fields
    let db_conn = code_graph.db_conn();
    let conn = db_conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT name, created_at, last_modified_at, change_count, author_count
         FROM code_entities
         WHERE file_path = ?",
    )?;

    let entities: Vec<(String, Option<i64>, Option<i64>, Option<i32>, Option<i32>)> = stmt
        .query_map([&source_file], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    assert!(!entities.is_empty(), "Should have indexed at least one entity");

    // Debug: print all entities
    println!("Found {} entities:", entities.len());
    for (name, created, modified, changes, authors) in &entities {
        println!(
            "  Entity '{}': created={:?}, modified={:?}, changes={:?}, authors={:?}",
            name, created, modified, changes, authors
        );
    }

    // Check that at least one entity has temporal metadata
    let entity_with_metadata = entities.iter().find(|(_, created, modified, changes, authors)| {
        created.is_some() && modified.is_some() && changes.is_some() && authors.is_some()
    });

    assert!(
        entity_with_metadata.is_some(),
        "At least one entity should have temporal metadata populated"
    );

    if let Some((name, created, modified, changes, authors)) = entity_with_metadata {
        println!("Entity '{}' has temporal metadata:", name);
        println!("  created_at: {:?}", created);
        println!("  last_modified_at: {:?}", modified);
        println!("  change_count: {:?}", changes);
        println!("  author_count: {:?}", authors);

        let created_val = created.unwrap();
        let modified_val = modified.unwrap();
        let changes_val = changes.unwrap();
        let authors_val = authors.unwrap();

        assert!(created_val > 0, "created_at should be positive");
        assert!(modified_val > 0, "last_modified_at should be positive");
        assert_eq!(changes_val, 1, "Should have 1 commit (initial)");
        assert_eq!(authors_val, 1, "Should have 1 author");
    }

    // Cleanup
    std::fs::remove_dir_all(temp_dir)?;
    std::fs::remove_file(db_path)?;

    Ok(())
}

/// Test 4: Neo4j temporal properties persist correctly
///
/// With Neo4j enabled, verifies that temporal fields appear as node properties
/// when entities are synced to the graph database.
#[tokio::test]
async fn test_neo4j_temporal_properties_persist() -> Result<()> {
    use git2::{Repository, Signature};
    use syncore::code_graph::neo4j_writer::create_code_entity_node;
    use syncore::code_graph::CodeEntity;
    use syncore::code_graph::EntityType;

    // Create temporary directory and git repo
    let temp_dir = "/tmp/test_phase3_neo4j";
    let _ = std::fs::remove_dir_all(temp_dir);
    std::fs::create_dir_all(temp_dir)?;

    let repo = Repository::init(temp_dir)?;
    let mut config = repo.config()?;
    config.set_str("user.name", "Neo4j Test")?;
    config.set_str("user.email", "neo4j@test.com")?;

    // Create source file and commit
    let source_file = format!("{}/service.rs", temp_dir);
    std::fs::write(
        &source_file,
        r#"
pub fn process() -> String {
    "result".to_string()
}
"#,
    )?;

    let mut index = repo.index()?;
    index.add_path(Path::new("service.rs"))?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = Signature::now("Neo4j Test", "neo4j@test.com")?;
    repo.commit(Some("HEAD"), &sig, &sig, "Add service", &tree, &[])?;

    // Extract temporal metadata
    let temporal = extract_temporal_metadata(&source_file)?;

    // Create entity with temporal metadata
    let mut entity = CodeEntity::new(
        source_file.clone(),
        EntityType::Function,
        "process".to_string(),
        Some("pub fn process() -> String".to_string()),
        2,
        4,
        None,
        "rust".to_string(),
    );

    // Set temporal fields
    entity.created_at = Some(temporal.created_at);
    entity.last_modified_at = Some(temporal.last_modified_at);
    entity.change_count = Some(temporal.change_count);
    entity.author_count = Some(temporal.author_count);

    // Connect to Neo4j
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let pass = std::env::var("NEO4J_PASS").unwrap_or_else(|_| "testpassword123".to_string());

    let neo4j = Neo4jClient::connect(&uri, &user, &pass).await?;

    // Clean test namespace
    neo4j
        .execute_query(
            "MATCH (n {namespace: $ns}) DETACH DELETE n",
            vec![("ns", serde_json::json!(neo4j.namespace()))],
        )
        .await?;

    // Create node with temporal properties
    let entity_id = 999_i64; // Test ID
    create_code_entity_node(&neo4j, entity_id, &entity).await?;

    // Query Neo4j to verify temporal properties
    let result = neo4j
        .execute_query(
            "MATCH (n {id: $id, namespace: $ns})
             RETURN n.created_at as created, n.last_modified_at as modified,
                    n.change_count as changes, n.author_count as authors",
            vec![
                ("id", serde_json::json!(entity_id)),
                ("ns", serde_json::json!(neo4j.namespace())),
            ],
        )
        .await?;

    assert!(!result.is_empty(), "Should find the created node");

    let record = &result[0];
    let created_neo = record.get("created").and_then(|v| v.as_i64());
    let modified_neo = record.get("modified").and_then(|v| v.as_i64());
    let changes_neo = record.get("changes").and_then(|v| v.as_i64());
    let authors_neo = record.get("authors").and_then(|v| v.as_i64());

    assert!(created_neo.is_some(), "created_at property should exist in Neo4j");
    assert!(modified_neo.is_some(), "last_modified_at property should exist in Neo4j");
    assert!(changes_neo.is_some(), "change_count property should exist in Neo4j");
    assert!(authors_neo.is_some(), "author_count property should exist in Neo4j");

    assert_eq!(created_neo.unwrap(), temporal.created_at, "created_at should match");
    assert_eq!(modified_neo.unwrap(), temporal.last_modified_at, "last_modified_at should match");
    assert_eq!(changes_neo.unwrap() as i32, temporal.change_count, "change_count should match");
    assert_eq!(authors_neo.unwrap() as i32, temporal.author_count, "author_count should match");

    // Cleanup
    std::fs::remove_dir_all(temp_dir)?;

    Ok(())
}
