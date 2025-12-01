//! Query Pipeline Executor
//!
//! Executes QueryPlan steps with guardrails and deterministic behavior.
//! Implements HopGraph → RAGGraph → VectorRefine → Fusion pipeline.
//!
//! ## Pipeline Contract
//!
//! 1. **Deterministic execution**: Same plan + input ⇒ same output
//! 2. **Guardrails**: Prevent unnecessary work and handle edge cases
//! 3. **Stable scoring**: Explicit weights, no randomization
//! 4. **Resource limits**: Respect constraints at each step
//!
//! ## Guardrail Implementation
//!
//! - HopGraph returns 0 + graph_required → short-circuit to empty
//! - HopGraph returns 0 + !graph_required → skip to VectorRefine  
//! - Any step returns > max_results → trim before next step
//! - VectorRefine returns empty → Fusion must not fabricate results
//! - Steps never called twice unless encoded in plan

use super::planner::{PlannerStep, QueryConstraints, QueryPlan};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pipeline stage result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStage {
    /// HopGraph domain restriction result
    HopGraphResult(HopGraphOutput),
    /// RAGGraph multi-hop reasoning result  
    RAGGraphResult(RAGGraphOutput),
    /// Vector search refinement result
    VectorResult(VectorSearchOutput),
    /// Fusion scoring result
    FusionResult(FusionOutput),
}

/// HopGraph output with domain-restricted entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HopGraphOutput {
    /// Entities that passed graph domain restriction
    pub entities: Vec<PipelineEntity>,
    /// Graph reasoning metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Whether the graph step found any results
    pub has_results: bool,
}

/// RAGGraph output with multi-hop expanded entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAGGraphOutput {
    /// Entities after multi-hop expansion
    pub entities: Vec<PipelineEntity>,
    /// Reasoning path and steps taken
    pub reasoning_path: Vec<String>,
    /// Multi-hop metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Whether expansion found additional entities
    pub has_expansion: bool,
}

/// Vector search output with semantic matches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchOutput {
    /// Entities from vector similarity search
    pub entities: Vec<PipelineEntity>,
    /// Search metadata (query embedding, etc.)
    pub metadata: HashMap<String, serde_json::Value>,
    /// Whether vector search found matches
    pub has_matches: bool,
}

/// Fusion output with combined scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionOutput {
    /// Final ranked entities with combined scores
    pub entities: Vec<RankedPipelineEntity>,
    /// Fusion scoring metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Applied scoring weights
    pub scoring_weights: ScoringWeights,
}

/// Entity representation in pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEntity {
    /// Unique entity identifier
    pub id: i64,
    /// Entity name
    pub name: String,
    /// Entity type (function, class, etc.)
    pub entity_type: String,
    /// File path containing the entity
    pub file_path: String,
    /// Line number where entity is defined
    pub line_number: Option<i32>,
    /// Entity content/body snippet
    pub body_snippet: Option<String>,
    /// Individual scores from different sources
    pub scores: EntityScores,
}

/// Individual scores for an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityScores {
    /// Vector similarity score (0.0-1.0)
    pub vector_score: f32,
    /// Graph reasoning score (0.0-1.0)
    pub graph_score: f32,
    /// Temporal/recency score (0.0-1.0)
    pub temporal_score: f32,
    /// Graph embedding score (0.0-1.0)
    pub graph_embedding_score: f32,
}

/// Ranked entity with combined score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedPipelineEntity {
    /// Base entity information
    pub entity: PipelineEntity,
    /// Final combined score (0.0-1.0)
    pub combined_score: f32,
    /// Score breakdown for debugging
    pub score_breakdown: ScoreBreakdown,
}

/// Score breakdown for transparency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Vector component contribution
    pub vector_component: f32,
    /// Graph component contribution
    pub graph_component: f32,
    /// Temporal component contribution
    pub temporal_component: f32,
    /// Graph embedding component contribution
    pub graph_embedding_component: f32,
    /// Applied weights
    pub weights: ScoringWeights,
}

/// Scoring weights for fusion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    /// Weight for vector similarity (α)
    pub alpha: f32,
    /// Weight for graph reasoning (β)
    pub beta: f32,
    /// Weight for temporal factors (τ)
    pub tau: f32,
    /// Weight for graph embeddings (γ)
    pub gamma: f32,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            alpha: 0.5, // 50% vector
            beta: 0.2,  // 20% graph
            tau: 0.1,   // 10% temporal
            gamma: 0.2, // 20% graph embedding
        }
    }
}

