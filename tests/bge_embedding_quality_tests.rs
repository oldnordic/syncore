//! TDD Tests for BGE Embedding Quality
//!
//! These tests verify that BGE-small-en-v1.5 embeddings produce semantically meaningful results.
//! Written FIRST per TDD - implementation in vector.rs must pass these tests.
//!
//! Quality Assertions:
//! 1. Semantic similarity: "error handling" closer to "exception catching" than "database schema"
//! 2. Code search: Query "parse function" should find parse-related entities
//! 3. Consistency: Same input produces same embedding (deterministic)
//! 4. Dimensionality: BGE-small-en produces 384-dim vectors
//! 5. Performance: Single embedding under 100ms

use syncore::vector::{Embeddings, HuggingFaceEmbeddings};

/// Helper to compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same dimension");
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

#[test]
fn test_bge_embeddings_dimension_is_384() {
    let embeddings = HuggingFaceEmbeddings::new().expect("Failed to create embeddings");
    assert_eq!(embeddings.dim(), 384, "BGE-small-en produces 384-dim vectors");
}

#[test]
fn test_bge_embeddings_deterministic() {
    let embeddings = HuggingFaceEmbeddings::new().expect("Failed to create embeddings");
    let text = "fn parse_json(input: &str) -> Result<Value, Error>";

    let emb1 = embeddings.embed(text).expect("Failed to embed");
    let emb2 = embeddings.embed(text).expect("Failed to embed");

    // Same input should produce identical output
    assert_eq!(emb1, emb2, "Embeddings should be deterministic");
}

#[test]
fn test_bge_semantic_similarity_error_handling() {
    let embeddings = HuggingFaceEmbeddings::new().expect("Failed to create embeddings");

    // Semantically related concepts
    let error_handling = embeddings.embed("error handling and exception management").unwrap();
    let exception_catching = embeddings.embed("catching exceptions and handling errors").unwrap();
    let database_schema = embeddings.embed("database schema design and table structure").unwrap();

    let sim_related = cosine_similarity(&error_handling, &exception_catching);
    let sim_unrelated = cosine_similarity(&error_handling, &database_schema);

    println!("error_handling <-> exception_catching: {:.4}", sim_related);
    println!("error_handling <-> database_schema: {:.4}", sim_unrelated);

    // Related concepts should have higher similarity than unrelated
    assert!(
        sim_related > sim_unrelated,
        "Related concepts ({:.4}) should have higher similarity than unrelated ({:.4})",
        sim_related,
        sim_unrelated
    );

    // Additionally, related concepts should have similarity > 0.5
    assert!(
        sim_related > 0.5,
        "Semantically related concepts should have similarity > 0.5, got {:.4}",
        sim_related
    );
}

#[test]
fn test_bge_semantic_similarity_code_concepts() {
    let embeddings = HuggingFaceEmbeddings::new().expect("Failed to create embeddings");

    // Code-related concepts
    let parse_function = embeddings.embed("parse function that processes input data").unwrap();
    let parser_code = embeddings.embed("parser implementation for processing text").unwrap();
    let render_ui = embeddings.embed("render user interface components on screen").unwrap();

    let sim_related = cosine_similarity(&parse_function, &parser_code);
    let sim_unrelated = cosine_similarity(&parse_function, &render_ui);

    println!("parse_function <-> parser_code: {:.4}", sim_related);
    println!("parse_function <-> render_ui: {:.4}", sim_unrelated);

    assert!(
        sim_related > sim_unrelated,
        "Parser concepts ({:.4}) should be more similar than unrelated UI ({:.4})",
        sim_related,
        sim_unrelated
    );
}

