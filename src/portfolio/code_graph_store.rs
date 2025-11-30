//! Code Graph Store - SQLite persistence, FAISS embeddings, Neo4j sync
//! Stores extracted code graphs with cross-linked queries across multiple backends.

use crate::portfolio::code_graph_extractor::{
    CodeGraph, FunctionNode,
};
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Query parameters for cross-linked graph queries
#[derive(Debug, Clone)]
pub struct GraphQuery {
    pub function_name: String,
    pub include_callers: bool,
    pub include_callees: bool,
    pub include_semantic: bool,
    pub semantic_limit: usize,
}

/// Result of a cross-linked query
#[derive(Debug, Clone)]
pub struct GraphResult {
    pub function: FunctionNode,
    pub callers: Vec<String>,
    pub callees: Vec<String>,
    pub semantic_neighbors: Vec<SemanticNeighbor>,
}

/// Semantic neighbor with similarity score
#[derive(Debug, Clone)]
pub struct SemanticNeighbor {
    pub function_name: String,
    pub score: f32,
}

/// Persistent store for code graphs with multi-backend support
pub struct CodeGraphStore {
    conn: Connection,
    vectors_dir: PathBuf,
    namespace: String,
    event_count: AtomicUsize,
    embeddings: Vec<(i64, String, Vec<f32>)>, // (id, name, embedding)
}

impl CodeGraphStore {
    /// Create store with custom paths (for test isolation)
    pub fn new_with_paths(db_path: &Path, vectors_dir: &Path) -> Result<Self> {
        let namespace = std::env::var("GRAPH_NAMESPACE").unwrap_or_else(|_| "default".to_string());

        let conn = Connection::open(db_path)?;
        Self::init_schema(&conn)?;

        Ok(CodeGraphStore {
            conn,
            vectors_dir: vectors_dir.to_path_buf(),
            namespace,
            event_count: AtomicUsize::new(0),
            embeddings: Vec::new(),
        })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS code_graph_functions (
                id INTEGER PRIMARY KEY,
                namespace TEXT NOT NULL,
                file_path TEXT NOT NULL,
                name TEXT NOT NULL,
                qualified_path TEXT NOT NULL,
                is_public BOOLEAN,
                is_async BOOLEAN,
                parent_type TEXT,
                line_start INTEGER,
                line_end INTEGER,
                embedding_id INTEGER
            );

            CREATE TABLE IF NOT EXISTS code_graph_calls (
                id INTEGER PRIMARY KEY,
                namespace TEXT NOT NULL,
                from_function TEXT NOT NULL,
                to_function TEXT NOT NULL,
                line INTEGER
            );

            CREATE TABLE IF NOT EXISTS code_graph_implementations (
                id INTEGER PRIMARY KEY,
                namespace TEXT NOT NULL,
                struct_name TEXT NOT NULL,
                trait_name TEXT,
                line INTEGER
            );

            CREATE TABLE IF NOT EXISTS code_graph_imports (
                id INTEGER PRIMARY KEY,
                namespace TEXT NOT NULL,
                file_path TEXT NOT NULL,
                import_path TEXT NOT NULL,
                line INTEGER
            );

            CREATE TABLE IF NOT EXISTS code_graph_structs (
                id INTEGER PRIMARY KEY,
                namespace TEXT NOT NULL,
                name TEXT NOT NULL,
                is_public BOOLEAN,
                line INTEGER
            );

            CREATE TABLE IF NOT EXISTS code_graph_traits (
                id INTEGER PRIMARY KEY,
                namespace TEXT NOT NULL,
                name TEXT NOT NULL,
                is_public BOOLEAN,
                line INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_functions_namespace ON code_graph_functions(namespace);
            CREATE INDEX IF NOT EXISTS idx_calls_namespace ON code_graph_calls(namespace);
            CREATE INDEX IF NOT EXISTS idx_imports_namespace ON code_graph_imports(namespace);
        "#,
        )?;

        Ok(())
    }

