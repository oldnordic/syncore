use anyhow::Result;
use syncore::vector::{Embeddings, SearchScope, VectorStore};

// Test for real embedding implementation
#[cfg(test)]
mod tests {
    use super::*;

    // This test should fail initially because RealEmbeddings produces deterministic
    // but not semantically meaningful embeddings
    #[test]
    fn test_real_embeddings_semantic_similarity() -> Result<()> {
        // We'll create a real embedding implementation that should understand
        // semantic similarity between related texts
        let embeddings = Box::new(syncore::vector::RealEmbeddings::new(384)?);
        let mut store = VectorStore::new(embeddings as Box<dyn Embeddings>);

        // Insert semantically similar texts
        store.insert_text(1, None, "The cat sits on the mat", "test")?;
        store.insert_text(2, None, "A kitten is resting on the rug", "test")?;
        store.insert_text(3, None, "The car drives down the highway", "test")?;

        // Search for similar content - should find cat/kitten texts first
        let results = store.search("feline pet", 2, SearchScope::Global)?;

        // The first two results should be about cats/kittens, not cars
        // This test will fail with RealEmbeddings since they don't understand semantics
        assert_eq!(results.len(), 2);
        assert!(results[0].id == 1 || results[0].id == 2); // Should be cat-related
        assert!(results[1].id == 1 || results[1].id == 2); // Should be cat-related

        Ok(())
    }

    #[test]
    fn test_real_embeddings_dimension_consistency() -> Result<()> {
        let embeddings = Box::new(syncore::vector::RealEmbeddings::new(384)?);

        let vec1 = embeddings.embed("hello")?;
        let vec2 = embeddings.embed("world")?;

        assert_eq!(vec1.len(), 384);
        assert_eq!(vec2.len(), 384);

        // Vectors should be normalized
        let norm1: f32 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();

        assert!((norm1 - 1.0).abs() < 1e-6);
        assert!((norm2 - 1.0).abs() < 1e-6);

        Ok(())
    }
}
