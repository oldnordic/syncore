use rusqlite::{Connection, OptionalExtension};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use anyhow::Result;

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

pub struct TaskMaster {
    db: Arc<Mutex<Connection>>,
}

impl TaskMaster {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = crate::db::open_db_with_wal(db_path)?;
        crate::db::ensure_schema(db_path)?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn add_task(&self, goal: &str, description: &str, priority: i32, parent: Option<i64>) -> Result<i64> {
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

    pub fn update_task(&self, id: i64, status: Option<&str>, priority: Option<i32>, desc: Option<&str>) -> Result<()> {
        let db = self.db.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut query = "UPDATE tasks SET updated_at = ?1".to_string();
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&now];
        let mut param_count = 2;

        if let Some(s) = status {
            query.push_str(&format!(", status = ?{}", param_count));
            params.push(s);
            param_count += 1;
        }

        if let Some(p) = priority {
            query.push_str(&format!(", priority = ?{}", param_count));
            params.push(&p);
            param_count += 1;
        }

        if let Some(d) = desc {
            query.push_str(&format!(", description = ?{}", param_count));
            params.push(d);
            param_count += 1;
        }

        query.push_str(&format!(" WHERE id = ?{}", param_count));
        params.push(&id);

        db.execute(&query, rusqlite::params_from_iter(params))?;
        Ok(())
    }

    pub fn next_task(&self, statuses: Option<&[&str]>, min_prio: Option<i32>) -> Result<Option<Task>> {
        let db = self.db.lock().unwrap();

        let mut query = "
            SELECT id, goal, description, status, priority, parent_id, created_at, updated_at
            FROM tasks
            WHERE status != 'done' AND status != 'cancelled'
        ".to_string();

        let mut params: Vec<&dyn rusqlite::ToSql> = vec![];

        if let Some(statuses) = statuses {
            let placeholders: Vec<String> = statuses.iter().enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect();
            query.push_str(&format!(" AND status IN ({})", placeholders.join(", ")));
            params.extend(statuses.iter().map(|s| s as &dyn rusqlite::ToSql));
        }

        if let Some(min_prio) = min_prio {
            query.push_str(&format!(" AND priority <= ?{}", params.len() + 1));
            params.push(&min_prio);
        }

        query.push_str(" ORDER BY priority ASC, created_at ASC LIMIT 1");

        let mut stmt = db.prepare(&query)?;
        let task = stmt.query_row(rusqlite::params_from_iter(params), |row| {
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
        }).optional()?;

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
        let task = db.query_row(
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
        ).optional()?;

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
}

// Export the exact functions the user requested
pub fn add_task(db: &Connection, goal: &str, description: &str, prio: i32, parent: Option<i64>) -> Result<i64> {
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

pub fn update_task(db: &Connection, id: i64, status: Option<&str>, prio: Option<i32>, desc: Option<&str>) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut query = "UPDATE tasks SET updated_at = ?1".to_string();
    let mut params: Vec<&dyn rusqlite::ToSql> = vec![&now];
    let mut param_count = 2;

    if let Some(s) = status {
        query.push_str(&format!(", status = ?{}", param_count));
        params.push(&s);
        param_count += 1;
    }

    if let Some(p) = prio {
        query.push_str(&format!(", priority = ?{}", param_count));
        params.push(&p);
        param_count += 1;
    }

    if let Some(d) = desc {
        query.push_str(&format!(", description = ?{}", param_count));
        params.push(&d);
        param_count += 1;
    }

    query.push_str(&format!(" WHERE id = ?{}", param_count));
    params.push(&id);

    db.execute(&query, rusqlite::params_from_iter(params))?;
    Ok(())
}

pub fn next_task(db: &Connection, statuses: Option<&[&str]>, min_prio: Option<i32>) -> Result<Option<Task>> {
    let mut query = "
        SELECT id, goal, description, status, priority, parent_id, created_at, updated_at
        FROM tasks
        WHERE status != 'done' AND status != 'cancelled'
    ".to_string();

    let mut params: Vec<&dyn rusqlite::ToSql> = vec![];

    if let Some(statuses) = statuses {
        let placeholders: Vec<String> = statuses.iter().enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        query.push_str(&format!(" AND status IN ({})", placeholders.join(", ")));
        params.extend(statuses.iter().map(|s| s as &dyn rusqlite::ToSql));
    }

    if let Some(min_prio) = min_prio {
        query.push_str(&format!(" AND priority <= ?{}", params.len() + 1));
        params.push(&min_prio);
    }

    query.push_str(" ORDER BY priority ASC, created_at ASC LIMIT 1");

    let mut stmt = db.prepare(&query)?;
    let task = stmt.query_row(rusqlite::params_from_iter(params), |row| {
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
    }).optional()?;

    Ok(task)
}

pub fn link_tasks(db: &Connection, src: i64, dst: i64, kind: &str) -> Result<()> {
    db.execute(
        "INSERT OR REPLACE INTO task_links (src_id, dst_id, kind) VALUES (?1, ?2, ?3)",
        (src, dst, kind),
    )?;
    Ok(())
}
