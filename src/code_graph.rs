use anyhow::{Result, anyhow};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::path::Path;
use crate::parser::{Parser, FunctionInfo};
use crate::vector::{VectorStore, SearchScope};

/// Represents a code entity (function, class, import, etc.)
#[derive(Debug, Clone)]
pub struct CodeEntity {
    pub id: Option<i64>,
    pub file_path: String,
    pub entity_type: EntityType,
    pub name: String,
    pub signature: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
    pub docstring: Option<String>,
    pub language: String,
}

/// Types of code entities we can extract
#[derive(Debug, Clone, PartialEq)]
pub enum EntityType {
    Function,
    Class,
    Method,
    Import,
    Struct,
    Enum,
    Trait,
}

impl EntityType {
    pub fn as_str(&self) -> &str {
        match self {
            EntityType::Function => "function",
            EntityType::Class => "class",
            EntityType::Method => "method",
            EntityType::Import => "import",
            EntityType::Struct => "struct",
            EntityType::Enum => "enum",
            EntityType::Trait => "trait",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "function" => Some(EntityType::Function),
            "class" => Some(EntityType::Class),
            "method" => Some(EntityType::Method),
            "import" => Some(EntityType::Import),
            "struct" => Some(EntityType::Struct),
            "enum" => Some(EntityType::Enum),
            "trait" => Some(EntityType::Trait),
            _ => None,
        }
    }
}

/// Represents a relationship between code entities
#[derive(Debug, Clone)]
pub struct CodeEdge {
    pub src_entity_id: i64,
    pub dst_entity_id: i64,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EdgeType {
    Calls,
    Imports,
    Inherits,
    References,
    Uses,
    Contains,
}

impl EdgeType {
    pub fn as_str(&self) -> &str {
        match self {
            EdgeType::Calls => "calls",
            EdgeType::Imports => "imports",
            EdgeType::Inherits => "inherits",
            EdgeType::References => "references",
            EdgeType::Uses => "uses",
            EdgeType::Contains => "contains",
        }
    }
}

/// Result from code search combining semantic and structural information
#[derive(Debug, Clone)]
pub struct CodeMatch {
    pub entity: CodeEntity,
    pub score: f32,
    pub match_type: MatchType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchType {
    Semantic,    // Found via vector similarity
    Structural,  // Found via graph traversal
    Combined,    // Found via both methods
}

/// Main code graph structure for indexing and searching code
pub struct CodeGraph {
    db: Arc<Mutex<Connection>>,
    vector_store: Arc<Mutex<VectorStore>>,
    parser: Parser,
}

impl CodeGraph {
    /// Create a new CodeGraph instance
    pub fn new(db_path: &str, vector_store: Arc<Mutex<VectorStore>>) -> Result<Self> {
        // Ensure schema exists (both core and code_graph tables)
        crate::db::ensure_schema(db_path)?;

        // Open database with WAL mode
        let db = crate::db::open_db_with_wal(db_path)?;

        // Double-check that code_graph schema exists (for test environments)
        // This is a safety net in case include_str! paths don't work in tests
        Self::ensure_code_graph_schema(&db)?;

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            vector_store,
            parser: Parser::new()?,
        })
    }