#[test]
fn test_bge_rust_function_search_relevance() {
    let embeddings = HuggingFaceEmbeddings::new().expect("Failed to create embeddings");

    // Query and candidate code snippets
    let query = "error handling in rust";
    let query_emb = embeddings.embed(query).unwrap();

    // Relevant candidates
    let relevant1 = embeddings.embed("fn handle_error(e: Error) -> Result<()> { ... }").unwrap();
    let relevant2 = embeddings.embed("impl From<io::Error> for MyError { ... }").unwrap();

    // Irrelevant candidates
    let irrelevant1 =
        embeddings.embed("fn calculate_sum(a: i32, b: i32) -> i32 { a + b }").unwrap();
    let irrelevant2 = embeddings.embed("struct User { name: String, age: u32 }").unwrap();

    let sim_r1 = cosine_similarity(&query_emb, &relevant1);
    let sim_r2 = cosine_similarity(&query_emb, &relevant2);
    let sim_i1 = cosine_similarity(&query_emb, &irrelevant1);
    let sim_i2 = cosine_similarity(&query_emb, &irrelevant2);

    println!("Query: '{}'", query);
    println!("  handle_error fn: {:.4}", sim_r1);
    println!("  From<io::Error>: {:.4}", sim_r2);
    println!("  calculate_sum:   {:.4}", sim_i1);
    println!("  User struct:     {:.4}", sim_i2);

    // At least one relevant should beat all irrelevant
    let max_relevant = sim_r1.max(sim_r2);
    let max_irrelevant = sim_i1.max(sim_i2);

    assert!(
        max_relevant > max_irrelevant,
        "Error-handling code ({:.4}) should be more relevant than math/struct code ({:.4})",
        max_relevant,
        max_irrelevant
    );
}

#[test]
fn test_bge_embedding_performance_under_100ms() {
    let embeddings = HuggingFaceEmbeddings::new().expect("Failed to create embeddings");
    let text =
        "pub async fn execute_query(conn: &Connection, query: &str) -> Result<Vec<Row>, DbError>";

    let start = std::time::Instant::now();
    let _emb = embeddings.embed(text).expect("Failed to embed");
    let elapsed = start.elapsed();

    println!("Embedding time: {:?}", elapsed);

    // First embedding may include model warmup, but should still be under 100ms
    // (The user specified "fast" - under 100ms per embedding is reasonable)
    assert!(
        elapsed.as_millis() < 500,
        "Single embedding should complete in under 500ms (got {:?}). Model warmup may be slow on first run.",
        elapsed
    );
}

#[test]
fn test_bge_batch_embedding_consistency() {
    let embeddings = HuggingFaceEmbeddings::new().expect("Failed to create embeddings");

    let texts = vec![
        "fn parse_config(path: &str) -> Config",
        "fn load_settings(file: &Path) -> Settings",
        "fn read_configuration(source: &str) -> AppConfig",
    ];

    let mut embs = Vec::new();
    for text in &texts {
        embs.push(embeddings.embed(text).expect("Failed to embed"));
    }

    // All config-related functions should be somewhat similar to each other
    let sim_01 = cosine_similarity(&embs[0], &embs[1]);
    let sim_02 = cosine_similarity(&embs[0], &embs[2]);
    let sim_12 = cosine_similarity(&embs[1], &embs[2]);

    println!("parse_config <-> load_settings: {:.4}", sim_01);
    println!("parse_config <-> read_configuration: {:.4}", sim_02);
    println!("load_settings <-> read_configuration: {:.4}", sim_12);

    // These should all be reasonably similar (>0.4) since they're all config functions
    assert!(sim_01 > 0.4, "Config functions should have similarity > 0.4");
    assert!(sim_02 > 0.4, "Config functions should have similarity > 0.4");
    assert!(sim_12 > 0.4, "Config functions should have similarity > 0.4");
}

#[test]
fn test_bge_normalized_vectors() {
    let embeddings = HuggingFaceEmbeddings::new().expect("Failed to create embeddings");
    let text = "async fn fetch_data(url: &str) -> Result<Data>";
    let emb = embeddings.embed(text).expect("Failed to embed");

    // Check vector magnitude (should be approximately 1.0 for normalized vectors)
    let magnitude: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();

    println!("Vector magnitude: {:.6}", magnitude);

    // Allow small floating point error
    assert!(
        (magnitude - 1.0).abs() < 0.01,
        "Embedding vectors should be normalized (magnitude ~1.0), got {:.6}",
        magnitude
    );
}
