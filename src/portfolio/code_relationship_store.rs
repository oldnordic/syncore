//! Code Relationship Store
//! Stores code relationships in SQLite, Neo4j, and FAISS for semantic search.

use anyhow::{anyhow, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use neo4rs::{query, Graph};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct CodeRelationshipStore {
    pub db: Arc<Mutex<Connection>>,
    neo4j: Option<Arc<Graph>>,
    embedder: Arc<TextEmbedding>,
    function_vectors: Arc<Mutex<Vec<FunctionVector>>>,
}

#[derive(Clone)]
struct FunctionVector {
    file: String,
    function_name: String,
    embedding: Vec<f32>,
}

impl CodeRelationshipStore {
    pub async fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS code_imports (
                id INTEGER PRIMARY KEY,
                file TEXT NOT NULL,
                imports TEXT NOT NULL,
                UNIQUE(file, imports)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS code_calls (
                id INTEGER PRIMARY KEY,
                file TEXT NOT NULL,
                caller TEXT NOT NULL,
                callee TEXT NOT NULL,
                UNIQUE(file, caller, callee)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS code_impls (
                id INTEGER PRIMARY KEY,
                file TEXT NOT NULL,
                struct_name TEXT NOT NULL,
                trait_name TEXT NOT NULL,
                UNIQUE(file, struct_name, trait_name)
            )",
            [],
        )?;

        // Initialize embedder
        let mut options = InitOptions::default();
        options.model_name = EmbeddingModel::AllMiniLML6V2;
        options.show_download_progress = false;
        let embedder = TextEmbedding::try_new(options)?;

        // Try to connect to Neo4j if available
        let neo4j = match Graph::new("bolt://localhost:7687", "neo4j", "password").await {
            Ok(graph) => Some(Arc::new(graph)),
            Err(_) => None,
        };

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            neo4j,
            embedder: Arc::new(embedder),
            function_vectors: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub async fn store_import(&self, file: &str, import: &str) -> Result<()> {
        let db = self.db.lock().await;
        db.execute(
            "INSERT OR IGNORE INTO code_imports (file, imports) VALUES (?1, ?2)",
            params![file, import],
        )?;
        Ok(())
    }

    pub async fn get_imports(&self, file: &str) -> Result<Vec<String>> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare("SELECT imports FROM code_imports WHERE file = ?1")?;
        let imports = stmt
            .query_map(params![file], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(imports)
    }

    pub async fn get_files_importing(&self, import: &str) -> Result<Vec<String>> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare("SELECT file FROM code_imports WHERE imports = ?1")?;
        let files = stmt
            .query_map(params![import], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(files)
    }

    pub async fn store_call(&self, file: &str, caller: &str, callee: &str) -> Result<()> {
        let db = self.db.lock().await;
        db.execute(
            "INSERT OR IGNORE INTO code_calls (file, caller, callee) VALUES (?1, ?2, ?3)",
            params![file, caller, callee],
        )?;
        Ok(())
    }

    pub async fn get_calls_from(&self, file: &str, caller: &str) -> Result<Vec<String>> {
        let db = self.db.lock().await;
        let mut stmt =
            db.prepare("SELECT callee FROM code_calls WHERE file = ?1 AND caller = ?2")?;
        let callees = stmt
            .query_map(params![file, caller], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(callees)
    }

    pub async fn get_callers_of(&self, callee: &str) -> Result<Vec<(String, String)>> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare("SELECT file, caller FROM code_calls WHERE callee = ?1")?;
        let callers = stmt
            .query_map(params![callee], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<Vec<(String, String)>, _>>()?;
        Ok(callers)
    }

    pub async fn store_impl(&self, file: &str, struct_name: &str, trait_name: &str) -> Result<()> {
        let db = self.db.lock().await;
        db.execute(
            "INSERT OR IGNORE INTO code_impls (file, struct_name, trait_name) VALUES (?1, ?2, ?3)",
            params![file, struct_name, trait_name],
        )?;
        Ok(())
    }

    pub async fn get_impls_for(&self, struct_name: &str) -> Result<Vec<String>> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare("SELECT trait_name FROM code_impls WHERE struct_name = ?1")?;
        let traits = stmt
            .query_map(params![struct_name], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(traits)
    }

    pub async fn sync_to_neo4j(&self) -> Result<()> {
        let graph = self
            .neo4j
            .as_ref()
            .ok_or_else(|| anyhow!("Neo4j not connected"))?;

        // Sync imports
        let imports: Vec<(String, String)> = {
            let db = self.db.lock().await;
            let mut stmt = db.prepare("SELECT file, imports FROM code_imports")?;
            let result = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            result
        };

        for (file, import) in imports {
            let q = query(
                "MERGE (f:File {path: $file})
                 MERGE (i:Module {name: $import})
                 MERGE (f)-[:IMPORTS]->(i)",
            )
            .param("file", file)
            .param("import", import);
            graph.run(q).await?;
        }

        // Sync calls
        let calls: Vec<(String, String)> = {
            let db = self.db.lock().await;
            let mut stmt = db.prepare("SELECT caller, callee FROM code_calls")?;
            let result = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            result
        };

        for (caller, callee) in calls {
            let q = query(
                "MERGE (c:Function {name: $caller})
                 MERGE (e:Function {name: $callee})
                 MERGE (c)-[:CALLS]->(e)",
            )
            .param("caller", caller)
            .param("callee", callee);
            graph.run(q).await?;
        }

        Ok(())
    }

    pub async fn query_neo4j_imports(&self, file: &str) -> Result<Vec<String>> {
        let graph = self
            .neo4j
            .as_ref()
            .ok_or_else(|| anyhow!("Neo4j not connected"))?;

        let q = query(
            "MATCH (f:File {path: $file})-[:IMPORTS]->(i:Module)
             RETURN i.name as name",
        )
        .param("file", file.to_string());

        let mut result = graph.execute(q).await?;
        let mut imports = Vec::new();

        while let Some(row) = result.next().await? {
            if let Ok(name) = row.get::<String>("name") {
                imports.push(name);
            }
        }

        Ok(imports)
    }

    pub async fn query_neo4j_calls(&self, function: &str) -> Result<Vec<String>> {
        let graph = self
            .neo4j
            .as_ref()
            .ok_or_else(|| anyhow!("Neo4j not connected"))?;

        let q = query(
            "MATCH (c:Function {name: $function})-[:CALLS]->(e:Function)
             RETURN e.name as name",
        )
        .param("function", function.to_string());

        let mut result = graph.execute(q).await?;
        let mut calls = Vec::new();

        while let Some(row) = result.next().await? {
            if let Ok(name) = row.get::<String>("name") {
                calls.push(name);
            }
        }

        Ok(calls)
    }

    pub async fn index_function(&self, file: &str, function_name: &str, body: &str) -> Result<()> {
        let embeddings = self.embedder.embed(vec![body], None)?;
        let embedding = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No embedding generated"))?;

        let mut vectors = self.function_vectors.lock().await;
        vectors.push(FunctionVector {
            file: file.to_string(),
            function_name: function_name.to_string(),
            embedding,
        });

        Ok(())
    }

    pub async fn find_similar_functions(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(String, String, f32)>> {
        let query_embedding = self
            .embedder
            .embed(vec![query], None)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No embedding generated"))?;

        let vectors = self.function_vectors.lock().await;

        let mut scored: Vec<(String, String, f32)> = vectors
            .iter()
            .map(|fv| {
                let score = cosine_similarity(&query_embedding, &fv.embedding);
                (fv.file.clone(), fv.function_name.clone(), score)
            })
            .collect();

        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
