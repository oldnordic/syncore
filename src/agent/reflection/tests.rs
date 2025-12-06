//! Reflection Tests
//!
//! Test suite for reflection functionality

use crate::memory::Memory;
use crate::raggraph::{HopGraphTransformer, RagGraphConfig};
use crate::reasoning::ToTEngine;
use crate::vector::{RealEmbeddings, VectorStore};
use std::sync::Arc;

use super::types::ReflectionReport;

/// Create a mock memory for testing
pub fn create_mock_memory() -> Memory {
    // Create an in-memory database for testing
    Memory::new(":memory:").expect("Failed to create in-memory database for testing")
}

/// Create a mock reasoning engine for testing
pub fn create_mock_reasoning_engine() -> ToTEngine {
    // Create a simple reasoning engine for testing
    let memory = Arc::new(create_mock_memory());
    let embeddings = Box::new(RealEmbeddings::new(384).expect("Failed to create embeddings for testing"));
    let vector_store = Arc::new(std::sync::Mutex::new(
        VectorStore::new(embeddings).expect("Failed to create vector store for testing")
    ));
    let hop_graph = HopGraphTransformer::new(RagGraphConfig::default());
    ToTEngine::new(memory, vector_store, hop_graph).expect("Failed to create ToTEngine for testing")
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::analysis::FailureAnalysisEngine;
    use super::super::engine::ReflectionEngine;
    use super::super::types::{FailureCategory};

    #[test]
    fn test_failure_category_classification() {
        let engine = create_mock_reflection_engine();

        // Test network failure
        let category = engine.analysis_engine.classifier().classify_failure_category("Network timeout occurred");
        assert_eq!(category, FailureCategory::Network);

        // Test database failure
        let category = engine.analysis_engine.classifier().classify_failure_category("Database connection failed");
        assert_eq!(category, FailureCategory::Database);

        // Test authentication failure
        let category = engine.analysis_engine.classifier().classify_failure_category("User unauthorized");
        assert_eq!(category, FailureCategory::Authentication);
    }

    #[test]
    fn test_severity_assessment() {
        let engine = create_mock_reflection_engine();

        let category = FailureCategory::Network;
        let severity = engine.analysis_engine.classifier().assess_failure_severity("Network timeout", &category);
        assert!(severity >= 1 && severity <= 10);

        let critical_severity =
            engine.analysis_engine.classifier().assess_failure_severity("Critical network failure", &category);
        assert!(critical_severity > severity); // Critical should increase severity
    }

    #[test]
    fn test_recoverability_assessment() {
        let engine = create_mock_reflection_engine();

        // Network failures should be recoverable
        let recoverable = engine.analysis_engine.classifier().assess_recoverability(&FailureCategory::Network, 5);
        assert!(recoverable);

        // High severity resource failures might not be recoverable
        let not_recoverable = engine.analysis_engine.classifier().assess_recoverability(&FailureCategory::Resource, 9);
        assert!(!not_recoverable);
    }

    #[test]
    fn test_reflection_report_creation() {
        let plan_id = "test_plan_123".to_string();
        let report = ReflectionReport::new(plan_id.clone());

        assert!(report.id.starts_with("reflection_"));
        assert_eq!(report.plan_id, plan_id);
        assert!(!report.failure_detected);
        assert!(report.insights.is_empty());
        assert!(report.created_at > 0);
    }

    /// Create a mock reflection engine for testing
    fn create_mock_reflection_engine() -> ReflectionEngine {
        let memory = Arc::new(create_mock_memory());
        let hop_graph = HopGraphTransformer::new(RagGraphConfig::default());
        let reasoning_engine = Arc::new(create_mock_reasoning_engine());

        ReflectionEngine::new(memory, hop_graph, reasoning_engine)
    }
}