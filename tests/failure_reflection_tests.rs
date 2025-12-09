//! PHASE 4 STEP 5.3: Failure Reflection Tests - TDD Failing Tests First
//!
//! These tests MUST fail initially. They define the reflection engine requirements
//! that will be implemented in subsequent steps.
//!
//! FOR EACH FAILING TEST:
//! • read all affected code areas (ripgrep + tree-sitter)
//! • write minimal patch
//! • re-run tests
//! • fix next failing test
//!
//! Do NOT:
//! × invent behaviors not grounded in existing systems
//! × introduce new agent sub-systems
//! × write pseudo-code
//! × create 'later' comments or TODO markers

use std::sync::Arc;
use syncore::agent::{ApreResult, ReflectionEngine, ReflectionReport};
use syncore::memory::Memory;
use syncore::raggraph::{HopGraphTransformer, RagGraphConfig};

#[cfg(test)]
mod failure_reflection_tests {
    use std::sync::Arc;
    use syncore::agent::{ApreResult, ReflectionEngine, ReflectionReport};
    use syncore::memory::Memory;
    use syncore::raggraph::{HopGraphTransformer, RagGraphConfig};

    // Helper function to create test reflection engine using existing systems
    async fn create_test_reflection_engine() -> ReflectionEngine {
        // Use existing Memory with test database
        let memory = Arc::new(Memory::new(":memory:").expect("Memory creation should succeed"));

        // Use existing HopGraphTransformer with default config
        let hop_graph = HopGraphTransformer::new(RagGraphConfig::default());

        // TDD: Use exact same pattern as working reasoning tests
        let reasoning_engine = Arc::new(create_mock_engine().await);

        // Create ReflectionEngine using existing constructor
        ReflectionEngine::new(memory, hop_graph, reasoning_engine)
    }

    // Helper function to create test ToTEngine using existing systems (async)
    // Following TDD methodology, implement the minimal functionality needed for tests
    async fn create_mock_engine() -> syncore::reasoning::ToTEngine {
        // Use the existing mock helper function pattern from the ReflectionEngine itself
        create_mock_reasoning_engine().await
    }

    // Implement the minimal mock reasoning engine needed for tests
    async fn create_mock_reasoning_engine() -> syncore::reasoning::ToTEngine {
        use syncore::graph::{GraphBackend, SQLiteGraphBackend};
        use syncore::reasoning::ToTEngine;
        use tempfile::tempdir;

        // Create a temporary SQLite database for testing
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let sqlite_backend = SQLiteGraphBackend::connect(
            db_path.to_str().expect("Invalid path"),
            "",
            "",
            "test_namespace",
        )
        .await
        .expect("Failed to create SQLite backend");

        // Use the new sqlitegraph-only constructor
        ToTEngine::with_sqlitegraph(sqlite_backend).await.expect("Failed to create ToT engine")
    }

    /// Test Case 1: ReflectionEngine analyzes failures correctly
    #[tokio::test]
    async fn test_reflection_engine_analyzes_failures() {
        // GIVEN: Existing ReflectionEngine with memory and reasoning
        let mut reflection_engine = create_test_reflection_engine().await;

        let action_description = "Parse Rust source code file";
        let error_message = "Failed to parse: unexpected token at line 42";
        let context = Some(&serde_json::json!({
            "file": "src/parser.rs",
            "line": 42,
            "token": "unexpected"
        }));

        // WHEN: Analyzing failure
        let analysis_result =
            reflection_engine.analyze_failure(action_description, error_message, context).await;

        // THEN: Should generate meaningful reflection report
        assert!(
            analysis_result.is_ok(),
            "Failure analysis should succeed: {:?}",
            analysis_result.err()
        );

        let reflection = analysis_result.unwrap();
        assert!(!reflection.summary.is_empty(), "Reflection summary should not be empty");
        assert!(!reflection.root_causes.is_empty(), "Should identify root causes");
        assert!(!reflection.recommendations.is_empty(), "Should provide recommendations");

        // Reflection should reference the original action and error
        assert!(
            reflection.action_description.contains("Parse Rust source code"),
            "Should reference original action"
        );
        assert!(
            reflection.error_summary.contains("unexpected token"),
            "Should reference error details"
        );
    }

    /// Test Case 2: ReflectionEngine detects infinite loops
    #[tokio::test]
    async fn test_reflection_engine_detects_infinite_loops() {
        // GIVEN: ReflectionEngine with failure pattern tracking
        let mut reflection_engine = create_test_reflection_engine().await;

        // Simulate repeated failures of the same action
        let action = "Parse file with syntax error";
        let error = "Parse error: invalid syntax";

        // Trigger multiple failures to simulate infinite loop
        for _ in 0..5 {
            let _ = reflection_engine.analyze_failure(action, error, None).await;
        }

        // WHEN: Analyzing the same failure again
        let loop_result = reflection_engine.analyze_failure(action, error, None).await;

        // THEN: Should detect infinite loop pattern
        assert!(loop_result.is_ok(), "Analysis should succeed");

        let reflection = loop_result.unwrap();
        let has_loop_warning = reflection.recommendations.iter().any(|r| {
            r.to_lowercase().contains("loop")
                || r.to_lowercase().contains("repeating")
                || r.to_lowercase().contains("infinite")
        });
        assert!(has_loop_warning, "Should detect and warn about infinite loop");
    }

