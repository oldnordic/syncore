use crate::polyglot::polyglot_model::UnifiedEntity;
use std::collections::HashMap;

pub fn calculate_complexity(entities: Vec<UnifiedEntity>) -> HashMap<String, usize> {
    let mut complexity_map = HashMap::new();
    for entity in entities {
        *complexity_map.entry(entity.file_path.clone()).or_insert(0) += 1;
    }
    complexity_map
}
