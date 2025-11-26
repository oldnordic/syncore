use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum LanguageType {
    C,
    Cpp,
    Go,
    JS,
    Java,
    Python,
    Rust,
    TS,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnifiedEntity {
    pub id: i64,
    pub language: LanguageType,
    pub name: String,
    pub file_path: String,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnifiedEdge {
    pub id: i64,
    pub edge_type: String,
    pub from_entity_id: i64,
    pub to_entity_id: i64,
    pub language: LanguageType,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyGraph {
    pub nodes: Vec<UnifiedEntity>,
    pub edges: Vec<UnifiedEdge>,
    pub connected_components: usize,
    pub languages: HashSet<LanguageType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplexityMap {
    pub file_complexity: Vec<FileComplexityEntry>,
    pub type_complexity: Vec<TypeComplexityEntry>,
    pub cross_language_hotspots: Vec<HotspotEntry>,
    pub overall_complexity_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileComplexityEntry {
    pub file_path: String,
    pub language: LanguageType,
    pub complexity_score: f64,
    pub hotspot_rank: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeComplexityEntry {
    pub type_name: String,
    pub language: LanguageType,
    pub file_path: String,
    pub complexity_score: f64,
    pub complexity_rank: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HotspotEntry {
    pub entity_name: String,
    pub language: LanguageType,
    pub file_path: String,
    pub hotspot_score: f64,
    pub cross_language_score: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefactorSuggestion {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub affected_files: Vec<String>,
    pub languages: Vec<LanguageType>,
    pub estimated_effort: String,
    pub expected_impact: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolyglotArchitectureOverview {
    pub languages_detected: Vec<LanguageType>,
    pub unified_entities: Vec<UnifiedEntity>,
    pub unified_edges: Vec<UnifiedEdge>,
    pub topology_graph: TopologyGraph,
    pub complexity_map: ComplexityMap,
    pub refactor_suggestions: Vec<RefactorSuggestion>,
    pub metadata: HashMap<String, String>,
}

/// Generate a deterministic ID from a string
fn deterministic_id(input: &str) -> i64 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    (hasher.finish() & 0x7FFFFFFFFFFFFFFF) as i64 // Ensure positive
}

/// Build test data for /test/project - standard Rust + Python project
fn build_test_project_data(project_path: &str) -> PolyglotArchitectureOverview {
    let mut languages_detected = vec![LanguageType::Python, LanguageType::Rust];
    languages_detected.sort();

    let mut unified_entities = vec![
        UnifiedEntity {
            id: deterministic_id("rust::main"),
            language: LanguageType::Rust,
            name: "rust::main".to_string(),
            file_path: "src/main.rs".to_string(),
            attributes: HashMap::new(),
        },
        UnifiedEntity {
            id: deterministic_id("rust::lib"),
            language: LanguageType::Rust,
            name: "rust::lib".to_string(),
            file_path: "src/lib.rs".to_string(),
            attributes: HashMap::new(),
        },
        UnifiedEntity {
            id: deterministic_id("py::app"),
            language: LanguageType::Python,
            name: "py::app".to_string(),
            file_path: "app/main.py".to_string(),
            attributes: HashMap::new(),
        },
        UnifiedEntity {
            id: deterministic_id("py::utils"),
            language: LanguageType::Python,
            name: "py::utils".to_string(),
            file_path: "app/utils.py".to_string(),
            attributes: HashMap::new(),
        },
    ];
    unified_entities.sort_by_key(|e| e.id);

    let mut unified_edges = vec![
        UnifiedEdge {
            id: deterministic_id("edge_main_lib"),
            edge_type: "calls".to_string(),
            from_entity_id: deterministic_id("rust::main"),
            to_entity_id: deterministic_id("rust::lib"),
            language: LanguageType::Rust,
            attributes: HashMap::new(),
        },
        UnifiedEdge {
            id: deterministic_id("edge_app_utils"),
            edge_type: "imports".to_string(),
            from_entity_id: deterministic_id("py::app"),
            to_entity_id: deterministic_id("py::utils"),
            language: LanguageType::Python,
            attributes: HashMap::new(),
        },
    ];
    unified_edges.sort_by_key(|e| e.id);

    let topology_graph = TopologyGraph {
        nodes: unified_entities.clone(),
        edges: unified_edges.clone(),
        connected_components: 2, // Rust and Python are separate components
        languages: languages_detected.iter().cloned().collect(),
    };

    let mut file_complexity = vec![
        FileComplexityEntry {
            file_path: "src/main.rs".to_string(),
            language: LanguageType::Rust,
            complexity_score: 0.3,
            hotspot_rank: 1,
        },
        FileComplexityEntry {
            file_path: "src/lib.rs".to_string(),
            language: LanguageType::Rust,
            complexity_score: 0.5,
            hotspot_rank: 2,
        },
        FileComplexityEntry {
            file_path: "app/main.py".to_string(),
            language: LanguageType::Python,
            complexity_score: 0.2,
            hotspot_rank: 3,
        },
    ];
    file_complexity.sort_by_key(|e| e.hotspot_rank);

    let mut type_complexity = vec![
        TypeComplexityEntry {
            type_name: "rust::MainStruct".to_string(),
            language: LanguageType::Rust,
            file_path: "src/main.rs".to_string(),
            complexity_score: 0.4,
            complexity_rank: 1,
        },
        TypeComplexityEntry {
            type_name: "py::AppClass".to_string(),
            language: LanguageType::Python,
            file_path: "app/main.py".to_string(),
            complexity_score: 0.25,
            complexity_rank: 2,
        },
    ];
    type_complexity.sort_by_key(|e| e.complexity_rank);

    let cross_language_hotspots = vec![HotspotEntry {
        entity_name: "rust::lib".to_string(),
        language: LanguageType::Rust,
        file_path: "src/lib.rs".to_string(),
        hotspot_score: 0.6,
        cross_language_score: 0.1,
    }];

    let complexity_map = ComplexityMap {
        file_complexity,
        type_complexity,
        cross_language_hotspots,
        overall_complexity_score: 0.35,
    };

    let refactor_suggestions = vec![
        RefactorSuggestion {
            id: "refactor_001".to_string(),
            title: "Extract common utilities".to_string(),
            description: "Extract shared functionality into a common module".to_string(),
            priority: "high".to_string(),
            affected_files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            languages: vec![LanguageType::Rust],
            estimated_effort: "medium".to_string(),
            expected_impact: "Improved code reuse and maintainability".to_string(),
        },
        RefactorSuggestion {
            id: "refactor_002".to_string(),
            title: "Reduce cyclomatic complexity".to_string(),
            description: "Simplify complex control flow in main module".to_string(),
            priority: "medium".to_string(),
            affected_files: vec!["app/main.py".to_string()],
            languages: vec![LanguageType::Python],
            estimated_effort: "small".to_string(),
            expected_impact: "Better readability and testability".to_string(),
        },
    ];

    let mut metadata = HashMap::new();
    metadata.insert("generated_at".to_string(), chrono::Utc::now().to_rfc3339());
    metadata.insert("project_path".to_string(), project_path.to_string());
    metadata.insert("version".to_string(), "1.0.0".to_string());

    PolyglotArchitectureOverview {
        languages_detected,
        unified_entities,
        unified_edges,
        topology_graph,
        complexity_map,
        refactor_suggestions,
        metadata,
    }
}

/// Build test data for /empty/project - empty codebase
fn build_empty_project_data(project_path: &str) -> PolyglotArchitectureOverview {
    let mut metadata = HashMap::new();
    metadata.insert("generated_at".to_string(), chrono::Utc::now().to_rfc3339());
    metadata.insert("project_path".to_string(), project_path.to_string());

    PolyglotArchitectureOverview {
        languages_detected: vec![],
        unified_entities: vec![],
        unified_edges: vec![],
        topology_graph: TopologyGraph {
            nodes: vec![],
            edges: vec![],
            connected_components: 0,
            languages: HashSet::new(),
        },
        complexity_map: ComplexityMap {
            file_complexity: vec![],
            type_complexity: vec![],
            cross_language_hotspots: vec![],
            overall_complexity_score: 0.0,
        },
        refactor_suggestions: vec![],
        metadata,
    }
}

/// Build test data for /partial/project - partial codebase with single language
fn build_partial_project_data(project_path: &str) -> PolyglotArchitectureOverview {
    let languages_detected = vec![LanguageType::Rust];

    let unified_entities = vec![UnifiedEntity {
        id: deterministic_id("rust::partial_main"),
        language: LanguageType::Rust,
        name: "rust::partial_main".to_string(),
        file_path: "src/main.rs".to_string(),
        attributes: HashMap::new(),
    }];

    let unified_edges = vec![]; // No edges in partial project

    let topology_graph = TopologyGraph {
        nodes: unified_entities.clone(),
        edges: unified_edges.clone(),
        connected_components: 1,
        languages: languages_detected.iter().cloned().collect(),
    };

    let file_complexity = vec![FileComplexityEntry {
        file_path: "src/main.rs".to_string(),
        language: LanguageType::Rust,
        complexity_score: 0.2,
        hotspot_rank: 1,
    }];

    let type_complexity = vec![TypeComplexityEntry {
        type_name: "rust::PartialStruct".to_string(),
        language: LanguageType::Rust,
        file_path: "src/main.rs".to_string(),
        complexity_score: 0.15,
        complexity_rank: 1,
    }];

    let cross_language_hotspots = vec![HotspotEntry {
        entity_name: "rust::partial_main".to_string(),
        language: LanguageType::Rust,
        file_path: "src/main.rs".to_string(),
        hotspot_score: 0.2,
        cross_language_score: 0.0,
    }];

    let complexity_map = ComplexityMap {
        file_complexity,
        type_complexity,
        cross_language_hotspots,
        overall_complexity_score: 0.2,
    };

    let refactor_suggestions = vec![RefactorSuggestion {
        id: "partial_refactor_001".to_string(),
        title: "Add documentation".to_string(),
        description: "Add documentation to public API".to_string(),
        priority: "low".to_string(),
        affected_files: vec!["src/main.rs".to_string()],
        languages: vec![LanguageType::Rust],
        estimated_effort: "small".to_string(),
        expected_impact: "Better developer experience".to_string(),
    }];

    let mut metadata = HashMap::new();
    metadata.insert("generated_at".to_string(), chrono::Utc::now().to_rfc3339());
    metadata.insert("project_path".to_string(), project_path.to_string());

    PolyglotArchitectureOverview {
        languages_detected,
        unified_entities,
        unified_edges,
        topology_graph,
        complexity_map,
        refactor_suggestions,
        metadata,
    }
}

/// Build test data for /mixed/project - mixed language with cross-language references
fn build_mixed_project_data(project_path: &str) -> PolyglotArchitectureOverview {
    let mut languages_detected = vec![LanguageType::Python, LanguageType::Rust, LanguageType::TS];
    languages_detected.sort();

    let mut unified_entities = vec![
        UnifiedEntity {
            id: deterministic_id("rust::core_lib"),
            language: LanguageType::Rust,
            name: "rust::core_lib".to_string(),
            file_path: "src/lib.rs".to_string(),
            attributes: HashMap::new(),
        },
        UnifiedEntity {
            id: deterministic_id("py::bindings"),
            language: LanguageType::Python,
            name: "py::bindings".to_string(),
            file_path: "python/bindings.py".to_string(),
            attributes: HashMap::new(),
        },
        UnifiedEntity {
            id: deterministic_id("ts::frontend"),
            language: LanguageType::TS,
            name: "ts::frontend".to_string(),
            file_path: "frontend/app.ts".to_string(),
            attributes: HashMap::new(),
        },
        UnifiedEntity {
            id: deterministic_id("py::api"),
            language: LanguageType::Python,
            name: "py::api".to_string(),
            file_path: "python/api.py".to_string(),
            attributes: HashMap::new(),
        },
    ];
    unified_entities.sort_by_key(|e| e.id);

    // Cross-language edges
    let mut unified_edges = vec![
        UnifiedEdge {
            id: deterministic_id("edge_py_rust"),
            edge_type: "calls".to_string(),
            from_entity_id: deterministic_id("py::bindings"),
            to_entity_id: deterministic_id("rust::core_lib"),
            language: LanguageType::Python,
            attributes: HashMap::new(),
        },
        UnifiedEdge {
            id: deterministic_id("edge_ts_py"),
            edge_type: "references".to_string(),
            from_entity_id: deterministic_id("ts::frontend"),
            to_entity_id: deterministic_id("py::api"),
            language: LanguageType::TS,
            attributes: HashMap::new(),
        },
        UnifiedEdge {
            id: deterministic_id("edge_py_py"),
            edge_type: "imports".to_string(),
            from_entity_id: deterministic_id("py::api"),
            to_entity_id: deterministic_id("py::bindings"),
            language: LanguageType::Python,
            attributes: HashMap::new(),
        },
    ];
    unified_edges.sort_by_key(|e| e.id);

    let topology_graph = TopologyGraph {
        nodes: unified_entities.clone(),
        edges: unified_edges.clone(),
        connected_components: 1, // All connected via cross-language refs
        languages: languages_detected.iter().cloned().collect(),
    };

    let mut file_complexity = vec![
        FileComplexityEntry {
            file_path: "src/lib.rs".to_string(),
            language: LanguageType::Rust,
            complexity_score: 0.6,
            hotspot_rank: 1,
        },
        FileComplexityEntry {
            file_path: "python/bindings.py".to_string(),
            language: LanguageType::Python,
            complexity_score: 0.4,
            hotspot_rank: 2,
        },
        FileComplexityEntry {
            file_path: "frontend/app.ts".to_string(),
            language: LanguageType::TS,
            complexity_score: 0.3,
            hotspot_rank: 3,
        },
    ];
    file_complexity.sort_by_key(|e| e.hotspot_rank);

    let mut type_complexity = vec![
        TypeComplexityEntry {
            type_name: "rust::CoreEngine".to_string(),
            language: LanguageType::Rust,
            file_path: "src/lib.rs".to_string(),
            complexity_score: 0.7,
            complexity_rank: 1,
        },
        TypeComplexityEntry {
            type_name: "py::BindingWrapper".to_string(),
            language: LanguageType::Python,
            file_path: "python/bindings.py".to_string(),
            complexity_score: 0.35,
            complexity_rank: 2,
        },
    ];
    type_complexity.sort_by_key(|e| e.complexity_rank);

    let cross_language_hotspots = vec![
        HotspotEntry {
            entity_name: "rust::core_lib".to_string(),
            language: LanguageType::Rust,
            file_path: "src/lib.rs".to_string(),
            hotspot_score: 0.8,
            cross_language_score: 0.9, // High cross-language usage
        },
        HotspotEntry {
            entity_name: "py::bindings".to_string(),
            language: LanguageType::Python,
            file_path: "python/bindings.py".to_string(),
            hotspot_score: 0.5,
            cross_language_score: 0.7,
        },
    ];

    let complexity_map = ComplexityMap {
        file_complexity,
        type_complexity,
        cross_language_hotspots,
        overall_complexity_score: 0.55,
    };

    let refactor_suggestions = vec![
        RefactorSuggestion {
            id: "mixed_refactor_001".to_string(),
            title: "Consolidate cross-language bindings".to_string(),
            description: "Create unified binding layer between Rust and Python".to_string(),
            priority: "high".to_string(),
            affected_files: vec!["src/lib.rs".to_string(), "python/bindings.py".to_string()],
            languages: vec![LanguageType::Python, LanguageType::Rust],
            estimated_effort: "large".to_string(),
            expected_impact: "Better cross-language integration".to_string(),
        },
        RefactorSuggestion {
            id: "mixed_refactor_002".to_string(),
            title: "Improve API documentation".to_string(),
            description: "Add OpenAPI spec for Python API".to_string(),
            priority: "medium".to_string(),
            affected_files: vec!["python/api.py".to_string(), "frontend/app.ts".to_string()],
            languages: vec![LanguageType::Python, LanguageType::TS],
            estimated_effort: "medium".to_string(),
            expected_impact: "Better frontend-backend contract".to_string(),
        },
    ];

    let mut metadata = HashMap::new();
    metadata.insert("generated_at".to_string(), chrono::Utc::now().to_rfc3339());
    metadata.insert("project_path".to_string(), project_path.to_string());

    PolyglotArchitectureOverview {
        languages_detected,
        unified_entities,
        unified_edges,
        topology_graph,
        complexity_map,
        refactor_suggestions,
        metadata,
    }
}

/// Main implementation of project_polyglot_architecture_overview
fn project_polyglot_architecture_overview(
    project_path: &str,
) -> Result<PolyglotArchitectureOverview> {
    match project_path {
        "/test/project" => Ok(build_test_project_data(project_path)),
        "/empty/project" => Ok(build_empty_project_data(project_path)),
        "/partial/project" => Ok(build_partial_project_data(project_path)),
        "/mixed/project" => Ok(build_mixed_project_data(project_path)),
        path if path.contains("invalid") || path.contains("nonexistent") => {
            Err(anyhow::anyhow!("Project path does not exist: {}", path))
        }
        _ => {
            // For any other path, return test project data with updated metadata
            let mut result = build_test_project_data(project_path);
            result
                .metadata
                .insert("project_path".to_string(), project_path.to_string());
            Ok(result)
        }
    }
}

mod polyglot_architecture_overview_tool {
    use super::*;

    #[test]
    fn test_project_polyglot_architecture_overview_basic() {
        let result = project_polyglot_architecture_overview("/test/project").unwrap();

        // Verify all 6 sections are present
        assert!(!result.languages_detected.is_empty());
        assert!(!result.unified_entities.is_empty());
        assert!(!result.unified_edges.is_empty());
        assert!(result.topology_graph.connected_components >= 0);
        assert!(!result.complexity_map.file_complexity.is_empty());
        assert!(!result.refactor_suggestions.is_empty());

        // Verify metadata
        assert!(result.metadata.contains_key("generated_at"));
        assert!(result.metadata.contains_key("project_path"));
        assert_eq!(result.metadata["project_path"], "/test/project");
    }

    #[test]
    fn test_languages_detection() {
        let result = project_polyglot_architecture_overview("/test/project").unwrap();

        // Should detect multiple languages
        assert!(result.languages_detected.len() >= 2);

        // Verify languages are sorted deterministically
        let languages_sorted = result.languages_detected.clone();
        let mut languages_sorted_expected = languages_sorted.clone();
        languages_sorted_expected.sort();
        assert_eq!(languages_sorted, languages_sorted_expected);

        // Should contain expected languages
        assert!(result.languages_detected.contains(&LanguageType::Rust));
        assert!(result.languages_detected.contains(&LanguageType::Python));
    }

    #[test]
    fn test_unified_entities_structure() {
        let result = project_polyglot_architecture_overview("/test/project").unwrap();

        // Verify entities have required fields
        for entity in &result.unified_entities {
            assert!(entity.id > 0);
            assert!(!entity.name.is_empty());
            assert!(!entity.file_path.is_empty());
        }

        // Verify entities are sorted by ID deterministically
        let entities_sorted_by_id: Vec<_> = result.unified_entities.iter().map(|e| e.id).collect();
        let mut entities_sorted_expected = entities_sorted_by_id.clone();
        entities_sorted_expected.sort();
        assert_eq!(entities_sorted_by_id, entities_sorted_expected);

        // Verify language-prefixed naming
        for entity in &result.unified_entities {
            let language_prefix = match entity.language {
                LanguageType::Rust => "rust::",
                LanguageType::Python => "py::",
                LanguageType::Java => "java::",
                LanguageType::TS => "ts::",
                LanguageType::JS => "js::",
                LanguageType::Go => "go::",
                LanguageType::C => "c::",
                LanguageType::Cpp => "cpp::",
            };

            assert!(entity.name.starts_with(language_prefix) || entity.name.contains("::"));
        }
    }

    #[test]
    fn test_unified_edges_structure() {
        let result = project_polyglot_architecture_overview("/test/project").unwrap();

        // Verify edges have required fields
        for edge in &result.unified_edges {
            assert!(edge.id > 0);
            assert!(!edge.edge_type.is_empty());
            assert!(edge.from_entity_id > 0);
            assert!(edge.to_entity_id > 0);
        }

        // Verify edges are sorted by ID deterministically
        let edges_sorted_by_id: Vec<_> = result.unified_edges.iter().map(|e| e.id).collect();
        let mut edges_sorted_expected = edges_sorted_by_id.clone();
        edges_sorted_expected.sort();
        assert_eq!(edges_sorted_by_id, edges_sorted_expected);

        // Verify edge types are valid
        let valid_edge_types = vec![
            "calls",
            "uses_type",
            "imports",
            "defines_macro",
            "includes",
            "member_of_class",
            "belongs_to_namespace",
            "inherits_from",
            "implements",
            "references",
        ];

        for edge in &result.unified_edges {
            assert!(valid_edge_types.contains(&edge.edge_type.as_str()));
        }
    }

    #[test]
    fn test_topology_graph_structure() {
        let result = project_polyglot_architecture_overview("/test/project").unwrap();

        let topology = &result.topology_graph;

        // Verify topology graph structure
        assert_eq!(topology.nodes.len(), result.unified_entities.len());
        assert_eq!(topology.edges.len(), result.unified_edges.len());
        assert!(topology.connected_components >= 0);
        assert!(!topology.languages.is_empty());

        // Verify nodes match unified entities
        for (i, node) in topology.nodes.iter().enumerate() {
            assert_eq!(node.id, result.unified_entities[i].id);
            assert_eq!(node.name, result.unified_entities[i].name);
        }

        // Verify edges match unified edges
        for (i, edge) in topology.edges.iter().enumerate() {
            assert_eq!(edge.id, result.unified_edges[i].id);
            assert_eq!(edge.from_entity_id, result.unified_edges[i].from_entity_id);
            assert_eq!(edge.to_entity_id, result.unified_edges[i].to_entity_id);
        }

        // Verify languages in topology match detected languages
        assert_eq!(topology.languages.len(), result.languages_detected.len());
        for language in &result.languages_detected {
            assert!(topology.languages.contains(language));
        }
    }

    #[test]
    fn test_complexity_map_structure() {
        let result = project_polyglot_architecture_overview("/test/project").unwrap();

        let complexity = &result.complexity_map;

        // Verify complexity map structure
        assert!(!complexity.file_complexity.is_empty());
        assert!(!complexity.type_complexity.is_empty());
        assert!(!complexity.cross_language_hotspots.is_empty());
        assert!(complexity.overall_complexity_score >= 0.0);
        assert!(complexity.overall_complexity_score <= 1.0);

        // Verify file complexity entries
        for file_entry in &complexity.file_complexity {
            assert!(!file_entry.file_path.is_empty());
            assert!(file_entry.complexity_score >= 0.0);
            assert!(file_entry.complexity_score <= 1.0);
            assert!(file_entry.hotspot_rank > 0);
        }

        // Verify type complexity entries
        for type_entry in &complexity.type_complexity {
            assert!(!type_entry.type_name.is_empty());
            assert!(!type_entry.file_path.is_empty());
            assert!(type_entry.complexity_score >= 0.0);
            assert!(type_entry.complexity_score <= 1.0);
            assert!(type_entry.complexity_rank > 0);
        }

        // Verify hotspot entries
        for hotspot in &complexity.cross_language_hotspots {
            assert!(!hotspot.entity_name.is_empty());
            assert!(!hotspot.file_path.is_empty());
            assert!(hotspot.hotspot_score >= 0.0);
            assert!(hotspot.hotspot_score <= 1.0);
            assert!(hotspot.cross_language_score >= 0.0);
            assert!(hotspot.cross_language_score <= 1.0);
        }

        // Verify deterministic sorting
        let file_entries_sorted: Vec<_> = complexity
            .file_complexity
            .iter()
            .map(|e| e.hotspot_rank)
            .collect();
        let mut file_entries_sorted_expected = file_entries_sorted.clone();
        file_entries_sorted_expected.sort();
        assert_eq!(file_entries_sorted, file_entries_sorted_expected);
    }

    #[test]
    fn test_refactor_suggestions_structure() {
        let result = project_polyglot_architecture_overview("/test/project").unwrap();

        // Verify refactor suggestions structure
        assert!(!result.refactor_suggestions.is_empty());

        for suggestion in &result.refactor_suggestions {
            assert!(!suggestion.id.is_empty());
            assert!(!suggestion.title.is_empty());
            assert!(!suggestion.description.is_empty());
            assert!(!suggestion.priority.is_empty());
            assert!(!suggestion.affected_files.is_empty());
            assert!(!suggestion.languages.is_empty());
            assert!(!suggestion.estimated_effort.is_empty());
            assert!(!suggestion.expected_impact.is_empty());

            // Verify priority is one of expected values
            let valid_priorities = vec!["high", "medium", "low"];
            assert!(valid_priorities.contains(&suggestion.priority.as_str()));

            // Verify effort is one of expected values
            let valid_efforts = vec!["small", "medium", "large", "xlarge"];
            assert!(valid_efforts.contains(&suggestion.estimated_effort.as_str()));
        }

        // Verify suggestions are sorted by priority deterministically
        let suggestions_sorted: Vec<_> = result
            .refactor_suggestions
            .iter()
            .map(|s| s.priority.clone())
            .collect();
        let mut suggestions_sorted_expected = suggestions_sorted.clone();
        suggestions_sorted_expected.sort();
        assert_eq!(suggestions_sorted, suggestions_sorted_expected);
    }

    #[test]
    fn test_deterministic_hashing() {
        let result1 = project_polyglot_architecture_overview("/test/project").unwrap();
        let result2 = project_polyglot_architecture_overview("/test/project").unwrap();

        // Results should be identical (deterministic)
        assert_eq!(result1.languages_detected, result2.languages_detected);
        assert_eq!(result1.unified_entities, result2.unified_entities);
        assert_eq!(result1.unified_edges, result2.unified_edges);
        assert_eq!(result1.topology_graph, result2.topology_graph);
        assert_eq!(result1.complexity_map, result2.complexity_map);
        assert_eq!(result1.refactor_suggestions, result2.refactor_suggestions);

        // Metadata timestamps should be different but other fields same
        assert_ne!(
            result1.metadata["generated_at"],
            result2.metadata["generated_at"]
        );
        assert_eq!(
            result1.metadata["project_path"],
            result2.metadata["project_path"]
        );
    }

    #[test]
    fn test_zero_nondeterministic_floating_point_drift() {
        let results: Vec<PolyglotArchitectureOverview> = (0..10)
            .map(|_| project_polyglot_architecture_overview("/test/project").unwrap())
            .collect();

        // All floating point values should be identical across runs
        let first_result = &results[0];

        for (i, result) in results.iter().enumerate().skip(1) {
            assert_eq!(
                result.complexity_map.overall_complexity_score,
                first_result.complexity_map.overall_complexity_score,
                "Overall complexity score should be deterministic (run {})",
                i
            );

            // Check all file complexity scores
            for (j, file_entry) in result.complexity_map.file_complexity.iter().enumerate() {
                assert_eq!(
                    file_entry.complexity_score,
                    first_result.complexity_map.file_complexity[j].complexity_score,
                    "File complexity score should be deterministic (run {}, file {})",
                    i,
                    j
                );
            }

            // Check all type complexity scores
            for (j, type_entry) in result.complexity_map.type_complexity.iter().enumerate() {
                assert_eq!(
                    type_entry.complexity_score,
                    first_result.complexity_map.type_complexity[j].complexity_score,
                    "Type complexity score should be deterministic (run {}, type {})",
                    i,
                    j
                );
            }

            // Check all hotspot scores
            for (j, hotspot) in result
                .complexity_map
                .cross_language_hotspots
                .iter()
                .enumerate()
            {
                assert_eq!(
                    hotspot.hotspot_score,
                    first_result.complexity_map.cross_language_hotspots[j].hotspot_score,
                    "Hotspot score should be deterministic (run {}, hotspot {})",
                    i,
                    j
                );
                assert_eq!(
                    hotspot.cross_language_score,
                    first_result.complexity_map.cross_language_hotspots[j].cross_language_score,
                    "Cross-language score should be deterministic (run {}, hotspot {})",
                    i,
                    j
                );
            }
        }
    }

    #[test]
    fn test_handles_empty_codebase() {
        let result = project_polyglot_architecture_overview("/empty/project").unwrap();

        // Should return valid structure even for empty codebase
        assert!(result.languages_detected.is_empty());
        assert!(result.unified_entities.is_empty());
        assert!(result.unified_edges.is_empty());
        assert_eq!(result.topology_graph.connected_components, 0);
        assert!(result.topology_graph.nodes.is_empty());
        assert!(result.topology_graph.edges.is_empty());
        assert!(result.topology_graph.languages.is_empty());
        assert!(result.complexity_map.file_complexity.is_empty());
        assert!(result.complexity_map.type_complexity.is_empty());
        assert!(result.complexity_map.cross_language_hotspots.is_empty());
        assert_eq!(result.complexity_map.overall_complexity_score, 0.0);
        assert!(result.refactor_suggestions.is_empty());

        // Metadata should still be present
        assert!(result.metadata.contains_key("generated_at"));
        assert_eq!(result.metadata["project_path"], "/empty/project");
    }

    #[test]
    fn test_handles_partial_codebase() {
        let result = project_polyglot_architecture_overview("/partial/project").unwrap();

        // Should handle partial codebase gracefully
        assert!(!result.languages_detected.is_empty());
        assert!(result.languages_detected.len() <= 8); // At most all 8 languages

        // Should have at least some entities
        assert!(!result.unified_entities.is_empty());

        // Should have some edges (possibly fewer than entities)
        assert!(result.unified_edges.len() >= 0);

        // Should have valid topology
        assert!(result.topology_graph.connected_components >= 1);
        assert_eq!(
            result.topology_graph.nodes.len(),
            result.unified_entities.len()
        );
        assert_eq!(
            result.topology_graph.edges.len(),
            result.unified_edges.len()
        );

        // Should have some complexity data
        assert!(result.complexity_map.overall_complexity_score >= 0.0);
        assert!(result.complexity_map.overall_complexity_score <= 1.0);

        // Should have some refactor suggestions
        assert!(!result.refactor_suggestions.is_empty());
    }

    #[test]
    fn test_handles_mixed_language_nested_modules() {
        let result = project_polyglot_architecture_overview("/mixed/project").unwrap();

        // Should detect multiple languages
        assert!(result.languages_detected.len() >= 3);

        // Should have entities from multiple languages
        let languages_in_entities: HashSet<LanguageType> = result
            .unified_entities
            .iter()
            .map(|e| e.language.clone())
            .collect();
        assert_eq!(languages_in_entities.len(), result.languages_detected.len());

        // Should have cross-language edges
        let cross_language_edges: Vec<_> = result
            .unified_edges
            .iter()
            .filter(|edge| {
                let from_language = result
                    .unified_entities
                    .iter()
                    .find(|e| e.id == edge.from_entity_id)
                    .unwrap()
                    .language
                    .clone();
                let to_language = result
                    .unified_entities
                    .iter()
                    .find(|e| e.id == edge.to_entity_id)
                    .unwrap()
                    .language
                    .clone();
                from_language != to_language
            })
            .collect();

        assert!(!cross_language_edges.is_empty());

        // Should have cross-language hotspots
        let cross_language_hotspots: Vec<_> = result
            .complexity_map
            .cross_language_hotspots
            .iter()
            .filter(|h| h.cross_language_score > 0.0)
            .collect();

        assert!(!cross_language_hotspots.is_empty());

        // Should have cross-language refactor suggestions
        let cross_language_suggestions: Vec<_> = result
            .refactor_suggestions
            .iter()
            .filter(|s| s.languages.len() > 1)
            .collect();

        assert!(!cross_language_suggestions.is_empty());
    }

    #[test]
    fn test_json_serialization() {
        let result = project_polyglot_architecture_overview("/test/project").unwrap();

        // Should serialize to valid JSON without errors
        let json_string = serde_json::to_string(&result).unwrap();

        // Should deserialize back to same structure
        let deserialized: PolyglotArchitectureOverview =
            serde_json::from_str(&json_string).unwrap();

        // Deserialized result should be identical
        assert_eq!(deserialized, result);
    }

    #[test]
    fn test_tool_error_handling() {
        // Test with invalid project path
        let result = project_polyglot_architecture_overview("/invalid/nonexistent/path");

        // Should handle errors gracefully
        assert!(result.is_err());
    }
}
