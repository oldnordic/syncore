//! Code file indexing logic

use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::path::Path;
use std::time::SystemTime;

use super::body_extractor::{extract_body_snippet}; // APEX v1.7 Phase 3
use super::graph::CodeGraph;
use super::incremental::{
    compute_file_sha256, get_file_mtime, get_stored_file_state, update_file_state,
    FileIndexState,
};
use super::neo4j_writer::create_code_entity_node;
use super::temporal_extractor::extract_temporal_metadata; // PHASE 3
use super::types::{CodeEntity, EdgeType, EntityType};
use super::utils::{format_entity_for_embedding, format_function_signature};
use crate::graph::Neo4jClient;

impl CodeGraph {
    /// Index a code file and optionally sync entities to Neo4j
    ///
    /// # Arguments
    /// * `file_path` - Path to the code file to index
    /// * `neo4j_opt` - Optional Neo4j client for graph synchronization (best-effort)
    ///
    /// # Returns
    /// Number of entities indexed
    ///
    /// # Neo4j Integration
    /// If neo4j_opt is Some, creates Neo4j nodes for all indexed entities.
    /// Neo4j failures are logged but DO NOT cause index_file to fail.
    /// SQLite is the source of truth.
    pub fn index_file_with_neo4j(
        &mut self,
        file_path: &Path,
        neo4j_opt: Option<&Neo4jClient>,
    ) -> Result<usize> {
        self.index_file_internal(file_path, neo4j_opt)
    }

    /// Index a code file (without Neo4j synchronization)
    ///
    /// This is the backward-compatible version that doesn't require Neo4j.
    /// Use index_file_with_neo4j() if you want Neo4j node creation.
    pub fn index_file(&mut self, file_path: &Path) -> Result<usize> {
        self.index_file_internal(file_path, None)
    }

    /// Internal implementation of index_file (shared by both public methods)
    fn index_file_internal(
        &mut self,
        file_path: &Path,
        neo4j_opt: Option<&Neo4jClient>,
    ) -> Result<usize> {
        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid file path"))?
            .to_string();

        // PHASE 5: Incremental indexing - check if file has changed
        // Early exit if unchanged to avoid unnecessary parsing and indexing
        {
            let db = self
                .db
                .lock()
                .map_err(|e| anyhow!("Failed to lock database for change detection: {}", e))?;

            // Check file change status
            let stored_state = get_stored_file_state(&db, &file_path_str)?;
            if let Some(ref state) = stored_state {
                // File was previously indexed - check if unchanged
                let current_sha256 = compute_file_sha256(file_path)?;
                let current_mtime = get_file_mtime(file_path)?;

                if state.sha256 == current_sha256 && state.mtime == current_mtime {
                    // File unchanged - skip indexing entirely
                    return Ok(0);
                }
            }
        } // Release lock before parsing

        // Parse the file
        let code_structure = self.parser.parse_file(file_path)?;

        // PHASE 3: Extract temporal metadata once for the entire file
        let temporal = extract_temporal_metadata(file_path.to_str().unwrap_or(""))?;

        // SMEL: Process macro expansions for Rust files
        if code_structure.language == "rust" {
            self.store_macro_expansions(file_path, &file_path_str)?;
        }

        let mut entities_indexed = 0;

        // Extract and store entities
        let mut db = self
            .db
            .lock()
            .map_err(|e| anyhow!("Failed to lock database: {}", e))?;

        // Use rusqlite Transaction API for proper RAII semantics
        let tx = db.transaction()?;

        // Delete existing entities for this file to allow re-indexing
        // file_path_str already set at function start
        tx.execute(
            "DELETE FROM code_entities WHERE file_path = ?",
            [&file_path_str],
        )?;

        // Store functions
        let mut entity_ids = Vec::new();
        let mut edges = Vec::new();
        for func in &code_structure.functions {
            // APEX v1.7 Phase 3: Extract function body snippet
            let body_snippet = extract_body_snippet(
                file_path,
                func.line_number,
                func.end_line,
            )
            .ok()
            .flatten();

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
                body_snippet,
                created_at: Some(temporal.created_at),
                last_modified_at: Some(temporal.last_modified_at),
                change_count: Some(temporal.change_count),
                author_count: Some(temporal.author_count),
            };

            let entity_id = self.store_entity_internal(&tx, &entity)?;
            entity_ids.push((entity_id, entity));
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
                body_snippet: None, // Classes don't get body snippets
                created_at: Some(temporal.created_at),
                last_modified_at: Some(temporal.last_modified_at),
                change_count: Some(temporal.change_count),
                author_count: Some(temporal.author_count),
            };

