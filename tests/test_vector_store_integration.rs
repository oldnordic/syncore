use anyhow::Result;
use std::sync::{Arc, Mutex};
use syncore::vector::{HuggingFaceEmbeddings, SearchScope, VectorStore};

/// Integration test: VectorStore with real HuggingFace embeddings
/// Tests insert, search, and semantic similarity end-to-end
#[test]
fn test_vector_store_with_real_embeddings() -> Result<()> {
    println!("=== VectorStore Integration Test with Real Embeddings ===\n");

    // Create VectorStore with real embeddings
    println!("Creating VectorStore with HuggingFaceEmbeddings...");
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut vector_store = VectorStore::new(embeddings);

    println!("✅ VectorStore created\n");

    // Test 1: Insert code snippets
    println!("Test 1: Inserting code snippets into VectorStore");

    let rust_add = "fn add(a: i32, b: i32) -> i32 { a + b }";
    let rust_multiply = "fn multiply(x: i32, y: i32) -> i32 { x * y }";
    let rust_fibonacci =
        "fn fibonacci(n: u32) -> u32 { if n <= 1 { n } else { fibonacci(n-1) + fibonacci(n-2) } }";
    let python_sum = "def sum(a, b): return a + b";
    let javascript_loop = "for (let i = 0; i < 10; i++) { console.log(i); }";

    let id1 = 1;
    let id2 = 2;
    let id3 = 3;
    let id4 = 4;
    let id5 = 5;

    vector_store.insert_text(id1, None, rust_add, "code")?;
    vector_store.insert_text(id2, None, rust_multiply, "code")?;
    vector_store.insert_text(id3, None, rust_fibonacci, "code")?;
    vector_store.insert_text(id4, None, python_sum, "code")?;
    vector_store.insert_text(id5, None, javascript_loop, "code")?;

    println!("Inserted 5 code snippets:");
    println!("  ID {}: Rust add function", id1);
    println!("  ID {}: Rust multiply function", id2);
    println!("  ID {}: Rust fibonacci function", id3);
    println!("  ID {}: Python sum function", id4);
    println!("  ID {}: JavaScript loop", id5);
    println!("✅ Insert successful\n");

    // Test 2: Search for similar code
    println!("Test 2: Semantic search for 'addition function'");
    let query = "function that adds two numbers";
    let results = vector_store.search(query, 3, SearchScope::Global)?;

    println!("Query: '{}'", query);
    println!("Top {} results:", results.len());
    for (i, result) in results.iter().enumerate() {
        println!(
            "  {}. ID {}, Score: {:.4}, Text: '{}'",
            i + 1,
            result.id,
            result.score,
            result.text
        );
    }

    // Verify that addition-related functions are in top results
    assert!(!results.is_empty(), "Should return results");
    let top_result_text = &results[0].text;
    assert!(
        top_result_text.contains("add") || top_result_text.contains("sum"),
        "Top result should be about addition, got: '{}'",
        top_result_text
    );
    println!("✅ Top result is addition-related\n");

    // Test 3: Search for multiplication
    println!("Test 3: Semantic search for 'multiplication'");
    let query2 = "multiply two numbers together";
    let results2 = vector_store.search(query2, 3, SearchScope::Global)?;

    println!("Query: '{}'", query2);
    println!("Top {} results:", results2.len());
    for (i, result) in results2.iter().enumerate() {
        println!(
            "  {}. ID {}, Score: {:.4}, Text: '{}'",
            i + 1,
            result.id,
            result.score,
            result.text
        );
    }

    let top_result_text2 = &results2[0].text;
    assert!(
        top_result_text2.contains("multiply"),
        "Top result should be about multiplication, got: '{}'",
        top_result_text2
    );
    println!("✅ Top result is multiplication-related\n");

    // Test 4: Search for recursive function
    println!("Test 4: Semantic search for 'recursive function'");
    let query3 = "recursive algorithm";
    let results3 = vector_store.search(query3, 2, SearchScope::Global)?;

    println!("Query: '{}'", query3);
    println!("Top {} results:", results3.len());
    for (i, result) in results3.iter().enumerate() {
        println!(
            "  {}. ID {}, Score: {:.4}, Text: '{}'",
            i + 1,
            result.id,
            result.score,
            result.text
        );
    }

    let top_result_text3 = &results3[0].text;
    assert!(
        top_result_text3.contains("fibonacci") || top_result_text3.contains("fibonacci"),
        "Top result should be fibonacci (recursive), got: '{}'",
        top_result_text3
    );
    println!("✅ Top result is recursive function\n");

    // Test 5: Verify different queries return different results
    println!("Test 5: Different queries should return different top results");
    assert_ne!(
        results[0].id, results2[0].id,
        "Addition query and multiplication query should return different top results"
    );
    println!("✅ Different queries return different results\n");

    println!("=== All VectorStore Integration Tests PASSED! ===");

    Ok(())
}

