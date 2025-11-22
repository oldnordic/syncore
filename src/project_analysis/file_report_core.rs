//! Project File Report Core
//! 
//! Core SQL queries and data extraction for file analysis.

use crate::project_analysis::{
    EntityInfo, ImportInfo, ProjectAnalysisEngine, RelationshipInfo, UseInfo,
};
use anyhow::Result;
use std::collections::HashMap;

impl ProjectAnalysisEngine {
    /// Get all entities in a specific file
    pub fn get_file_entities(
        &self,
        conn: &rusqlite::Connection,
        file_path: &str,
    ) -> Result<Vec<EntityInfo>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, entity_type, line_start, line_end, signature, docstring, language
            FROM code_entities 
            WHERE file_path = ?1
            ORDER BY line_start
            "#,
        )?;

        let rows = stmt.query_map([file_path], |row| {
            Ok(EntityInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                file_path: file_path.to_string(),
                line_start: row.get(3)?,
                line_end: row.get(4)?,
                signature: row.get(5)?,
                docstring: row.get(6)?,
                language: row.get(7)?,
                visibility: None,
            })
        })?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }

        Ok(entities)
    }

    /// Get outgoing relationships from entities in a file
    pub fn get_outgoing_relationships(
        &self,
        conn: &rusqlite::Connection,
        file_path: &str,
    ) -> Result<Vec<RelationshipInfo>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                e1.id as src_id, e2.id as dst_id, ce.edge_type,
                e1.name as src_name, e2.name as dst_name,
                e1.file_path as src_file, e2.file_path as dst_file
            FROM code_entities e1
            JOIN code_edges ce ON e1.id = ce.src_entity_id
            JOIN code_entities e2 ON ce.dst_entity_id = e2.id
            WHERE e1.file_path = ?1
            ORDER BY e1.line_start, e2.line_start
            "#,
        )?;

        let rows = stmt.query_map([file_path], |row| {
            Ok(RelationshipInfo {
                src_entity_id: row.get(0)?,
                dst_entity_id: row.get(1)?,
                edge_type: row.get(2)?,
                src_entity_name: row.get(3)?,
                dst_entity_name: row.get(4)?,
                src_file_path: row.get(5)?,
                dst_file_path: row.get(6)?,
            })
        })?;

        let mut relationships = Vec::new();
        for row in rows {
            relationships.push(row?);
        }

        Ok(relationships)
    }

    /// Get incoming relationships to entities in a file
    pub fn get_incoming_relationships(
        &self,
        conn: &rusqlite::Connection,
        file_path: &str,
    ) -> Result<Vec<RelationshipInfo>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                e1.id as src_id, e2.id as dst_id, ce.edge_type,
                e1.name as src_name, e2.name as dst_name,
                e1.file_path as src_file, e2.file_path as dst_file
            FROM code_entities e1
            JOIN code_edges ce ON e1.id = ce.src_entity_id
            JOIN code_entities e2 ON ce.dst_entity_id = e2.id
            WHERE e2.file_path = ?1
            ORDER BY e1.line_start, e2.line_start
            "#,
        )?;

        let rows = stmt.query_map([file_path], |row| {
            Ok(RelationshipInfo {
                src_entity_id: row.get(0)?,
                dst_entity_id: row.get(1)?,
                edge_type: row.get(2)?,
                src_entity_name: row.get(3)?,
                dst_entity_name: row.get(4)?,
                src_file_path: row.get(5)?,
                dst_file_path: row.get(6)?,
            })
        })?;

        let mut relationships = Vec::new();
        for row in rows {
            relationships.push(row?);
        }

        Ok(relationships)
    }

    /// Get imports used in a file
    pub fn get_file_imports(
        &self,
        conn: &rusqlite::Connection,
        file_path: &str,
    ) -> Result<Vec<ImportInfo>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT DISTINCT
                e.signature as module_name,
                e.file_path as module_file,
                NULL as resolved_target,
                e.line_start as import_line
            FROM code_entities e
            WHERE e.file_path = ?1
            AND e.entity_type = 'import'
            ORDER BY e.line_start
            "#,
        )?;

        let rows = stmt.query_map([file_path], |row| {
            Ok(ImportInfo {
                module: row.get(0)?,
                file_path: row.get(1)?,
                resolved_target: row.get(2)?,
                line: row.get(3)?,
            })
        })?;

        let mut imports = Vec::new();
        for row in rows {
            imports.push(row?);
        }

        Ok(imports)
    }

    /// Get uses relationships within a file
    pub fn get_file_uses(
        &self,
        conn: &rusqlite::Connection,
        file_path: &str,
    ) -> Result<Vec<UseInfo>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                e1.name as from_entity,
                e2.name as to_entity,
                ce.edge_type as relation_type
            FROM code_entities e1
            JOIN code_edges ce ON e1.id = ce.src_entity_id
            JOIN code_entities e2 ON ce.dst_entity_id = e2.id
            WHERE e1.file_path = ?1 
            AND e2.file_path = ?1
            AND ce.edge_type = 'uses'
            ORDER BY e1.line_start, e2.line_start
            "#,
        )?;

        let rows = stmt.query_map([file_path], |row| {
            Ok(UseInfo {
                from_entity: row.get(0)?,
                to_entity: row.get(1)?,
                relation_type: row.get(2)?,
            })
        })?;

        let mut uses = Vec::new();
        for row in rows {
            uses.push(row?);
        }

        Ok(uses)
    }

    /// Estimate lines of code based on entity positions
    pub fn estimate_file_loc(&self, entities: &[EntityInfo]) -> Option<u32> {
        if entities.is_empty() {
            return None;
        }

        // Simple heuristic: use last entity's line_end as rough LOC estimate
        let max_line = entities
            .iter()
            .map(|e| e.line_end)
            .max()
            .unwrap_or(0);

        Some(max_line as u32)
    }
}