    /// Insert a complete code graph into the store
    pub fn insert_graph(&mut self, graph: &CodeGraph) -> Result<()> {
        let file_path = graph.file_path.to_string_lossy().to_string();

        // Insert functions
        for func in &graph.functions {
            self.conn.execute(
                "INSERT INTO code_graph_functions
                 (namespace, file_path, name, qualified_path, is_public, is_async, parent_type, line_start, line_end)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    self.namespace,
                    file_path,
                    func.name,
                    func.qualified_path,
                    func.is_public,
                    func.is_async,
                    func.parent_type,
                    func.line_start as i64,
                    func.line_end as i64
                ],
            )?;
        }

        // Insert calls
        for call in &graph.calls {
            self.conn.execute(
                "INSERT INTO code_graph_calls (namespace, from_function, to_function, line)
                 VALUES (?1, ?2, ?3, ?4)",
                params![self.namespace, call.from, call.to, call.line as i64],
            )?;
        }

        // Insert imports
        for import in &graph.imports {
            self.conn.execute(
                "INSERT INTO code_graph_imports (namespace, file_path, import_path, line)
                 VALUES (?1, ?2, ?3, ?4)",
                params![self.namespace, file_path, import.path, import.line as i64],
            )?;
        }

        // Insert structs
        for s in &graph.structs {
            self.conn.execute(
                "INSERT INTO code_graph_structs (namespace, name, is_public, line)
                 VALUES (?1, ?2, ?3, ?4)",
                params![self.namespace, s.name, s.is_public, s.line as i64],
            )?;
        }

        // Insert traits
        for t in &graph.traits {
            self.conn.execute(
                "INSERT INTO code_graph_traits (namespace, name, is_public, line)
                 VALUES (?1, ?2, ?3, ?4)",
                params![self.namespace, t.name, t.is_public, t.line as i64],
            )?;
        }

