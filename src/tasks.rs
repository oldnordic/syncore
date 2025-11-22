use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: i64,
    pub goal: String,
    pub description: String,
    pub status: String,
    pub priority: i32,
    pub parent_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug)]
pub struct Tasks {
    /// Database connection. Public for RealExecutor access only.
    pub db: Arc<Mutex<Connection>>,
}

impl Clone for Tasks {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
        }
    }
}

impl Tasks {
    /// Create Tasks using an existing database connection from DbManager.
    ///
    /// This is the preferred constructor when using DbManager. It reuses long-lived
    /// connections instead of creating new ones per-call.
    ///
    /// # Arguments
    ///
    /// * `db` - Arc<Mutex<Connection>> from DbManager.main_conn()
    ///
    /// # Example
    ///
    /// ```rust
    /// let db_manager = DbManager::new("syncore.db", "syncore_code_graph.db")?;
    /// let tasks = Tasks::with_connection(db_manager.main_conn())?;
    /// ```
    pub fn with_connection(db: Arc<Mutex<Connection>>) -> Result<Self> {
        Ok(Self { db })
    }

    /// Legacy constructor - opens its own connection (deprecated, use with_connection instead).
    ///
    /// This method is kept for backward compatibility with existing code that hasn't
    /// been refactored to use DbManager yet.
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = crate::db::open_db_with_wal(db_path)?;
        crate::db::ensure_schema(db_path)?;

        Self::with_connection(Arc::new(Mutex::new(conn)))
    }

    pub fn add_task(
        &self,
        goal: &str,
        description: &str,
        priority: i32,
        parent: Option<i64>,
    ) -> Result<i64> {
        let db = self.db.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        db.execute(
            "INSERT INTO tasks (goal, description, status, priority, parent_id, created_at, updated_at)
             VALUES (?1, ?2, 'open', ?3, ?4, ?5, ?5)",
            (goal, description, priority, parent, now),
        )?;

        Ok(db.last_insert_rowid())
    }

    pub fn update_task(
        db: &Connection,
        id: i64,
        status: Option<&str>,
        prio: Option<i32>,
        desc: Option<&str>,
    ) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut query_parts: Vec<String> = vec!["UPDATE tasks SET updated_at = ?1".to_string()];
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now)];
        let mut param_count = 2;

        if let Some(s) = status {
            query_parts.push(format!(", status = ?{}", param_count));
            params.push(Box::new(s.to_string()));
            param_count += 1;
        }

        if let Some(p) = prio {
            query_parts.push(format!(", priority = ?{}", param_count));
            params.push(Box::new(p));
            param_count += 1;
        }

        if let Some(d) = desc {
            query_parts.push(format!(", description = ?{}", param_count));
            params.push(Box::new(d.to_string()));
            param_count += 1;
        }

        query_parts.push(format!(" WHERE id = ?{}", param_count));
        params.push(Box::new(id));

        let query: String = query_parts.join("");
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        db.execute(&query, rusqlite::params_from_iter(param_refs))?;
        Ok(())
    }

    pub fn next_task(
        &self,
        statuses: Option<&[&str]>,
        min_prio: Option<i32>,
    ) -> Result<Option<Task>> {
        let db = self.db.lock().unwrap();

        let mut query = "
            SELECT id, goal, description, status, priority, parent_id, created_at, updated_at
            FROM tasks
            WHERE status != 'done' AND status != 'cancelled'
        "
        .to_string();

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(statuses) = statuses {
            let params_list: Vec<String> = statuses
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect();
            query.push_str(&format!(" AND status IN ({})", params_list.join(", ")));
            for s in statuses {
                params.push(Box::new(s.to_string()) as Box<dyn rusqlite::ToSql>);
            }
        }

        if let Some(min_prio) = min_prio {
            query.push_str(&format!(" AND priority <= ?{}", params.len() + 1));
            params.push(Box::new(min_prio));
        }

        query.push_str(" ORDER BY priority ASC, created_at ASC LIMIT 1");

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = db.prepare(&query)?;
        let task = stmt
            .query_row(rusqlite::params_from_iter(param_refs), |row| {
                Ok(Task {
                    id: row.get(0)?,
                    goal: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    priority: row.get(4)?,
                    parent_id: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })
            .optional()?;

        Ok(task)
    }

    pub fn link_tasks(&self, src: i64, dst: i64, kind: &str) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT OR REPLACE INTO task_links (src_id, dst_id, kind) VALUES (?1, ?2, ?3)",
            (src, dst, kind),
        )?;
        Ok(())
    }

    pub fn get_task(&self, id: i64) -> Result<Option<Task>> {
        let db = self.db.lock().unwrap();
        let task = db
            .query_row(
                "SELECT id, goal, description, status, priority, parent_id, created_at, updated_at
             FROM tasks WHERE id = ?1",
                [id],
                |row| {
                    Ok(Task {
                        id: row.get(0)?,
                        goal: row.get(1)?,
                        description: row.get(2)?,
                        status: row.get(3)?,
                        priority: row.get(4)?,
                        parent_id: row.get(5)?,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                    })
                },
            )
            .optional()?;

        Ok(task)
    }

    pub fn get_task_dependencies(&self, task_id: i64) -> Result<Vec<Task>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT t.id, t.goal, t.description, t.status, t.priority, t.parent_id, t.created_at, t.updated_at
             FROM tasks t
             JOIN task_links tl ON t.id = tl.dst_id
             WHERE tl.src_id = ?1 AND tl.kind = 'depends_on'
             ORDER BY t.priority ASC, t.created_at ASC"
        )?;

        let tasks = stmt.query_map([task_id], |row| {
            Ok(Task {
                id: row.get(0)?,
                goal: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                parent_id: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        let mut result = Vec::new();
        for task in tasks {
            result.push(task?);
        }

        Ok(result)
    }

    pub fn get_db(&self) -> Arc<Mutex<Connection>> {
        self.db.clone()
    }

    pub fn with_db<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Connection) -> Result<R>,
    {
        let db = self.db.lock().unwrap();
        f(&db)
    }

    pub fn complete_task(&self, task_id: i64) -> Result<()> {
        let db = self.db.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        db.execute(
            "UPDATE tasks SET status = 'done', updated_at = ?1 WHERE id = ?2",
            (now, task_id),
        )?;
        Ok(())
    }
}

// Export the exact functions the user requested
pub fn add_task(
    db: &Connection,
    goal: &str,
    description: &str,
    prio: i32,
    parent: Option<i64>,
) -> Result<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    db.execute(
        "INSERT INTO tasks (goal, description, status, priority, parent_id, created_at, updated_at)
         VALUES (?1, ?2, 'open', ?3, ?4, ?5, ?5)",
        (goal, description, prio, parent, now),
    )?;

    Ok(db.last_insert_rowid())
}

