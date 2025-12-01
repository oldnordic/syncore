//! Project Cycles Detection Tool
//!
//! Detects circular dependencies between files and modules.

use crate::project_analysis::{CycleInfo, PAEResponse, ProjectAnalysisEngine};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Request parameters for project_cycles
#[derive(Debug, Deserialize)]
pub struct CyclesRequest {
    pub max_cycles: u32,
    pub max_depth: u32,
}

/// Cycles analysis response data
#[derive(Debug, Serialize, Deserialize)]
pub struct CyclesData {
    pub cycles: Vec<CycleInfo>,
}

impl ProjectAnalysisEngine {
    /// Detect circular dependencies in the project
    pub async fn cycles(&self, request: CyclesRequest) -> Result<PAEResponse<CyclesData>> {
        match self.detect_cycles(request.max_cycles, request.max_depth).await {
            Ok(data) => Ok(PAEResponse::success(data)),
            Err(e) => Ok(PAEResponse::error(e.to_string())),
        }
    }

    async fn detect_cycles(&self, max_cycles: u32, max_depth: u32) -> Result<CyclesData> {
        let conn = self.code_graph_conn();
        let conn_guard = conn.lock().unwrap();

        // Build dependency graph
        let graph = self.build_dependency_graph(&conn_guard)?;

        // Find cycles using DFS
        let cycles = self.find_cycles_in_graph(graph, max_cycles, max_depth)?;

        Ok(CyclesData {
            cycles,
        })
    }

    fn build_dependency_graph(
        &self,
        conn: &rusqlite::Connection,
    ) -> Result<HashMap<String, Vec<(String, String)>>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT DISTINCT
                e1.file_path as from_file,
                e2.file_path as to_file,
                ce.edge_type
            FROM code_edges ce
            JOIN code_entities e1 ON ce.src_entity_id = e1.id
            JOIN code_entities e2 ON ce.dst_entity_id = e2.id
            WHERE e1.file_path != e2.file_path
            ORDER BY e1.file_path, e2.file_path
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?, // from_file
                row.get::<_, String>(1)?, // to_file
                row.get::<_, String>(2)?, // edge_type
            ))
        })?;

        let mut graph: HashMap<String, Vec<(String, String)>> = HashMap::new();

        for row in rows {
            let (from_file, to_file, edge_type) = row?;
            graph.entry(from_file.clone()).or_default().push((to_file, edge_type));
        }

        Ok(graph)
    }

    fn find_cycles_in_graph(
        &self,
        graph: HashMap<String, Vec<(String, String)>>,
        max_cycles: u32,
        max_depth: u32,
    ) -> Result<Vec<CycleInfo>> {
        let mut cycles = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut recursion_stack: HashSet<String> = HashSet::new();

        for node in graph.keys() {
            if !visited.contains(node) {
                if let Some(cycle) = self.dfs_find_cycle(
                    node,
                    &graph,
                    &mut visited,
                    &mut recursion_stack,
                    Vec::new(),
                    Vec::new(),
                    max_depth,
                ) {
                    cycles.push(cycle);
                    if cycles.len() >= max_cycles as usize {
                        break;
                    }
                }
            }
        }

        Ok(cycles)
    }

    #[allow(clippy::too_many_arguments)]
    fn dfs_find_cycle(
        &self,
        current: &str,
        graph: &HashMap<String, Vec<(String, String)>>,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
        path: Vec<String>,
        relation_types: Vec<String>,
        max_depth: u32,
    ) -> Option<CycleInfo> {
        visited.insert(current.to_string());
        recursion_stack.insert(current.to_string());

        let mut new_path = path.clone();
        new_path.push(current.to_string());

        if let Some(neighbors) = graph.get(current) {
            for (neighbor, relation_type) in neighbors {
                if !visited.contains(neighbor) {
                    let mut new_relations = relation_types.clone();
                    new_relations.push(relation_type.clone());

                    if new_path.len() < max_depth as usize {
                        if let Some(cycle) = self.dfs_find_cycle(
                            neighbor,
                            graph,
                            visited,
                            recursion_stack,
                            new_path.clone(),
                            new_relations,
                            max_depth,
                        ) {
                            return Some(cycle);
                        }
                    }
                } else if recursion_stack.contains(neighbor) {
                    // Found a cycle
                    let cycle_start_index =
                        new_path.iter().position(|p| p == neighbor).unwrap_or(0);
                    let cycle_files = new_path[cycle_start_index..].to_vec();
                    let cycle_relations = if relation_types.len() >= cycle_files.len() {
                        relation_types[cycle_start_index..cycle_files.len()].to_vec()
                    } else {
                        relation_types.clone()
                    };

                    return Some(CycleInfo {
                        files: cycle_files,
                        relation_kinds: cycle_relations,
                        cycle_length: new_path.len() - cycle_start_index,
                    });
                }
            }
        }

        recursion_stack.remove(current);
        None
    }
}