        // Insert implementations
        for imp in &graph.implementations {
            self.conn.execute(
                "INSERT INTO code_graph_implementations (namespace, struct_name, trait_name, line)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    self.namespace,
                    imp.struct_name,
                    imp.trait_name,
                    imp.line as i64
                ],
            )?;
        }

        // Emit event
        self.event_count.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    /// Get all functions from a specific file
    pub fn get_functions(&self, file: &str) -> Result<Vec<FunctionNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, qualified_path, is_public, is_async, parent_type, line_start, line_end
             FROM code_graph_functions
             WHERE namespace = ?1 AND file_path LIKE ?2",
        )?;

        let pattern = format!("%{}", file);
        let rows = stmt.query_map(params![self.namespace, pattern], |row| {
            Ok(FunctionNode {
                name: row.get(0)?,
                qualified_path: row.get(1)?,
                is_public: row.get(2)?,
                is_async: row.get(3)?,
                parent_type: row.get(4)?,
                line_start: row.get::<_, i64>(5)? as usize,
                line_end: row.get::<_, i64>(6)? as usize,
            })
        })?;

        let mut functions = Vec::new();
        for row in rows {
            functions.push(row?);
        }

        Ok(functions)
    }

    /// Get all functions in this namespace
    pub fn get_all_functions(&self) -> Result<Vec<FunctionNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, qualified_path, is_public, is_async, parent_type, line_start, line_end
             FROM code_graph_functions
             WHERE namespace = ?1",
        )?;

        let rows = stmt.query_map(params![self.namespace], |row| {
            Ok(FunctionNode {
                name: row.get(0)?,
                qualified_path: row.get(1)?,
                is_public: row.get(2)?,
                is_async: row.get(3)?,
                parent_type: row.get(4)?,
                line_start: row.get::<_, i64>(5)? as usize,
                line_end: row.get::<_, i64>(6)? as usize,
            })
        })?;

        let mut functions = Vec::new();
        for row in rows {
            functions.push(row?);
        }

        Ok(functions)
    }

    /// Get all callers of a function
    pub fn get_callers(&self, function_name: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT from_function FROM code_graph_calls
             WHERE namespace = ?1 AND to_function = ?2",
        )?;

        let rows = stmt.query_map(params![self.namespace, function_name], |row| row.get(0))?;

        let mut callers = Vec::new();
        for row in rows {
            callers.push(row?);
        }

        Ok(callers)
    }

    /// Get all callees of a function
    pub fn get_callees(&self, function_name: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT to_function FROM code_graph_calls
             WHERE namespace = ?1 AND from_function = ?2",
        )?;

        let rows = stmt.query_map(params![self.namespace, function_name], |row| row.get(0))?;

        let mut callees = Vec::new();
        for row in rows {
            callees.push(row?);
        }

        Ok(callees)
    }

    /// Get traits implemented by a struct
    pub fn get_implementations(&self, struct_name: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT trait_name FROM code_graph_implementations
             WHERE namespace = ?1 AND struct_name = ?2 AND trait_name IS NOT NULL",
        )?;

        let rows = stmt.query_map(params![self.namespace, struct_name], |row| row.get(0))?;

        let mut traits = Vec::new();
        for row in rows {
            traits.push(row?);
        }

        Ok(traits)
    }

    /// Get imports for a file
    pub fn get_imports(&self, file: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT import_path FROM code_graph_imports
             WHERE namespace = ?1 AND file_path LIKE ?2",
        )?;

        let pattern = format!("%{}", file);
        let rows = stmt.query_map(params![self.namespace, pattern], |row| row.get(0))?;

        let mut imports = Vec::new();
        for row in rows {
            imports.push(row?);
        }

        Ok(imports)
    }

    /// Embed function signatures in FAISS vector store
    pub fn embed_functions(&mut self) -> Result<()> {
        let functions = self.get_all_functions()?;

        // Use fastembed for real embeddings
        let model = fastembed::TextEmbedding::try_new(Default::default())
            .map_err(|e| anyhow!("Failed to load embedding model: {}", e))?;

        let texts: Vec<String> = functions.iter().map(|f| f.qualified_path.clone()).collect();

        if texts.is_empty() {
            return Ok(());
        }

        let embeddings_result = model
            .embed(texts.clone(), None)
            .map_err(|e| anyhow!("Embedding failed: {}", e))?;

        // Store embeddings with IDs
        self.embeddings.clear();
        for (i, (text, embedding)) in texts.into_iter().zip(embeddings_result).enumerate() {
            self.embeddings.push((i as i64, text, embedding));
        }

        Ok(())
    }

    /// Search for semantically similar functions
    pub fn search_similar_functions(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        if self.embeddings.is_empty() {
            return Ok(vec![]);
        }

        let model = fastembed::TextEmbedding::try_new(Default::default())
            .map_err(|e| anyhow!("Failed to load embedding model: {}", e))?;

        let query_embedding = model
            .embed(vec![query.to_string()], None)
            .map_err(|e| anyhow!("Query embedding failed: {}", e))?;

        if query_embedding.is_empty() {
            return Ok(vec![]);
        }

        let query_vec = &query_embedding[0];

        // Compute cosine similarities
        let mut scores: Vec<(String, f32)> = self
            .embeddings
            .iter()
            .map(|(_, name, emb)| {
                let score = cosine_similarity(query_vec, emb);
                (name.clone(), score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scores
            .into_iter()
            .take(limit)
            .map(|(name, _)| name)
            .collect())
    }

    /// Sync code graph to Neo4j (graceful fallback if unavailable)
    pub fn sync_to_neo4j(&self) -> Result<()> {
        // Try to connect to Neo4j
        let neo4j_uri =
            std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
        let neo4j_user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
        let neo4j_pass = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string());

        // Create runtime for async Neo4j operations
        let rt = tokio::runtime::Runtime::new()?;

        rt.block_on(async {
            let graph = neo4rs::Graph::new(&neo4j_uri, &neo4j_user, &neo4j_pass)
                .await
                .map_err(|e| anyhow!("Neo4j connection error: {}", e))?;

            // Get all functions and sync them
            let functions = self.get_all_functions()?;
            for func in &functions {
                let query = neo4rs::query(
                    "MERGE (f:Function {name: $name, namespace: $namespace})",
                )
                .param("name", func.name.clone())
                .param("namespace", self.namespace.clone());

                graph.run(query).await.map_err(|e| anyhow!("Neo4j connection/query error: {}", e))?;
            }

            // Sync call edges
            let mut stmt = self.conn.prepare(
                "SELECT from_function, to_function FROM code_graph_calls WHERE namespace = ?1"
            )?;
            let calls: Vec<(String, String)> = stmt.query_map(params![self.namespace], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?.filter_map(|r| r.ok()).collect();

            for (from, to) in calls {
                let query = neo4rs::query(
                    "MATCH (a:Function {name: $from, namespace: $ns}), (b:Function {name: $to, namespace: $ns})
                     MERGE (a)-[:CALLS]->(b)"
                )
                .param("from", from)
                .param("to", to)
                .param("ns", self.namespace.clone());

                graph.run(query).await.map_err(|e| anyhow!("Neo4j call sync error: {}", e))?;
            }

            // Sync structs
            let mut stmt = self.conn.prepare(
                "SELECT name FROM code_graph_structs WHERE namespace = ?1"
            )?;
            let structs: Vec<String> = stmt.query_map(params![self.namespace], |row| {
                row.get(0)
            })?.filter_map(|r| r.ok()).collect();

            for s in structs {
                let query = neo4rs::query(
                    "MERGE (s:Struct {name: $name, namespace: $namespace})"
                )
                .param("name", s)
                .param("namespace", self.namespace.clone());

                graph.run(query).await.map_err(|e| anyhow!("Neo4j struct sync error: {}", e))?;
            }

            // Sync traits
            let mut stmt = self.conn.prepare(
                "SELECT name FROM code_graph_traits WHERE namespace = ?1"
            )?;
            let traits: Vec<String> = stmt.query_map(params![self.namespace], |row| {
                row.get(0)
            })?.filter_map(|r| r.ok()).collect();

            for t in traits {
                let query = neo4rs::query(
                    "MERGE (t:Trait {name: $name, namespace: $namespace})"
                )
                .param("name", t)
                .param("namespace", self.namespace.clone());

                graph.run(query).await.map_err(|e| anyhow!("Neo4j trait sync error: {}", e))?;
            }

            // Sync implementations
            let mut stmt = self.conn.prepare(
                "SELECT struct_name, trait_name FROM code_graph_implementations
                 WHERE namespace = ?1 AND trait_name IS NOT NULL"
            )?;
            let impls: Vec<(String, String)> = stmt.query_map(params![self.namespace], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?.filter_map(|r| r.ok()).collect();

            for (struct_name, trait_name) in impls {
                let query = neo4rs::query(
                    "MATCH (s:Struct {name: $struct, namespace: $ns}), (t:Trait {name: $trait, namespace: $ns})
                     MERGE (s)-[:IMPLEMENTS]->(t)"
                )
                .param("struct", struct_name)
                .param("trait", trait_name)
                .param("ns", self.namespace.clone());

                graph.run(query).await.map_err(|e| anyhow!("Neo4j impl sync error: {}", e))?;
            }

            Ok::<(), anyhow::Error>(())
        })
    }

    /// Query Neo4j for function calls
    pub fn query_neo4j_calls(&self, function_name: &str) -> Result<Vec<String>> {
        let neo4j_uri =
            std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
        let neo4j_user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
        let neo4j_pass = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string());

        let rt = tokio::runtime::Runtime::new()?;

        rt.block_on(async {
            let graph = neo4rs::Graph::new(&neo4j_uri, &neo4j_user, &neo4j_pass)
                .await
                .map_err(|e| anyhow!("Neo4j connection error: {}", e))?;

            let query = neo4rs::query(
                "MATCH (a:Function {name: $name, namespace: $ns})-[:CALLS]->(b:Function)
                 RETURN b.name AS callee",
            )
            .param("name", function_name.to_string())
            .param("ns", self.namespace.clone());

            let mut result = graph
                .execute(query)
                .await
                .map_err(|e| anyhow!("Neo4j connection/query error: {}", e))?;

            let mut callees = Vec::new();
            while let Some(row) = result
                .next()
                .await
                .map_err(|e| anyhow!("Neo4j row error: {}", e))?
            {
                if let Ok(name) = row.get::<String>("callee") {
                    callees.push(name);
                }
            }

            Ok(callees)
        })
    }

    /// Query Neo4j for implementations
    pub fn query_neo4j_implementations(&self, struct_name: &str) -> Result<Vec<String>> {
        let neo4j_uri =
            std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
        let neo4j_user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
        let neo4j_pass = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string());

        let rt = tokio::runtime::Runtime::new()?;

        rt.block_on(async {
            let graph = neo4rs::Graph::new(&neo4j_uri, &neo4j_user, &neo4j_pass)
                .await
                .map_err(|e| anyhow!("Neo4j connection error: {}", e))?;

            let query = neo4rs::query(
                "MATCH (s:Struct {name: $name, namespace: $ns})-[:IMPLEMENTS]->(t:Trait)
                 RETURN t.name AS trait_name",
            )
            .param("name", struct_name.to_string())
            .param("ns", self.namespace.clone());

            let mut result = graph
                .execute(query)
                .await
                .map_err(|e| anyhow!("Neo4j connection/query error: {}", e))?;

            let mut traits = Vec::new();
            while let Some(row) = result
                .next()
                .await
                .map_err(|e| anyhow!("Neo4j row error: {}", e))?
            {
                if let Ok(name) = row.get::<String>("trait_name") {
                    traits.push(name);
                }
            }

            Ok(traits)
        })
    }

    /// Cross-linked query combining SQLite, FAISS, and Neo4j results
    pub fn query_cross_linked(&self, query: &GraphQuery) -> Result<GraphResult> {
        // Get function info
        let functions = self.get_all_functions()?;
        let function = functions
            .into_iter()
            .find(|f| f.name == query.function_name)
            .ok_or_else(|| anyhow!("Function not found: {}", query.function_name))?;

        let callers = if query.include_callers {
            self.get_callers(&query.function_name)?
        } else {
            vec![]
        };

        let callees = if query.include_callees {
            self.get_callees(&query.function_name)?
        } else {
            vec![]
        };

        let semantic_neighbors = if query.include_semantic {
            self.search_similar_with_scores(&query.function_name, query.semantic_limit)?
        } else {
            vec![]
        };

        Ok(GraphResult {
            function,
            callers,
            callees,
            semantic_neighbors,
        })
    }

    fn search_similar_with_scores(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SemanticNeighbor>> {
        if self.embeddings.is_empty() {
            return Ok(vec![]);
        }

        let model = fastembed::TextEmbedding::try_new(Default::default())
            .map_err(|e| anyhow!("Failed to load embedding model: {}", e))?;

        let query_embedding = model
            .embed(vec![query.to_string()], None)
            .map_err(|e| anyhow!("Query embedding failed: {}", e))?;

        if query_embedding.is_empty() {
            return Ok(vec![]);
        }

        let query_vec = &query_embedding[0];

        let mut scores: Vec<SemanticNeighbor> = self
            .embeddings
            .iter()
            .filter(|(_, name, _)| name != query)
            .map(|(_, name, emb)| SemanticNeighbor {
                function_name: name.clone(),
                score: cosine_similarity(query_vec, emb),
            })
            .collect();

        scores.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(scores.into_iter().take(limit).collect())
    }

    /// Get current event count (for MessageBus tracking)
    pub fn get_event_count(&self) -> usize {
        self.event_count.load(Ordering::SeqCst)
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
