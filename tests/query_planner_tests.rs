//! Query Planner Tests
//!
//! Tests for the Graph-Accelerated Query Planner.
//! Verifies deterministic planning behavior and explicit rules.

use anyhow::Result;
use syncore::query::{PlannerStep, QueryConstraints, QueryPlan, QueryPlanner};

#[test]
fn test_planner_simple_file_query() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        scope: "file".to_string(),
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("fmt", constraints)?;

    // Should be: [VectorRefine, Fusion]
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0], PlannerStep::VectorRefine);
    assert_eq!(plan.steps[1], PlannerStep::Fusion);
    assert_eq!(
        plan.metadata.get("rationale").unwrap(),
        "Short file query, no structural hints"
    );
    assert_eq!(plan.metadata.get("planning_rule").unwrap(), "file_scope");

    Ok(())
}

#[test]
fn test_planner_structural_file_query() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        scope: "file".to_string(),
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("std::fmt::Display", constraints)?;

    // Should be: [HopGraph, VectorRefine, Fusion]
    assert_eq!(plan.steps.len(), 3);
    assert_eq!(plan.steps[0], PlannerStep::HopGraph);
    assert_eq!(plan.steps[1], PlannerStep::VectorRefine);
    assert_eq!(plan.steps[2], PlannerStep::Fusion);
    assert_eq!(
        plan.metadata.get("rationale").unwrap(),
        "File query with structural hints"
    );

    Ok(())
}

#[test]
fn test_planner_project_scope() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        scope: "project".to_string(),
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("test function", constraints)?;

    // Should be: [HopGraph, VectorRefine, Fusion]
    assert_eq!(plan.steps.len(), 3);
    assert_eq!(plan.steps[0], PlannerStep::HopGraph);
    assert_eq!(plan.steps[1], PlannerStep::VectorRefine);
    assert_eq!(plan.steps[2], PlannerStep::Fusion);
    assert_eq!(
        plan.metadata.get("rationale").unwrap(),
        "Project scope requires graph restriction"
    );

    Ok(())
}

#[test]
fn test_planner_global_scope() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        scope: "global".to_string(),
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("search query", constraints)?;

    // Should be: [VectorRefine, Fusion]
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0], PlannerStep::VectorRefine);
    assert_eq!(plan.steps[1], PlannerStep::Fusion);
    assert_eq!(
        plan.metadata.get("rationale").unwrap(),
        "Global scope, no graph restriction needed"
    );

    Ok(())
}

#[test]
fn test_planner_workspace_scope() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        scope: "workspace".to_string(),
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("workspace query", constraints)?;

    // Should be: [HopGraph, VectorRefine, Fusion]
    assert_eq!(plan.steps.len(), 3);
    assert_eq!(plan.steps[0], PlannerStep::HopGraph);
    assert_eq!(plan.steps[1], PlannerStep::VectorRefine);
    assert_eq!(plan.steps[2], PlannerStep::Fusion);
    assert_eq!(
        plan.metadata.get("rationale").unwrap(),
        "Workspace scope with light graph filtering"
    );

    Ok(())
}

#[test]
fn test_planner_auto_scope_semantic() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        scope: "auto".to_string(),
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("explain why this fails", constraints)?;

    // Should detect semantic pattern: [VectorRefine, Fusion]
    assert!(plan.steps.contains(&PlannerStep::VectorRefine));
    assert!(plan.steps.contains(&PlannerStep::Fusion));
    assert_eq!(plan.metadata.get("planning_rule").unwrap(), "auto_scope");

    Ok(())
}

#[test]
fn test_planner_auto_scope_structural() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        scope: "auto".to_string(),
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("src/main.rs::function", constraints)?;

    // Should detect structural pattern: [HopGraph, VectorRefine, Fusion]
    assert!(plan.steps.contains(&PlannerStep::HopGraph));
    assert!(plan.steps.contains(&PlannerStep::VectorRefine));
    assert!(plan.steps.contains(&PlannerStep::Fusion));

    Ok(())
}

#[test]
fn test_planner_causal_keywords_add_raggraph() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        scope: "project".to_string(),
        allow_raggraph: true,
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("trace dependency from A to B", constraints)?;

    // Should include RAGGraph: [HopGraph, RAGGraph, VectorRefine, Fusion]
    assert!(plan.steps.contains(&PlannerStep::RAGGraph));
    assert_eq!(
        plan.metadata.get("causal_override").unwrap(),
        "Added RAGGraph for causal reasoning"
    );

    Ok(())
}

#[test]
fn test_planner_permission_constraints() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        scope: "project".to_string(),
        allow_hopgraph: false,
        allow_vector: true,
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("test query", constraints)?;

    // Should not include HopGraph: [VectorRefine, Fusion]
    assert!(!plan.steps.contains(&PlannerStep::HopGraph));
    assert!(plan.steps.contains(&PlannerStep::VectorRefine));
    assert!(plan.steps.contains(&PlannerStep::Fusion));

    Ok(())
}

#[test]
fn test_planner_all_permissions_disabled() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        scope: "project".to_string(),
        allow_hopgraph: false,
        allow_raggraph: false,
        allow_vector: false,
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("test query", constraints)?;

    // Should only have Fusion (always allowed)
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0], PlannerStep::Fusion);

    Ok(())
}

