//! Polyglot Graph Reasoning: Cross-language path analysis, cycle detection, and connected components
//!
//! This module implements the graph reasoning capabilities for the unified polyglot
//! reasoning layer, enabling multi-hop path reasoning across languages, cycle
//! detection, and connected component analysis.

use crate::polyglot::polyglot_model::*;
use crate::polyglot::polyglot_aggregator::PolyglotAggregator;
use std::collections::{HashMap, HashSet, VecDeque};

/// Graph reasoning engine for polyglot analysis
#[derive(Debug, Clone)]
pub struct PolyglotGraphReasoner {
    aggregator: PolyglotAggregator,
    adjacency_list: HashMap<i64, Vec<(i64, UnifiedEdge)>>, // entity_id -> [(target_id, edge)]
    reverse_adjacency_list: HashMap<i64, Vec<(i64, UnifiedEdge)>>, // entity_id -> [(source_id, edge)]
}

impl PolyglotGraphReasoner {
    /// Create a new graph reasoner from an aggregator
    pub fn new(aggregator: PolyglotAggregator) -> Self {
        let mut reasoner = Self {
            aggregator,
            adjacency_list: HashMap::new(),
            reverse_adjacency_list: HashMap::new(),
        };

        // Build adjacency lists
        reasoner.build_graph();

        reasoner
    }

    /// Build adjacency lists from edges
    fn build_graph(&mut self) {
        self.adjacency_list.clear();
        self.reverse_adjacency_list.clear();

        let entities = self.aggregator.get_entities();
        let entity_map: HashMap<i64, &UnifiedEntity> = entities.iter()
            .map(|e| (e.id, e))
            .collect();

        for edge in self.aggregator.get_edges() {
            // Skip edges where source or target entity doesn't exist
            if !entity_map.contains_key(&edge.source_id) || !entity_map.contains_key(&edge.target_id) {
                continue;
            }

            // Add to forward adjacency list
            self.adjacency_list
                .entry(edge.source_id)
                .or_insert_with(Vec::new)
                .push((edge.target_id, edge.clone()));

            // Add to reverse adjacency list
            self.reverse_adjacency_list
                .entry(edge.target_id)
                .or_insert_with(Vec::new)
                .push((edge.source_id, edge.clone()));
        }
    }

    /// Find a path from source to target entity using BFS
    pub fn find_path(&self, source_id: i64, target_id: i64) -> Option<Vec<UnifiedEntity>> {
        // BFS to find shortest path
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<i64, i64> = HashMap::new();

        queue.push_back(source_id);
        visited.insert(source_id);

        while let Some(current) = queue.pop_front() {
            // If we reached the target, reconstruct path
            if current == target_id {
                return Some(self.reconstruct_path(source_id, target_id, &parent));
            }

            // Explore neighbors
            if let Some(neighbors) = self.adjacency_list.get(&current) {
                for (next_id, _) in neighbors {
                    if !visited.contains(next_id) {
                        visited.insert(*next_id);
                        parent.insert(*next_id, current);
                        queue.push_back(*next_id);
                    }
                }
            }
        }

        None // No path found
    }

    /// Find all paths from source to target entity (with max depth limit)
    pub fn find_all_paths(&self, source_id: i64, target_id: i64, max_depth: usize) -> Vec<Vec<UnifiedEntity>> {
        let mut paths = Vec::new();
        let mut current_path = Vec::new();
        let mut visited = HashSet::new();
        let entity_map: HashMap<i64, &UnifiedEntity> = self.aggregator.get_entities()
            .iter()
            .map(|e| (e.id, e))
            .collect();

        self.dfs_paths(source_id, target_id, max_depth, &mut current_path, &mut visited, &entity_map, &mut paths);
        paths
    }

