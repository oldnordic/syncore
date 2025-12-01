//! Code Change Tracking Tool
//!
//! Tracks and records code changes with:
//! - Change type (add, modify, delete)
//! - Old and new content
//! - Line ranges and descriptions
//! - Task correlation
//! - Semantic search over changes
//!
//! Integrates with MessageBus, SQLite, and FAISS

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::message_bus::message::{AgentId, Msg, MsgKind};
use crate::router::SynCoreState;
use crate::vector::SearchScope;

/// A single code change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    pub file_path: String,
    pub change_type: String, // "add", "modify", "delete"
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub line_start: i32,
    pub line_end: i32,
    pub description: String,
    pub task_id: Option<i64>,
}

/// Code change tracking tool
#[derive(Clone)]
pub struct ApplicationTool {
    state: SynCoreState,
}

impl ApplicationTool {
    /// Create a new application tool
    pub fn new(state: SynCoreState) -> Self {
        Self::initialize_schema(&state).expect("Failed to initialize code change schema");
        Self { state }
    }

    /// Initialize SQLite schema for code changes
    fn initialize_schema(state: &SynCoreState) -> Result<()> {
        state.tasks.with_db(|conn| {
            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS code_changes (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    file_path TEXT NOT NULL,
                    change_type TEXT NOT NULL,
                    old_content TEXT,
                    new_content TEXT,
                    line_start INTEGER NOT NULL,
                    line_end INTEGER NOT NULL,
                    description TEXT NOT NULL,
                    task_id INTEGER,
                    created_at INTEGER NOT NULL
                )
                "#,
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_code_changes_task ON code_changes(task_id)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_code_changes_file ON code_changes(file_path)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_code_changes_time ON code_changes(created_at)",
                [],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    /// Record a code change
    pub fn record_change(&self, change: &CodeChange) -> Result<i64> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs() as i64;

        // Persist to SQLite
        let change_id: i64 = self.state.tasks.with_db(|conn| {
            conn.execute(
                r#"
                INSERT INTO code_changes (file_path, change_type, old_content, new_content, line_start, line_end, description, task_id, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                rusqlite::params![
                    change.file_path,
                    change.change_type,
                    change.old_content,
                    change.new_content,
                    change.line_start,
                    change.line_end,
                    change.description,
                    change.task_id,
                    now,
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        // Index in FAISS for semantic search
        let description = format!(
            "{} {} at {}:{}-{} task:{} {}",
            change.change_type,
            change.file_path,
            change.line_start,
            change.line_end,
            change.task_id.unwrap_or(-1),
            change.description,
            change.new_content.as_deref().unwrap_or("")
        );
        {
            let mut store = self.state.general_store.lock().unwrap();
            store.insert_text(change_id, change.task_id, &description, "code_change")?;
        }

        // Neo4j integration: Use canonical portfolio_graph module
        if let Some(neo4j) = &self.state.neo4j {
            use crate::databases::portfolio_graph::{
                create_applies_to_relationship, create_for_task_relationship, upsert_patch,
                upsert_task, PatchProperties,
            };

            let neo4j = neo4j.clone();
            let patch_id = change_id;
            let file_path = change.file_path.clone();
            let task_id_opt = change.task_id;

            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async move {
                    // Create Patch node
                    let _ = upsert_patch(
                        &neo4j,
                        PatchProperties {
                            id: patch_id,
                            metadata: None,
                        },
                    )
                    .await;

                    // Create APPLIES_TO relationship to File
                    let _ = create_applies_to_relationship(&neo4j, patch_id, &file_path).await;

                    // Create FOR_TASK relationship if task_id exists
                    if let Some(task_id) = task_id_opt {
                        // Ensure Task node exists
                        use crate::databases::portfolio_graph::TaskProperties;
                        let _ = upsert_task(
                            &neo4j,
                            TaskProperties {
                                id: task_id,
                                metadata: None,
                            },
                        )
                        .await;

                        // Create relationship
                        let _ = create_for_task_relationship(&neo4j, patch_id, task_id).await;
                    }
                })
            });
        }

        // Broadcast event via MessageBus
        if let Some(bus) = &self.state.message_bus {
            let msg = Msg {
                id: bus.next_message_id(),
                from: AgentId::Internal("application_tool".into()),
                to: None, // Broadcast
                kind: MsgKind::Event("code_change".to_string()),
                payload: serde_json::json!({
                    "change_id": change_id,
                    "file_path": change.file_path,
                    "change_type": change.change_type,
                    "line_start": change.line_start,
                    "line_end": change.line_end,
                    "description": change.description,
                    "task_id": change.task_id,
                }),
                timestamp: SystemTime::now(),
            };
            bus.send(msg);
        }

        Ok(change_id)
    }

    /// Get all changes for a specific task
    pub fn get_changes_for_task(&self, task_id: i64) -> Result<Vec<CodeChange>> {
        self.state.tasks.with_db(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT file_path, change_type, old_content, new_content, line_start, line_end, description, task_id
                FROM code_changes
                WHERE task_id = ?1
                ORDER BY created_at ASC
                "#,
            )?;

            let changes = stmt
                .query_map([task_id], |row| {
                    Ok(CodeChange {
                        file_path: row.get(0)?,
                        change_type: row.get(1)?,
                        old_content: row.get(2)?,
                        new_content: row.get(3)?,
                        line_start: row.get(4)?,
                        line_end: row.get(5)?,
                        description: row.get(6)?,
                        task_id: row.get(7)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(changes)
        })
    }

    /// Get change history for a specific file
    pub fn get_file_history(&self, file_path: &str) -> Result<Vec<CodeChange>> {
        self.state.tasks.with_db(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT file_path, change_type, old_content, new_content, line_start, line_end, description, task_id
                FROM code_changes
                WHERE file_path = ?1
                ORDER BY created_at ASC
                "#,
            )?;

            let changes = stmt
                .query_map([file_path], |row| {
                    Ok(CodeChange {
                        file_path: row.get(0)?,
                        change_type: row.get(1)?,
                        old_content: row.get(2)?,
                        new_content: row.get(3)?,
                        line_start: row.get(4)?,
                        line_end: row.get(5)?,
                        description: row.get(6)?,
                        task_id: row.get(7)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(changes)
        })
    }

    /// Search changes by semantic content
    pub fn search_changes(&self, query: &str) -> Result<Vec<CodeChange>> {
        let results = {
            let store = self.state.general_store.lock().unwrap();
            store.search(query, 10, SearchScope::Global)?
        };

        let mut changes = Vec::new();
        for result in results {
            if let Some(change) = self.get_change_by_id(result.id)? {
                changes.push(change);
            }
        }

        Ok(changes)
    }

    /// Get a single change by its database ID
    fn get_change_by_id(&self, id: i64) -> Result<Option<CodeChange>> {
        self.state.tasks.with_db(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT file_path, change_type, old_content, new_content, line_start, line_end, description, task_id
                FROM code_changes
                WHERE id = ?1
                "#,
            )?;

            let change = stmt.query_row([id], |row| {
                Ok(CodeChange {
                    file_path: row.get(0)?,
                    change_type: row.get(1)?,
                    old_content: row.get(2)?,
                    new_content: row.get(3)?,
                    line_start: row.get(4)?,
                    line_end: row.get(5)?,
                    description: row.get(6)?,
                    task_id: row.get(7)?,
                })
            });

            match change {
                Ok(c) => Ok(Some(c)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.into()),
            }
        })
    }

    /// Get recent changes across all files
    pub fn get_recent_changes(&self, limit: usize) -> Result<Vec<CodeChange>> {
        self.state.tasks.with_db(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT file_path, change_type, old_content, new_content, line_start, line_end, description, task_id
                FROM code_changes
                ORDER BY created_at DESC
                LIMIT ?1
                "#,
            )?;

            let changes = stmt
                .query_map([limit as i64], |row| {
                    Ok(CodeChange {
                        file_path: row.get(0)?,
                        change_type: row.get(1)?,
                        old_content: row.get(2)?,
                        new_content: row.get(3)?,
                        line_start: row.get(4)?,
                        line_end: row.get(5)?,
                        description: row.get(6)?,
                        task_id: row.get(7)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(changes)
        })
    }
}
