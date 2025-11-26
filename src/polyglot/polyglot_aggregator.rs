use crate::polyglot::polyglot_model::{LanguageType, UnifiedEdge, UnifiedEntity};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

pub fn merge_entities(entities: Vec<UnifiedEntity>) -> Vec<UnifiedEntity> {
    let mut merged_entities: HashMap<String, UnifiedEntity> = HashMap::new();
    for entity in entities {
        let entry = merged_entities.entry(entity.name.clone());
        use std::collections::hash_map::Entry;
        match entry {
            Entry::Occupied(mut e) => {
                if e.get().language != LanguageType::Rust && entity.language == LanguageType::Rust {
                    e.insert(entity);
                }
            }
            Entry::Vacant(e) => {
                e.insert(entity);
            }
        }
    }
    merged_entities
        .into_values()
        .map(|mut entity| {
            let mut hasher = DefaultHasher::new();
            entity.name.hash(&mut hasher);
            entity.language.hash(&mut hasher);
            entity.id = hasher.finish() as i64;

            normalize_entity_namespace(&mut entity);

            entity
        })
        .collect()
}

pub fn merge_edges(edges: Vec<UnifiedEdge>) -> Vec<UnifiedEdge> {
    let mut merged_edges = HashSet::new();
    for edge in edges {
        merged_edges.insert(edge);
    }
    merged_edges.into_iter().collect()
}

pub fn normalize_entity_namespace(entity: &mut UnifiedEntity) {
    let namespace = match entity.language {
        LanguageType::Rust => {
            if let Some(ns) = entity.attributes.get("namespace") {
                ns.replace("crate::", "")
            } else {
                entity.name.clone()
            }
        }
        LanguageType::Python => entity.name.clone(),
        LanguageType::Java => entity.name.clone(),
        LanguageType::TypeScript => entity.name.clone(),
        LanguageType::JavaScript => entity.name.clone(),
        LanguageType::Go => entity.name.clone(),
        LanguageType::C => entity.name.clone(),
        LanguageType::Cpp => entity.name.clone(),
    };
    entity.attributes.insert("namespace".to_string(), namespace);
}

pub fn find_paths(edges: Vec<UnifiedEdge>, start_id: i64, end_id: i64) -> Vec<Vec<i64>> {
    let mut adj = HashMap::new();
    for edge in edges {
        adj.entry(edge.from_id).or_insert(vec![]).push(edge.to_id);
    }

    let mut paths = vec![];
    let mut queue = vec![(start_id, vec![start_id])];

    while let Some((curr, path)) = queue.pop() {
        if curr == end_id {
            paths.push(path);
            continue;
        }

        if let Some(neighbors) = adj.get(&curr) {
            for &neighbor in neighbors {
                if !path.contains(&neighbor) {
                    let mut new_path = path.clone();
                    new_path.push(neighbor);
                    queue.push((neighbor, new_path));
                }
            }
        }
    }

    paths
}

pub fn has_cycle(edges: Vec<UnifiedEdge>) -> bool {
    let mut adj = HashMap::new();
    for edge in &edges {
        adj.entry(edge.from_id).or_insert(vec![]).push(edge.to_id);
    }

    let mut visited = HashSet::new();
    let mut recursion_stack = HashSet::new();

    for edge in &edges {
        let node = edge.from_id;
        if !visited.contains(&node) {
            if has_cycle_util(node, &adj, &mut visited, &mut recursion_stack) {
                return true;
            }
        }
    }

    false
}

fn has_cycle_util(
    node: i64,
    adj: &HashMap<i64, Vec<i64>>,
    visited: &mut HashSet<i64>,
    recursion_stack: &mut HashSet<i64>,
) -> bool {
    visited.insert(node);
    recursion_stack.insert(node);

    if let Some(neighbors) = adj.get(&node) {
        for &neighbor in neighbors {
            if !visited.contains(&neighbor) {
                if has_cycle_util(neighbor, adj, visited, recursion_stack) {
                    return true;
                }
            } else if recursion_stack.contains(&neighbor) {
                return true;
            }
        }
    }

    recursion_stack.remove(&node);
    false
}

pub fn find_components(edges: Vec<UnifiedEdge>) -> Vec<Vec<i64>> {
    let mut adj = HashMap::new();
    let mut nodes = HashSet::new();
    for edge in &edges {
        adj.entry(edge.from_id).or_insert(vec![]).push(edge.to_id);
        adj.entry(edge.to_id).or_insert(vec![]).push(edge.from_id);
        nodes.insert(edge.from_id);
        nodes.insert(edge.to_id);
    }

    let mut visited = HashSet::new();
    let mut components = vec![];

    for node in nodes {
        if !visited.contains(&node) {
            let mut component = vec![];
            let mut stack = vec![node];
            visited.insert(node);

            while let Some(curr) = stack.pop() {
                component.push(curr);
                if let Some(neighbors) = adj.get(&curr) {
                    for &neighbor in neighbors {
                        if !visited.contains(&neighbor) {
                            visited.insert(neighbor);
                            stack.push(neighbor);
                        }
                    }
                }
            }
            components.push(component);
        }
    }

    components
}