#[test]
fn test_planner_unknown_scope_fallback() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        scope: "unknown".to_string(),
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("test query", constraints)?;

    // Should default to project behavior: [HopGraph, VectorRefine, Fusion]
    assert_eq!(plan.steps.len(), 3);
    assert_eq!(plan.steps[0], PlannerStep::HopGraph);
    assert_eq!(plan.steps[1], PlannerStep::VectorRefine);
    assert_eq!(plan.steps[2], PlannerStep::Fusion);
    assert_eq!(
        plan.metadata.get("planning_rule").unwrap(),
        "unknown_scope_fallback"
    );

    Ok(())
}

#[test]
fn test_planner_deterministic_behavior() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints::default();

    let plan1 = planner.plan_with_constraints("test query", constraints.clone())?;
    let plan2 = planner.plan_with_constraints("test query", constraints)?;

    // Same input should produce same plan
    assert_eq!(plan1.steps, plan2.steps);
    assert_eq!(plan1.metadata, plan2.metadata);

    Ok(())
}

#[test]
fn test_planner_max_results_constraint() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        max_results: Some(5),
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("test query", constraints)?;

    // Constraint should be preserved in plan
    assert_eq!(plan.constraints.max_results, Some(5));

    Ok(())
}

#[test]
fn test_planner_graph_required_constraint() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        graph_required: true,
        scope: "file".to_string(),
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("short", constraints)?;

    // Even for short file query, graph_required should influence planning
    assert_eq!(plan.constraints.graph_required, true);

    Ok(())
}

#[test]
fn test_planner_project_label_constraint() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        project_label: Some("SynCore".to_string()),
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("test query", constraints)?;

    // Project label should be preserved
    assert_eq!(plan.constraints.project_label, Some("SynCore".to_string()));

    Ok(())
}

#[test]
fn test_planner_local_root_constraint() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints {
        local_root: Some("src/code_graph/".to_string()),
        ..Default::default()
    };

    let plan = planner.plan_with_constraints("test query", constraints)?;

    // Local root should be preserved
    assert_eq!(
        plan.constraints.local_root,
        Some("src/code_graph/".to_string())
    );

    Ok(())
}

#[test]
fn test_planner_metadata_completeness() -> Result<()> {
    let planner = QueryPlanner::new();
    let constraints = QueryConstraints::default();

    let plan = planner.plan_with_constraints("test query with multiple words", constraints)?;

    // Should have required metadata fields
    assert!(plan.metadata.contains_key("query_length"));
    assert!(plan.metadata.contains_key("token_count"));
    assert!(plan.metadata.contains_key("scope"));
    assert!(plan.metadata.contains_key("planning_rule"));
    assert!(plan.metadata.contains_key("rationale"));
    assert!(plan.metadata.contains_key("step_count"));
    assert!(plan.metadata.contains_key("steps"));

    // Verify metadata values
    assert_eq!(plan.metadata.get("query_length").unwrap(), "30");
    assert_eq!(plan.metadata.get("token_count").unwrap(), "5");
    assert_eq!(plan.metadata.get("scope").unwrap(), "project");

    Ok(())
}

#[test]
fn test_planner_default_constraints() -> Result<()> {
    let planner = QueryPlanner::new();
    let plan = planner.plan("test query")?;

    // Should use default constraints
    assert_eq!(plan.constraints.scope, "project");
    assert_eq!(plan.constraints.max_results, Some(10));
    assert_eq!(plan.constraints.graph_required, false);
    assert_eq!(plan.constraints.allow_hopgraph, true);
    assert_eq!(plan.constraints.allow_raggraph, true);
    assert_eq!(plan.constraints.allow_vector, true);
    assert_eq!(plan.constraints.project_label, None);
    assert_eq!(plan.constraints.local_root, None);

    Ok(())
}

#[test]
fn test_planner_custom_default_constraints() -> Result<()> {
    let custom_constraints = QueryConstraints {
        scope: "global".to_string(),
        max_results: Some(20),
        graph_required: true,
        allow_hopgraph: false,
        allow_raggraph: false,
        allow_vector: true,
        project_label: Some("TestProject".to_string()),
        local_root: Some("src/".to_string()),
    };

    let planner = QueryPlanner::with_constraints(custom_constraints.clone());
    let plan = planner.plan("test query")?;

    // Should use custom defaults
    assert_eq!(plan.constraints.scope, custom_constraints.scope);
    assert_eq!(plan.constraints.max_results, custom_constraints.max_results);
    assert_eq!(
        plan.constraints.graph_required,
        custom_constraints.graph_required
    );
    assert_eq!(
        plan.constraints.allow_hopgraph,
        custom_constraints.allow_hopgraph
    );
    assert_eq!(
        plan.constraints.allow_raggraph,
        custom_constraints.allow_raggraph
    );
    assert_eq!(
        plan.constraints.allow_vector,
        custom_constraints.allow_vector
    );
    assert_eq!(
        plan.constraints.project_label,
        custom_constraints.project_label
    );
    assert_eq!(plan.constraints.local_root, custom_constraints.local_root);

    Ok(())
}
