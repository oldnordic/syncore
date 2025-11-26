//! Unified Dependency Analysis for Multi-Language Projects
//!
//! Provides a unified view of dependencies across all languages indexed by SynCore.
//! Aggregates code_entities and code_edges into module-level dependency maps.

use std::collections::HashMap;

use super::ProjectAnalysisEngine;
use serde::{Deserialize, Serialize};

/// Unified module representation across all languages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedModule {
    pub file_path: String,
    pub language: String,
    pub entity_count: u32,
    pub incoming_edges: u32,
    pub outgoing_edges: u32,
}

/// Unified dependency between modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedDependency {
    pub from_file: String,
    pub from_language: String,
    pub to_file: String,
    pub to_language: String,
    pub edge_count: u32,
    pub edge_types: Vec<String>,
}

/// Complete unified dependency summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedDependencySummary {
    pub modules: Vec<UnifiedModule>,
    pub dependencies: Vec<UnifiedDependency>,
}

/// Internal aggregation key for modules
type ModuleKey = (String, String); // (file_path, language)

/// Internal aggregation data for dependencies
#[derive(Debug, Default)]
struct DependencyAggregation {
    edge_count: u32,
    edge_types: std::collections::HashSet<String>,
}

impl ProjectAnalysisEngine {
    /// Build unified dependency summary across all languages
    ///
    /// Aggregates entities and edges from code_entities and code_edges tables
    /// into module-level dependency maps.
    ///
    /// # Arguments
    /// * `max_modules` - Optional limit on number of modules to return
    /// * `min_edge_count` - Optional minimum edge count for dependencies
    ///
    /// # Returns
    /// `UnifiedDependencySummary` containing modules and their dependencies
    pub fn build_unified_dependency_summary(
        &self,
        max_modules: Option<u32>,
        min_edge_count: Option<u32>,
    ) -> anyhow::Result<UnifiedDependencySummary> {
        let conn = self.code_graph_conn();
        let db = conn.lock().unwrap();

        // Query all entities grouped by (file_path, language)
        let mut entity_stmt = db.prepare(
            r#"
            SELECT file_path, language, COUNT(*) as entity_count
            FROM code_entities
            GROUP BY file_path, language
            ORDER BY file_path, language
            "#,
        )?;

        let entity_rows = entity_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?, // file_path
                row.get::<_, String>(1)?, // language
                row.get::<_, u32>(2)?,    // entity_count
            ))
        })?;

        // Build initial modules map
        let mut modules: HashMap<ModuleKey, UnifiedModule> = HashMap::new();
        for row_result in entity_rows {
            let (file_path, language, entity_count) = row_result?;
            let key = (file_path.clone(), language.clone());

            modules.insert(
                key,
                UnifiedModule {
                    file_path,
                    language,
                    entity_count,
                    incoming_edges: 0,
                    outgoing_edges: 0,
                },
            );
        }

        // Query all edges with entity information
        let mut edge_stmt = db.prepare(
            r#"
            SELECT 
                src.file_path as src_file,
                src.language as src_lang,
                dst.file_path as dst_file,
                dst.language as dst_lang,
                e.edge_type,
                COUNT(*) as edge_count
            FROM code_edges e
            JOIN code_entities src ON e.src_entity_id = src.id
            JOIN code_entities dst ON e.dst_entity_id = dst.id
            GROUP BY src.file_path, src.language, dst.file_path, dst.language, e.edge_type
            ORDER BY src.file_path, dst.file_path
            "#,
        )?;

        let edge_rows = edge_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?, // src_file
                row.get::<_, String>(1)?, // src_lang
                row.get::<_, String>(2)?, // dst_file
                row.get::<_, String>(3)?, // dst_lang
                row.get::<_, String>(4)?, // edge_type
                row.get::<_, u32>(5)?,    // edge_count
            ))
        })?;

        // Aggregate dependencies by module pair
        let mut dependencies: HashMap<(ModuleKey, ModuleKey), DependencyAggregation> =
            HashMap::new();
        let mut incoming_counts: HashMap<ModuleKey, u32> = HashMap::new();
        let mut outgoing_counts: HashMap<ModuleKey, u32> = HashMap::new();

        for row_result in edge_rows {
            let (src_file, src_lang, dst_file, dst_lang, edge_type, edge_count) = row_result?;

            let src_key = (src_file.clone(), src_lang.clone());
            let dst_key = (dst_file.clone(), dst_lang.clone());
            let dep_key = (src_key.clone(), dst_key.clone());

            // Aggregate edge data
            let agg = dependencies.entry(dep_key).or_default();
            agg.edge_count += edge_count;
            agg.edge_types.insert(edge_type);

            // Update degree counts
            *outgoing_counts.entry(src_key).or_insert(0) += edge_count;
            *incoming_counts.entry(dst_key).or_insert(0) += edge_count;
        }

        // Update modules with degree information
        for (key, module) in modules.iter_mut() {
            module.incoming_edges = incoming_counts.get(key).copied().unwrap_or(0);
            module.outgoing_edges = outgoing_counts.get(key).copied().unwrap_or(0);
        }

        // Convert dependencies to final structure
        let mut final_dependencies: Vec<UnifiedDependency> = dependencies
            .into_iter()
            .filter_map(|((src_key, dst_key), agg)| {
                // Apply min_edge_count filter
                if let Some(min_count) = min_edge_count {
                    if agg.edge_count < min_count {
                        return None;
                    }
                }

                Some(UnifiedDependency {
                    from_file: src_key.0,
                    from_language: src_key.1,
                    to_file: dst_key.0,
                    to_language: dst_key.1,
                    edge_count: agg.edge_count,
                    edge_types: agg.edge_types.into_iter().collect(),
                })
            })
            .collect();

        // Sort dependencies by edge count (descending)
        final_dependencies.sort_by(|a, b| b.edge_count.cmp(&a.edge_count));

        // Convert modules to final structure and apply max_modules filter
        let mut final_modules: Vec<UnifiedModule> = modules.into_values().collect();

        // Sort by total degree (incoming + outgoing) for prioritization
        final_modules.sort_by(|a, b| {
            let total_a = a.incoming_edges + a.outgoing_edges;
            let total_b = b.incoming_edges + b.outgoing_edges;
            total_b.cmp(&total_a)
        });

        // Apply max_modules limit
        if let Some(max_mods) = max_modules {
            final_modules.truncate(max_mods as usize);
        }

        Ok(UnifiedDependencySummary {
            modules: final_modules,
            dependencies: final_dependencies,
        })
    }
}
