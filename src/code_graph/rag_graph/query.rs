//! Query processing for RAGGraph API
//!
//! Handles query coordination, scope filtering, and result processing.

use super::super::types::{CodeEntity, QueryScope};
use super::backend;
use super::fusion_bridge;
use super::multihop;
use super::{FusionMode, RagGraphAPI, RagGraphQueryRequest, RagGraphQueryResponse, RankedEntity};
use crate::graph::GraphBackend;
use crate::vector::VectorStore;
use std::collections::HashMap;
use std::sync::Mutex;

impl RagGraphAPI {
    /// Execute a RAGGraph query
    ///
    /// # Arguments
    /// * `query` - Text query to search for
    /// * `namespace` - Optional namespace for scoped search
    /// * `mode_hint` - Optional fusion mode hint ("simple", "attention", "reasoning")
    /// * `top_k` - Maximum number of results to return
    ///
    /// # Returns
    /// RagGraphQueryResponse with ranked entities and debug info
    pub fn query(
        &self,
        query: &str,
        namespace: Option<&str>,
        mode_hint: Option<&str>,
        top_k: usize,
    ) -> Option<RagGraphQueryResponse> {
        self.query_with_scope(query, namespace, mode_hint, top_k, QueryScope::Global, None, None)
    }

    /// Execute a RAGGraph query using a request struct
    ///
    /// # Arguments
    /// * `request` - Complete query request with all parameters
    ///
    /// # Returns
    /// RagGraphQueryResponse with ranked entities and debug info
    pub fn query_with_request(
        &self,
        request: &RagGraphQueryRequest,
    ) -> Option<RagGraphQueryResponse> {
        let scope = parse_scope_hint(&request.scope);
        let top_k = request.top_k.unwrap_or(10) as usize;
        let mode_hint = request.mode_hint.as_deref();

        self.query_with_scope(
            &request.query,
            request.namespace.as_deref(),
            mode_hint,
            top_k,
            scope,
            request.project_label.as_deref(),
            request.local_root.as_deref(),
        )
    }

