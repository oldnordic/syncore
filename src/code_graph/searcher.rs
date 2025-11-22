//! Code search logic combining semantic and structural search

use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::collections::HashSet;

use super::graph::CodeGraph;
use super::types::{CodeEntity, CodeMatch, EntityType, MatchType};
use crate::vector::SearchScope;

impl CodeGraph {
    pub fn search_code(&self, query: &str, k: usize) -> Result<Vec<CodeMatch>> {
        // Step 1: Semantic search via vector embeddings
        let vector_results = {
            let vector_store = self
                .vector_store
                .lock()
                .map_err(|e| anyhow!("Failed to lock vector store: {}", e))?;

            vector_store.search(query, k * 2, SearchScope::Global)?
        };

        // Step 2: Map vector results to code entities
        let mut matches = Vec::new();
        let db = self
            .db
            .lock()
            .map_err(|e| anyhow!("Failed to lock database: {}", e))?;

        for hit in vector_results {
            // Lookup entity by vector ID
            // FIX 3: Log failures instead of silently ignoring them
            match self.get_entity_by_vector_id(&db, hit.id) {
                Ok(entity) => {
                    matches.push(CodeMatch {
                        entity,
                        score: hit.score,
                        match_type: MatchType::Semantic,
                    });
                }
                Err(e) => {
                    eprintln!(
                        "[WARN] Vector→Entity mapping failed: vector_id={} not found in code_embeddings table. \
                         Error: {}. This indicates stale vector snapshot data.",
                        hit.id, e
                    );
                }
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
                created_at: None,
                last_modified_at: None,
                change_count: None,
                author_count: None,
            })
        },
    )?;

        Ok(entity)
    }
}
