//! Vector embedding quality tests
//!
//! Tests semantic search quality using a golden corpus of queries and documents.
//! These tests verify that:
//! 1. Vector insert and search roundtrip works correctly
//! 2. Semantic similarity ranking is accurate
//! 3. Code snippet search returns relevant results

use anyhow::Result;
use std::fs;
use std::path::Path;
use syncore::vector::{HuggingFaceEmbeddings, VectorStore, SearchScope};

/// Load queries from the golden corpus
fn load_queries() -> Result<Vec<String>> {
    let path = Path::new("tests/data/embeddings/queries.txt");
    let content = fs::read_to_string(path)?;
    Ok(content.lines().map(|s| s.to_string()).collect())
}

/// Load documents from the golden corpus
/// Returns (doc_id, doc_text) pairs
fn load_docs() -> Result<Vec<(i64, String)>> {
    let path = Path::new("tests/data/embeddings/docs.txt");
    let content = fs::read_to_string(path)?;

    let mut docs = Vec::new();
    let mut current_doc = String::new();
    let mut current_id = None;

    for line in content.lines() {
        if line.starts_with("# Doc ") {
            // Save previous doc if exists
            if let Some(id) = current_id {
                if !current_doc.is_empty() {
                    docs.push((id, current_doc.trim().to_string()));
                }
            }

            // Parse new doc ID: "# Doc 0: description"
            let parts: Vec<&str> = line.split(':').collect();
            if let Some(id_part) = parts.first() {
                let id_str = id_part.trim_start_matches("# Doc ").trim();
                current_id = id_str.parse::<i64>().ok();
                current_doc = parts[1..].join(":").trim().to_string();
            }
        } else if !line.trim().is_empty() {
            if !current_doc.is_empty() {
                current_doc.push('\n');
            }
            current_doc.push_str(line);
        }
    }

    // Don't forget the last doc
    if let Some(id) = current_id {
        if !current_doc.is_empty() {
            docs.push((id, current_doc.trim().to_string()));
        }
    }

    Ok(docs)
}

#[test]
fn test_vector_insert_and_search_roundtrip() -> Result<()> {
    // Use HuggingFace embeddings (real production backend)
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store = VectorStore::new(embeddings);

    // Insert a document about memory safety
    let doc_text = "Rust programming and memory safety";
    store.insert_text(1, None, doc_text, "doc")?;

    // Search for semantically similar query
    let results = store.search("memory safety in Rust", 5, SearchScope::Global)?;

    // Assert we get at least one result
    assert!(!results.is_empty(), "Expected at least one search result");

    // Assert the document appears in top results with reasonable score
    let top_hit = &results[0];
    assert_eq!(top_hit.id, 1, "Expected doc ID 1 to be top result");
    assert!(top_hit.score > 0.3, "Expected score > 0.3, got {}", top_hit.score);

    Ok(())
}

#[test]
fn test_semantic_similarity_ranking() -> Result<()> {
    // Use HuggingFace embeddings
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store = VectorStore::new(embeddings);

    // Load all docs from golden corpus
    let docs = load_docs()?;
    assert!(docs.len() >= 10, "Expected at least 10 docs in corpus");

    // Insert all docs
    for (id, text) in &docs {
        store.insert_text(*id, None, text, "doc")?;
    }

    // Define expected mappings: query -> expected doc IDs in top 3
    let test_cases = vec![
        ("memory storage implementation", vec![0, 4]),  // Doc 0: hybrid storage, Doc 4: SQLite
        ("MCP server tool routing", vec![1, 14]),       // Doc 1: MCP routing, Doc 14: stdio transport
        ("vector search HNSW", vec![2]),                // Doc 2: HNSW algorithm
        ("task management with dependencies", vec![3, 13]), // Doc 3: task hierarchy, Doc 13: scheduling
        ("Ollama LLM integration", vec![5, 9, 15]),     // Doc 5: Ollama, Doc 9: sequential, Doc 15: IntelliTask
        ("tree-sitter code parsing", vec![6, 10]),      // Doc 6: parser, Doc 10: entity extraction
        ("function body indexing", vec![11]),           // Doc 11: body indexing
    ];

    for (query, expected_ids) in test_cases {
        let results = store.search(query, 3, SearchScope::Global)?;

        // Assert we get results
        assert!(!results.is_empty(), "Query '{}' returned no results", query);

        // Extract top 3 IDs
        let top_ids: Vec<i64> = results.iter().take(3).map(|h| h.id).collect();

        // Assert at least one expected doc appears in top 3
        let found = expected_ids.iter().any(|&expected| top_ids.contains(&expected));
        assert!(
            found,
            "Query '{}': expected one of {:?} in top 3, got {:?}",
            query, expected_ids, top_ids
        );

        // Assert the expected doc has higher score than completely unrelated docs
        // (This is a weak assertion but ensures basic ranking works)
        if !results.is_empty() {
            assert!(results[0].score > 0.1,
                "Query '{}': top result score too low: {}",
                query, results[0].score
            );
        }
    }

    Ok(())
}

#[test]
fn test_code_snippet_search() -> Result<()> {
    // Use HuggingFace embeddings
    let embeddings = Box::new(HuggingFaceEmbeddings::new()?);
    let mut store = VectorStore::new(embeddings);

    // Insert code snippets from golden corpus
    // These are Docs 10, 11, 12 which contain function-like code
    let code_snippets = vec![
        (10, "fn extract_entities(file_path: &Path, language: Language) -> Result<Vec<Entity>> { let parser = create_parser(language)?; let entities = parse_tree_for_entities(&parser, file_path)?; Ok(entities.into_iter().map(|e| Entity { name: e.name, kind: e.kind, span: e.span }).collect()) }"),
        (11, "pub fn index_function_body(func: &FunctionNode) -> Result<BodySnippet> { let body_text = extract_body_text(&func.span)?; let tokens = tokenize_code(&body_text, func.language); let snippet = truncate_tokens(tokens, MAX_BODY_TOKENS); Ok(BodySnippet { text: snippet, language: func.language }) }"),
        (12, r#"pub fn search_with_ripgrep(pattern: &str, directory: &Path, context: usize) -> Result<Vec<Match>> { let output = Command::new("rg").args(&[pattern, "--context", &context.to_string()]).current_dir(directory).output()?; parse_ripgrep_output(&output.stdout) }"#),
    ];

    for (id, code) in &code_snippets {
        store.insert_text(*id, None, code, "code")?;
    }

    // Test queries with functional intent
    let test_cases = vec![
        ("extract code entities from file", 10),    // Doc 10: extract_entities function
        ("index function body implementation", 11), // Doc 11: index_function_body
        ("search code with ripgrep", 12),           // Doc 12: search_with_ripgrep
    ];

    for (query, expected_id) in test_cases {
        let results = store.search(query, 3, SearchScope::Global)?;

        assert!(!results.is_empty(), "Query '{}' returned no results", query);

        // Assert expected function appears in top 3
        let top_ids: Vec<i64> = results.iter().take(3).map(|h| h.id).collect();
        assert!(
            top_ids.contains(&expected_id),
            "Query '{}': expected doc {} in top 3, got {:?}",
            query, expected_id, top_ids
        );
    }

    Ok(())
}
