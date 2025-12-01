//! TDD Tests for Code Relationship Store
//! Tests SQLite storage, Neo4j graph, and FAISS similarity indexing.

use syncore::portfolio::code_relationship_store::CodeRelationshipStore;
use tempfile::tempdir;

#[tokio::test]
async fn test_store_and_retrieve_imports() {
    let temp = tempdir().unwrap();
    let db_path = temp.path().join("test.db");
    let store = CodeRelationshipStore::new(&db_path).await.unwrap();

    // Store imports
    store.store_import("src/main.rs", "std::collections::HashMap").await.unwrap();
    store.store_import("src/main.rs", "anyhow::Result").await.unwrap();
    store.store_import("src/lib.rs", "serde::Serialize").await.unwrap();

    // Retrieve imports for main.rs
    let main_imports = store.get_imports("src/main.rs").await.unwrap();
    assert_eq!(main_imports.len(), 2);
    assert!(main_imports.contains(&"std::collections::HashMap".to_string()));
    assert!(main_imports.contains(&"anyhow::Result".to_string()));

    // Retrieve imports for lib.rs
    let lib_imports = store.get_imports("src/lib.rs").await.unwrap();
    assert_eq!(lib_imports.len(), 1);
    assert!(lib_imports.contains(&"serde::Serialize".to_string()));
}

#[tokio::test]
async fn test_store_and_retrieve_calls() {
    let temp = tempdir().unwrap();
    let db_path = temp.path().join("test.db");
    let store = CodeRelationshipStore::new(&db_path).await.unwrap();

    // Store function calls
    store.store_call("src/main.rs", "main", "init_database").await.unwrap();
    store.store_call("src/main.rs", "main", "start_server").await.unwrap();
    store.store_call("src/lib.rs", "process", "validate").await.unwrap();

    // Retrieve calls from main
    let main_calls = store.get_calls_from("src/main.rs", "main").await.unwrap();
    assert_eq!(main_calls.len(), 2);
    assert!(main_calls.contains(&"init_database".to_string()));
    assert!(main_calls.contains(&"start_server".to_string()));

    // Retrieve calls from process
    let process_calls = store.get_calls_from("src/lib.rs", "process").await.unwrap();
    assert_eq!(process_calls.len(), 1);
    assert!(process_calls.contains(&"validate".to_string()));
}

#[tokio::test]
async fn test_store_trait_impls() {
    let temp = tempdir().unwrap();
    let db_path = temp.path().join("test.db");
    let store = CodeRelationshipStore::new(&db_path).await.unwrap();

    // Store trait implementations
    store.store_impl("src/models.rs", "User", "Default").await.unwrap();
    store.store_impl("src/models.rs", "User", "Clone").await.unwrap();
    store.store_impl("src/models.rs", "Session", "Drop").await.unwrap();

    // Retrieve impls for User
    let user_impls = store.get_impls_for("User").await.unwrap();
    assert_eq!(user_impls.len(), 2);
    assert!(user_impls.contains(&"Default".to_string()));
    assert!(user_impls.contains(&"Clone".to_string()));

    // Retrieve impls for Session
    let session_impls = store.get_impls_for("Session").await.unwrap();
    assert_eq!(session_impls.len(), 1);
    assert!(session_impls.contains(&"Drop".to_string()));
}

#[tokio::test]
#[ignore = "Requires running Neo4j instance with authentication"]
async fn test_neo4j_import_relationship() {
    let temp = tempdir().unwrap();
    let db_path = temp.path().join("test.db");
    let store = CodeRelationshipStore::new(&db_path).await.unwrap();

    // Store import and sync to Neo4j
    store.store_import("src/main.rs", "std::io").await.unwrap();
    store.sync_to_neo4j().await.unwrap();

    // Query Neo4j for import relationship
    let result = store.query_neo4j_imports("src/main.rs").await.unwrap();
    assert!(result.contains(&"std::io".to_string()));
}

#[tokio::test]
#[ignore = "Requires running Neo4j instance with authentication"]
async fn test_neo4j_call_relationship() {
    let temp = tempdir().unwrap();
    let db_path = temp.path().join("test.db");
    let store = CodeRelationshipStore::new(&db_path).await.unwrap();

    // Store call and sync to Neo4j
    store.store_call("src/lib.rs", "handler", "validate_request").await.unwrap();
    store.sync_to_neo4j().await.unwrap();

    // Query Neo4j for call relationship
    let result = store.query_neo4j_calls("handler").await.unwrap();
    assert!(result.contains(&"validate_request".to_string()));
}

#[tokio::test]
async fn test_faiss_similarity_query() {
    let temp = tempdir().unwrap();
    let db_path = temp.path().join("test.db");
    let store = CodeRelationshipStore::new(&db_path).await.unwrap();

    // Index function bodies
    let func1_body =
        "fn validate_user(user: &User) -> bool { user.is_active && user.has_permission() }";
    let func2_body = "fn check_user_valid(u: &User) -> bool { u.active && u.permission_granted() }";
    let func3_body = "fn compute_sum(a: i32, b: i32) -> i32 { a + b }";

    store.index_function("src/auth.rs", "validate_user", func1_body).await.unwrap();
    store.index_function("src/auth2.rs", "check_user_valid", func2_body).await.unwrap();
    store.index_function("src/math.rs", "compute_sum", func3_body).await.unwrap();

    // Search for similar to validate_user
    let similar = store.find_similar_functions("validate user permissions check", 2).await.unwrap();

    // Should find validate_user and check_user_valid, not compute_sum
    assert_eq!(similar.len(), 2);
    assert!(similar.iter().any(|(file, func, _)| file == "src/auth.rs" && func == "validate_user"));
    assert!(similar
        .iter()
        .any(|(file, func, _)| file == "src/auth2.rs" && func == "check_user_valid"));
}

#[tokio::test]
async fn test_get_all_files_importing() {
    let temp = tempdir().unwrap();
    let db_path = temp.path().join("test.db");
    let store = CodeRelationshipStore::new(&db_path).await.unwrap();

    // Multiple files importing the same module
    store.store_import("src/main.rs", "anyhow::Result").await.unwrap();
    store.store_import("src/lib.rs", "anyhow::Result").await.unwrap();
    store.store_import("src/utils.rs", "anyhow::Result").await.unwrap();

    let files = store.get_files_importing("anyhow::Result").await.unwrap();
    assert_eq!(files.len(), 3);
    assert!(files.contains(&"src/main.rs".to_string()));
    assert!(files.contains(&"src/lib.rs".to_string()));
    assert!(files.contains(&"src/utils.rs".to_string()));
}

#[tokio::test]
async fn test_get_callers_of() {
    let temp = tempdir().unwrap();
    let db_path = temp.path().join("test.db");
    let store = CodeRelationshipStore::new(&db_path).await.unwrap();

    // Multiple functions calling the same function
    store.store_call("src/main.rs", "main", "validate").await.unwrap();
    store.store_call("src/lib.rs", "process", "validate").await.unwrap();
    store.store_call("src/test.rs", "test_fn", "validate").await.unwrap();

    let callers = store.get_callers_of("validate").await.unwrap();
    assert_eq!(callers.len(), 3);
    assert!(callers.iter().any(|(file, func)| file == "src/main.rs" && func == "main"));
    assert!(callers.iter().any(|(file, func)| file == "src/lib.rs" && func == "process"));
    assert!(callers.iter().any(|(file, func)| file == "src/test.rs" && func == "test_fn"));
}