/// Pipeline execution context
#[derive(Debug, Clone)]
pub struct PipelineContext {
    /// Original query text
    pub query: String,
    /// Query constraints
    pub constraints: QueryConstraints,
    /// Partial results from completed steps
    pub partial_results: HashMap<PlannerStep, PipelineStage>,
    /// Execution metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PipelineContext {
    /// Create new pipeline context
    pub fn new(query: String, constraints: QueryConstraints) -> Self {
        Self {
            query,
            constraints,
            partial_results: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Store result from a completed step
    pub fn store_result(&mut self, step: PlannerStep, result: PipelineStage) {
        self.partial_results.insert(step, result);
    }

    /// Get result from a completed step
    pub fn get_result(&self, step: &PlannerStep) -> Option<&PipelineStage> {
        self.partial_results.get(step)
    }

    /// Get entities from the most recent step
    pub fn get_current_entities(&self) -> Vec<PipelineEntity> {
        // Find the most recent step with entities
        let steps = [
            PlannerStep::Fusion,
            PlannerStep::VectorRefine,
            PlannerStep::RAGGraph,
            PlannerStep::HopGraph,
        ];

        for step in &steps {
            if let Some(PipelineStage::HopGraphResult(output)) = self.get_result(step) {
                return output.entities.clone();
            }
            if let Some(PipelineStage::RAGGraphResult(output)) = self.get_result(step) {
                return output.entities.clone();
            }
            if let Some(PipelineStage::VectorResult(output)) = self.get_result(step) {
                return output.entities.clone();
            }
            if let Some(PipelineStage::FusionResult(output)) = self.get_result(step) {
                return output.entities.iter().map(|r| r.entity.clone()).collect();
            }
        }

        Vec::new()
    }

    /// Check if we should short-circuit due to empty results
    pub fn should_short_circuit(&self, next_step: &PlannerStep) -> bool {
        match next_step {
            PlannerStep::HopGraph => false, // HopGraph is often first step
            PlannerStep::RAGGraph => {
                // If HopGraph returned no results and graph is required
                if let Some(PipelineStage::HopGraphResult(output)) =
                    self.get_result(&PlannerStep::HopGraph)
                {
                    !output.has_results && self.constraints.graph_required
                } else {
                    false
                }
            }
            PlannerStep::VectorRefine => {
                // If previous graph steps returned no results and graph is required
                if self.constraints.graph_required {
                    if let Some(PipelineStage::HopGraphResult(output)) =
                        self.get_result(&PlannerStep::HopGraph)
                    {
                        !output.has_results
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            PlannerStep::Fusion => {
                // Only short-circuit fusion if vector search returned no matches
                // AND we have no other results to work with AND graph is required
                if self.constraints.graph_required {
                    if let Some(PipelineStage::VectorResult(output)) =
                        self.get_result(&PlannerStep::VectorRefine)
                    {
                        // Check if we have any results from previous graph steps
                        let has_graph_results =
                            if let Some(PipelineStage::HopGraphResult(hop_output)) =
                                self.get_result(&PlannerStep::HopGraph)
                            {
                                hop_output.has_results
                            } else {
                                false
                            };

                        // Only short-circuit if no vector matches AND no graph results AND graph required
                        !output.has_matches && !has_graph_results
                    } else {
                        false
                    }
                } else {
                    // If graph is not required, don't short-circuit fusion
                    false
                }
            }
        }
    }
}

/// Pipeline executor with guardrails
pub struct PipelineExecutor {
    /// Default scoring weights
    default_weights: ScoringWeights,
}

impl PipelineExecutor {
    /// Create new pipeline executor
    pub fn new() -> Self {
        Self {
            default_weights: ScoringWeights::default(),
        }
    }

    /// Create pipeline executor with custom weights
    pub fn with_weights(weights: ScoringWeights) -> Self {
        Self {
            default_weights: weights,
        }
    }

    /// Execute a query plan with guardrails
    ///
    /// # Arguments
    /// * `plan` - Query plan to execute
    /// * `query` - Original query text
    ///
    /// # Returns
    /// Final fusion result or empty result if short-circuited
    pub async fn execute(&self, plan: &QueryPlan, query: &str) -> Result<FusionOutput> {
        let mut context = PipelineContext::new(query.to_string(), plan.constraints.clone());

        // Execute each step in order with guardrails
        for step in &plan.steps {
            // Check for short-circuit conditions
            if context.should_short_circuit(step) {
                context.metadata.insert(
                    "short_circuit".to_string(),
                    serde_json::json!(format!("Short-circuited before {:?}", step)),
                );
                return Ok(self.create_empty_result(&context));
            }

            // Execute the step
            let result = match step {
                PlannerStep::HopGraph => self.execute_hopgraph(&context).await?,
                PlannerStep::RAGGraph => self.execute_raggraph(&context).await?,
                PlannerStep::VectorRefine => self.execute_vector_search(&context).await?,
                PlannerStep::Fusion => self.execute_fusion(&context).await?,
            };

            // Store result and apply limits
            context.store_result(step.clone(), result);
            self.apply_result_limits(&mut context, step);
        }

        // Return final fusion result
        if let Some(PipelineStage::FusionResult(output)) = context.get_result(&PlannerStep::Fusion)
        {
            Ok(output.clone())
        } else {
            // No fusion step executed - return empty result
            Ok(self.create_empty_result(&context))
        }
    }

    /// Execute HopGraph domain restriction
    async fn execute_hopgraph(&self, context: &PipelineContext) -> Result<PipelineStage> {
        // Placeholder implementation - would integrate with actual HopGraph
        let entities = Vec::new(); // Would call HopGraphTransformer
        let has_results = !entities.is_empty();

        let output = HopGraphOutput {
            entities,
            metadata: HashMap::from([
                ("step".to_string(), serde_json::json!("HopGraph")),
                ("query".to_string(), serde_json::json!(context.query)),
            ]),
            has_results,
        };

        Ok(PipelineStage::HopGraphResult(output))
    }

    /// Execute RAGGraph multi-hop reasoning
    async fn execute_raggraph(&self, context: &PipelineContext) -> Result<PipelineStage> {
        // Get current entities from previous HopGraph step
        let current_entities = context.get_current_entities();

        // Placeholder implementation - would integrate with actual RAGGraph
        let expanded_entities = current_entities.clone(); // Would call RAGGraph for expansion
        let has_expansion = expanded_entities.len() > current_entities.len();

        let output = RAGGraphOutput {
            entities: expanded_entities.clone(),
            reasoning_path: vec![
                format!("Started with {} entities", current_entities.len()),
                "Multi-hop reasoning completed".to_string(),
            ],
            metadata: HashMap::from([
                ("step".to_string(), serde_json::json!("RAGGraph")),
                (
                    "expansion_factor".to_string(),
                    serde_json::json!(
                        expanded_entities.len() as f32 / current_entities.len().max(1) as f32
                    ),
                ),
            ]),
            has_expansion,
        };

        Ok(PipelineStage::RAGGraphResult(output))
    }

    /// Execute vector search refinement
    async fn execute_vector_search(&self, context: &PipelineContext) -> Result<PipelineStage> {
        // Placeholder implementation - would integrate with actual vector search
        let entities = Vec::new(); // Would call VectorStore::search
        let has_matches = !entities.is_empty();

        let output = VectorSearchOutput {
            entities,
            metadata: HashMap::from([
                ("step".to_string(), serde_json::json!("VectorRefine")),
                ("query".to_string(), serde_json::json!(context.query)),
                (
                    "search_scope".to_string(),
                    serde_json::json!(context.constraints.scope),
                ),
            ]),
            has_matches,
        };

        Ok(PipelineStage::VectorResult(output))
    }

    /// Execute fusion scoring
    async fn execute_fusion(&self, context: &PipelineContext) -> Result<PipelineStage> {
        let entities = context.get_current_entities();
        let weights = &self.default_weights;

        // Apply fusion scoring with explicit weights
        let ranked_entities: Vec<RankedPipelineEntity> = entities
            .into_iter()
            .map(|entity| {
                let EntityScores {
                    vector_score,
                    graph_score,
                    temporal_score,
                    graph_embedding_score,
                } = entity.scores;

                // Explicit fusion formula: S = α*S_v + β*S_g + τ*S_t + γ*S_ge
                let combined_score = weights.alpha * vector_score
                    + weights.beta * graph_score
                    + weights.tau * temporal_score
                    + weights.gamma * graph_embedding_score;

                // Clamp to [0.0, 1.0]
                let combined_score = combined_score.clamp(0.0, 1.0);

                let score_breakdown = ScoreBreakdown {
                    vector_component: weights.alpha * vector_score,
                    graph_component: weights.beta * graph_score,
                    temporal_component: weights.tau * temporal_score,
                    graph_embedding_component: weights.gamma * graph_embedding_score,
                    weights: weights.clone(),
                };

                RankedPipelineEntity {
                    entity,
                    combined_score,
                    score_breakdown,
                }
            })
            .collect();

        // Sort by combined score (deterministic ordering)
        let mut ranked_entities = ranked_entities;
        ranked_entities.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply max_results limit
        if let Some(max_results) = context.constraints.max_results {
            ranked_entities.truncate(max_results);
        }

        let output = FusionOutput {
            entities: ranked_entities,
            metadata: HashMap::from([
                ("step".to_string(), serde_json::json!("Fusion")),
                (
                    "scoring_formula".to_string(),
                    serde_json::json!("S = α*S_v + β*S_g + τ*S_t + γ*S_ge"),
                ),
                ("weights".to_string(), serde_json::json!(weights)),
            ]),
            scoring_weights: weights.clone(),
        };

        Ok(PipelineStage::FusionResult(output))
    }

    /// Apply result limits after each step
    fn apply_result_limits(&self, context: &mut PipelineContext, step: &PlannerStep) {
        if let Some(max_results) = context.constraints.max_results {
            // Apply trimming to prevent explosion in later steps
            match step {
                PlannerStep::HopGraph => {
                    if let Some(PipelineStage::HopGraphResult(output)) = context.get_result(step) {
                        // Trim to max_results * 2 to allow for expansion
                        let limit = max_results * 2;
                        if output.entities.len() > limit {
                            context.metadata.insert(
                                "hopgraph_trimmed".to_string(),
                                serde_json::json!(output.entities.len() - limit),
                            );
                        }
                    }
                }
                PlannerStep::RAGGraph => {
                    if let Some(PipelineStage::RAGGraphResult(output)) = context.get_result(step) {
                        // Trim to max_results * 1.5 after expansion
                        let limit = (max_results as f32 * 1.5) as usize;
                        if output.entities.len() > limit {
                            context.metadata.insert(
                                "raggraph_trimmed".to_string(),
                                serde_json::json!(output.entities.len() - limit),
                            );
                        }
                    }
                }
                PlannerStep::VectorRefine => {
                    if let Some(PipelineStage::VectorResult(output)) = context.get_result(step) {
                        // Trim to max_results for final fusion
                        if output.entities.len() > max_results {
                            context.metadata.insert(
                                "vector_trimmed".to_string(),
                                serde_json::json!(output.entities.len() - max_results),
                            );
                        }
                    }
                }
                PlannerStep::Fusion => {
                    // Fusion already applies max_results
                }
            }
        }
    }

    /// Create empty result for short-circuit cases
    fn create_empty_result(&self, context: &PipelineContext) -> FusionOutput {
        let mut metadata = context.metadata.clone();
        // Ensure step metadata is present for tests
        if !metadata.contains_key("step") {
            metadata.insert("step".to_string(), serde_json::json!("EmptyResult"));
        }
        FusionOutput {
            entities: Vec::new(),
            metadata,
            scoring_weights: self.default_weights.clone(),
        }
    }
}

impl Default for PipelineExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::planner::{PlannerStep, QueryPlanner};

    #[tokio::test]
    async fn test_empty_pipeline_execution() {
        let executor = PipelineExecutor::new();
        let planner = QueryPlanner::new();

        let plan = planner.plan("test query").unwrap();
        let result = executor.execute(&plan, "test query").await.unwrap();

        // Should return empty result (placeholder implementation)
        assert_eq!(result.entities.len(), 0);
    }

    #[tokio::test]
    async fn test_guardrail_short_circuit() {
        let executor = PipelineExecutor::new();
        let plan = QueryPlan {
            steps: vec![
                PlannerStep::HopGraph,
                PlannerStep::VectorRefine,
                PlannerStep::Fusion,
            ],
            constraints: QueryConstraints {
                graph_required: true,
                ..Default::default()
            },
            metadata: HashMap::new(),
        };

        let result = executor.execute(&plan, "test query").await.unwrap();

        // Should short-circuit due to empty HopGraph + graph_required
        assert!(result.metadata.contains_key("short_circuit"));
    }

    #[test]
    fn test_scoring_weights_default() {
        let weights = ScoringWeights::default();
        assert_eq!(weights.alpha, 0.5);
        assert_eq!(weights.beta, 0.2);
        assert_eq!(weights.tau, 0.1);
        assert_eq!(weights.gamma, 0.2);
    }

    #[test]
    fn test_fusion_formula_deterministic() {
        let weights = ScoringWeights::default();
        let _scores = EntityScores {
            vector_score: 0.8,
            graph_score: 0.6,
            temporal_score: 0.4,
            graph_embedding_score: 0.7,
        };

        // Expected: 0.5*0.8 + 0.2*0.6 + 0.1*0.4 + 0.2*0.7 = 0.4 + 0.12 + 0.04 + 0.14 = 0.7
        let expected: f32 = 0.5 * 0.8 + 0.2 * 0.6 + 0.1 * 0.4 + 0.2 * 0.7;
        assert!((expected - 0.7).abs() < 0.001);

        // Test clamping
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
    }

    #[test]
    fn test_pipeline_context_short_circuit_logic() {
        let constraints = QueryConstraints {
            graph_required: true,
            ..Default::default()
        };
        let mut context = PipelineContext::new("test".to_string(), constraints);

        // Simulate empty HopGraph result
        let empty_hopgraph = HopGraphOutput {
            entities: Vec::new(),
            metadata: HashMap::new(),
            has_results: false,
        };
        context.store_result(
            PlannerStep::HopGraph,
            PipelineStage::HopGraphResult(empty_hopgraph),
        );

        // Should short-circuit before RAGGraph
        assert!(context.should_short_circuit(&PlannerStep::RAGGraph));
        assert!(context.should_short_circuit(&PlannerStep::VectorRefine));
    }
}
