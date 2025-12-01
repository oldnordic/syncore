//! Portfolio Graph Read Operations
//!
//! Type-safe queries for portfolio tracking entities.

use super::schema::portfolio_namespace;
use crate::graph::Neo4jClient;
use anyhow::Result;

/// Result of a Patch query
#[derive(Debug, Clone)]
pub struct PatchResult {
    pub id: i64,
    pub metadata: Option<String>,
}

/// Result of a Step query
#[derive(Debug, Clone)]
pub struct StepResult {
    pub id: i64,
    pub step_number: i64,
    pub metadata: Option<String>,
}

/// Result of a Task query
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub id: i64,
    pub metadata: Option<String>,
}

impl PatchResult {
    pub fn from_neo4j_value(value: &serde_json::Value) -> Option<Self> {
        Some(PatchResult {
            id: value.get("id")?.as_i64()?,
            metadata: value
                .get("metadata")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }
}

impl StepResult {
    pub fn from_neo4j_value(value: &serde_json::Value) -> Option<Self> {
        Some(StepResult {
            id: value.get("id")?.as_i64()?,
            step_number: value.get("step_number")?.as_i64()?,
            metadata: value
                .get("metadata")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }
}

impl TaskResult {
    pub fn from_neo4j_value(value: &serde_json::Value) -> Option<Self> {
        Some(TaskResult {
            id: value.get("id")?.as_i64()?,
            metadata: value
                .get("metadata")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }
}

/// Get a patch by ID
pub async fn get_patch_by_id(client: &Neo4jClient, id: i64) -> Result<Option<PatchResult>> {
    let query = r#"
        MATCH (p:Patch {id: $id, namespace: $ns})
        RETURN p.id as id,
               p.metadata as metadata
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
            ],
        )
        .await?;

    Ok(results.first().and_then(PatchResult::from_neo4j_value))
}

/// Get a step by ID
pub async fn get_step_by_id(client: &Neo4jClient, id: i64) -> Result<Option<StepResult>> {
    let query = r#"
        MATCH (s:Step {id: $id, namespace: $ns})
        RETURN s.id as id,
               s.step_number as step_number,
               s.metadata as metadata
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
            ],
        )
        .await?;

    Ok(results.first().and_then(StepResult::from_neo4j_value))
}

/// Get a task by ID
pub async fn get_task_by_id(client: &Neo4jClient, id: i64) -> Result<Option<TaskResult>> {
    let query = r#"
        MATCH (t:Task {id: $id, namespace: $ns})
        RETURN t.id as id,
               t.metadata as metadata
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("id", serde_json::json!(id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
            ],
        )
        .await?;

    Ok(results.first().and_then(TaskResult::from_neo4j_value))
}

/// Get all patches for a task
pub async fn get_patches_for_task(client: &Neo4jClient, task_id: i64) -> Result<Vec<PatchResult>> {
    let query = r#"
        MATCH (p:Patch {namespace: $ns})-[:FOR_TASK]->(t:Task {id: $task_id, namespace: $ns})
        RETURN p.id as id,
               p.metadata as metadata
        ORDER BY p.id
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("task_id", serde_json::json!(task_id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
            ],
        )
        .await?;

    Ok(results
        .iter()
        .filter_map(PatchResult::from_neo4j_value)
        .collect())
}

/// Get all steps for a task
pub async fn get_steps_for_task(client: &Neo4jClient, task_id: i64) -> Result<Vec<StepResult>> {
    let query = r#"
        MATCH (s:Step {namespace: $ns})-[:FOR_TASK]->(t:Task {id: $task_id, namespace: $ns})
        RETURN s.id as id,
               s.step_number as step_number,
               s.metadata as metadata
        ORDER BY s.step_number
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("task_id", serde_json::json!(task_id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
            ],
        )
        .await?;

    Ok(results
        .iter()
        .filter_map(StepResult::from_neo4j_value)
        .collect())
}

/// Get files that a patch applies to
pub async fn get_patch_files(client: &Neo4jClient, patch_id: i64) -> Result<Vec<String>> {
    let query = r#"
        MATCH (p:Patch {id: $patch_id, namespace: $ns})-[:APPLIES_TO]->(f:File)
        RETURN f.path as path
        ORDER BY f.path
    "#;

    let results = client
        .execute_query(
            query,
            vec![
                ("patch_id", serde_json::json!(patch_id)),
                ("ns", serde_json::json!(portfolio_namespace(client))),
            ],
        )
        .await?;

    Ok(results
        .iter()
        .filter_map(|r| {
            r.get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect())
}

/// Count patches, steps, and tasks
pub async fn count_portfolio_nodes(client: &Neo4jClient) -> Result<(i64, i64, i64)> {
    let query = r#"
        MATCH (n {namespace: $ns})
        WHERE n:Patch OR n:Step OR n:Task
        WITH labels(n)[0] as label, count(n) as count
        RETURN label, count
    "#;

    let results = client
        .execute_query(
            query,
            vec![("ns", serde_json::json!(portfolio_namespace(client)))],
        )
        .await?;

    let mut patches = 0i64;
    let mut steps = 0i64;
    let mut tasks = 0i64;

    for result in results {
        if let (Some(label), Some(count)) = (
            result.get("label").and_then(|v| v.as_str()),
            result.get("count").and_then(|v| v.as_i64()),
        ) {
            match label {
                "Patch" => patches = count,
                "Step" => steps = count,
                "Task" => tasks = count,
                _ => {}
            }
        }
    }

    Ok((patches, steps, tasks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_result_parsing() {
        let json = serde_json::json!({
            "id": 42,
            "metadata": "{\"source\": \"test\"}"
        });

        let result = PatchResult::from_neo4j_value(&json).unwrap();
        assert_eq!(result.id, 42);
        assert_eq!(result.metadata, Some("{\"source\": \"test\"}".to_string()));
    }

    #[test]
    fn test_step_result_parsing() {
        let json = serde_json::json!({
            "id": 10,
            "step_number": 5,
            "metadata": null
        });

        let result = StepResult::from_neo4j_value(&json).unwrap();
        assert_eq!(result.id, 10);
        assert_eq!(result.step_number, 5);
        assert_eq!(result.metadata, None);
    }
}