            let class_id = self.store_entity_internal(&tx, &class_entity)?;
            entity_ids.push((class_id, class_entity));
            entities_indexed += 1;

            // Store methods
            for method in &class.methods {
                // APEX v1.7 Phase 3: Extract method body snippet
                let body_snippet = extract_body_snippet(
                    file_path,
                    method.line_number,
                    method.end_line,
                )
                .ok()
                .flatten();

                let method_entity = CodeEntity {
                    id: None,
                    file_path: code_structure.file_path.clone(),
                    entity_type: EntityType::Method,
                    name: format!("{}.{}", class.name, method.name),
                    signature: Some(format_function_signature(method)),
                    line_start: method.line_number,
                    line_end: method.end_line,
                    docstring: None,
                    language: code_structure.language.clone(),
                    body_snippet,
                    created_at: Some(temporal.created_at),
                    last_modified_at: Some(temporal.last_modified_at),
                    change_count: Some(temporal.change_count),
                    author_count: Some(temporal.author_count),
                };

                let method_id = self.store_entity_internal(&tx, &method_entity)?;
                entity_ids.push((method_id, method_entity.clone()));

                // Create edge: class contains method
                self.store_edge_internal(&tx, class_id, method_id, EdgeType::Contains)?;
                edges.push((class_id, method_id, EdgeType::Contains));

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
                body_snippet: None,
                created_at: Some(temporal.created_at),
                last_modified_at: Some(temporal.last_modified_at),
                change_count: Some(temporal.change_count),
                author_count: Some(temporal.author_count),
            };

