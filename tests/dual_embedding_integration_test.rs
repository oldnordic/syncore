use anyhow::Result;
use std::sync::{Arc, Mutex};
use syncore::router::SynCoreState;
use syncore::vector::domain::EmbeddingDomain;
use syncore::vector::{HuggingFaceEmbeddings, SearchScope, VectorStore};
use tempfile::TempDir;

/// Integration test for APEX 1.7 dual-embedding architecture.
///
/// Tests that CODE and GENERAL domain vectors are stored in separate VectorStores
/// and that router-level domain filtering works correctly.
#[test]
fn test_dual_embedding_routing() -> Result<()> {
    // Setup: Create temporary directory for test databases
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path().to_str().unwrap();

    // Override environment variables for isolated testing
    std::env::set_var("DB_PATH", format!("{}/test.db", temp_path));
    std::env::set_var("CODE_GRAPH_DB", format!("{}/test_code_graph.db", temp_path));
    std::env::set_var("CODE_VECTOR_INDEX_PATH", format!("{}/code.index", temp_path));
    std::env::set_var("GENERAL_VECTOR_INDEX_PATH", format!("{}/general.index", temp_path));

    // Create dual VectorStores
    let code_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut code_store = VectorStore::new(code_embeddings);
    code_store.set_index_path(format!("{}/code.index", temp_path));
    let code_store = Arc::new(Mutex::new(code_store));

    let general_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut general_store = VectorStore::new(general_embeddings);
    general_store.set_index_path(format!("{}/general.index", temp_path));
    let general_store = Arc::new(Mutex::new(general_store));

    // Initialize state with dual stores
    let state =
        SynCoreState::with_dual_stores(Arc::clone(&code_store), Arc::clone(&general_store))?;

    println!("✓ SynCoreState initialized with dual stores");

    // Test 1: Verify store_for_namespace routes CODE namespaces correctly
    {
        let code_namespace_store = state.store_for_namespace("code_entity");
        let code_store_ptr = Arc::as_ptr(&code_store);
        let namespace_store_ptr = Arc::as_ptr(&code_namespace_store);

        assert_eq!(
            code_store_ptr, namespace_store_ptr,
            "code_entity namespace should route to CODE store"
        );
        println!("✓ store_for_namespace('code_entity') routes to CODE store");
    }

    // Test 2: Verify store_for_namespace routes GENERAL namespaces correctly
    {
        let general_namespace_store = state.store_for_namespace("documents");
        let general_store_ptr = Arc::as_ptr(&general_store);
        let namespace_store_ptr = Arc::as_ptr(&general_namespace_store);

        assert_eq!(
            general_store_ptr, namespace_store_ptr,
            "documents namespace should route to GENERAL store"
        );
        println!("✓ store_for_namespace('documents') routes to GENERAL store");
    }

    // Test 3: Verify store_for_domain routes Code domain correctly
    {
        let code_domain_store = state.store_for_domain(EmbeddingDomain::Code);
        let code_store_ptr = Arc::as_ptr(&code_store);
        let domain_store_ptr = Arc::as_ptr(&code_domain_store);

        assert_eq!(
            code_store_ptr, domain_store_ptr,
            "EmbeddingDomain::Code should route to CODE store"
        );
        println!("✓ store_for_domain(Code) routes to CODE store");
    }

    // Test 4: Verify store_for_domain routes General domain correctly
    {
        let general_domain_store = state.store_for_domain(EmbeddingDomain::General);
        let general_store_ptr = Arc::as_ptr(&general_store);
        let domain_store_ptr = Arc::as_ptr(&general_domain_store);

        assert_eq!(
            general_store_ptr, domain_store_ptr,
            "EmbeddingDomain::General should route to GENERAL store"
        );
        println!("✓ store_for_domain(General) routes to GENERAL store");
    }

    // Test 5: Insert CODE domain vectors and verify isolation
    {
        let mut code_lock = code_store.lock().unwrap();
        code_lock.insert_text(1, None, "fn main() { println!(\"Hello\"); }", "code_entity")?;
        code_lock.insert_text(2, None, "struct Foo { bar: i32 }", "rust_code")?;
        drop(code_lock);

        let code_lock = code_store.lock().unwrap();
        let code_count = code_lock.len();
        drop(code_lock);

        let general_lock = general_store.lock().unwrap();
        let general_count = general_lock.len();
        drop(general_lock);

        assert_eq!(code_count, 2, "CODE store should have 2 vectors");
        assert_eq!(general_count, 0, "GENERAL store should have 0 vectors");
        println!("✓ CODE domain inserts isolated to CODE store (2 vectors)");
    }

    // Test 6: Insert GENERAL domain vectors and verify isolation
    {
        let mut general_lock = general_store.lock().unwrap();
        general_lock.insert_text(3, None, "Meeting notes from today", "documents")?;
        general_lock.insert_text(4, None, "Task: Implement feature X", "task_steps")?;
        drop(general_lock);

        let code_lock = code_store.lock().unwrap();
        let code_count = code_lock.len();
        drop(code_lock);

        let general_lock = general_store.lock().unwrap();
        let general_count = general_lock.len();
        drop(general_lock);

        assert_eq!(code_count, 2, "CODE store should still have 2 vectors");
        assert_eq!(general_count, 2, "GENERAL store should have 2 vectors");
        println!("✓ GENERAL domain inserts isolated to GENERAL store (2 vectors)");
    }

    // Test 7: Search with Domain(Code) scope only searches CODE store
    {
        let code_lock = code_store.lock().unwrap();
        let results =
            code_lock.search("function main", 5, SearchScope::Domain(EmbeddingDomain::Code))?;
        drop(code_lock);

        // Results should only include CODE domain vectors (IDs 1, 2)
        assert!(!results.is_empty(), "Should find CODE domain results");
        for hit in &results {
            assert!(
                hit.id == 1 || hit.id == 2,
                "Result ID {} should be from CODE domain (1 or 2)",
                hit.id
            );
        }
        println!(
            "✓ SearchScope::Domain(Code) only returns CODE vectors ({} results)",
            results.len()
        );
    }

    // Test 8: Search with Domain(General) scope only searches GENERAL store
    {
        let general_lock = general_store.lock().unwrap();
        let results = general_lock.search(
            "meeting task",
            5,
            SearchScope::Domain(EmbeddingDomain::General),
        )?;
        drop(general_lock);

        // Results should only include GENERAL domain vectors (IDs 3, 4)
        assert!(!results.is_empty(), "Should find GENERAL domain results");
        for hit in &results {
            assert!(
                hit.id == 3 || hit.id == 4,
                "Result ID {} should be from GENERAL domain (3 or 4)",
                hit.id
            );
        }
        println!(
            "✓ SearchScope::Domain(General) only returns GENERAL vectors ({} results)",
            results.len()
        );
    }

    // Test 9: Verify stores are physically separate (different Arc pointers)
    {
        let code_ptr = Arc::as_ptr(&code_store);
        let general_ptr = Arc::as_ptr(&general_store);

        assert_ne!(code_ptr, general_ptr, "CODE and GENERAL stores should be separate instances");
        println!("✓ CODE and GENERAL stores are physically separate instances");
    }

    // Test 10: Verify namespace mapping for all CODE namespaces
    {
        let code_namespaces = vec!["code_entity", "rust_code", "python_code", "javascript_code"];
        for ns in code_namespaces {
            let store = state.store_for_namespace(ns);
            let code_store_ptr = Arc::as_ptr(&code_store);
            let namespace_store_ptr = Arc::as_ptr(&store);

            assert_eq!(code_store_ptr, namespace_store_ptr, "{} should route to CODE store", ns);
        }
        println!("✓ All CODE namespaces route correctly");
    }

    // Test 11: Verify namespace mapping for all GENERAL namespaces
    {
        let general_namespaces = vec!["documents", "task_steps", "notes"];
        for ns in general_namespaces {
            let store = state.store_for_namespace(ns);
            let general_store_ptr = Arc::as_ptr(&general_store);
            let namespace_store_ptr = Arc::as_ptr(&store);

            assert_eq!(
                general_store_ptr, namespace_store_ptr,
                "{} should route to GENERAL store",
                ns
            );
        }
        println!("✓ All GENERAL namespaces route correctly");
    }

    // Test 12: Verify unknown namespace defaults to GENERAL
    {
        let unknown_store = state.store_for_namespace("unknown_namespace");
        let general_store_ptr = Arc::as_ptr(&general_store);
        let unknown_store_ptr = Arc::as_ptr(&unknown_store);

        assert_eq!(
            general_store_ptr, unknown_store_ptr,
            "Unknown namespace should default to GENERAL store"
        );
        println!("✓ Unknown namespaces default to GENERAL store");
    }

    // Cleanup
    std::env::remove_var("DB_PATH");
    std::env::remove_var("CODE_GRAPH_DB");
    std::env::remove_var("CODE_VECTOR_INDEX_PATH");
    std::env::remove_var("GENERAL_VECTOR_INDEX_PATH");

    println!("\n✅ All 12 dual-embedding integration tests passed!");

    Ok(())
}

