//! Application Structure Mapping Tool
//!
//! Maps and tracks application structure including:
//! - File nodes with metadata (imports, exports, dependencies)
//! - Dependency graph traversal
//! - Semantic search for related files
//!
//! Integrates with MessageBus, SQLite, and FAISS

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::time::SystemTime;

use crate::message_bus::message::{AgentId, Msg, MsgKind};
use crate::path_resolver::PathResolver; // APEX v1.7 Phase 6
use crate::router::SynCoreState;
use crate::vector::SearchScope;

/// File node representing a single file in the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub path: String,
    pub kind: String, // "file", "directory", "module"
    pub language: Option<String>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub dependencies: Vec<String>, // paths of files this depends on
}

/// Application structure mapping tool
#[derive(Clone)]
pub struct MappingTool {
    state: SynCoreState,
}

impl MappingTool {
    /// Create a new mapping tool with shared state
    pub fn new(state: SynCoreState) -> Self {
        // Ensure schema exists
        Self::initialize_schema(&state).expect("Failed to initialize mapping schema");
        Self { state }
    }

    /// Initialize SQLite schema for file mapping
    fn initialize_schema(state: &SynCoreState) -> Result<()> {
        state.tasks.with_db(|conn| {
            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS file_nodes (
                    path TEXT PRIMARY KEY,
                    kind TEXT NOT NULL,
                    language TEXT,
                    imports TEXT NOT NULL,  -- JSON array
                    exports TEXT NOT NULL,  -- JSON array
                    dependencies TEXT NOT NULL, -- JSON array
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                )
                "#,
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_file_nodes_language ON file_nodes(language)",
                [],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    /// Record a file node in SQLite and FAISS
    pub fn record_file(&self, node: &FileNode) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs() as i64;

        // Persist to SQLite
        self.state.tasks.with_db(|conn| {
            conn.execute(
                r#"
                INSERT INTO file_nodes (path, kind, language, imports, exports, dependencies, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
                ON CONFLICT(path) DO UPDATE SET
                    kind = excluded.kind,
                    language = excluded.language,
                    imports = excluded.imports,
                    exports = excluded.exports,
                    dependencies = excluded.dependencies,
                    updated_at = excluded.updated_at
                "#,
                rusqlite::params![
                    node.path,
                    node.kind,
                    node.language,
                    serde_json::to_string(&node.imports)?,
                    serde_json::to_string(&node.exports)?,
                    serde_json::to_string(&node.dependencies)?,
                    now,
                ],
            )?;
            Ok(())
        })?;

        // Index in FAISS for semantic search
        let description = format!(
            "{} {} imports:{} exports:{} deps:{}",
            node.path,
            node.language.as_deref().unwrap_or("unknown"),
            node.imports.join(","),
            node.exports.join(","),
            node.dependencies.join(",")
        );
        {
            let mut store = self.state.general_store.lock().unwrap();
            // Use path hash as ID
            let id = self.path_to_id(&node.path);
            store.insert_text(id, None, &description, "file_mapping")?;
        }

        // Neo4j integration: Use canonical neo4j module for File dependency tracking
        if let Some(neo4j) = &self.state.neo4j {
            use crate::databases::neo4j::{create_file_dependency, upsert_file_by_path};

            let neo4j = neo4j.clone();
            let path = node.path.clone();
            let deps = node.dependencies.clone();

            // Spawn async task for Neo4j operations
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async move {
                    // Create File node
                    let _ = upsert_file_by_path(&neo4j, &path).await;

                    // Create DEPENDS_ON relationships for each dependency
                    for dep in deps {
                        let _ = create_file_dependency(&neo4j, &path, &dep).await;
                    }
                })
            });
        }

        // Broadcast event via MessageBus
        if let Some(bus) = &self.state.message_bus {
            let msg = Msg {
                id: bus.next_message_id(),
                from: AgentId::Internal("mapping_tool".into()),
                to: None, // Broadcast
                kind: MsgKind::Event("mapping_update".to_string()),
                payload: serde_json::json!({
                    "path": node.path,
                    "kind": node.kind,
                    "language": node.language,
                    "imports_count": node.imports.len(),
                    "exports_count": node.exports.len(),
                    "dependencies_count": node.dependencies.len(),
                }),
                timestamp: SystemTime::now(),
            };
            bus.send(msg);
        }

        Ok(())
    }

