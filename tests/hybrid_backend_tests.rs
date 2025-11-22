//! TDD Tests for Hybrid/USearch Backend Stabilization
//! Ensures feature flag controls availability and panics are eliminated.

use syncore::vector::{HuggingFaceEmbeddings, HybridVectorStore, VectorBackend};

/// Test that HybridVectorStore returns an error when feature is disabled
/// This test verifies the SAFE behavior - no panics, just errors
#[test]
fn test_hybrid_backend_disabled_returns_error() {
    // When hybrid-backend feature is NOT enabled (default),
    // attempting to create HybridVectorStore should return an error, NOT panic

    let embeddings = HuggingFaceEmbeddings::new().expect("Should create embeddings");

    // This SHOULD return an error, not panic
    let result = HybridVectorStore::new(Box::new(embeddings), VectorBackend::Linear);

    #[cfg(not(feature = "hybrid-backend"))]
    {
        assert!(
            result.is_err(),
            "HybridVectorStore::new should return error when feature is disabled"
        );

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("not yet implemented") || err_msg.contains("feature"),
            "Error message should indicate feature is not implemented: {}",
            err_msg
        );
    }

    #[cfg(feature = "hybrid-backend")]
    {
        assert!(
            result.is_ok(),
            "HybridVectorStore::new should succeed when feature is enabled"
        );
    }
}

/// Test that no panic occurs when creating HybridVectorStore
#[test]
fn test_hybrid_backend_no_panic_on_creation() {
    let embeddings = HuggingFaceEmbeddings::new().expect("Should create embeddings");

    // This MUST NOT panic, regardless of feature flag
    // It should either succeed or return an error
    let _result = HybridVectorStore::new(Box::new(embeddings), VectorBackend::Linear);

    // If we reach here, no panic occurred - test passes
}

/// Test that USearch backend also handles disabled feature safely
#[test]
fn test_usearch_backend_disabled_returns_error() {
    let embeddings = HuggingFaceEmbeddings::new().expect("Should create embeddings");

    let result = HybridVectorStore::new(Box::new(embeddings), VectorBackend::USearch);

    #[cfg(not(feature = "hybrid-backend"))]
    {
        assert!(
            result.is_err(),
            "USearch backend should return error when feature is disabled"
        );
    }

    #[cfg(feature = "hybrid-backend")]
    {
        // When enabled, USearch backend should work
        assert!(
            result.is_ok(),
            "USearch backend should succeed when feature is enabled"
        );
    }
}

/// Test that error messages are informative
#[test]
fn test_hybrid_backend_error_message_is_informative() {
    #[cfg(not(feature = "hybrid-backend"))]
    {
        let embeddings = HuggingFaceEmbeddings::new().expect("Should create embeddings");

        let result = HybridVectorStore::new(Box::new(embeddings), VectorBackend::Linear);
        let err = result.expect_err("Should return error when feature disabled");

        let err_msg = err.to_string().to_lowercase();

        // Error message should be helpful
        assert!(
            err_msg.contains("hybrid")
                || err_msg.contains("backend")
                || err_msg.contains("feature"),
            "Error message should mention hybrid backend or feature flag: {}",
            err_msg
        );
    }
}

/// Test that the box_clone panic is eliminated
/// This test would have panicked with the old implementation
#[test]
fn test_no_panic_on_box_clone() {
    // The panic was in embeddings.box_clone() inside HybridVectorStore::new()
    // We need to ensure this doesn't panic even when we call it

    let embeddings = HuggingFaceEmbeddings::new().expect("Should create embeddings");

    #[cfg(feature = "hybrid-backend")]
    {
        // When enabled, should use Arc instead of box_clone
        let result = HybridVectorStore::new(Box::new(embeddings), VectorBackend::Linear);
        assert!(
            result.is_ok(),
            "Should not panic on creation when feature enabled"
        );
    }

    #[cfg(not(feature = "hybrid-backend"))]
    {
        // When disabled, should return error before reaching box_clone
        let result = HybridVectorStore::new(Box::new(embeddings), VectorBackend::Linear);
        assert!(
            result.is_err(),
            "Should return error before reaching box_clone"
        );
    }
}

/// Test feature flag conditional compilation
#[test]
fn test_feature_flag_conditional_compilation() {
    #[cfg(feature = "hybrid-backend")]
    {
        // Feature is enabled - test passes if this code compiles
        let _ = "hybrid-backend feature is enabled";
    }

    #[cfg(not(feature = "hybrid-backend"))]
    {
        // Feature is disabled - this is the default
        let _ = "hybrid-backend feature is disabled (default)";
    }

    // Test passes either way - verifies conditional compilation works
}