    /// Test Case 3: ReflectionEngine uses existing memory for context
    #[tokio::test]
    async fn test_reflection_engine_uses_existing_memory_context() {
        // GIVEN: Memory with relevant failure context
        let mut reflection_engine = create_test_reflection_engine().await;

        // WHEN: Analyzing related failure
        let action = "Parse source file";
        let error = "unexpected token in src/parser.rs";

        let analysis_result = reflection_engine.analyze_failure(action, error, None).await;

        // THEN: Should leverage memory context in analysis
        assert!(analysis_result.is_ok(), "Analysis should succeed with memory context");

        let reflection = analysis_result.unwrap();

        // Recommendations should reference memory context
        let has_memory_context = reflection
            .recommendations
            .iter()
            .any(|r| r.contains("recovery") || r.contains("fallback") || r.contains("skip"));
        assert!(has_memory_context, "Should use memory context for recommendations");
    }

    /// Test Case 4: ReflectionEngine integrates with existing HopGraph
    #[tokio::test]
    async fn test_reflection_engine_integrates_hopgraph() {
        // GIVEN: ReflectionEngine with HopGraph integration
        let mut reflection_engine = create_test_reflection_engine().await;

        // WHEN: Analyzing complex failure requiring multi-hop reasoning
        let action = "Execute multi-step refactoring";
        let error = "Dependency cycle detected between modules";

        let analysis_result = reflection_engine.analyze_failure(action, error, None).await;

        // THEN: Should use HopGraph for semantic analysis
        assert!(analysis_result.is_ok(), "Analysis should succeed with HopGraph");

        let reflection = analysis_result.unwrap();
        assert!(!reflection.root_causes.is_empty(), "Should identify root causes");

        // Should have semantic understanding beyond surface error
        let has_semantic_analysis = reflection.root_causes.iter().any(|cause| {
            cause.contains("dependency") || cause.contains("cycle") || cause.contains("module")
        });
        assert!(has_semantic_analysis, "Should provide semantic analysis using HopGraph");
    }

    /// Test Case 5: ReflectionEngine maintains deterministic behavior
    #[tokio::test]
    async fn test_reflection_engine_maintains_deterministic_behavior() {
        // GIVEN: Identical inputs and state
        let mut reflection_engine1 = create_test_reflection_engine().await;
        let mut reflection_engine2 = create_test_reflection_engine().await;

        let action = "Parse JSON configuration";
        let error = "Invalid JSON format";

        // WHEN: Analyzing same failure with different engines
        let result1 = reflection_engine1.analyze_failure(action, error, None).await;
        let result2 = reflection_engine2.analyze_failure(action, error, None).await;

        // THEN: Should produce identical results
        assert!(result1.is_ok(), "First analysis should succeed");
        assert!(result2.is_ok(), "Second analysis should succeed");

        let reflection1 = result1.unwrap();
        let reflection2 = result2.unwrap();

        assert_eq!(
            reflection1.summary, reflection2.summary,
            "Reflection summaries should be identical"
        );
        assert_eq!(
            reflection1.root_causes.len(),
            reflection2.root_causes.len(),
            "Should have same number of root causes"
        );
        assert_eq!(
            reflection1.recommendations.len(),
            reflection2.recommendations.len(),
            "Should have same number of recommendations"
        );
    }

    /// Test Case 6: ReflectionEngine handles complex failure scenarios
    #[tokio::test]
    async fn test_reflection_engine_handles_complex_failure_scenarios() {
        // GIVEN: Complex multi-layered failure
        let mut reflection_engine = create_test_reflection_engine().await;

        let complex_context = serde_json::json!({
            "stack_trace": [
                "parse_function() at parser.rs:123",
                "process_file() at main.rs:45",
                "handle_request() at server.rs:89"
            ],
            "system_state": {
                "memory_usage": "85%",
                "active_connections": 150,
                "error_rate": "12%"
            },
            "recent_failures": [
                "timeout in database connection",
                "memory allocation failed",
                "parse error in config file"
            ]
        });

        // WHEN: Analyzing complex failure
        let action = "Process user request with database query";
        let error = "System overload: too many concurrent operations";

        let analysis_result =
            reflection_engine.analyze_failure(action, error, Some(&complex_context)).await;

        // THEN: Should handle complexity gracefully
        assert!(analysis_result.is_ok(), "Complex analysis should succeed");

        let reflection = analysis_result.unwrap();

        // Should identify systemic issues beyond surface error
        let has_systemic_analysis = reflection.root_causes.iter().any(|cause| {
            cause.contains("concurrent") || cause.contains("overload") || cause.contains("resource")
        });
        assert!(has_systemic_analysis, "Should identify systemic issues");

        // Should provide actionable recommendations
        assert!(!reflection.recommendations.is_empty(), "Should provide recommendations");
    }

    /// Test Case 7: ReflectionEngine respects existing error patterns
    #[tokio::test]
    async fn test_reflection_engine_respects_existing_error_patterns() {
        // GIVEN: ReflectionEngine with established error patterns
        let mut reflection_engine = create_test_reflection_engine().await;

        // Note: In a real implementation, established patterns would be stored in memory
        // For this test, we assume the ReflectionEngine has access to existing patterns

        // WHEN: Analyzing error that matches established pattern
        let action = "Parse user configuration file";
        let error = "File not found: /etc/app/config.json";

        let analysis_result = reflection_engine.analyze_failure(action, error, None).await;

        // THEN: Should respect and use established patterns
        assert!(analysis_result.is_ok(), "Analysis should succeed");

        let reflection = analysis_result.unwrap();

        // Should reference established recovery mechanisms
        let has_established_patterns = reflection.recommendations.iter().any(|rec| {
            rec.contains("retry") || rec.contains("fallback") || rec.contains("graceful")
        });
        assert!(has_established_patterns, "Should use established error handling patterns");
    }
}