pub fn update_task(
    db: &Connection,
    id: i64,
    status: Option<&str>,
    prio: Option<i32>,
    desc: Option<&str>,
) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut query = "UPDATE tasks SET updated_at = ?1".to_string();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now)];
    let mut param_count = 2;

    if let Some(s) = status {
        query.push_str(&format!(", status = ?{}", param_count));
        params.push(Box::new(s.to_string()));
        param_count += 1;
    }

    if let Some(p) = prio {
        query.push_str(&format!(", priority = ?{}", param_count));
        params.push(Box::new(p));
        param_count += 1;
    }

    if let Some(d) = desc {
        query.push_str(&format!(", description = ?{}", param_count));
        params.push(Box::new(d.to_string()));
        param_count += 1;
    }

    query.push_str(&format!(" WHERE id = ?{}", param_count));
    params.push(Box::new(id) as Box<dyn rusqlite::ToSql>);

    db.execute(&query, rusqlite::params_from_iter(params))?;
    Ok(())
}

pub fn next_task(
    db: &Connection,
    statuses: Option<&[&str]>,
    min_prio: Option<i32>,
) -> Result<Option<Task>> {
    let mut query_parts: Vec<String> = vec![
        "SELECT id, goal, description, status, priority, parent_id, created_at, updated_at"
            .to_string(),
        "FROM tasks".to_string(),
        "WHERE status != 'done' AND status != 'cancelled'".to_string(),
    ];
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];
    if let Some(statuses) = statuses {
        let params_list: Vec<String> = statuses
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        query_parts.push(format!("AND status IN ({})", params_list.join(", ")));
        for s in statuses {
            params.push(Box::new(s.to_string()) as Box<dyn rusqlite::ToSql>);
        }
    }

    if let Some(min_prio) = min_prio {
        query_parts.push(format!("AND priority <= ?{}", params.len() + 1));
        params.push(Box::new(min_prio));
    }

    query_parts.push("ORDER BY priority ASC, created_at ASC LIMIT 1".to_string());
    let query = query_parts.join(" ");

    let mut stmt = db.prepare(&query)?;
    let task = stmt
        .query_row(rusqlite::params_from_iter(params), |row| {
            Ok(Task {
                id: row.get(0)?,
                goal: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                parent_id: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .optional()?;

    Ok(task)
}

pub fn link_tasks(db: &Connection, src: i64, dst: i64, kind: &str) -> Result<()> {
    db.execute(
        "INSERT OR REPLACE INTO task_links (src_id, dst_id, kind) VALUES (?1, ?2, ?3)",
        (src, dst, kind),
    )?;
    Ok(())
}

pub fn get_task_links(db: &Connection, task_id: i64, direction: &str) -> Result<Vec<TaskLink>> {
    let (query, _field) = match direction {
        "outgoing" => (
            "SELECT dst_id as linked_id, kind FROM task_links WHERE src_id = ?1",
            "dst_id",
        ),
        "incoming" => (
            "SELECT src_id as linked_id, kind FROM task_links WHERE dst_id = ?1",
            "src_id",
        ),
        "both" => (
            "SELECT 
                CASE WHEN src_id = ?1 THEN dst_id ELSE src_id END as linked_id, 
                kind 
             FROM task_links 
             WHERE src_id = ?1 OR dst_id = ?1",
            "linked_id",
        ),
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid direction: {}. Use 'outgoing', 'incoming', or 'both'",
                direction
            ))
        }
    };

    let mut stmt = db.prepare(query)?;
    let links = stmt
        .query_map([task_id], |row| {
            Ok(TaskLink {
                id: row.get::<_, i64>(0)?,
                kind: row.get::<_, String>(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(links)
}

#[derive(Debug, serde::Serialize)]
pub struct TaskLink {
    pub id: i64,
    pub kind: String,
}
