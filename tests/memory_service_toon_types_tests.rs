//! TDD Tests for TOON Memory Operation Types

use syncore::memory_service::toon::ToonMemoryOp;

#[test]
fn test_toon_store_op_construction() {
    // Test constructing a Store operation
    let op = ToonMemoryOp::Store {
        summary: "Test summary".to_string(),
        importance: 0.8,
        tags: vec!["test".to_string()],
        raw_text: "Raw text content".to_string(),
    };

    match op {
        ToonMemoryOp::Store {
            summary,
            importance,
            tags,
            raw_text,
        } => {
            assert_eq!(summary, "Test summary");
            assert_eq!(importance, 0.8);
            assert_eq!(tags.len(), 1);
            assert_eq!(raw_text, "Raw text content");
        }
        _ => panic!("Expected Store variant"),
    }
}

#[test]
fn test_toon_controller_max_context_tokens() {
    use std::sync::{Arc, Mutex};
    use syncore::memory_service::{MemoryService, ToonController, ToonGraph};

    // Test ToonController with max_context_tokens
    let max_tokens = 1000;
    let graph = ToonGraph::new();
    let memory = Arc::new(Mutex::new(MemoryService::new(128, 10));
    let controller = ToonController::new(graph, memory, max_tokens);

    assert_eq!(
        controller.max_context_tokens(),
        max_tokens,
        "Should return configured max tokens"
    );

    // Test prompt fitting
    let short_prompt = "Short prompt that should fit";
    assert!(
        controller.can_fit_prompt(short_prompt),
        "Short prompt should fit"
    );

    let long_prompt = "A".repeat(500); // ~2000 chars = ~500 tokens
    assert!(
        !controller.can_fit_prompt(&long_prompt),
        "Long prompt should not fit"
    );
}

#[test]
fn test_toon_retrieve_op_construction() {
    // Test constructing a Retrieve operation
    let op = ToonMemoryOp::Retrieve {
        query: "search query".to_string(),
        k: 5,
    };

    match op {
        ToonMemoryOp::Retrieve { query, k } => {
            assert_eq!(query, "search query");
            assert_eq!(k, 5);
        }
        _ => panic!("Expected Retrieve variant"),
    }
}

#[test]
fn test_toon_pointer_op_construction() {
    // Test constructing a Pointer operation
    let op = ToonMemoryOp::Pointer {
        id: "N123".to_string(),
    };

    match op {
        ToonMemoryOp::Pointer { id } => {
            assert_eq!(id, "N123");
        }
        _ => panic!("Expected Pointer variant"),
    }
}

#[test]
fn test_toon_store_with_multiple_tags() {
    // Test Store operation with multiple tags
    let op = ToonMemoryOp::Store {
        summary: "Multi-tag summary".to_string(),
        importance: 0.9,
        tags: vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()],
        raw_text: "Content".to_string(),
    };

    match op {
        ToonMemoryOp::Store { tags, .. } => {
            assert_eq!(tags.len(), 3);
        }
        _ => panic!("Expected Store variant"),
    }
}

#[test]
fn test_toon_retrieve_with_k_zero() {
    // Test that k=0 is valid (though semantically unusual)
    let op = ToonMemoryOp::Retrieve {
        query: "query".to_string(),
        k: 0,
    };

    match op {
        ToonMemoryOp::Retrieve { k, .. } => {
            assert_eq!(k, 0);
        }
        _ => panic!("Expected Retrieve variant"),
    }
}