    /// Find all cross-language edges in the graph
    pub fn find_cross_language_edges(&self) -> Vec<UnifiedEdge> {
        let entities = self.aggregator.get_entities();
        let entity_language: HashMap<i64, LanguageType> = entities.iter()
            .map(|e| (e.id, e.language))
            .collect();

        self.aggregator.get_edges()
            .into_iter()
            .filter(|edge| {
                let source_lang = entity_language.get(&edge.source_id);
                let target_lang = entity_language.get(&edge.target_id);

                source_lang.is_some() && target_lang.is_some() && source_lang != target_lang
            })
            .collect()
    }

    /// Find all cycles in the graph
    pub fn find_cycles(&self) -> Vec<Vec<UnifiedEntity>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();
        let mut path = Vec::new();
        let entity_map: HashMap<i64, &UnifiedEntity> = self.aggregator.get_entities()
            .iter()
            .map(|e| (e.id, e))
            .collect();

        for entity in self.aggregator.get_entities() {
            if !visited.contains(&entity.id) {
                self.dfs_cycles(entity.id, &mut visited, &mut recursion_stack, &mut path, &entity_map, &mut cycles);
            }
        }

        cycles
    }

    /// Find connected components in the graph
    pub fn find_connected_components(&self) -> Vec<Vec<UnifiedEntity>> {
        let mut components = Vec::new();
        let mut visited = HashSet::new();
        let entity_map: HashMap<i64, &UnifiedEntity> = self.aggregator.get_entities()
            .iter()
            .map(|e| (e.id, e))
            .collect();

        for entity in self.aggregator.get_entities() {
            if !visited.contains(&entity.id) {
                let mut component = Vec::new();
                self.dfs_component(entity.id, &mut visited, &entity_map, &mut component);
                components.push(component);
            }
        }

        components
    }

    /// Compute statistics about the graph
    pub fn compute_statistics(&self) -> GraphStatistics {
        let entities = self.aggregator.get_entities();
        let edges = self.aggregator.get_edges();

        // Count entities by language
        let mut language_counts = HashMap::new();
        for entity in &entities {
            *language_counts.entry(entity.language).or_insert(0) += 1;
        }

        // Count edges by type
        let mut edge_type_counts = HashMap::new();
        let mut cross_language_edges = 0;

        let entity_language: HashMap<i64, LanguageType> = entities.iter()
            .map(|e| (e.id, e.language))
            .collect();

        for edge in &edges {
            *edge_type_counts.entry(edge.kind.clone()).or_insert(0) += 1;

            let source_lang = entity_language.get(&edge.source_id);
            let target_lang = entity_language.get(&edge.target_id);

            if source_lang.is_some() && target_lang.is_some() && source_lang != target_lang {
                cross_language_edges += 1;
            }
        }

        GraphStatistics {
            total_entities: entities.len(),
            total_edges: edges.len(),
            language_counts,
            cross_language_edges,
            edge_type_counts,
        }
    }

    /// Reconstruct path from parent map
    fn reconstruct_path(&self, source_id: i64, target_id: i64, parent: &HashMap<i64, i64>) -> Vec<UnifiedEntity> {
        let mut path = Vec::new();
        let mut current = target_id;
        let entity_map: HashMap<i64, &UnifiedEntity> = self.aggregator.get_entities()
            .iter()
            .map(|e| (e.id, e))
            .collect();

        // Build path backwards
        while let Some(&parent_id) = parent.get(&current) {
            if let Some(entity) = entity_map.get(&current) {
                path.push(entity.clone());
            }
            current = parent_id;
        }

        // Add source entity
        if let Some(entity) = entity_map.get(&source_id) {
            path.push(entity.clone());
        }

        // Reverse to get correct order (source -> target)
        path.reverse();
        path
    }

    /// DFS helper for finding all paths
    fn dfs_paths(
        &self,
        current_id: i64,
        target_id: i64,
        max_depth: usize,
        current_path: &mut Vec<i64>,
        visited: &mut HashSet<i64>,
        entity_map: &HashMap<i64, &UnifiedEntity>,
        paths: &mut Vec<Vec<UnifiedEntity>>,
    ) {
        if current_path.len() >= max_depth {
            return;
        }

        current_path.push(current_id);
        visited.insert(current_id);

        if current_id == target_id {
            // Convert path of IDs to path of entities
            let entity_path: Vec<UnifiedEntity> = current_path.iter()
                .filter_map(|&id| entity_map.get(&id).cloned())
                .collect();
            paths.push(entity_path);
        } else if let Some(neighbors) = self.adjacency_list.get(&current_id) {
            for (next_id, _) in neighbors {
                if !visited.contains(next_id) {
                    self.dfs_paths(*next_id, target_id, max_depth, current_path, visited, entity_map, paths);
                }
            }
        }

        // Backtrack
        current_path.pop();
        visited.remove(&current_id);
    }

    /// DFS helper for finding cycles
    fn dfs_cycles(
        &self,
        node_id: i64,
        visited: &mut HashSet<i64>,
        recursion_stack: &mut HashSet<i64>,
        path: &mut Vec<i64>,
        entity_map: &HashMap<i64, &UnifiedEntity>,
        cycles: &mut Vec<Vec<UnifiedEntity>>,
    ) {
        visited.insert(node_id);
        recursion_stack.insert(node_id);
        path.push(node_id);

        if let Some(neighbors) = self.adjacency_list.get(&node_id) {
            for (neighbor_id, _) in neighbors {
                if recursion_stack.contains(neighbor_id) {
                    // Found a cycle, extract it
                    let cycle_start = path.iter().position(|&id| id == *neighbor_id).unwrap_or(0);
                    let cycle_ids = &path[cycle_start..];
                    let cycle: Vec<UnifiedEntity> = cycle_ids.iter()
                        .filter_map(|&id| entity_map.get(&id).cloned())
                        .collect();
                    cycles.push(cycle);
                } else if !visited.contains(neighbor_id) {
                    self.dfs_cycles(*neighbor_id, visited, recursion_stack, path, entity_map, cycles);
                }
            }
        }

        // Backtrack
        path.pop();
        recursion_stack.remove(&node_id);
    }

    /// DFS helper for finding connected components
    fn dfs_component(
        &self,
        node_id: i64,
        visited: &mut HashSet<i64>,
        entity_map: &HashMap<i64, &UnifiedEntity>,
        component: &mut Vec<UnifiedEntity>,
    ) {
        visited.insert(node_id);
        if let Some(entity) = entity_map.get(&node_id) {
            component.push(entity.clone());
        }

        if let Some(neighbors) = self.adjacency_list.get(&node_id) {
            for (neighbor_id, _) in neighbors {
                if !visited.contains(neighbor_id) {
                    self.dfs_component(*neighbor_id, visited, entity_map, component);
                }
            }
        }
    }
}