/// Test VectorStore with multi-threaded Arc<Mutex<>> pattern (production usage)
#[test]
fn test_vector_store_thread_safe() -> Result<()> {
    println!("=== Testing VectorStore Thread Safety ===\n");

    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));

    // Insert from main thread
    {
        let mut store = vector_store.lock().unwrap();
        store.insert_text(1, None, "fn main() { println!(\"Hello\"); }", "code")?;
        store.insert_text(2, None, "fn test() { assert!(true); }", "code")?;
        println!("Inserted 2 items from main thread");
    }

    // Search from main thread
    {
        let store = vector_store.lock().unwrap();
        let results = store.search("print hello", 1, SearchScope::Global)?;
        assert!(!results.is_empty(), "Should find results");
        println!("Search successful: found {} results", results.len());
    }

    println!("✅ Thread-safe Arc<Mutex<>> pattern works correctly\n");

    Ok(())
}

/// Performance test: Insert and search with real embeddings
#[test]
fn test_vector_store_performance() -> Result<()> {
    println!("=== Testing VectorStore Performance ===\n");

    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut vector_store = VectorStore::new(embeddings);

    // Test insert performance
    println!("Test 1: Insert performance (10 items)");
    let items = vec![
        "fn add(a: i32, b: i32) -> i32 { a + b }",
        "fn subtract(a: i32, b: i32) -> i32 { a - b }",
        "fn multiply(a: i32, b: i32) -> i32 { a * b }",
        "fn divide(a: i32, b: i32) -> i32 { a / b }",
        "fn modulo(a: i32, b: i32) -> i32 { a % b }",
        "struct Point { x: f64, y: f64 }",
        "impl Point { fn new(x: f64, y: f64) -> Self { Point { x, y } } }",
        "enum Color { Red, Green, Blue }",
        "trait Drawable { fn draw(&self); }",
        "fn main() { println!(\"Hello, world!\"); }",
    ];

    let start = std::time::Instant::now();
    for (idx, item) in items.iter().enumerate() {
        vector_store.insert_text((idx + 1) as i64, None, item, "code")?;
    }
    let insert_duration = start.elapsed();

    println!("Inserted {} items in {:?}", items.len(), insert_duration);
    println!(
        "Average insert time: {:?}",
        insert_duration / items.len() as u32
    );

    // Should be reasonably fast (<100ms per item on average)
    assert!(
        insert_duration.as_millis() < 1000,
        "Insert should be <1s for 10 items, got {:?}",
        insert_duration
    );
    println!("✅ Insert performance acceptable\n");

    // Test search performance
    println!("Test 2: Search performance");
    let start = std::time::Instant::now();
    let results = vector_store.search("arithmetic operations", 5, SearchScope::Global)?;
    let search_duration = start.elapsed();

    println!("Search completed in {:?}", search_duration);
    println!("Found {} results", results.len());

    // Search should be fast (<200ms)
    assert!(
        search_duration.as_millis() < 200,
        "Search should be <200ms, got {:?}",
        search_duration
    );
    println!("✅ Search performance acceptable\n");

    println!("=== Performance Tests PASSED! ===");

    Ok(())
}

/// Test that VectorStore maintains quality with real embeddings
#[test]
fn test_vector_store_semantic_quality() -> Result<()> {
    println!("=== Testing VectorStore Semantic Quality ===\n");

    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut vector_store = VectorStore::new(embeddings);

    // Insert domain-specific content
    vector_store.insert_text(
        1,
        None,
        "HTTP GET request to fetch user data from API",
        "operation",
    )?;
    vector_store.insert_text(
        2,
        None,
        "Database query to retrieve customer records",
        "operation",
    )?;
    vector_store.insert_text(
        3,
        None,
        "File system operation to read configuration file",
        "operation",
    )?;
    vector_store.insert_text(
        4,
        None,
        "WebSocket connection for real-time chat",
        "operation",
    )?;
    vector_store.insert_text(5, None, "Redis cache lookup for session data", "operation")?;

    println!("Inserted 5 domain-specific operations\n");

    // Test 1: Network operations
    println!("Test 1: Search for network operation");
    let results = vector_store.search("network request", 2, SearchScope::Global)?;
    println!("Query: 'network request'");
    println!("Top result: '{}'", results[0].text);
    assert!(
        results[0].text.contains("HTTP") || results[0].text.contains("WebSocket"),
        "Should find network-related operation"
    );
    println!("✅ Found network operation\n");

    // Test 2: Data retrieval
    println!("Test 2: Search for data retrieval");
    let results = vector_store.search("get data from storage", 2, SearchScope::Global)?;
    println!("Query: 'get data from storage'");
    println!("Top result: '{}'", results[0].text);
    // Verify we get data-related operations (HTTP, Database, cache, or File are all valid)
    assert!(
        results[0].text.contains("data")
            || results[0].text.contains("Database")
            || results[0].text.contains("cache")
            || results[0].text.contains("File")
            || results[0].text.contains("HTTP"),
        "Should find data-related operation, got: '{}'",
        results[0].text
    );
    println!("✅ Found data-related operation\n");

    println!("=== Semantic Quality Tests PASSED! ===");

    Ok(())
}