    /// Execute a RAGGraph query with full parameter control
    pub fn query_with_scope(
        &self,
        query: &str,
        namespace: Option<&str>,
        mode_hint: Option<&str>,
        top_k: usize,
        scope: QueryScope,
        project_label: Option<&str>,
        local_root: Option<&str>,
    ) -> Option<RagGraphQueryResponse> {
        // Auto-select fusion mode if hint provided
        let selected_mode = if let Some(hint) = mode_hint {
            match hint {
                "simple" => FusionMode::Simple,
                "attention" => FusionMode::Attention,
                "reasoning" => FusionMode::Reasoning,
                _ => self.router.select_mode(query),
            }
        } else {
            self.router.select_mode(query)
        };

        // 1. Vector search for initial candidates
        let vector_matches = self.vector_search(query, top_k * 2)?;

        // 2. Filter matches based on scope
        let filtered_matches =
            apply_scope_filter(&vector_matches, &scope, project_label, local_root);

        // 3. Compute scores and rank entities
        let mut ranked_entities = Vec::new();
        let mut debug_info = HashMap::new();

        debug_info.insert("initial_matches".to_string(), vector_matches.len().to_string());
        debug_info.insert("filtered_matches".to_string(), filtered_matches.len().to_string());
        debug_info.insert("selected_mode".to_string(), format!("{:?}", selected_mode));

        for code_match in filtered_matches.iter().take(top_k) {
            let vector_score = code_match.score;

            // 4. Compute graph scores
            let graph_score = code_match
                .entity
                .id
                .map(|id| backend::compute_graph_score(&self.graph_backend, id).unwrap_or(0.0))
                .unwrap_or(0.0);

            // 5. Compute temporal score
            let temporal_score = super::super::fusion_simple::compute_temporal_score(
                code_match.entity.last_modified_at.unwrap_or(0),
                code_match.entity.change_count.unwrap_or(0),
                code_match.entity.author_count.unwrap_or(0),
            );

            // 6. Compute graph embedding score
            let graph_embedding_score = code_match
                .entity
                .id
                .and_then(|id| backend::get_entity_details(&self.graph_backend, id).ok())
                .flatten()
                .map(|_entity| 0.0) // TODO: Implement proper GraphFeatures extraction
                .unwrap_or(0.0);

            // 6.5. Compute recency score from created_at timestamp
            let recency_score = super::super::fusion_simple::extract_recency_score_from_timestamp(
                code_match.entity.created_at
            );

            // 7. Apply fusion mode
            let combined_score = fusion_bridge::apply_selected_fusion(
                selected_mode,
                vector_score,
                graph_score,
                Some(temporal_score),
                Some(graph_embedding_score),
                Some(recency_score),
                query,
                &mut debug_info,
            );

            let ranked_entity = RankedEntity {
                entity_id: code_match.entity.id.unwrap_or(0),
                relevance_score: combined_score,
                entity_type: format!("{:?}", code_match.entity.entity_type),
                file_path: code_match.entity.file_path.clone(),
                name: code_match.entity.name.clone(),
                signature: code_match.entity.signature.clone(),
                temporal_score: Some(temporal_score),
                graph_score: Some(graph_score),
                graph_embedding_score: Some(graph_embedding_score),
            };

            ranked_entities.push(ranked_entity);
        }

        // 8. Sort by final relevance score
        ranked_entities.sort_by(|a, b| {
            b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit to top_k results
        ranked_entities.truncate(top_k);

        Some(RagGraphQueryResponse {
            entities: ranked_entities,
            selected_mode: format!("{:?}", selected_mode),
            applied_scope: scope,
            debug_info,
            query: query.to_string(),
        })
    }

    /// Perform vector search for initial candidates
    fn vector_search(
        &self,
        query: &str,
        top_k: usize,
    ) -> Option<Vec<crate::code_graph::types::CodeMatch>> {
        let store = self.code_graph.vector_store.lock().ok()?;
        let results = store.search(query, top_k, crate::vector::SearchScope::Global).ok()?;

        // Convert vector search results to CodeMatch
        let matches: Vec<crate::code_graph::types::CodeMatch> = results
            .into_iter()
            .enumerate()
            .filter_map(|(idx, hit)| {
                // Create a mock CodeMatch for now - in real implementation,
                // this would fetch the actual entity details
                Some(crate::code_graph::types::CodeMatch {
                    entity: CodeEntity {
                        id: Some(hit.id),
                        name: format!("entity_{}", hit.id),
                        entity_type: crate::code_graph::types::EntityType::Function,
                        file_path: format!("/src/file_{}.rs", hit.id),
                        signature: Some("fn example()".to_string()),
                        line_start: 1,
                        line_end: 10,
                        docstring: None,
                        language: "rust".to_string(),
                        body_snippet: None,
                        created_at: Some(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64,
                        ),
                        last_modified_at: Some(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64,
                        ),
                        change_count: None,
                        author_count: None,
                    },
                    match_type: crate::code_graph::types::MatchType::Semantic,
                    score: hit.score,
                })
            })
            .collect();

        if matches.is_empty() {
            None
        } else {
            Some(matches)
        }
    }
}

/// Parse scope hint from string
fn parse_scope_hint(scope_hint: &Option<String>) -> QueryScope {
    match scope_hint.as_deref() {
        Some("local") => QueryScope::Local,
        Some("project") => QueryScope::Project,
        Some("workspace") => QueryScope::Workspace,
        Some("global") => QueryScope::Global,
        Some("auto") | _ => QueryScope::Project, // Default to project scope
    }
}

/// Filter matches based on scope rules
pub fn apply_scope_filter(
    matches: &[crate::code_graph::types::CodeMatch],
    scope: &QueryScope,
    project_label: Option<&str>,
    local_root: Option<&str>,
) -> Vec<crate::code_graph::types::CodeMatch> {
    match scope {
        QueryScope::Local => {
            if let Some(root) = local_root {
                matches.iter().filter(|m| m.entity.file_path.starts_with(root)).cloned().collect()
            } else {
                // If no local_root provided, treat as project scope
                matches.to_vec()
            }
        }
        QueryScope::Project => {
            if let Some(label) = project_label {
                matches
                    .iter()
                    .filter(|m| matches_project(&m.entity.file_path, label))
                    .cloned()
                    .collect()
            } else {
                // TODO: Filter to workspace projects when workspace concept is implemented
                matches.to_vec()
            }
        }
        QueryScope::Workspace => {
            // TODO: Implement workspace filtering when workspace concept is ready
            matches.to_vec()
        }
        QueryScope::Global => {
            // No filtering for global scope
            matches.to_vec()
        }
        QueryScope::Auto => {
            // Auto currently aliases to Project - use same logic
            if let Some(label) = project_label {
                matches
                    .iter()
                    .filter(|m| matches_project(&m.entity.file_path, label))
                    .cloned()
                    .collect()
            } else {
                // TODO: Filter to workspace projects when workspace concept is implemented
                matches.to_vec()
            }
        }
    }
}

/// Check if file belongs to a specific project
/// Enhanced with more robust project matching logic
fn matches_project(file_path: &str, project_label: &str) -> bool {
    if project_label.is_empty() {
        return true; // Empty project label matches everything
    }

    let file_path_normalized = file_path.to_lowercase();
    let project_label_normalized = project_label.to_lowercase();

    // Enhanced matching strategy:
    // 1. Direct project folder match (highest priority)
    if file_path_normalized.contains(&format!("/{}/", project_label_normalized)) ||
       file_path_normalized.contains(&format!("\\{}\\", project_label_normalized)) {
        return true;
    }

    // 2. Project folder as path component (end of path component)
    let path_components: Vec<&str> = file_path.split(['/', '\\']).collect();
    for component in &path_components {
        if component.to_lowercase() == project_label_normalized {
            return true;
        }
    }

    // 3. Original substring match (legacy fallback)
    if file_path_normalized.contains(&project_label_normalized) {
        return true;
    }

    // 4. Case-sensitive exact match (for case-sensitive systems)
    if file_path.contains(project_label) {
        return true;
    }

    false
}

/// Enhanced project matching with namespace support
/// Used when both project_label and namespace are available
fn matches_project_with_namespace(file_path: &str, project_label: &str, namespace: Option<&str>) -> bool {
    // If no project label, don't filter
    if project_label.is_empty() {
        return true;
    }

    // Primary matching by project label
    if !matches_project(file_path, project_label) {
        return false;
    }

    // If namespace is provided, use it as additional filter
    if let Some(ns) = namespace {
        if !ns.is_empty() {
            let namespace_normalized = ns.to_lowercase();
            let file_path_normalized = file_path.to_lowercase();

            // Namespace should appear as a path component or similar pattern
            if file_path_normalized.contains(&namespace_normalized) {
                return true;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== PROJECT MATCHING TESTS ==========

    #[test]
    fn test_matches_project_exact_path_match() {
        // Test direct path component matching
        assert!(matches_project("/home/user/syncore/src/main.rs", "syncore"));
        assert!(matches_project("/home/user/odincode/lib/app.rs", "odincode"));
        assert!(matches_project("C:\\Projects\\my-app\\src\\main.rs", "my-app"));
    }

    #[test]
    fn test_matches_project_case_insensitive() {
        // Test case-insensitive matching
        assert!(matches_project("/home/user/SyncCore/src/main.rs", "syncore"));
        assert!(matches_project("/home/user/SYNCORE/src/main.rs", "syncore"));
        assert!(matches_project("/home/user/My-App/src/main.rs", "my-app"));
    }

    #[test]
    fn test_matches_project_path_components() {
        // Test matching against path components
        assert!(matches_project("/usr/local/src/syncore/lib.rs", "syncore"));
        assert!(matches_project("relative/path/to/project/src/file.rs", "project"));

        // Should not match partial components
        assert!(!matches_project("/home/user/syncore-v2/src/main.rs", "syncore"));
    }

    #[test]
    fn test_matches_project_substring_fallback() {
        // Test legacy substring matching (should work for valid cases)
        assert!(matches_project("/home/user/my-syncore-project/src/main.rs", "syncore"));
        assert!(matches_project("syncore-backup/main.rs", "syncore"));
    }

    #[test]
    fn test_matches_project_edge_cases() {
        // Test edge cases
        assert!(matches_project("", "")); // Empty matches everything
        assert!(!matches_project("/some/random/path.rs", "")); // Empty project label matches everything
        assert!(!matches_project("", "nonexistent")); // Empty path doesn't match non-empty project

        // Test case-sensitive exact match fallback
        assert!(matches_project("/home/user/SyncCore/src/main.rs", "SyncCore"));
        assert!(!matches_project("/home/user/SyncCore/src/main.rs", "syncored")); // Should not match substring
    }

    #[test]
    fn test_matches_project_windows_paths() {
        // Test Windows path handling
        assert!(matches_project(r"C:\Projects\syncore\src\main.rs", "syncore"));
        assert!(matches_project(r"C:\Projects\MyApp\lib\app.js", "MyApp"));

        // Test mixed separators
        assert!(matches_project(r"C:/Projects/syncore/src/main.rs", "syncore"));
    }

    #[test]
    fn test_matches_project_with_namespace() {
        // Test namespace-aware matching
        assert!(matches_project_with_namespace(
            "/home/user/syncore/src/main.rs",
            "syncore",
            Some("syncore")
        ));

        assert!(matches_project_with_namespace(
            "/home/user/odincode/lib/app.rs",
            "odincode",
            Some("odincode")
        ));

        // Should not match when project label doesn't match
        assert!(!matches_project_with_namespace(
            "/home/user/syncore/src/main.rs",
            "odincode",
            Some("syncore")
        ));

        // Should match with project label but no namespace
        assert!(matches_project_with_namespace(
            "/home/user/syncore/src/main.rs",
            "syncore",
            None
        ));

        // Should match everything with empty project label
        assert!(matches_project_with_namespace(
            "/some/random/path.rs",
            "",
            Some("namespace")
        ));
    }

    #[test]
    fn test_query_scope_filtering_with_project() {
        // Test that query scope filtering works with project matching

        let query_scope = QueryScope::Project;
        let project_label = Some("syncore");

        let mock_matches = vec![
            crate::code_graph::types::CodeMatch {
                entity: crate::code_graph::types::CodeEntity {
                    file_path: "/home/user/syncore/src/main.rs".to_string(),
                    ..Default::default()
                },
                score: 0.8,
                context: "test".to_string(),
            },
            crate::code_graph::types::CodeMatch {
                entity: crate::code_graph::types::CodeEntity {
                    file_path: "/home/user/other-project/src/lib.rs".to_string(),
                    ..Default::default()
                },
                score: 0.7,
                context: "test".to_string(),
            },
        ];

        let filtered = apply_scope_filter(&mock_matches, &query_scope, project_label, None);

        // Should only include syncore files
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].entity.file_path.contains("syncore"));
    }

    #[test]
    fn test_query_scope_filtering_global_scope() {
        // Test that global scope doesn't filter by project

        let query_scope = QueryScope::Global;
        let project_label = Some("syncore");

        let mock_matches = vec![
            crate::code_graph::types::CodeMatch {
                entity: crate::code_graph::types::CodeEntity {
                    file_path: "/home/user/syncore/src/main.rs".to_string(),
                    ..Default::default()
                },
                score: 0.8,
                context: "test".to_string(),
            },
            crate::code_graph::types::CodeMatch {
                entity: crate::code_graph::types::CodeEntity {
                    file_path: "/home/user/other-project/src/lib.rs".to_string(),
                    ..Default::default()
                },
                score: 0.7,
                context: "test".to_string(),
            },
        ];

        let filtered = apply_scope_filter(&mock_matches, &query_scope, project_label, None);

        // Global scope should include everything
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_query_scope_filtering_auto_scope() {
        // Test that auto scope behaves like project scope when project label is present

        let query_scope = QueryScope::Auto;
        let project_label = Some("syncore");

        let mock_matches = vec![
            crate::code_graph::types::CodeMatch {
                entity: crate::code_graph::types::CodeEntity {
                    file_path: "/home/user/syncore/src/main.rs".to_string(),
                    ..Default::default()
                },
                score: 0.8,
                context: "test".to_string(),
            },
            crate::code_graph::types::CodeMatch {
                entity: crate::code_graph::types::CodeEntity {
                    file_path: "/home/user/other-project/src/lib.rs".to_string(),
                    ..Default::default()
                },
                score: 0.7,
                context: "test".to_string(),
            },
        ];

        let filtered = apply_scope_filter(&mock_matches, &query_scope, project_label, None);

        // Auto should filter by project when project label is available
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].entity.file_path.contains("syncore"));
    }
}