            let import_id = self.store_entity_internal(&tx, &import_entity)?;
            entity_ids.push((import_id, import_entity));
            entities_indexed += 1;
        }

        // EDGE EXTRACTION: Extract and store code relationships
        // Only for Rust files (other languages need separate extractors)
        if code_structure.language == "rust" {
            // Re-parse file to extract edges from AST
            let source_code = std::fs::read_to_string(file_path)?;
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(unsafe { tree_sitter_rust::language() })?;

            if let Some(tree) = parser.parse(&source_code, None) {
                // PHASE 1a: Extract and store trait definitions (needed for Inherits edges)
                // Traits aren't extracted by parser, so we extract them here
                let trait_names = extract_trait_definitions(&source_code, tree.root_node());
                for trait_name in &trait_names {
                    let trait_entity = CodeEntity {
                        id: None,
                        file_path: code_structure.file_path.clone(),
                        entity_type: EntityType::Trait,
                        name: trait_name.clone(),
                        signature: None,
                        line_start: 0, // TODO: extract actual line number
                        line_end: 0,
                        docstring: None,
                        language: code_structure.language.clone(),
                        body_snippet: None,
                        created_at: Some(temporal.created_at),
                        last_modified_at: Some(temporal.last_modified_at),
                        change_count: Some(temporal.change_count),
                        author_count: Some(temporal.author_count),
                    };
                    let trait_id = self.store_entity_internal(&tx, &trait_entity)?;
                    entity_ids.push((trait_id, trait_entity));
                    entities_indexed += 1;
                }

                // PHASE 1b: Extract and store const/static definitions (needed for References edges)
                // Constants aren't extracted by parser, so we extract them here
                let const_names = extract_const_definitions(&source_code, tree.root_node());
                for const_name in &const_names {
                    let const_entity = CodeEntity {
                        id: None,
                        file_path: code_structure.file_path.clone(),
                        entity_type: EntityType::Function, // Use Function for now (no Constant type)
                        name: const_name.clone(),
                        signature: Some("const".to_string()), // Mark as const via signature
                        line_start: 0,                        // TODO: extract actual line number
                        line_end: 0,
                        docstring: None,
                        language: code_structure.language.clone(),
                        body_snippet: None,
                        created_at: Some(temporal.created_at),
                        last_modified_at: Some(temporal.last_modified_at),
                        change_count: Some(temporal.change_count),
                        author_count: Some(temporal.author_count),
                    };
                    let const_id = self.store_entity_internal(&tx, &const_entity)?;
                    entity_ids.push((const_id, const_entity));
                    entities_indexed += 1;
                }

                // PHASE 2: Extract edges using edge_extractor module
                if let Ok(extracted_edges) = super::edge_extractor::extract_edges_from_rust_ast(
                    &source_code,
                    tree.root_node(),
                ) {
                    // Build name -> entity_id mapping
                    let mut name_to_id: std::collections::HashMap<String, i64> =
                        std::collections::HashMap::new();
                    for (id, entity) in &entity_ids {
                        name_to_id.insert(entity.name.clone(), *id);
                    }

                    // Store edges in database
                    for edge in extracted_edges {
                        // Look up entity IDs by name
                        if let Some(&src_id) = name_to_id.get(&edge.src_entity_name) {
                            if let Some(&dst_id) = name_to_id.get(&edge.dst_entity_name) {
                                self.store_edge_internal(&tx, src_id, dst_id, edge.edge_type)?;
                            }
                        }
                    }
                }
            }
        }

        // Commit transaction using RAII (automatic rollback on drop if not committed)
        tx.commit()?;

        // CRITICAL FIX: Explicitly checkpoint WAL to main DB BEFORE connection closes
        // This ensures WAL frames are merged and persisted
        db.execute_batch("PRAGMA wal_checkpoint(RESTART)")?;

        // PHASE 5: Update file_index_state after successful indexing
        // This enables incremental indexing to skip unchanged files
        let sha256 = compute_file_sha256(file_path)?;
        let mtime = get_file_mtime(file_path)?;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs() as i64;
        let state = FileIndexState {
            file_path: file_path_str.clone(),
            sha256,
            mtime,
            last_indexed_at: now,
            status: "ok".to_string(),
        };
        update_file_state(&db, &state)?;

        // Create embeddings AFTER transaction commits (best-effort, non-blocking)
        // This decouples persistence from embedding success
        for (entity_id, entity) in &entity_ids {
            if let Err(e) = self.create_entity_embedding(&db, *entity_id, entity) {
                eprintln!(
                    "[WARN] Failed to create embedding for entity {}: {}",
                    entity.name, e
                );
                // Continue - embeddings are optional
            }
        }

        // Release lock after checkpoint completes
        drop(db);

        // Sync to Neo4j AFTER SQLite persistence complete (best-effort, non-blocking)
        // This decouples Neo4j from SQLite persistence
        if let Some(neo4j) = neo4j_opt {
            // Fire-and-forget async task for Neo4j node and relationship creation
            // Production: truly async, returns immediately
            // Tests: may need small delay to verify nodes created
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let neo4j_clone = neo4j.clone();
                let entities_clone: Vec<_> = entity_ids
                    .iter()
                    .map(|(id, entity)| (*id, entity.clone()))
                    .collect();
                let edges_clone = edges.clone();

                handle.spawn(async move {
                    // First create all entity nodes
                    for (entity_id, entity) in entities_clone {
                        if let Err(e) =
                            create_code_entity_node(&neo4j_clone, entity_id, &entity).await
                        {
                            eprintln!(
                                "[WARN] Failed to create Neo4j node for entity {}: {}",
                                entity.name, e
                            );
                        }
                    }

                    // Then create relationships between entities
                    use super::neo4j_relationships::create_code_relationship;
                    use super::types::CodeEdge;
                    for (src_id, dst_id, edge_type) in edges_clone {
                        let edge = CodeEdge {
                            src_entity_id: src_id,
                            dst_entity_id: dst_id,
                            edge_type,
                        };
                        if let Err(e) = create_code_relationship(&neo4j_clone, &edge).await {
                            eprintln!(
                                "[WARN] Failed to create Neo4j relationship {:?} from {} to {}: {}",
                                edge.edge_type, src_id, dst_id, e
                            );
                        }
                    }
                });
                // Fire and forget - don't block
            } else {
                eprintln!("[WARN] No tokio runtime available, skipping Neo4j sync");
            }
        }

        Ok(entities_indexed)
    }
    fn store_entity_internal<C: std::ops::Deref<Target = rusqlite::Connection>>(
        &self,
        db: &C,
        entity: &CodeEntity,
    ) -> Result<i64> {
        // STEP 3: Log INSERT attempt
        let insert_result = db.execute(
            "INSERT INTO code_entities
             (file_path, entity_type, name, signature, line_start, line_end, docstring, language, indexed_at,
              body_snippet, created_at, last_modified_at, change_count, author_count)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
                &entity.body_snippet,
                &entity.created_at,
                &entity.last_modified_at,
                &entity.change_count,
                &entity.author_count,
            ],
        );

        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/code_graph_diagnostic.log")
            .and_then(|mut f| {
                use std::io::Write;
                match &insert_result {
                    Ok(rows) => writeln!(
                        f,
                        "STEP 3: INSERT OK - {} rows, entity: {}",
                        rows, entity.name
                    )?,
                    Err(e) => writeln!(
                        f,
                        "STEP 3: INSERT FAILED - entity: {}, error: {}",
                        entity.name, e
                    )?,
                }
                Ok(())
            });

        insert_result?;

        let entity_id = db.last_insert_rowid();

        // STEP 2: Verify INSERT visibility inside transaction
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM code_entities WHERE id = ?",
            [entity_id],
            |row| row.get(0),
        )?;

        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/code_graph_diagnostic.log")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(
                    f,
                    "STEP 2: INSERT visibility check - entity_id={}, count={}",
                    entity_id, count
                )?;
                if count == 0 {
                    writeln!(
                        f,
                        "STEP 2: ❌ CRITICAL - INSERT not visible inside transaction!"
                    )?;
                }
                Ok(())
            });

        Ok(entity_id)
    }

    /// Create embedding for an entity (internal, requires db connection from caller)

    fn create_entity_embedding(
        &self,
        db: &Connection,
        entity_id: i64,
        entity: &CodeEntity,
    ) -> Result<()> {
        // Create text representation for embedding
        let text = format_entity_for_embedding(entity);

        // Store in vector store (scope the lock to release it before using db)
        {
            let mut vector_store = self
                .vector_store
                .lock()
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

    fn store_edge_internal<C: std::ops::Deref<Target = rusqlite::Connection>>(
        &self,
        db: &C,
        src_id: i64,
        dst_id: i64,
        edge_type: EdgeType,
    ) -> Result<()> {
        db.execute(
            "INSERT OR IGNORE INTO code_edges (src_entity_id, dst_entity_id, edge_type)
             VALUES (?, ?, ?)",
            rusqlite::params![src_id, dst_id, edge_type.as_str()],
        )?;

        Ok(())
    }

    /// Store macro expansions for Rust files
    fn store_macro_expansions(&self, file_path: &Path, file_path_str: &str) -> Result<()> {
        // Create a RustLanguageParser for macro expansion
        let rust_parser = super::parsers::rust_parser::RustLanguageParser::new()?;
        let macro_expansions = rust_parser.parse_macro_expansions(file_path)?;

        // Read source code to extract original macro invocations
        let source_code = std::fs::read_to_string(file_path)?;

        // Store macro expansions directly in database
        let mut macro_db = self
            .db
            .lock()
            .map_err(|e| anyhow!("Failed to lock database for macro expansions: {}", e))?;

        let tx = (*macro_db).transaction()?;

        // Clear existing macro expansions for this file
        tx.execute(
            "DELETE FROM code_macro_expansions WHERE file_path = ?",
            [file_path_str],
        )?;

        // Insert new macro expansions
        let mut stmt = tx.prepare(
            r#"
            INSERT INTO code_macro_expansions (
                file_path, macro_name, span_start, span_end, 
                original_code, expanded_code, expansion_type
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )?;

        let mut inserted = 0;
        for expansion in &macro_expansions.expansions {
            // Extract original code from source using span
            let original_code = source_code
                .get(expansion.span_start..expansion.span_end)
                .unwrap_or(&expansion.macro_name)
                .to_string();

            // Determine expansion type based on macro name
            let expansion_type = if expansion.macro_name == "vec!" {
                "vec".to_string()
            } else if expansion.macro_name == "format!" {
                "format".to_string()
            } else if expansion.macro_name.contains("info")
                || expansion.macro_name.contains("warn")
                || expansion.macro_name.contains("error")
                || expansion.macro_name.contains("debug")
                || expansion.macro_name.contains("trace")
            {
                "log".to_string()
            } else if expansion.macro_name == "assert!" {
                "assert".to_string()
            } else {
                "declarative".to_string()
            };

            stmt.execute((
                file_path_str,
                &expansion.macro_name,
                expansion.span_start as i64,
                expansion.span_end as i64,
                original_code,
                &expansion.expanded_code,
                &expansion_type,
            ))?;
            inserted += 1;
        }

        drop(stmt);
        tx.commit()?;

        if inserted > 0 {
            println!(
                "[INFO] Stored {} macro expansions for {}",
                inserted, file_path_str
            );
        }

        Ok(())
    }
}

/// Extract trait definition names from Rust AST
/// Returns a vector of trait names found in the file
fn extract_trait_definitions(source_code: &str, root_node: tree_sitter::Node) -> Vec<String> {
    let mut trait_names = Vec::new();
    let mut cursor = root_node.walk();

    visit_nodes_for_traits(&mut cursor, source_code, &mut |node, src| {
        if node.kind() == "trait_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = &src[name_node.byte_range()];
                trait_names.push(name.to_string());
            }
        }
    });

    trait_names
}

/// Extract const/static definition names from Rust AST
/// Returns a vector of const/static names found in the file
fn extract_const_definitions(source_code: &str, root_node: tree_sitter::Node) -> Vec<String> {
    let mut const_names = Vec::new();
    let mut cursor = root_node.walk();

    visit_nodes_for_traits(&mut cursor, source_code, &mut |node, src| {
        let kind = node.kind();
        if kind == "const_item" || kind == "static_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = &src[name_node.byte_range()];
                const_names.push(name.to_string());
            }
        }
    });

    const_names
}

/// Helper: Recursively visit all nodes in AST (for trait extraction)
fn visit_nodes_for_traits<F>(
    cursor: &mut tree_sitter::TreeCursor,
    source_code: &str,
    visitor: &mut F,
) where
    F: FnMut(tree_sitter::Node, &str),
{
    loop {
        let node = cursor.node();
        visitor(node, source_code);

        // Recurse into children
        if cursor.goto_first_child() {
            visit_nodes_for_traits(cursor, source_code, visitor);
            cursor.goto_parent();
        }

        // Move to next sibling
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}