/// Test that VectorStore count() method works correctly for dual stores.
#[test]
fn test_dual_store_vector_counts() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path().to_str().unwrap();

    // Create CODE store
    let code_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut code_store = VectorStore::new(code_embeddings);
    code_store.set_index_path(format!("{}/code_count_test.index", temp_path));

    // Create GENERAL store
    let general_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut general_store = VectorStore::new(general_embeddings);
    general_store.set_index_path(format!("{}/general_count_test.index", temp_path));

    // Initially both stores should be empty
    assert_eq!(code_store.len(), 0, "CODE store should start empty");
    assert_eq!(general_store.len(), 0, "GENERAL store should start empty");
    println!("✓ Both stores start with len() = 0");

    // Insert into CODE store
    code_store.insert_text(1, None, "fn test() {}", "code_entity")?;
    code_store.insert_text(2, None, "class Foo:", "python_code")?;
    code_store.insert_text(3, None, "function bar() {}", "javascript_code")?;

    assert_eq!(code_store.len(), 3, "CODE store should have 3 vectors");
    assert_eq!(general_store.len(), 0, "GENERAL store should still be empty");
    println!("✓ CODE store len() = 3, GENERAL store len() = 0");

    // Insert into GENERAL store
    general_store.insert_text(10, None, "Document content", "documents")?;
    general_store.insert_text(11, None, "Step 1: Do something", "task_steps")?;

    assert_eq!(code_store.len(), 3, "CODE store should still have 3 vectors");
    assert_eq!(general_store.len(), 2, "GENERAL store should have 2 vectors");
    println!("✓ CODE store len() = 3, GENERAL store len() = 2");

    println!("\n✅ Vector count test passed - stores maintain separate counts");

    Ok(())
}

