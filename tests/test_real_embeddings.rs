use anyhow::Result;
use syncore::vector::{Embeddings, HuggingFaceEmbeddings};

/// Test that HuggingFaceEmbeddings produces real, meaningful embeddings
/// This is a TDD test to verify the implementation works with REAL text
#[test]
fn test_huggingface_embeddings_real_text() -> Result<()> {
    println!("Creating HuggingFaceEmbeddings model...");
    let embeddings = HuggingFaceEmbeddings::new()?;

    println!("Model created successfully! Dimension: {}", embeddings.dim());
    assert_eq!(embeddings.dim(), 384, "all-MiniLM-L6-v2 should have 384 dimensions");

    // Test 1: Simple text embedding
    println!("\nTest 1: Embedding simple text");
    let text1 = "Hello world";
    let vec1 = embeddings.embed(text1)?;

    println!("Text: '{}'", text1);
    println!("Embedding length: {}", vec1.len());
    println!("First 5 values: {:?}", &vec1[..5]);

    assert_eq!(vec1.len(), 384, "Embedding should have 384 dimensions");
    assert!(vec1.iter().any(|&x| x != 0.0), "Embedding should not be all zeros");

    // Test 2: Different text produces different embeddings
    println!("\nTest 2: Different text produces different embeddings");
    let text2 = "Goodbye cruel world";
    let vec2 = embeddings.embed(text2)?;

    println!("Text: '{}'", text2);
    println!("First 5 values: {:?}", &vec2[..5]);

    assert_ne!(vec1, vec2, "Different texts should produce different embeddings");

    // Test 3: Semantic similarity
    println!("\nTest 3: Semantic similarity test");
    let text1 = "The quick brown fox jumps over the lazy dog";
    let text2 = "A fast brown fox leaps over a sleepy dog";
    let text3 = "Python is a programming language used for data science";

    let vec_text1 = embeddings.embed(text1)?;
    let vec_text2 = embeddings.embed(text2)?;
    let vec_text3 = embeddings.embed(text3)?;

    // Cosine similarity helper
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot / (mag_a * mag_b)
    }

    let sim_similar = cosine_similarity(&vec_text1, &vec_text2);
    let sim_different = cosine_similarity(&vec_text1, &vec_text3);

    println!("Similarity (fox texts): {:.4}", sim_similar);
    println!("Similarity (fox vs python): {:.4}", sim_different);

    // Similar texts should have higher similarity than unrelated texts
    assert!(sim_similar > 0.5, "Similar texts should have >0.5 similarity, got {:.4}", sim_similar);
    assert!(sim_similar > sim_different,
        "Similar texts should be more similar than unrelated texts. Got similar: {:.4}, different: {:.4}",
        sim_similar, sim_different);

    println!("\n✅ All tests passed! HuggingFaceEmbeddings is working with REAL embeddings!");

    Ok(())
}

/// Test that embeddings are deterministic (same input = same output)
#[test]
fn test_embeddings_deterministic() -> Result<()> {
    println!("Testing embeddings determinism...");
    let embeddings = HuggingFaceEmbeddings::new()?;

    let text = "Rust programming language";
    let vec1 = embeddings.embed(text)?;
    let vec2 = embeddings.embed(text)?;

    assert_eq!(vec1, vec2, "Same text should produce identical embeddings");

    println!("✅ Embeddings are deterministic!");
    Ok(())
}

/// Performance test - embeddings should be reasonably fast
#[test]
fn test_embeddings_performance() -> Result<()> {
    println!("Testing embeddings performance...");
    let embeddings = HuggingFaceEmbeddings::new()?;

    let start = std::time::Instant::now();
    let text = "This is a performance test for embedding generation speed.";
    let _vec = embeddings.embed(text)?;
    let duration = start.elapsed();

    println!("Embedding generation time: {:?}", duration);

    // Should be fast on CPU (<100ms for single embedding)
    assert!(
        duration.as_millis() < 500,
        "Embedding should be generated in <500ms, got {:?}",
        duration
    );

    println!("✅ Performance is acceptable!");
    Ok(())
}
