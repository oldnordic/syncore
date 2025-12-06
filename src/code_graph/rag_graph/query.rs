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
    pub fn query_with_request(&self, request: &RagGraphQueryRequest) -> Option<RagGraphQueryResponse> {
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
        let filtered_matches = apply_scope_filter(
            &vector_matches,
            &scope,
            project_label,
            local_root,
        );

        // 3. Compute scores and rank entities
        let mut ranked_entities = Vec::new();
        let mut debug_info = HashMap::new();

        debug_info.insert("initial_matches".to_string(), vector_matches.len().to_string());
        debug_info.insert("filtered_matches".to_string(), filtered_matches.len().to_string());
        debug_info.insert("selected_mode".to_string(), format!("{:?}", selected_mode));

        for code_match in filtered_matches.iter().take(top_k) {
            let vector_score = code_match.score;

            // 4. Compute graph scores
            let graph_score = backend::compute_graph_score(&self.graph_backend, code_match.id).unwrap_or(0.0);

            // 5. Compute temporal score
            let temporal_score = super::super::fusion_simple::compute_temporal_score(&code_match.entity);

            // 6. Compute graph embedding score
            let graph_embedding_score = backend::get_entity_details(&self.graph_backend, code_match.id)
                .ok()
                .flatten()
                .map(|entity| super::super::fusion_simple::compute_graph_embedding_score(&entity))
                .unwrap_or(0.0);

            // 7. Apply fusion mode
            let combined_score = fusion_bridge::apply_selected_fusion(
                selected_mode,
                vector_score,
                graph_score,
                Some(temporal_score),
                Some(graph_embedding_score),
                query,
                &mut debug_info,
            );

            let ranked_entity = RankedEntity {
                entity_id: code_match.id,
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
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
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
    fn vector_search(&self, query: &str, top_k: usize) -> Option<Vec<crate::code_graph::types::CodeMatch>> {
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
                    id: hit.id,
                    score: hit.score,
                    entity: CodeEntity {
                        id: Some(hit.id),
                        name: format!("entity_{}", hit.id),
                        entity_type: crate::code_graph::types::EntityType::Function,
                        file_path: format!("/src/file_{}.rs", hit.id),
                        start_line: Some(1),
                        end_line: Some(10),
                        signature: Some("fn example()".to_string()),
                        body_snippet: None,
                        created_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                        updated_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                        ..Default::default()
                    },
                    match_type: crate::code_graph::types::MatchType::Exact,
                    context_lines: 0,
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
                matches
                    .iter()
                    .filter(|m| m.entity.file_path.starts_with(root))
                    .cloned()
                    .collect()
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
fn matches_project(file_path: &str, project_label: &str) -> bool {
    // Simple heuristic: check if project label appears in path
    // In a real implementation, this would use project metadata
    file_path.contains(project_label) || file_path.contains(&project_label.to_lowercase())
}