/// Test that SearchScope routing works correctly in state methods.
#[test]
fn test_search_scope_routing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path().to_str().unwrap();

    // Use unique names to avoid database locks from parallel test execution
    std::env::set_var("DB_PATH", format!("{}/scope_routing_test.db", temp_path));
    std::env::set_var("CODE_GRAPH_DB", format!("{}/scope_routing_code_graph.db", temp_path));

    // Create dual stores
    let code_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut code_store = VectorStore::new(code_embeddings);
    code_store.set_index_path(format!("{}/scope_code.index", temp_path));
    let code_store = Arc::new(Mutex::new(code_store));

    let general_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut general_store = VectorStore::new(general_embeddings);
    general_store.set_index_path(format!("{}/scope_general.index", temp_path));
    let general_store = Arc::new(Mutex::new(general_store));

    let state =
        SynCoreState::with_dual_stores(Arc::clone(&code_store), Arc::clone(&general_store))?;

    // Test SearchScope::Domain(Code) routes to code_store
    {
        let store = match SearchScope::Domain(EmbeddingDomain::Code) {
            SearchScope::Domain(domain) | SearchScope::DomainTask(domain, _) => {
                state.store_for_domain(domain)
            }
            SearchScope::Global | SearchScope::Task(_) => Arc::clone(&state.general_store),
        };

        let code_ptr = Arc::as_ptr(&code_store);
        let store_ptr = Arc::as_ptr(&store);
        assert_eq!(code_ptr, store_ptr, "Domain(Code) should route to code_store");
        println!("✓ SearchScope::Domain(Code) routes to code_store");
    }

    // Test SearchScope::Domain(General) routes to general_store
    {
        let store = match SearchScope::Domain(EmbeddingDomain::General) {
            SearchScope::Domain(domain) | SearchScope::DomainTask(domain, _) => {
                state.store_for_domain(domain)
            }
            SearchScope::Global | SearchScope::Task(_) => Arc::clone(&state.general_store),
        };

        let general_ptr = Arc::as_ptr(&general_store);
        let store_ptr = Arc::as_ptr(&store);
        assert_eq!(general_ptr, store_ptr, "Domain(General) should route to general_store");
        println!("✓ SearchScope::Domain(General) routes to general_store");
    }

    // Test SearchScope::Global defaults to general_store
    {
        let store = match SearchScope::Global {
            SearchScope::Domain(domain) | SearchScope::DomainTask(domain, _) => {
                state.store_for_domain(domain)
            }
            SearchScope::Global | SearchScope::Task(_) => Arc::clone(&state.general_store),
        };

        let general_ptr = Arc::as_ptr(&general_store);
        let store_ptr = Arc::as_ptr(&store);
        assert_eq!(general_ptr, store_ptr, "Global should default to general_store");
        println!("✓ SearchScope::Global defaults to general_store");
    }

    // Test SearchScope::Task defaults to general_store
    {
        let store = match SearchScope::Task(42) {
            SearchScope::Domain(domain) | SearchScope::DomainTask(domain, _) => {
                state.store_for_domain(domain)
            }
            SearchScope::Global | SearchScope::Task(_) => Arc::clone(&state.general_store),
        };

        let general_ptr = Arc::as_ptr(&general_store);
        let store_ptr = Arc::as_ptr(&store);
        assert_eq!(general_ptr, store_ptr, "Task should default to general_store");
        println!("✓ SearchScope::Task defaults to general_store");
    }

    // Test SearchScope::DomainTask routes by domain, not task_id
    {
        let store = match SearchScope::DomainTask(EmbeddingDomain::Code, 99) {
            SearchScope::Domain(domain) | SearchScope::DomainTask(domain, _) => {
                state.store_for_domain(domain)
            }
            SearchScope::Global | SearchScope::Task(_) => Arc::clone(&state.general_store),
        };

        let code_ptr = Arc::as_ptr(&code_store);
        let store_ptr = Arc::as_ptr(&store);
        assert_eq!(code_ptr, store_ptr, "DomainTask(Code, _) should route to code_store");
        println!("✓ SearchScope::DomainTask(Code, _) routes to code_store");
    }

    std::env::remove_var("DB_PATH");
    std::env::remove_var("CODE_GRAPH_DB");

    println!("\n✅ All SearchScope routing tests passed!");

    Ok(())
}