/// Compute cross-language paths from source to target entity
pub fn compute_cross_language_paths(
    source_id: i64,
    target_id: i64,
    entities: &[UnifiedEntity],
    edges: &[UnifiedEdge]
) -> Option<Vec<UnifiedEntity>> {
    let mut aggregator = PolyglotAggregator::new();
    aggregator.add_entities(entities.to_vec());
    aggregator.add_edges(edges.to_vec());

    let reasoner = PolyglotGraphReasoner::new(aggregator);
    reasoner.find_path(source_id, target_id)
}

/// Detect cross-language cycles in the graph
pub fn detect_cross_language_cycles(
    entities: &[UnifiedEntity],
    edges: &[UnifiedEdge]
) -> Vec<Vec<UnifiedEntity>> {
    let mut aggregator = PolyglotAggregator::new();
    aggregator.add_entities(entities.to_vec());
    aggregator.add_edges(edges.to_vec());

    let reasoner = PolyglotGraphReasoner::new(aggregator);
    reasoner.find_cycles()
}

/// Find cross-language connected components
pub fn find_cross_language_connected_components(
    entities: &[UnifiedEntity],
    edges: &[UnifiedEdge]
) -> Vec<Vec<UnifiedEntity>> {
    let mut aggregator = PolyglotAggregator::new();
    aggregator.add_entities(entities.to_vec());
    aggregator.add_edges(edges.to_vec());

    let reasoner = PolyglotGraphReasoner::new(aggregator);
    reasoner.find_connected_components()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polyglot::polyglot_model::*;
    use std::collections::HashMap;

    #[test]
    fn test_find_path() {
        let mut aggregator = PolyglotAggregator::new();

        // Create entities
        let rust_main = UnifiedEntity {
            id: 1,
            language: LanguageType::Rust,
            kind: EntityKind::Function,
            name: "main".to_string(),
            file_path: "src/main.rs".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        let python_service = UnifiedEntity {
            id: 2,
            language: LanguageType::Python,
            kind: EntityKind::Function,
            name: "service".to_string(),
            file_path: "service.py".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        let java_handler = UnifiedEntity {
            id: 3,
            language: LanguageType::Java,
            kind: EntityKind::Method,
            name: "handler".to_string(),
            file_path: "Handler.java".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        // Add entities
        aggregator.add_entity(rust_main);
        aggregator.add_entity(python_service);
        aggregator.add_entity(java_handler);

        // Create edges
        let rust_to_python = UnifiedEdge {
            id: 101,
            language: LanguageType::Rust,
            kind: EdgeKind::Calls,
            source_id: 1,
            target_id: 2,
            source_path: "src/main.rs".to_string(),
            target_path: "service.py".to_string(),
            metadata: HashMap::from([("binding_type".to_string(), "pyo3".to_string())]),
        };

        let python_to_java = UnifiedEdge {
            id: 102,
            language: LanguageType::Python,
            kind: EdgeKind::Calls,
            source_id: 2,
            target_id: 3,
            source_path: "service.py".to_string(),
            target_path: "Handler.java".to_string(),
            metadata: HashMap::from([("binding_type".to_string(), "jni".to_string())]),
        };

        // Add edges
        aggregator.add_edge(rust_to_python);
        aggregator.add_edge(python_to_java);

        let reasoner = PolyglotGraphReasoner::new(aggregator);

        // Find path from Rust main to Java handler
        let path = reasoner.find_path(1, 3);

        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].id, 1); // Rust main
        assert_eq!(path[1].id, 2); // Python service
        assert_eq!(path[2].id, 3); // Java handler
    }

    #[test]
    fn test_find_cross_language_edges() {
        let mut aggregator = PolyglotAggregator::new();

        // Create entities
        let rust_entity = UnifiedEntity {
            id: 1,
            language: LanguageType::Rust,
            kind: EntityKind::Function,
            name: "rust_func".to_string(),
            file_path: "rust.rs".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        let python_entity = UnifiedEntity {
            id: 2,
            language: LanguageType::Python,
            kind: EntityKind::Function,
            name: "python_func".to_string(),
            file_path: "python.py".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        let java_entity = UnifiedEntity {
            id: 3,
            language: LanguageType::Java,
            kind: EntityKind::Method,
            name: "java_func".to_string(),
            file_path: "Java.java".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        // Add entities
        aggregator.add_entity(rust_entity);
        aggregator.add_entity(python_entity);
        aggregator.add_entity(java_entity);

        // Create edges
        let rust_to_rust = UnifiedEdge {
            id: 101,
            language: LanguageType::Rust,
            kind: EdgeKind::Calls,
            source_id: 1,
            target_id: 1, // Self-call
            source_path: "rust.rs".to_string(),
            target_path: "rust.rs".to_string(),
            metadata: HashMap::new(),
        };

        let rust_to_python = UnifiedEdge {
            id: 102,
            language: LanguageType::Rust,
            kind: EdgeKind::Calls,
            source_id: 1,
            target_id: 2,
            source_path: "rust.rs".to_string(),
            target_path: "python.py".to_string(),
            metadata: HashMap::from([("binding_type".to_string(), "pyo3".to_string())]),
        };

        let python_to_java = UnifiedEdge {
            id: 103,
            language: LanguageType::Python,
            kind: EdgeKind::Calls,
            source_id: 2,
            target_id: 3,
            source_path: "python.py".to_string(),
            target_path: "Java.java".to_string(),
            metadata: HashMap::from([("binding_type".to_string(), "jni".to_string())]),
        };

        // Add edges
        aggregator.add_edge(rust_to_rust);
        aggregator.add_edge(rust_to_python);
        aggregator.add_edge(python_to_java);

        let reasoner = PolyglotGraphReasoner::new(aggregator);

        // Find cross-language edges
        let cross_lang_edges = reasoner.find_cross_language_edges();

        assert_eq!(cross_lang_edges.len(), 2); // rust->python and python->java

        // Verify edges
        let rust_python_edge = cross_lang_edges.iter()
            .find(|e| e.source_id == 1 && e.target_id == 2);
        assert!(rust_python_edge.is_some());

        let python_java_edge = cross_lang_edges.iter()
            .find(|e| e.source_id == 2 && e.target_id == 3);
        assert!(python_java_edge.is_some());
    }

    #[test]
    fn test_find_cycles() {
        let mut aggregator = PolyglotAggregator::new();

        // Create entities
        let rust_entity = UnifiedEntity {
            id: 1,
            language: LanguageType::Rust,
            kind: EntityKind::Function,
            name: "rust_func".to_string(),
            file_path: "rust.rs".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        let python_entity = UnifiedEntity {
            id: 2,
            language: LanguageType::Python,
            kind: EntityKind::Function,
            name: "python_func".to_string(),
            file_path: "python.py".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        // Add entities
        aggregator.add_entity(rust_entity);
        aggregator.add_entity(python_entity);

        // Create edges forming a cycle
        let rust_to_python = UnifiedEdge {
            id: 101,
            language: LanguageType::Rust,
            kind: EdgeKind::Calls,
            source_id: 1,
            target_id: 2,
            source_path: "rust.rs".to_string(),
            target_path: "python.py".to_string(),
            metadata: HashMap::from([("binding_type".to_string(), "pyo3".to_string())]),
        };

        let python_to_rust = UnifiedEdge {
            id: 102,
            language: LanguageType::Python,
            kind: EdgeKind::Calls,
            source_id: 2,
            target_id: 1,
            source_path: "python.py".to_string(),
            target_path: "rust.rs".to_string(),
            metadata: HashMap::from([("binding_type".to_string(), "ctypes".to_string())]),
        };

        // Add edges
        aggregator.add_edge(rust_to_python);
        aggregator.add_edge(python_to_rust);

        let reasoner = PolyglotGraphReasoner::new(aggregator);

        // Find cycles
        let cycles = reasoner.find_cycles();

        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2); // Two entities in the cycle

        // Verify cycle contains both entities
        let entity_ids: HashSet<i64> = cycles[0].iter().map(|e| e.id).collect();
        assert!(entity_ids.contains(&1)); // Rust entity
        assert!(entity_ids.contains(&2)); // Python entity
    }

    #[test]
    fn test_find_connected_components() {
        let mut aggregator = PolyglotAggregator::new();

        // Component 1: Rust -> Python
        let rust_entity1 = UnifiedEntity {
            id: 1,
            language: LanguageType::Rust,
            kind: EntityKind::Function,
            name: "rust_func1".to_string(),
            file_path: "rust1.rs".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        let python_entity1 = UnifiedEntity {
            id: 2,
            language: LanguageType::Python,
            kind: EntityKind::Function,
            name: "python_func1".to_string(),
            file_path: "python1.py".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        // Component 2: Isolated Java
        let java_entity = UnifiedEntity {
            id: 3,
            language: LanguageType::Java,
            kind: EntityKind::Method,
            name: "java_func".to_string(),
            file_path: "Java.java".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        // Add entities
        aggregator.add_entity(rust_entity1);
        aggregator.add_entity(python_entity1);
        aggregator.add_entity(java_entity);

        // Create edge connecting Rust and Python
        let rust_to_python = UnifiedEdge {
            id: 101,
            language: LanguageType::Rust,
            kind: EdgeKind::Calls,
            source_id: 1,
            target_id: 2,
            source_path: "rust1.rs".to_string(),
            target_path: "python1.py".to_string(),
            metadata: HashMap::from([("binding_type".to_string(), "pyo3".to_string())]),
        };

        // Add edge
        aggregator.add_edge(rust_to_python);

        let reasoner = PolyglotGraphReasoner::new(aggregator);

        // Find connected components
        let components = reasoner.find_connected_components();

        assert_eq!(components.len(), 2);

        // Component 1: Contains Rust and Python
        let component1 = components.iter()
            .find(|c| c.iter().any(|e| e.id == 1))
            .unwrap();
        assert_eq!(component1.len(), 2);
        let entity_ids: HashSet<i64> = component1.iter().map(|e| e.id).collect();
        assert!(entity_ids.contains(&1)); // Rust entity
        assert!(entity_ids.contains(&2)); // Python entity

        // Component 2: Contains only Java
        let component2 = components.iter()
            .find(|c| c.iter().any(|e| e.id == 3))
            .unwrap();
        assert_eq!(component2.len(), 1);
        assert_eq!(component2[0].id, 3); // Java entity
    }

    #[test]
    fn test_compute_statistics() {
        let mut aggregator = PolyglotAggregator::new();

        // Add entities from different languages
        let rust_entity = UnifiedEntity {
            id: 1,
            language: LanguageType::Rust,
            kind: EntityKind::Function,
            name: "rust_func".to_string(),
            file_path: "rust.rs".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        let python_entity = UnifiedEntity {
            id: 2,
            language: LanguageType::Python,
            kind: EntityKind::Function,
            name: "python_func".to_string(),
            file_path: "python.py".to_string(),
            span: (1, 10),
            attributes: HashMap::new(),
        };

        // Add entities
        aggregator.add_entity(rust_entity);
        aggregator.add_entity(python_entity);

        // Add edges
        let rust_to_python = UnifiedEdge {
            id: 101,
            language: LanguageType::Rust,
            kind: EdgeKind::Calls,
            source_id: 1,
            target_id: 2,
            source_path: "rust.rs".to_string(),
            target_path: "python.py".to_string(),
            metadata: HashMap::from([("binding_type".to_string(), "pyo3".to_string())]),
        };

        let python_to_rust = UnifiedEdge {
            id: 102,
            language: LanguageType::Python,
            kind: EdgeKind::UsesType,
            source_id: 2,
            target_id: 1,
            source_path: "python.py".to_string(),
            target_path: "rust.rs".to_string(),
            metadata: HashMap::from([("binding_type".to_string(), "ctypes".to_string())]),
        };

        // Add edges
        aggregator.add_edge(rust_to_python);
        aggregator.add_edge(python_to_rust);

        let reasoner = PolyglotGraphReasoner::new(aggregator);

        // Compute statistics
        let stats = reasoner.compute_statistics();

        assert_eq!(stats.total_entities, 2);
        assert_eq!(stats.total_edges, 2);
        assert_eq!(stats.language_counts.get(&LanguageType::Rust), Some(&1));
        assert_eq!(stats.language_counts.get(&LanguageType::Python), Some(&1));
        assert_eq!(stats.cross_language_edges, 2); // Both edges are cross-language
        assert_eq!(stats.edge_type_counts.get(&EdgeKind::Calls), Some(&1));
        assert_eq!(stats.edge_type_counts.get(&EdgeKind::UsesType), Some(&1));
    }
}
