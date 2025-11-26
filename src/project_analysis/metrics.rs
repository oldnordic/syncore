//! Project Metrics Collection
//!
//! Provides common metrics calculation utilities for PAE tools.

use anyhow::Result;

/// Metrics calculator for project analysis
pub struct MetricsCalculator;

impl MetricsCalculator {
    /// Calculate fan-in for a file (number of incoming relationships)
    pub fn calculate_fan_in(conn: &rusqlite::Connection, file_path: &str) -> Result<u32> {
        let mut stmt = conn.prepare(
            r#"
            SELECT COUNT(DISTINCT ce.src_entity_id)
            FROM code_edges ce
            JOIN code_entities e ON ce.dst_entity_id = e.id
            WHERE e.file_path = ?1
            "#,
        )?;

        let fan_in: i64 = stmt.query_row([file_path], |row| row.get(0))?;
        Ok(fan_in as u32)
    }

    /// Calculate fan-out for a file (number of outgoing relationships)
    pub fn calculate_fan_out(conn: &rusqlite::Connection, file_path: &str) -> Result<u32> {
        let mut stmt = conn.prepare(
            r#"
            SELECT COUNT(DISTINCT ce.dst_entity_id)
            FROM code_edges ce
            JOIN code_entities e ON ce.src_entity_id = e.id
            WHERE e.file_path = ?1
            "#,
        )?;

        let fan_out: i64 = stmt.query_row([file_path], |row| row.get(0))?;
        Ok(fan_out as u32)
    }

    /// Count entities in a file
    pub fn count_entities(conn: &rusqlite::Connection, file_path: &str) -> Result<u32> {
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM code_entities WHERE file_path = ?1")?;

        let count: i64 = stmt.query_row([file_path], |row| row.get(0))?;
        Ok(count as u32)
    }

    /// Estimate lines of code for a file
    pub fn estimate_loc(conn: &rusqlite::Connection, file_path: &str) -> Result<Option<u32>> {
        let mut stmt =
            conn.prepare("SELECT MAX(line_end) FROM code_entities WHERE file_path = ?1")?;

        let loc: Option<i64> = stmt.query_row([file_path], |row| row.get(0))?;
        Ok(loc.map(|l| l as u32))
    }

    /// Get entity type distribution for a file
    pub fn get_entity_distribution(
        conn: &rusqlite::Connection,
        file_path: &str,
    ) -> Result<Vec<(String, u32)>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT entity_type, COUNT(*) as count
            FROM code_entities
            WHERE file_path = ?1
            GROUP BY entity_type
            ORDER BY count DESC
            "#,
        )?;

        let rows = stmt.query_map([file_path], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u32))
        })?;

        let mut distribution = Vec::new();
        for row in rows {
            distribution.push(row?);
        }
        Ok(distribution)
    }

    /// Calculate complexity score for a file based on multiple factors
    pub fn calculate_complexity_score(conn: &rusqlite::Connection, file_path: &str) -> Result<f32> {
        let fan_in = Self::calculate_fan_in(conn, file_path)?;
        let fan_out = Self::calculate_fan_out(conn, file_path)?;
        let entity_count = Self::count_entities(conn, file_path)?;
        let loc = Self::estimate_loc(conn, file_path)?.unwrap_or(0);

        // Weighted complexity score
        let score = fan_in as f32 * 0.3
            + fan_out as f32 * 0.3
            + entity_count as f32 * 0.2
            + loc as f32 * 0.2;

        Ok(score)
    }
}