    /// Get a file node by path
    pub fn get_file(&self, path: &str) -> Result<Option<FileNode>> {
        self.state.tasks.with_db(|conn| {
            let mut stmt = conn.prepare(
                "SELECT path, kind, language, imports, exports, dependencies FROM file_nodes WHERE path = ?1"
            )?;

            let node = stmt.query_row([path], |row| {
                let imports_json: String = row.get(3)?;
                let exports_json: String = row.get(4)?;
                let deps_json: String = row.get(5)?;

                Ok(FileNode {
                    path: row.get(0)?,
                    kind: row.get(1)?,
                    language: row.get(2)?,
                    imports: serde_json::from_str(&imports_json).unwrap_or_default(),
                    exports: serde_json::from_str(&exports_json).unwrap_or_default(),
                    dependencies: serde_json::from_str(&deps_json).unwrap_or_default(),
                })
            });

            match node {
                Ok(n) => Ok(Some(n)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.into()),
            }
        })
    }

    /// Search for files related to a query using semantic search
    pub fn search_related(&self, query: &str) -> Result<Vec<FileNode>> {
        let results = {
            let store = self.state.general_store.lock().unwrap();
            store.search(query, 10, SearchScope::Global)?
        };

        let mut nodes = Vec::new();
        for result in results {
            // Extract path from result (stored in metadata or reconstructed)
            if let Some(path) = self.id_to_path(result.id) {
                if let Some(node) = self.get_file(&path)? {
                    nodes.push(node);
                }
            }
        }

        Ok(nodes)
    }

    /// Get all transitive dependencies for a file
    ///
    /// Falls back to parsing Rust source files when file_nodes table is empty.
    pub fn get_all_dependencies(&self, path: &str) -> Result<Vec<String>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Try file_nodes table first
        if let Some(node) = self.get_file(path)? {
            if !node.dependencies.is_empty() {
                for dep in &node.dependencies {
                    if !visited.contains(dep) {
                        queue.push_back(dep.clone());
                        visited.insert(dep.clone());
                    }
                }
            }
        }

        // If no dependencies found, fall back to parsing Rust imports
        if queue.is_empty() && path.ends_with(".rs") {
            let deps = self.extract_rust_dependencies(path)?;
            for dep in deps {
                if !visited.contains(&dep) {
                    queue.push_back(dep.clone());
                    visited.insert(dep);
                }
            }
        }

        // BFS to find all transitive dependencies
        while let Some(current) = queue.pop_front() {
            result.push(current.clone());

            // Try file_nodes first
            if let Some(node) = self.get_file(&current)? {
                for dep in &node.dependencies {
                    if !visited.contains(dep) {
                        queue.push_back(dep.clone());
                        visited.insert(dep.clone());
                    }
                }
            } else if current.ends_with(".rs") {
                // Fall back to parsing
                let deps = self.extract_rust_dependencies(&current)?;
                for dep in deps {
                    if !visited.contains(&dep) {
                        queue.push_back(dep.clone());
                        visited.insert(dep);
                    }
                }
            }
        }

        Ok(result)
    }

    /// Extract Rust dependencies by parsing use statements
    fn extract_rust_dependencies(&self, path: &str) -> Result<Vec<String>> {
        use crate::macro_tools::import_extractor::{extract_rust_imports, resolve_import_to_file};

        // Check if file exists first
        if !std::path::Path::new(path).exists() {
            // Return empty vec for non-existent files (graceful handling)
            return Ok(Vec::new());
        }

        // Read the file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Ok(Vec::new()), // Gracefully return empty on read error
        };

        // Extract imports
        let imports = extract_rust_imports(&content);

        // APEX v1.7 Phase 6: Use PathResolver instead of current_dir()
        let mut resolver = PathResolver::new();
        let project_root = resolver
            .resolve_workspace_root(Path::new(path))
            .ok()
            .flatten()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        let mut dependencies = Vec::new();
        for import in imports {
            if let Some(resolved) = resolve_import_to_file(&import.path, path, &project_root) {
                dependencies.push(resolved);
            }
        }

        Ok(dependencies)
    }

    /// Convert path to numeric ID for FAISS
    fn path_to_id(&self, path: &str) -> i64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        (hasher.finish() & 0x7FFFFFFFFFFFFFFF) as i64
    }

    /// Convert ID back to path (from SQLite lookup)
    fn id_to_path(&self, id: i64) -> Option<String> {
        // Query all paths and find matching hash
        let paths: Vec<String> = self
            .state
            .tasks
            .with_db(|conn| {
                let mut stmt = conn.prepare("SELECT path FROM file_nodes")?;
                let paths: Result<Vec<String>, _> = stmt.query_map([], |row| row.get(0))?.collect();
                paths.map_err(|e| anyhow::anyhow!("Failed to get paths: {}", e))
            })
            .unwrap_or_default();

        paths.into_iter().find(|p| self.path_to_id(p) == id)
    }
}