    /// Ensure code_graph schema exists (fallback for test environments)
    fn ensure_code_graph_schema(db: &Connection) -> Result<()> {
        // Check if code_entities table exists
        let mut stmt = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='code_entities'")?;
        let has_table = stmt.exists([])?;

        if !has_table {
            // Create code_graph schema inline
            db.execute_batch(r#"
                PRAGMA foreign_keys=ON;

                CREATE TABLE IF NOT EXISTS code_entities (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    file_path TEXT NOT NULL,
                    entity_type TEXT NOT NULL,
                    name TEXT NOT NULL,
                    signature TEXT,
                    line_start INTEGER NOT NULL,
                    line_end INTEGER NOT NULL,
                    docstring TEXT,
                    language TEXT NOT NULL,
                    indexed_at INTEGER NOT NULL,
                    UNIQUE(file_path, entity_type, name, line_start)
                );

                CREATE INDEX IF NOT EXISTS idx_entities_name ON code_entities(name);
                CREATE INDEX IF NOT EXISTS idx_entities_file ON code_entities(file_path);
                CREATE INDEX IF NOT EXISTS idx_entities_type ON code_entities(entity_type);
                CREATE INDEX IF NOT EXISTS idx_entities_lang ON code_entities(language);

                CREATE TABLE IF NOT EXISTS code_edges (
                    src_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
                    dst_entity_id INTEGER NOT NULL REFERENCES code_entities(id) ON DELETE CASCADE,
                    edge_type TEXT NOT NULL,
                    PRIMARY KEY (src_entity_id, dst_entity_id, edge_type)
                );

                CREATE INDEX IF NOT EXISTS idx_edges_src ON code_edges(src_entity_id);
                CREATE INDEX IF NOT EXISTS idx_edges_dst ON code_edges(dst_entity_id);
                CREATE INDEX IF NOT EXISTS idx_edges_type ON code_edges(edge_type);

                CREATE TABLE IF NOT EXISTS code_embeddings (
                    entity_id INTEGER PRIMARY KEY REFERENCES code_entities(id) ON DELETE CASCADE,
                    vector_id INTEGER NOT NULL,
                    model_version TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_code_embeddings_vector ON code_embeddings(vector_id);
            "#)?;
        }

        Ok(())
    }

    /// Index a source file, extracting entities and creating embeddings
    ///
    /// This is the core indexing operation that:
    /// 1. Parses the file with tree-sitter
    /// 2. Extracts entities (functions, classes, etc.)
    /// 3. Stores entities in database
    /// 4. Creates embeddings for semantic search
    /// 5. Builds graph edges (relationships)
    ///
    /// If the file is already indexed, old entries are deleted first to allow re-indexing.
    ///
    /// Returns the number of entities indexed
    pub fn index_file(&mut self, file_path: &Path) -> Result<usize> {
        // Parse the file
        let code_structure = self.parser.parse_file(file_path)?;

        let mut entities_indexed = 0;

        // Extract and store entities
        let db = self.db.lock().map_err(|e| anyhow!("Failed to lock database: {}", e))?;

        // Start transaction for atomic indexing
        db.execute("BEGIN TRANSACTION", [])?;

        // Delete existing entities for this file to allow re-indexing
        let file_path_str = code_structure.file_path.clone();
        db.execute(
            "DELETE FROM code_entities WHERE file_path = ?",
            [&file_path_str],
        )?;

        // Store functions
        for func in &code_structure.functions {
            let entity = CodeEntity {
                id: None,
                file_path: code_structure.file_path.clone(),
                entity_type: EntityType::Function,
                name: func.name.clone(),
                signature: Some(format_function_signature(func)),
                line_start: func.line_number,
                line_end: func.end_line,
                docstring: func.docstring.clone(),
                language: code_structure.language.clone(),
            };

            let entity_id = self.store_entity_internal(&db, &entity)?;

            // Create embedding for semantic search
            self.create_entity_embedding(&db, entity_id, &entity)?;

            entities_indexed += 1;
        }

        // Store classes and their methods
        for class in &code_structure.classes {
            let class_entity = CodeEntity {
                id: None,
                file_path: code_structure.file_path.clone(),
                entity_type: EntityType::Class,
                name: class.name.clone(),
                signature: None,
                line_start: class.line_number,
                line_end: class.line_number,
                docstring: None,
                language: code_structure.language.clone(),
            };

            let class_id = self.store_entity_internal(&db, &class_entity)?;
            self.create_entity_embedding(&db, class_id, &class_entity)?;
            entities_indexed += 1;

            // Store methods
            for method in &class.methods {
                let method_entity = CodeEntity {
                    id: None,
                    file_path: code_structure.file_path.clone(),
                    entity_type: EntityType::Method,
                    name: format!("{}.{}", class.name, method.name),
                    signature: Some(format_function_signature(method)),
                    line_start: method.line_number,
                    line_end: method.line_number,
                    docstring: None,
                    language: code_structure.language.clone(),
                };

                let method_id = self.store_entity_internal(&db, &method_entity)?;
                self.create_entity_embedding(&db, method_id, &method_entity)?;

                // Create edge: class contains method
                self.store_edge_internal(&db, class_id, method_id, EdgeType::Contains)?;

                entities_indexed += 1;
            }
        }

        // Store imports
        for import in &code_structure.imports {
            let import_entity = CodeEntity {
                id: None,
                file_path: code_structure.file_path.clone(),
                entity_type: EntityType::Import,
                name: import.module.clone(),
                signature: import.alias.clone(),
                line_start: import.line_number,
                line_end: import.line_number,
                docstring: None,
                language: code_structure.language.clone(),
            };

            let _import_id = self.store_entity_internal(&db, &import_entity)?;
            entities_indexed += 1;
        }

        db.execute("COMMIT", [])?;

        Ok(entities_indexed)
    }

    /// Search for code entities using dual search (semantic + structural)
    ///
    /// This combines:
    /// 1. Vector search for semantic similarity
    /// 2. Graph traversal for structural relationships
    /// 3. Re-ranking by combined score
    ///
    /// Returns top k matches sorted by relevance
    pub fn search_code(&self, query: &str, k: usize) -> Result<Vec<CodeMatch>> {
        // Step 1: Semantic search via vector embeddings
        let vector_results = {
            let vector_store = self.vector_store.lock()
                .map_err(|e| anyhow!("Failed to lock vector store: {}", e))?;

            vector_store.search(query, k * 2, SearchScope::Global)?
        };

        // Step 2: Map vector results to code entities
        let mut matches = Vec::new();
        let db = self.db.lock().map_err(|e| anyhow!("Failed to lock database: {}", e))?;

        for hit in vector_results {
            // Lookup entity by vector ID
            if let Ok(entity) = self.get_entity_by_vector_id(&db, hit.id) {
                matches.push(CodeMatch {
                    entity,
                    score: hit.score,
                    match_type: MatchType::Semantic,
                });
            }
        }

        // Step 3: Sort by score and take top k
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        matches.truncate(k);

        Ok(matches)
    }

    /// Get entity by vector ID
    fn get_entity_by_vector_id(&self, db: &Connection, vector_id: i64) -> Result<CodeEntity> {
        // First get entity_id from code_embeddings
        let entity_id: i64 = db.query_row(
            "SELECT entity_id FROM code_embeddings WHERE vector_id = ?",
            [vector_id],
            |row| row.get(0),
        )?;

        // Then get the full entity
        self.get_entity_by_id(db, entity_id)
    }

    /// Get entity by ID
    fn get_entity_by_id(&self, db: &Connection, entity_id: i64) -> Result<CodeEntity> {
        let entity = db.query_row(
            "SELECT file_path, entity_type, name, signature, line_start, line_end, docstring, language
             FROM code_entities WHERE id = ?",
            [entity_id],
            |row| {
                Ok(CodeEntity {
                    id: Some(entity_id),
                    file_path: row.get(0)?,
                    entity_type: EntityType::from_str(&row.get::<_, String>(1)?).unwrap(),
                    name: row.get(2)?,
                    signature: row.get(3)?,
                    line_start: row.get::<_, i64>(4)? as usize,
                    line_end: row.get::<_, i64>(5)? as usize,
                    docstring: row.get(6)?,
                    language: row.get(7)?,
                })
            },
        )?;

        Ok(entity)
    }

    /// Store entity in database (internal, assumes transaction)
    fn store_entity_internal(&self, db: &Connection, entity: &CodeEntity) -> Result<i64> {
        db.execute(
            "INSERT INTO code_entities
             (file_path, entity_type, name, signature, line_start, line_end, docstring, language, indexed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                &entity.file_path,
                entity.entity_type.as_str(),
                &entity.name,
                &entity.signature,
                entity.line_start as i64,
                entity.line_end as i64,
                &entity.docstring,
                &entity.language,
                chrono::Utc::now().timestamp(),
            ],
        )?;

        Ok(db.last_insert_rowid())
    }

    /// Create embedding for an entity (internal, requires db connection from caller)
    fn create_entity_embedding(&self, db: &Connection, entity_id: i64, entity: &CodeEntity) -> Result<()> {
        // Create text representation for embedding
        let text = format_entity_for_embedding(entity);

        // Store in vector store (scope the lock to release it before using db)
        {
            let mut vector_store = self.vector_store.lock()
                .map_err(|e| anyhow!("Failed to lock vector store: {}", e))?;

            vector_store.insert_text(entity_id, None, &text, "code_entity")?;
        } // vector_store lock released here

        // Link embedding to entity (db already locked by caller)
        db.execute(
            "INSERT INTO code_embeddings (entity_id, vector_id, model_version, created_at)
             VALUES (?, ?, ?, ?)",
            rusqlite::params![
                entity_id,
                entity_id, // vector_id same as entity_id for now
                "all-MiniLM-L6-v2",
                chrono::Utc::now().timestamp(),
            ],
        )?;

        Ok(())
    }

    /// Store edge (relationship) between entities
    fn store_edge_internal(&self, db: &Connection, src_id: i64, dst_id: i64, edge_type: EdgeType) -> Result<()> {
        db.execute(
            "INSERT OR IGNORE INTO code_edges (src_entity_id, dst_entity_id, edge_type)
             VALUES (?, ?, ?)",
            rusqlite::params![src_id, dst_id, edge_type.as_str()],
        )?;

        Ok(())
    }
}

/// Format function signature for display
fn format_function_signature(func: &FunctionInfo) -> String {
    if func.parameters.is_empty() {
        format!("{}()", func.name)
    } else {
        format!("{}({})", func.name, func.parameters.join(", "))
    }
}

/// Format entity as text for embedding
fn format_entity_for_embedding(entity: &CodeEntity) -> String {
    let mut parts = vec![
        entity.entity_type.as_str().to_string(),
        entity.name.clone(),
    ];

    if let Some(sig) = &entity.signature {
        parts.push(sig.clone());
    }

    if let Some(doc) = &entity.docstring {
        parts.push(doc.clone());
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_type_conversion() {
        assert_eq!(EntityType::Function.as_str(), "function");
        assert_eq!(EntityType::from_str("function"), Some(EntityType::Function));
        assert_eq!(EntityType::from_str("invalid"), None);
    }

    #[test]
    fn test_edge_type_conversion() {
        assert_eq!(EdgeType::Calls.as_str(), "calls");
        assert_eq!(EdgeType::Contains.as_str(), "contains");
    }

    #[test]
    fn test_format_function_signature() {
        let func = FunctionInfo {
            name: "add".to_string(),
            line_number: 10,
            end_line: 15,
            parameters: vec!["a".to_string(), "b".to_string()],
            return_type: Some("i32".to_string()),
            docstring: None,
            visibility: Some("pub".to_string()),
        };

        // format_function_signature only includes name and parameters, not return type
        assert_eq!(format_function_signature(&func), "add(a, b)");
    }

    #[test]
    fn test_index_file_no_deadlock() {
        use tempfile::Builder;
        use std::io::Write;
        use std::sync::{Arc, Mutex};
        use crate::vector::HuggingFaceEmbeddings;

        // Create a temporary Rust source file to index (with .rs extension)
        let mut temp_file = Builder::new()
            .prefix("test_")
            .suffix(".rs")
            .tempfile()
            .unwrap();
        writeln!(temp_file, "fn test_function(x: i32) -> i32 {{").unwrap();
        writeln!(temp_file, "    x + 1").unwrap();
        writeln!(temp_file, "}}").unwrap();
        writeln!(temp_file, "").unwrap();
        writeln!(temp_file, "struct TestStruct {{").unwrap();
        writeln!(temp_file, "    field: String,").unwrap();
        writeln!(temp_file, "}}").unwrap();
        temp_file.flush().unwrap();

        // Initialize VectorStore with real embeddings and CodeGraph
        let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let mut code_graph = CodeGraph::new(":memory:", vector_store).unwrap();

        // This test verifies Fix #2: index_file() should not deadlock
        // when calling create_entity_embedding() while holding db lock
        let result = code_graph.index_file(temp_file.path());

        // Should complete without hanging (deadlock would cause timeout)
        match result {
            Ok(count) => {
                assert!(count >= 2, "Should index at least function and struct");
            }
            Err(e) => {
                panic!("index_file failed with error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_search_code_no_deadlock() {
        use tempfile::Builder;
        use std::io::Write;
        use std::sync::{Arc, Mutex};
        use crate::vector::HuggingFaceEmbeddings;

        // Create and index a test file (with .rs extension)
        let mut temp_file = Builder::new()
            .prefix("test_")
            .suffix(".rs")
            .tempfile()
            .unwrap();
        writeln!(temp_file, "fn calculate_sum(a: i32, b: i32) -> i32 {{").unwrap();
        writeln!(temp_file, "    a + b").unwrap();
        writeln!(temp_file, "}}").unwrap();
        temp_file.flush().unwrap();

        let embeddings = Box::new(HuggingFaceEmbeddings::new().unwrap());
        let vector_store = Arc::new(Mutex::new(VectorStore::new(embeddings)));
        let mut code_graph = CodeGraph::new(":memory:", vector_store).unwrap();
        code_graph.index_file(temp_file.path()).unwrap();

        // Test that search doesn't deadlock
        let result = code_graph.search_code("sum calculation", 5);

        assert!(result.is_ok(), "search_code should complete without deadlock");
        let matches = result.unwrap();
        assert!(!matches.is_empty(), "Should find indexed function");
    }
}
