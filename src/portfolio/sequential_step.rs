//! Sequential Reasoning Step Tool
//!
//! Records and tracks reasoning chain steps (thought-action-observation cycles)
//! for task execution. Provides semantic search over reasoning history.
//!
//! Integrates with MessageBus, SQLite, and FAISS

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::message_bus::message::{AgentId, Msg, MsgKind};
use crate::router::SynCoreState;
use crate::vector::SearchScope;

/// A single step in a reasoning chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtStep {
    pub task_id: Option<i64>,
    pub step_number: i32,
    pub thought: String,
    pub action: Option<String>,
    pub observation: Option<String>,
    pub reasoning: String,
}

/// Sequential reasoning step tracker
#[derive(Clone)]
pub struct SequentialStep {
    state: SynCoreState,
}

impl SequentialStep {
    /// Create a new sequential step tracker
    pub fn new(state: SynCoreState) -> Self {
        Self::initialize_schema(&state).expect("Failed to initialize sequential step schema");
        Self { state }
    }

    /// Initialize SQLite schema for thought steps
    fn initialize_schema(state: &SynCoreState) -> Result<()> {
        state.tasks.with_db(|conn| {
            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS thought_steps (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    task_id INTEGER,
                    step_number INTEGER NOT NULL,
                    thought TEXT NOT NULL,
                    action TEXT,
                    observation TEXT,
                    reasoning TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                )
                "#,
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_thought_steps_task ON thought_steps(task_id)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_thought_steps_order ON thought_steps(task_id, step_number)",
                [],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    /// Record a thought step
    pub fn record_step(&self, step: &ThoughtStep) -> Result<i64> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs() as i64;

        // Persist to SQLite
        let step_id: i64 = self.state.tasks.with_db(|conn| {
            conn.execute(
                r#"
                INSERT INTO thought_steps (task_id, step_number, thought, action, observation, reasoning, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                rusqlite::params![
                    step.task_id,
                    step.step_number,
                    step.thought,
                    step.action,
                    step.observation,
                    step.reasoning,
                    now,
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        // Index in FAISS for semantic search
        let description = format!(
            "Step {} task:{} thought:{} action:{} observation:{} reasoning:{}",
            step.step_number,
            step.task_id.unwrap_or(-1),
            step.thought,
            step.action.as_deref().unwrap_or("none"),
            step.observation.as_deref().unwrap_or("none"),
            step.reasoning
        );
        {
            let mut store = self.state.general_store.lock().unwrap();
            store.insert_text(step_id, step.task_id, &description, "thought_step")?;
        }

        // Neo4j integration: Use canonical portfolio_graph module
        if let Some(neo4j) = &self.state.neo4j {
            use crate::databases::portfolio_graph::{
                upsert_step, create_for_task_relationship,
                upsert_task, StepProperties, TaskProperties
            };

            let neo4j = neo4j.clone();
            let step_id_val = step_id;
            let step_num = step.step_number;
            let task_id_opt = step.task_id;

            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async move {
                    // Create Step node
                    let _ = upsert_step(&neo4j, StepProperties {
                        id: step_id_val,
                        step_number: step_num as i64,
                        metadata: None,
                    }).await;

                    // Create FOR_TASK relationship if task_id exists
                    if let Some(task_id) = task_id_opt {
                        // Ensure Task node exists
                        let _ = upsert_task(&neo4j, TaskProperties {
                            id: task_id,
                            metadata: None,
                        }).await;

                        // Create relationship
                        let _ = create_for_task_relationship(&neo4j, step_id_val, task_id).await;
                    }
                })
            });
        }

        // Broadcast event via MessageBus
        if let Some(bus) = &self.state.message_bus {
            let msg = Msg {
                id: bus.next_message_id(),
                from: AgentId::Internal("sequential_step".into()),
                to: None, // Broadcast
                kind: MsgKind::Event("sequential_step".to_string()),
                payload: serde_json::json!({
                    "step_id": step_id,
                    "task_id": step.task_id,
                    "step_number": step.step_number,
                    "thought": step.thought,
                    "action": step.action,
                    "has_observation": step.observation.is_some(),
                }),
                timestamp: SystemTime::now(),
            };
            bus.send(msg);
        }

        Ok(step_id)
    }

    /// Get all steps for a specific task
    pub fn get_steps_for_task(&self, task_id: i64) -> Result<Vec<ThoughtStep>> {
        self.state.tasks.with_db(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT task_id, step_number, thought, action, observation, reasoning
                FROM thought_steps
                WHERE task_id = ?1
                ORDER BY step_number ASC
                "#,
            )?;

            let steps = stmt
                .query_map([task_id], |row| {
                    Ok(ThoughtStep {
                        task_id: row.get(0)?,
                        step_number: row.get(1)?,
                        thought: row.get(2)?,
                        action: row.get(3)?,
                        observation: row.get(4)?,
                        reasoning: row.get(5)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(steps)
        })
    }

    /// Search steps by semantic content
    pub fn search_steps(&self, query: &str) -> Result<Vec<ThoughtStep>> {
        let results = {
            let store = self.state.general_store.lock().unwrap();
            store.search(query, 10, SearchScope::Global)?
        };

        let mut steps = Vec::new();
        for result in results {
            // Retrieve step from SQLite by ID
            if let Some(step) = self.get_step_by_id(result.id)? {
                steps.push(step);
            }
        }

        Ok(steps)
    }

    /// Get a single step by its database ID
    fn get_step_by_id(&self, id: i64) -> Result<Option<ThoughtStep>> {
        self.state.tasks.with_db(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT task_id, step_number, thought, action, observation, reasoning
                FROM thought_steps
                WHERE id = ?1
                "#,
            )?;

            let step = stmt.query_row([id], |row| {
                Ok(ThoughtStep {
                    task_id: row.get(0)?,
                    step_number: row.get(1)?,
                    thought: row.get(2)?,
                    action: row.get(3)?,
                    observation: row.get(4)?,
                    reasoning: row.get(5)?,
                })
            });

            match step {
                Ok(s) => Ok(Some(s)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.into()),
            }
        })
    }

    /// Get the latest step for a task
    pub fn get_latest_step(&self, task_id: i64) -> Result<Option<ThoughtStep>> {
        self.state.tasks.with_db(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT task_id, step_number, thought, action, observation, reasoning
                FROM thought_steps
                WHERE task_id = ?1
                ORDER BY step_number DESC
                LIMIT 1
                "#,
            )?;

            let step = stmt.query_row([task_id], |row| {
                Ok(ThoughtStep {
                    task_id: row.get(0)?,
                    step_number: row.get(1)?,
                    thought: row.get(2)?,
                    action: row.get(3)?,
                    observation: row.get(4)?,
                    reasoning: row.get(5)?,
                })
            });

            match step {
                Ok(s) => Ok(Some(s)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.into()),
            }
        })
    }
}
