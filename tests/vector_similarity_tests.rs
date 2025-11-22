//! TDD Tests for Vector Normalization and Cosine Similarity
//! Ensures correct cosine similarity implementation: sim = dot(a, b) / (||a|| * ||b||)

use syncore::vector::{HuggingFaceEmbeddings, VectorStore};

/// Helper to create a test VectorStore
fn create_test_store() -> VectorStore {
    let embeddings = HuggingFaceEmbeddings::new().expect("Should create embeddings");
    VectorStore::new(Box::new(embeddings))
}

#[test]
fn test_identical_vectors_have_similarity_one() {
    let store = create_test_store();

    let vec_a = vec![1.0, 2.0, 3.0];
    let vec_b = vec![1.0, 2.0, 3.0];

    let similarity = store.cosine_similarity_public(&vec_a, &vec_b);

    // Identical vectors should have similarity = 1.0
    assert!(
        (similarity - 1.0).abs() < 1e-6,
        "Identical vectors should have similarity 1.0, got {}",
        similarity
    );
}

#[test]
fn test_orthogonal_vectors_have_similarity_zero() {
    let store = create_test_store();

    // Orthogonal vectors (dot product = 0)
    let vec_a = vec![1.0, 0.0, 0.0];
    let vec_b = vec![0.0, 1.0, 0.0];

    let similarity = store.cosine_similarity_public(&vec_a, &vec_b);

    // Orthogonal vectors should have similarity = 0.0
    assert!(
        similarity.abs() < 1e-6,
        "Orthogonal vectors should have similarity 0.0, got {}",
        similarity
    );
}

#[test]
fn test_opposite_vectors_have_negative_similarity() {
    let store = create_test_store();

    let vec_a = vec![1.0, 2.0, 3.0];
    let vec_b = vec![-1.0, -2.0, -3.0];

    let similarity = store.cosine_similarity_public(&vec_a, &vec_b);

    // Opposite vectors should have similarity = -1.0
    assert!(
        (similarity - (-1.0)).abs() < 1e-6,
        "Opposite vectors should have similarity -1.0, got {}",
        similarity
    );
}

#[test]
fn test_different_magnitudes_same_direction_equal_similarity() {
    let store = create_test_store();

    // Same direction, different magnitudes
    let vec_a = vec![1.0, 2.0, 3.0];
    let vec_b = vec![2.0, 4.0, 6.0]; // 2x magnitude
    let vec_c = vec![10.0, 20.0, 30.0]; // 10x magnitude

    let sim_ab = store.cosine_similarity_public(&vec_a, &vec_b);
    let sim_ac = store.cosine_similarity_public(&vec_a, &vec_c);
    let sim_bc = store.cosine_similarity_public(&vec_b, &vec_c);

    // All should be 1.0 (same direction)
    assert!(
        (sim_ab - 1.0).abs() < 1e-6,
        "Same direction vectors should have similarity 1.0, got a-b={}",
        sim_ab
    );
    assert!(
        (sim_ac - 1.0).abs() < 1e-6,
        "Same direction vectors should have similarity 1.0, got a-c={}",
        sim_ac
    );
    assert!(
        (sim_bc - 1.0).abs() < 1e-6,
        "Same direction vectors should have similarity 1.0, got b-c={}",
        sim_bc
    );
}

#[test]
fn test_cosine_similarity_is_symmetric() {
    let store = create_test_store();

    let vec_a = vec![1.0, 3.0, 5.0];
    let vec_b = vec![2.0, 4.0, 1.0];

    let sim_ab = store.cosine_similarity_public(&vec_a, &vec_b);
    let sim_ba = store.cosine_similarity_public(&vec_b, &vec_a);

    // Cosine similarity is symmetric
    assert!(
        (sim_ab - sim_ba).abs() < 1e-6,
        "Cosine similarity should be symmetric: ab={}, ba={}",
        sim_ab,
        sim_ba
    );
}

#[test]
fn test_similarity_range() {
    let store = create_test_store();

    // Various test vectors
    let vectors = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
        vec![1.0, 1.0, 1.0],
        vec![-1.0, -1.0, -1.0],
        vec![3.0, -2.0, 7.0],
    ];

    for a in &vectors {
        for b in &vectors {
            let sim = store.cosine_similarity_public(a, b);
            assert!(
                sim >= -1.0 - 1e-6 && sim <= 1.0 + 1e-6,
                "Cosine similarity should be in [-1, 1], got {}",
                sim
            );
        }
    }
}

#[test]
fn test_zero_vector_handling() {
    let store = create_test_store();

    let vec_a = vec![1.0, 2.0, 3.0];
    let zero = vec![0.0, 0.0, 0.0];

    let similarity = store.cosine_similarity_public(&vec_a, &zero);

    // Zero vector should return 0.0 (undefined, but safe default)
    assert!(
        similarity.abs() < 1e-6 || similarity.is_nan(),
        "Zero vector similarity should be 0.0 or NaN, got {}",
        similarity
    );
}

#[test]
fn test_unit_vectors() {
    let store = create_test_store();

    // Unit vectors
    let vec_a = vec![1.0, 0.0, 0.0];
    let vec_b = vec![0.0, 1.0, 0.0];
    let vec_c = vec![1.0 / f32::sqrt(2.0), 1.0 / f32::sqrt(2.0), 0.0];

    // a · b = 0
    let sim_ab = store.cosine_similarity_public(&vec_a, &vec_b);
    assert!(
        sim_ab.abs() < 1e-6,
        "Perpendicular unit vectors: {}",
        sim_ab
    );

    // a · c = 1/sqrt(2)
    let sim_ac = store.cosine_similarity_public(&vec_a, &vec_c);
    let expected = 1.0 / f32::sqrt(2.0);
    assert!(
        (sim_ac - expected).abs() < 1e-5,
        "Expected {}, got {}",
        expected,
        sim_ac
    );
}

#[test]
fn test_high_dimensional_vectors() {
    let store = create_test_store();

    // High-dimensional vectors (typical embedding size)
    let dim = 384;
    let vec_a: Vec<f32> = (0..dim).map(|i| (i as f32) / 100.0).collect();
    let vec_b: Vec<f32> = (0..dim).map(|i| ((dim - i) as f32) / 100.0).collect();

    let sim = store.cosine_similarity_public(&vec_a, &vec_b);

    // Just verify it's in valid range
    assert!(
        sim >= -1.0 && sim <= 1.0,
        "High-dim similarity should be in [-1, 1], got {}",
        sim
    );
}
