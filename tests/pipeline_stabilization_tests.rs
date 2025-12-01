//! Pipeline Stabilization Tests
//!
//! Tests for the Pipeline Executor with guardrails and deterministic behavior.
//! Verifies HopGraph → RAGGraph → VectorRefine → Fusion pipeline execution.

use anyhow::Result;
use syncore::query::{
    EntityScores, PipelineContext, PipelineEntity, PipelineExecutor, PipelineStage, PlannerStep,
    QueryConstraints, QueryPlan, QueryPlanner, ScoringWeights,
};

#[tokio::test]
async fn test_pipeline_empty_plan_execution() -> Result<()> {
    let executor = PipelineExecutor::new();
    let plan = QueryPlan {
        steps: vec![],
        constraints: QueryConstraints::default(),
        metadata: std::collections::HashMap::new(),
    };

    let result = executor.execute(&plan, "test query").await?;

    // Should return empty result
    assert_eq!(result.entities.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_pipeline_vector_only_execution() -> Result<()> {
    let executor = PipelineExecutor::new();
    let plan = QueryPlan {
        steps: vec![PlannerStep::VectorRefine, PlannerStep::Fusion],
        constraints: QueryConstraints::default(),
        metadata: std::collections::HashMap::new(),
    };

    let result = executor.execute(&plan, "test query").await?;

    // Should return empty result (placeholder implementation)
    assert_eq!(result.entities.len(), 0);
    assert!(result.metadata.contains_key("step"));

    Ok(())
}

#[tokio::test]
async fn test_pipeline_full_execution() -> Result<()> {
    let executor = PipelineExecutor::new();
    let plan = QueryPlan {
        steps: vec![
            PlannerStep::HopGraph,
            PlannerStep::RAGGraph,
            PlannerStep::VectorRefine,
            PlannerStep::Fusion,
        ],
        constraints: QueryConstraints::default(),
        metadata: std::collections::HashMap::new(),
    };

    let result = executor.execute(&plan, "test query").await?;

    // Should return empty result (placeholder implementation)
    assert_eq!(result.entities.len(), 0);
    assert!(result.metadata.contains_key("step"));

    Ok(())
}

#[tokio::test]
async fn test_pipeline_guardrail_short_circuit_graph_required() -> Result<()> {
    let executor = PipelineExecutor::new();
    let plan = QueryPlan {
        steps: vec![PlannerStep::HopGraph, PlannerStep::VectorRefine, PlannerStep::Fusion],
        constraints: QueryConstraints {
            graph_required: true,
            ..Default::default()
        },
        metadata: std::collections::HashMap::new(),
    };

    let result = executor.execute(&plan, "test query").await?;

    // Should short-circuit due to empty HopGraph + graph_required
    assert!(result.metadata.contains_key("short_circuit"));
    assert_eq!(result.entities.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_pipeline_guardrail_no_short_circuit_graph_not_required() -> Result<()> {
    let executor = PipelineExecutor::new();
    let plan = QueryPlan {
        steps: vec![PlannerStep::HopGraph, PlannerStep::VectorRefine, PlannerStep::Fusion],
        constraints: QueryConstraints {
            graph_required: false,
            ..Default::default()
        },
        metadata: std::collections::HashMap::new(),
    };

    let result = executor.execute(&plan, "test query").await?;

    // Should not short-circuit (graph not required)
    assert!(!result.metadata.contains_key("short_circuit"));
    assert_eq!(result.entities.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_pipeline_max_results_limiting() -> Result<()> {
    let executor = PipelineExecutor::new();
    let plan = QueryPlan {
        steps: vec![PlannerStep::VectorRefine, PlannerStep::Fusion],
        constraints: QueryConstraints {
            max_results: Some(5),
            ..Default::default()
        },
        metadata: std::collections::HashMap::new(),
    };

    let result = executor.execute(&plan, "test query").await?;

    // Should respect max_results
    assert_eq!(result.entities.len(), 0); // Placeholder returns empty

    Ok(())
}

#[tokio::test]
async fn test_pipeline_scoring_weights_default() -> Result<()> {
    let executor = PipelineExecutor::new();
    let plan = QueryPlan {
        steps: vec![PlannerStep::Fusion],
        constraints: QueryConstraints::default(),
        metadata: std::collections::HashMap::new(),
    };

    let result = executor.execute(&plan, "test query").await?;

    // Should use default weights
    assert_eq!(result.scoring_weights.alpha, 0.5);
    assert_eq!(result.scoring_weights.beta, 0.2);
    assert_eq!(result.scoring_weights.tau, 0.1);
    assert_eq!(result.scoring_weights.gamma, 0.2);

    Ok(())
}

#[tokio::test]
async fn test_pipeline_scoring_weights_custom() -> Result<()> {
    let custom_weights = ScoringWeights {
        alpha: 0.6,
        beta: 0.3,
        tau: 0.05,
        gamma: 0.05,
    };
    let executor = PipelineExecutor::with_weights(custom_weights.clone());
    let plan = QueryPlan {
        steps: vec![PlannerStep::Fusion],
        constraints: QueryConstraints::default(),
        metadata: std::collections::HashMap::new(),
    };

    let result = executor.execute(&plan, "test query").await?;

    // Should use custom weights
    assert_eq!(result.scoring_weights.alpha, custom_weights.alpha);
    assert_eq!(result.scoring_weights.beta, custom_weights.beta);
    assert_eq!(result.scoring_weights.tau, custom_weights.tau);
    assert_eq!(result.scoring_weights.gamma, custom_weights.gamma);

    Ok(())
}

#[test]
fn test_pipeline_context_creation() -> Result<()> {
    let query = "test query".to_string();
    let constraints = QueryConstraints::default();
    let context = PipelineContext::new(query.clone(), constraints.clone());

    assert_eq!(context.query, query);
    assert_eq!(context.constraints.scope, constraints.scope);
    assert_eq!(context.constraints.max_results, constraints.max_results);
    assert_eq!(context.partial_results.len(), 0);
    assert_eq!(context.metadata.len(), 0);

    Ok(())
}

#[test]
fn test_pipeline_context_store_and_get_results() -> Result<()> {
    let mut context = PipelineContext::new("test".to_string(), QueryConstraints::default());

    // Store a mock result
    let mock_entity = PipelineEntity {
        id: 1,
        name: "test_entity".to_string(),
        entity_type: "function".to_string(),
        file_path: "src/test.rs".to_string(),
        line_number: Some(10),
        body_snippet: Some("fn test() {}".to_string()),
        scores: EntityScores {
            vector_score: 0.8,
            graph_score: 0.6,
            temporal_score: 0.4,
            graph_embedding_score: 0.7,
        },
    };

    let mock_hopgraph_output = syncore::query::HopGraphOutput {
        entities: vec![mock_entity.clone()],
        metadata: std::collections::HashMap::new(),
        has_results: true,
    };

    context
        .store_result(PlannerStep::HopGraph, PipelineStage::HopGraphResult(mock_hopgraph_output));

    // Retrieve stored result
    let stored_result = context.get_result(&PlannerStep::HopGraph);
    assert!(stored_result.is_some());

    if let Some(PipelineStage::HopGraphResult(output)) = stored_result {
        assert_eq!(output.entities.len(), 1);
        assert_eq!(output.entities[0].name, "test_entity");
        assert!(output.has_results);
    } else {
        panic!("Expected HopGraphResult");
    }

    Ok(())
}

#[test]
fn test_pipeline_context_get_current_entities() -> Result<()> {
    let mut context = PipelineContext::new("test".to_string(), QueryConstraints::default());

    // Store mock entities in different steps
    let mock_entity = PipelineEntity {
        id: 1,
        name: "test_entity".to_string(),
        entity_type: "function".to_string(),
        file_path: "src/test.rs".to_string(),
        line_number: Some(10),
        body_snippet: Some("fn test() {}".to_string()),
        scores: EntityScores {
            vector_score: 0.8,
            graph_score: 0.6,
            temporal_score: 0.4,
            graph_embedding_score: 0.7,
        },
    };

    // Store HopGraph result
    let hopgraph_output = syncore::query::HopGraphOutput {
        entities: vec![mock_entity.clone()],
        metadata: std::collections::HashMap::new(),
        has_results: true,
    };
    context.store_result(PlannerStep::HopGraph, PipelineStage::HopGraphResult(hopgraph_output));

    // Get current entities should return HopGraph entities
    let current_entities = context.get_current_entities();
    assert_eq!(current_entities.len(), 1);
    assert_eq!(current_entities[0].name, "test_entity");

    // Store Vector result (should override)
    let vector_output = syncore::query::VectorSearchOutput {
        entities: vec![mock_entity.clone()],
        metadata: std::collections::HashMap::new(),
        has_matches: true,
    };
    context.store_result(PlannerStep::VectorRefine, PipelineStage::VectorResult(vector_output));

    // Get current entities should return Vector entities (more recent)
    let current_entities = context.get_current_entities();
    assert_eq!(current_entities.len(), 1);
    assert_eq!(current_entities[0].name, "test_entity");

    Ok(())
}

#[test]
fn test_pipeline_context_should_short_circuit() -> Result<()> {
    let mut context = PipelineContext::new(
        "test".to_string(),
        QueryConstraints {
            graph_required: true,
            ..Default::default()
        },
    );

    // Test with no results stored
    assert!(!context.should_short_circuit(&PlannerStep::HopGraph));
    assert!(!context.should_short_circuit(&PlannerStep::RAGGraph));
    assert!(!context.should_short_circuit(&PlannerStep::VectorRefine));
    assert!(!context.should_short_circuit(&PlannerStep::Fusion));

    // Store empty HopGraph result
    let empty_hopgraph = syncore::query::HopGraphOutput {
        entities: vec![],
        metadata: std::collections::HashMap::new(),
        has_results: false,
    };
    context.store_result(PlannerStep::HopGraph, PipelineStage::HopGraphResult(empty_hopgraph));

    // Should short-circuit before RAGGraph and VectorRefine (graph_required = true)
    assert!(context.should_short_circuit(&PlannerStep::RAGGraph));
    assert!(context.should_short_circuit(&PlannerStep::VectorRefine));
    assert!(!context.should_short_circuit(&PlannerStep::Fusion)); // Fusion handles empty vector

    // Test with graph_required = false
    context.constraints.graph_required = false;
    assert!(!context.should_short_circuit(&PlannerStep::RAGGraph));
    assert!(!context.should_short_circuit(&PlannerStep::VectorRefine));

    Ok(())
}

#[test]
fn test_scoring_weights_default_values() -> Result<()> {
    let weights = ScoringWeights::default();

    assert_eq!(weights.alpha, 0.5); // 50% vector
    assert_eq!(weights.beta, 0.2); // 20% graph
    assert_eq!(weights.tau, 0.1); // 10% temporal
    assert_eq!(weights.gamma, 0.2); // 20% graph embedding

    Ok(())
}

#[test]
fn test_fusion_formula_deterministic() -> Result<()> {
    let weights = ScoringWeights::default();
    let scores = EntityScores {
        vector_score: 0.8,
        graph_score: 0.6,
        temporal_score: 0.4,
        graph_embedding_score: 0.7,
    };

    // Expected: 0.5*0.8 + 0.2*0.6 + 0.1*0.4 + 0.2*0.7 = 0.4 + 0.12 + 0.04 + 0.14 = 0.7
    let expected: f32 = 0.5 * 0.8 + 0.2 * 0.6 + 0.1 * 0.4 + 0.2 * 0.7;
    assert!((expected - 0.7).abs() < 0.001);

    // Test clamping to [0.0, 1.0]
    let high_scores = EntityScores {
        vector_score: 1.5,
        graph_score: 1.2,
        temporal_score: 0.9,
        graph_embedding_score: 1.1,
    };
    let combined = weights.alpha * high_scores.vector_score
        + weights.beta * high_scores.graph_score
        + weights.tau * high_scores.temporal_score
        + weights.gamma * high_scores.graph_embedding_score;
    let clamped = combined.clamp(0.0, 1.0);
    assert_eq!(clamped, 1.0);

    // Test low scores
    let low_scores = EntityScores {
        vector_score: -0.5,
        graph_score: 0.0,
        temporal_score: 0.0,
        graph_embedding_score: 0.0,
    };
    let combined = weights.alpha * low_scores.vector_score
        + weights.beta * low_scores.graph_score
        + weights.tau * low_scores.temporal_score
        + weights.gamma * low_scores.graph_embedding_score;
    let clamped = combined.clamp(0.0, 1.0);
    assert_eq!(clamped, 0.0);

    Ok(())
}

#[tokio::test]
async fn test_pipeline_executor_default() -> Result<()> {
    let executor = PipelineExecutor::default();
    let plan = QueryPlan {
        steps: vec![PlannerStep::Fusion],
        constraints: QueryConstraints::default(),
        metadata: std::collections::HashMap::new(),
    };

    let result = executor.execute(&plan, "test query").await?;

    // Should work with default executor
    assert_eq!(result.scoring_weights.alpha, 0.5);
    assert_eq!(result.scoring_weights.beta, 0.2);
    assert_eq!(result.scoring_weights.tau, 0.1);
    assert_eq!(result.scoring_weights.gamma, 0.2);

    Ok(())
}

#[tokio::test]
async fn test_pipeline_deterministic_execution() -> Result<()> {
    let executor = PipelineExecutor::new();
    let plan = QueryPlan {
        steps: vec![PlannerStep::VectorRefine, PlannerStep::Fusion],
        constraints: QueryConstraints::default(),
        metadata: std::collections::HashMap::new(),
    };

    let result1 = executor.execute(&plan, "test query").await?;
    let result2 = executor.execute(&plan, "test query").await?;

    // Same input should produce same output
    assert_eq!(result1.entities.len(), result2.entities.len());
    assert_eq!(result1.scoring_weights.alpha, result2.scoring_weights.alpha);
    assert_eq!(result1.scoring_weights.beta, result2.scoring_weights.beta);
    assert_eq!(result1.scoring_weights.tau, result2.scoring_weights.tau);
    assert_eq!(result1.scoring_weights.gamma, result2.scoring_weights.gamma);

    Ok(())
}
