//! Cycle Detection for Planner
//!
//! Implements DFS-based cycle detection with complete path tracking

use std::collections::{HashMap, HashSet};

/// Detect cycles in a dependency graph
#[derive(Debug)]
pub struct CycleDetector;

impl CycleDetector {
    /// Create a new cycle detector
    pub fn new() -> Self {
        Self
    }

    /// Detect all cycles in the dependency graph
    pub fn detect_cycles(&self, graph: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
        let mut visited = HashSet::new();
        let mut deadlocks = Vec::new();

        for node_id in graph.keys() {
            if !visited.contains(node_id) {
                let mut rec_stack = HashSet::new();
                if let Some(cycle) = self.detect_cycle(node_id, graph, &mut visited, &mut rec_stack) {
                    deadlocks.push(cycle);
                }
            }
        }

        deadlocks
    }

    /// Detect cycle in dependency graph using DFS
    fn detect_cycle(
        &self,
        node_id: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> Option<Vec<String>> {
        self.detect_cycle_with_path(node_id, graph, visited, rec_stack, &mut Vec::new())
    }

    /// Detect cycle with path tracking
    fn detect_cycle_with_path(
        &self,
        node_id: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node_id.to_string());
        rec_stack.insert(node_id.to_string());
        path.push(node_id.to_string());

        if let Some(dependencies) = graph.get(node_id) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    if let Some(cycle) = self.detect_cycle_with_path(dep, graph, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep) {
                    // Found a cycle - build the complete cycle path
                    if let Some(cycle_start_idx) = path.iter().position(|n| n == dep) {
                        let cycle = path[cycle_start_idx..].to_vec();
                        return Some(cycle);
                    }
                }
            }
        }

        rec_stack.remove(node_id);
        path.pop();
        None
    }
}

impl Default for CycleDetector {
    fn default() -> Self {
        Self::new()
    }
}