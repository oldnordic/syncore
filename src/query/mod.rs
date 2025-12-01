//! Query Planning and Pipeline Execution
//!
//! This module provides deterministic query planning and pipeline execution
//! for the Graph-Accelerated Query system.
//!
//! ## Components
//!
//! - **planner**: QueryPlanner with explicit rules and deterministic behavior
//! - **pipeline**: PipelineExecutor with guardrails and stable scoring
//!
//! ## Usage
//!
//! ```rust
//! use syncore::query::{QueryPlanner, PipelineExecutor};
//!
//! // Create planner and executor
//! let planner = QueryPlanner::new();
//! let executor = PipelineExecutor::new();
//!
//! // Plan a query
//! let plan = planner.plan("find format function")?;
//!
//! // Execute the plan
//! let result = executor.execute(&plan, "find format function").await?;
//! ```

pub mod pipeline;
pub mod planner;

// Re-export main types for convenience
pub use pipeline::{
    EntityScores, FusionOutput, HopGraphOutput, PipelineContext, PipelineEntity, PipelineExecutor,
    PipelineStage, RAGGraphOutput, RankedPipelineEntity, ScoreBreakdown, ScoringWeights,
    VectorSearchOutput,
};
pub use planner::{PlannerStep, QueryConstraints, QueryPlan, QueryPlanner};
