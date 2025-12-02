use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CogState {
    Think,
    Decide,
    Act,
    Observe,
    Reflect,
}

impl fmt::Display for CogState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CogState::Think => write!(f, "Think"),
            CogState::Decide => write!(f, "Decide"),
            CogState::Act => write!(f, "Act"),
            CogState::Observe => write!(f, "Observe"),
            CogState::Reflect => write!(f, "Reflect"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Step {
    pub id: i64,
    pub task_id: Option<i64>,
    pub state: String,
    pub content: String,
    pub meta_json: String,
    pub created_at: i64,
}

pub struct CognitiveEngine {
    db: rusqlite::Connection,
}

impl CognitiveEngine {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = crate::db::open_db_with_wal(db_path)?;
        crate::db::ensure_schema(db_path)?;

        Ok(Self {
            db,
        })
    }

    pub fn store_step(
        &self,
        task_id: Option<i64>,
        state: &str,
        content: &str,
        meta_json: &str,
    ) -> Result<i64> {
        let now =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
                as i64;

        self.db.execute(
            "INSERT INTO steps (task_id, state, content, meta_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (task_id, state, content, meta_json, now),
        )?;

        Ok(self.db.last_insert_rowid())
    }

    pub fn recent_steps(&self, task_id: i64, n: usize) -> Result<Vec<Step>> {
        let mut stmt = self.db.prepare(
            "SELECT id, task_id, state, content, meta_json, created_at
             FROM steps
             WHERE task_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;

        let steps = stmt.query_map([task_id, n as i64], |row| {
            Ok(Step {
                id: row.get(0)?,
                task_id: row.get(1)?,
                state: row.get(2)?,
                content: row.get(3)?,
                meta_json: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        let mut result = Vec::new();
        for step in steps {
            result.push(step?);
        }

        Ok(result)
    }

    pub fn get_step(&self, step_id: i64) -> Result<Option<Step>> {
        let step = self
            .db
            .query_row(
                "SELECT id, task_id, state, content, meta_json, created_at
             FROM steps WHERE id = ?1",
                [step_id],
                |row| {
                    Ok(Step {
                        id: row.get(0)?,
                        task_id: row.get(1)?,
                        state: row.get(2)?,
                        content: row.get(3)?,
                        meta_json: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                },
            )
            .optional()?;

        Ok(step)
    }

    pub fn create_think_step(&self, task_id: Option<i64>, content: &str) -> Result<i64> {
        self.store_step(task_id, "Think", content, "{}")
    }

    pub fn create_decide_step(
        &self,
        task_id: Option<i64>,
        content: &str,
        decision: &str,
    ) -> Result<i64> {
        let meta = serde_json::json!({"decision": decision}).to_string();
        self.store_step(task_id, "Decide", content, &meta)
    }

    pub fn create_act_step(
        &self,
        task_id: Option<i64>,
        content: &str,
        action: &str,
    ) -> Result<i64> {
        let meta = serde_json::json!({"action": action}).to_string();
        self.store_step(task_id, "Act", content, &meta)
    }

    pub fn create_observe_step(
        &self,
        task_id: Option<i64>,
        content: &str,
        observation: &str,
    ) -> Result<i64> {
        let meta = serde_json::json!({"observation": observation}).to_string();
        self.store_step(task_id, "Observe", content, &meta)
    }

    pub fn create_reflect_step(
        &self,
        task_id: Option<i64>,
        content: &str,
        reflection: &str,
    ) -> Result<i64> {
        let meta = serde_json::json!({"reflection": reflection}).to_string();
        self.store_step(task_id, "Reflect", content, &meta)
    }
}

// Export the exact functions the user requested
pub fn store_step(
    db: &Connection,
    task_id: Option<i64>,
    state: &str,
    content: &str,
    meta_json: &str,
) -> Result<i64> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        as i64;

    db.execute(
        "INSERT INTO steps (task_id, state, content, meta_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (task_id, state, content, meta_json, now),
    )?;

    Ok(db.last_insert_rowid())
}

pub fn recent_steps(db: &Connection, task_id: i64, n: usize) -> Result<Vec<Step>> {
    let mut stmt = db.prepare(
        "SELECT id, task_id, state, content, meta_json, created_at
         FROM steps
         WHERE task_id = ?1
         ORDER BY created_at DESC
         LIMIT ?2",
    )?;

    let steps = stmt.query_map([task_id, n as i64], |row| {
        Ok(Step {
            id: row.get(0)?,
            task_id: row.get(1)?,
            state: row.get(2)?,
            content: row.get(3)?,
            meta_json: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;

    let mut result = Vec::new();
    for step in steps {
        result.push(step?);
    }

    Ok(result)
}
