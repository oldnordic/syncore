//! Embedding Domain Abstractions
//!
//! Provides type-safe domain routing for CODE vs GENERAL embeddings.
//! Follows TDD methodology - tests written before implementation.
//!
//! ## Domain Separation
//!
//! - **CODE domain**: Code entities (functions, structs, traits) optimized for code search
//! - **GENERAL domain**: Documents, tasks, notes, reasoning steps - general semantic search
//!
//! ## Key Types
//!
//! - `EmbeddingDomain`: Enum distinguishing CODE vs GENERAL
//! - `EmbeddingConfig`: Domain-specific configuration (model, index path, dimension)
//! - `DomainRouter`: Routes namespace strings to domains

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Embedding domain - CODE, GENERAL, or GRAPH
///
/// This enum enforces type-safe domain routing throughout the codebase.
/// Every vector operation must specify a domain to prevent mixing embeddings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingDomain {
    /// CODE domain - code entities optimized for code search
    /// Namespaces: "code_entity"
    Code,

    /// GENERAL domain - documents, tasks, notes, reasoning steps
    /// Namespaces: "documents", "plan", "sequential_cycle", etc.
    General,

    /// GRAPH domain - graph entities, nodes, edges, relationships
    /// Namespaces: "graph_entity", "rag_graph", "hop_graph", "code_graph"
    Graph,
}

impl EmbeddingDomain {
    /// Map namespace string to domain
    ///
    /// # Examples
    ///
    /// ```
    /// use syncore::vector::domain::EmbeddingDomain;
    ///
    /// assert_eq!(EmbeddingDomain::from_namespace("code_entity"), EmbeddingDomain::Code);
    /// assert_eq!(EmbeddingDomain::from_namespace("rust_code"), EmbeddingDomain::Code);
    /// assert_eq!(EmbeddingDomain::from_namespace("documents"), EmbeddingDomain::General);
    /// assert_eq!(EmbeddingDomain::from_namespace("plan"), EmbeddingDomain::General);
    /// assert_eq!(EmbeddingDomain::from_namespace("graph_entity"), EmbeddingDomain::Graph);
    /// assert_eq!(EmbeddingDomain::from_namespace("rag_graph"), EmbeddingDomain::Graph);
    /// ```
    pub fn from_namespace(namespace: &str) -> Self {
        match namespace {
            // CODE domain namespaces
            "code_entity" | "rust_code" | "python_code" | "javascript_code" => Self::Code,
            // GRAPH domain namespaces
            "graph_entity" | "rag_graph" | "hop_graph" | "code_graph" => Self::Graph,
            // GENERAL domain (default for all others)
            _ => Self::General,
        }
    }

    /// Get default index path for domain
    pub fn default_index_path(&self) -> &'static str {
        match self {
            Self::Code => "syncore_code.index",
            Self::General => "syncore_general.index",
            Self::Graph => "syncore_graph.index",
        }
    }

    /// Get recommended model for domain
    pub fn recommended_model(&self) -> &'static str {
        match self {
            // Start with all-MiniLM-L6-v2 for both, can upgrade CODE to GraphCodeBERT later
            Self::Code => "all-MiniLM-L6-v2",
            Self::General => "all-MiniLM-L6-v2",
            // GRAPH domain uses same model initially (real implementation, not mock)
            Self::Graph => "all-MiniLM-L6-v2",
        }
    }
}

impl fmt::Display for EmbeddingDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Code => write!(f, "code"),
            Self::General => write!(f, "general"),
            Self::Graph => write!(f, "graph"),
        }
    }
}

/// Domain-specific embedding configuration
///
/// Each domain has its own model, index path, and settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Domain this config applies to
    pub domain: EmbeddingDomain,

    /// Model name (e.g., "all-MiniLM-L6-v2", "microsoft/graphcodebert-base")
    pub model_name: String,

    /// HNSW index path (e.g., "syncore_code.index")
    pub index_path: String,

    /// Embedding dimension (e.g., 384 for all-MiniLM-L6-v2)
    pub dimension: usize,
}

impl EmbeddingConfig {
    /// Create default config for CODE domain
    /// APEX 1.9-E: Uses BGE-small-en-v1.5 for code-specific embeddings
    pub fn for_code() -> Self {
        Self {
            domain: EmbeddingDomain::Code,
            model_name: "BGE-small-en-v1.5".to_string(),
            index_path: "syncore_code.index".to_string(),
            dimension: 384,
        }
    }

    /// Create default config for GENERAL domain
    pub fn for_general() -> Self {
        Self {
            domain: EmbeddingDomain::General,
            model_name: "all-MiniLM-L6-v2".to_string(),
            index_path: "syncore_general.index".to_string(),
            dimension: 384,
        }
    }

    /// Create default config for GRAPH domain
    /// Uses GraphBERT-compatible settings from SyncoreConfig or defaults
    pub fn for_graph() -> Self {
        // Try to get graph configuration from global config, fall back to defaults
        let graph_config = if let Some(config) = crate::config::SyncoreConfig::try_global() {
            &config.graph_embeddings
        } else {
            // Use default GraphBERT settings
            return Self {
                domain: EmbeddingDomain::Graph,
                model_name: "graphbert-base".to_string(),
                index_path: crate::common::db_paths::graph_vector_index_path(),
                dimension: 384,
            };
        };

        Self {
            domain: EmbeddingDomain::Graph,
            model_name: graph_config.model_name.clone(),
            index_path: crate::common::db_paths::graph_vector_index_path(),
            dimension: graph_config.dimensions,
        }
    }

    /// Create config for specific domain with defaults
    pub fn for_domain(domain: EmbeddingDomain) -> Self {
        match domain {
            EmbeddingDomain::Code => Self::for_code(),
            EmbeddingDomain::General => Self::for_general(),
            EmbeddingDomain::Graph => Self::for_graph(),
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.dimension == 0 {
            anyhow::bail!("Dimension must be > 0, got {}", self.dimension);
        }

        if self.model_name.is_empty() {
            anyhow::bail!("Model name cannot be empty");
        }

        if self.index_path.is_empty() {
            anyhow::bail!("Index path cannot be empty");
        }

        Ok(())
    }
}

/// Domain-aware embedding service trait
///
/// This trait provides the interface for domain-specific embedding operations.
/// Implementations must route to the correct embedding model based on domain.
pub trait EmbeddingService: Send + Sync {
    /// Embed text with domain-aware model selection
    fn embed(&self, text: &str, domain: EmbeddingDomain) -> Result<Vec<f32>>;

    /// Get embedding dimension for domain
    fn dimension(&self, domain: EmbeddingDomain) -> usize;

    /// Get configuration for domain
    fn config(&self, domain: EmbeddingDomain) -> &EmbeddingConfig;
}

/// Hit result from semantic search
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub id: i64,
    pub score: f32,
    pub text: String,
}

// ============================================================================
// TDD TESTS - Written BEFORE implementation
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // EmbeddingDomain Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_domain_from_namespace_code() {
        assert_eq!(EmbeddingDomain::from_namespace("code_entity"), EmbeddingDomain::Code);
    }

    #[test]
    fn test_domain_from_namespace_general() {
        assert_eq!(EmbeddingDomain::from_namespace("documents"), EmbeddingDomain::General);
        assert_eq!(EmbeddingDomain::from_namespace("plan"), EmbeddingDomain::General);
        assert_eq!(EmbeddingDomain::from_namespace("sequential_cycle"), EmbeddingDomain::General);
        assert_eq!(EmbeddingDomain::from_namespace("thought_step"), EmbeddingDomain::General);
    }

    #[test]
    fn test_domain_from_namespace_graph() {
        assert_eq!(EmbeddingDomain::from_namespace("graph_entity"), EmbeddingDomain::Graph);
        assert_eq!(EmbeddingDomain::from_namespace("rag_graph"), EmbeddingDomain::Graph);
        assert_eq!(EmbeddingDomain::from_namespace("hop_graph"), EmbeddingDomain::Graph);
        assert_eq!(EmbeddingDomain::from_namespace("code_graph"), EmbeddingDomain::Graph);
    }

    #[test]
    fn test_domain_from_namespace_unknown_defaults_to_general() {
        assert_eq!(EmbeddingDomain::from_namespace("unknown_namespace"), EmbeddingDomain::General);
    }

    #[test]
    fn test_domain_display() {
        assert_eq!(format!("{}", EmbeddingDomain::Code), "code");
        assert_eq!(format!("{}", EmbeddingDomain::General), "general");
        assert_eq!(format!("{}", EmbeddingDomain::Graph), "graph");
    }

    #[test]
    fn test_domain_default_index_paths_differ() {
        let code_path = EmbeddingDomain::Code.default_index_path();
        let general_path = EmbeddingDomain::General.default_index_path();
        let graph_path = EmbeddingDomain::Graph.default_index_path();

        assert_ne!(code_path, general_path, "CODE and GENERAL must have separate indices");
        assert_ne!(code_path, graph_path, "CODE and GRAPH must have separate indices");
        assert_ne!(general_path, graph_path, "GENERAL and GRAPH must have separate indices");
        assert!(code_path.contains("code"));
        assert!(general_path.contains("general"));
        assert!(graph_path.contains("graph"));
    }

    #[test]
    fn test_domain_equality() {
        assert_eq!(EmbeddingDomain::Code, EmbeddingDomain::Code);
        assert_eq!(EmbeddingDomain::General, EmbeddingDomain::General);
        assert_eq!(EmbeddingDomain::Graph, EmbeddingDomain::Graph);
        assert_ne!(EmbeddingDomain::Code, EmbeddingDomain::General);
        assert_ne!(EmbeddingDomain::Code, EmbeddingDomain::Graph);
        assert_ne!(EmbeddingDomain::General, EmbeddingDomain::Graph);
    }

    #[test]
    fn test_domain_serialization() {
        let code = EmbeddingDomain::Code;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, r#""code""#);

        let general = EmbeddingDomain::General;
        let json = serde_json::to_string(&general).unwrap();
        assert_eq!(json, r#""general""#);

        let graph = EmbeddingDomain::Graph;
        let json = serde_json::to_string(&graph).unwrap();
        assert_eq!(json, r#""graph""#);
    }

    #[test]
    fn test_domain_deserialization() {
        let code: EmbeddingDomain = serde_json::from_str(r#""code""#).unwrap();
        assert_eq!(code, EmbeddingDomain::Code);

        let general: EmbeddingDomain = serde_json::from_str(r#""general""#).unwrap();
        assert_eq!(general, EmbeddingDomain::General);

        let graph: EmbeddingDomain = serde_json::from_str(r#""graph""#).unwrap();
        assert_eq!(graph, EmbeddingDomain::Graph);
    }

    // ------------------------------------------------------------------------
    // EmbeddingConfig Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_config_for_code() {
        let config = EmbeddingConfig::for_code();

        assert_eq!(config.domain, EmbeddingDomain::Code);
        assert!(!config.model_name.is_empty());
        assert!(config.index_path.contains("code"));
        assert_eq!(config.dimension, 384);
    }

    #[test]
    fn test_config_for_general() {
        let config = EmbeddingConfig::for_general();

        assert_eq!(config.domain, EmbeddingDomain::General);
        assert!(!config.model_name.is_empty());
        assert!(config.index_path.contains("general"));
        assert_eq!(config.dimension, 384);
    }

    #[test]
    fn test_config_for_graph() {
        let config = EmbeddingConfig::for_graph();

        assert_eq!(config.domain, EmbeddingDomain::Graph);
        assert!(!config.model_name.is_empty());
        assert!(config.index_path.contains("graph"));
        assert_eq!(config.dimension, 384);
    }

    #[test]
    fn test_config_for_domain() {
        let code_config = EmbeddingConfig::for_domain(EmbeddingDomain::Code);
        assert_eq!(code_config.domain, EmbeddingDomain::Code);

        let general_config = EmbeddingConfig::for_domain(EmbeddingDomain::General);
        assert_eq!(general_config.domain, EmbeddingDomain::General);

        let graph_config = EmbeddingConfig::for_domain(EmbeddingDomain::Graph);
        assert_eq!(graph_config.domain, EmbeddingDomain::Graph);
    }

    #[test]
    fn test_config_index_paths_differ_by_domain() {
        let code_config = EmbeddingConfig::for_code();
        let general_config = EmbeddingConfig::for_general();
        let graph_config = EmbeddingConfig::for_graph();

        assert_ne!(
            code_config.index_path, general_config.index_path,
            "CODE and GENERAL must use separate HNSW indices"
        );
        assert_ne!(
            code_config.index_path, graph_config.index_path,
            "CODE and GRAPH must use separate HNSW indices"
        );
        assert_ne!(
            general_config.index_path, graph_config.index_path,
            "GENERAL and GRAPH must use separate HNSW indices"
        );
    }

    #[test]
    fn test_config_validation_succeeds_for_valid() {
        let config = EmbeddingConfig::for_code();
        assert!(config.validate().is_ok());

        let config = EmbeddingConfig::for_general();
        assert!(config.validate().is_ok());

        let config = EmbeddingConfig::for_graph();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_fails_for_zero_dimension() {
        let mut config = EmbeddingConfig::for_code();
        config.dimension = 0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_fails_for_empty_model() {
        let mut config = EmbeddingConfig::for_code();
        config.model_name = String::new();

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_fails_for_empty_index_path() {
        let mut config = EmbeddingConfig::for_code();
        config.index_path = String::new();

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_serialization() {
        // APEX 1.9-E: CODE domain now uses BGE-small-en-v1.5
        let config = EmbeddingConfig::for_code();
        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("code"));
        assert!(json.contains("BGE-small-en-v1.5"));
        assert!(json.contains("384"));
    }

    #[test]
    fn test_config_deserialization() {
        // APEX 1.9-E: Test deserialization with BGE model
        let json = r#"{
            "domain": "code",
            "model_name": "BGE-small-en-v1.5",
            "index_path": "syncore_code.index",
            "dimension": 384
        }"#;

        let config: EmbeddingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.domain, EmbeddingDomain::Code);
        assert_eq!(config.model_name, "BGE-small-en-v1.5");
        assert_eq!(config.dimension, 384);
    }

    #[test]
    fn test_config_roundtrip_serialization() {
        let original = EmbeddingConfig::for_code();
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: EmbeddingConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(original.domain, deserialized.domain);
        assert_eq!(original.model_name, deserialized.model_name);
        assert_eq!(original.index_path, deserialized.index_path);
        assert_eq!(original.dimension, deserialized.dimension);
    }

    // ------------------------------------------------------------------------
    // EmbeddingService Tests (TDD - tests written first)
    // ------------------------------------------------------------------------

    /// Mock embedding service for testing domain routing
    struct MockEmbeddingService {
        code_config: EmbeddingConfig,
        general_config: EmbeddingConfig,
        graph_config: EmbeddingConfig,
    }

    impl MockEmbeddingService {
        fn new() -> Self {
            Self {
                code_config: EmbeddingConfig::for_code(),
                general_config: EmbeddingConfig::for_general(),
                graph_config: EmbeddingConfig::for_graph(),
            }
        }
    }

    impl EmbeddingService for MockEmbeddingService {
        fn embed(&self, text: &str, domain: EmbeddingDomain) -> Result<Vec<f32>> {
            // Mock: return different embeddings based on domain
            // CODE: embedding starts with 1.0
            // GENERAL: embedding starts with 0.5
            // GRAPH: embedding starts with 0.25
            let dim = self.dimension(domain);
            let mut vec = vec![0.0; dim];

            vec[0] = match domain {
                EmbeddingDomain::Code => 1.0,
                EmbeddingDomain::General => 0.5,
                EmbeddingDomain::Graph => 0.25,
            };

            // Add text-specific variation
            let text_hash = text.len() as f32 / 100.0;
            vec[1] = text_hash;

            Ok(vec)
        }

        fn dimension(&self, domain: EmbeddingDomain) -> usize {
            match domain {
                EmbeddingDomain::Code => self.code_config.dimension,
                EmbeddingDomain::General => self.general_config.dimension,
                EmbeddingDomain::Graph => self.graph_config.dimension,
            }
        }

        fn config(&self, domain: EmbeddingDomain) -> &EmbeddingConfig {
            match domain {
                EmbeddingDomain::Code => &self.code_config,
                EmbeddingDomain::General => &self.general_config,
                EmbeddingDomain::Graph => &self.graph_config,
            }
        }
    }

    #[test]
    fn test_embedding_service_domain_routing() {
        let service = MockEmbeddingService::new();

        let code_vec = service.embed("fn main() {}", EmbeddingDomain::Code).unwrap();
        let general_vec = service.embed("fn main() {}", EmbeddingDomain::General).unwrap();
        let graph_vec = service.embed("fn main() {}", EmbeddingDomain::Graph).unwrap();

        // Different domains produce different embeddings for same text
        assert_ne!(code_vec[0], general_vec[0]);
        assert_ne!(code_vec[0], graph_vec[0]);
        assert_ne!(general_vec[0], graph_vec[0]);
        assert_eq!(code_vec[0], 1.0); // CODE marker
        assert_eq!(general_vec[0], 0.5); // GENERAL marker
        assert_eq!(graph_vec[0], 0.25); // GRAPH marker
    }

    #[test]
    fn test_embedding_service_dimension_differs_by_domain() {
        let service = MockEmbeddingService::new();

        let code_dim = service.dimension(EmbeddingDomain::Code);
        let general_dim = service.dimension(EmbeddingDomain::General);
        let graph_dim = service.dimension(EmbeddingDomain::Graph);

        // All default to 384, but could be different
        assert_eq!(code_dim, 384);
        assert_eq!(general_dim, 384);
        assert_eq!(graph_dim, 384);
    }

    #[test]
    fn test_embedding_service_config_returns_correct_domain() {
        let service = MockEmbeddingService::new();

        let code_cfg = service.config(EmbeddingDomain::Code);
        let general_cfg = service.config(EmbeddingDomain::General);
        let graph_cfg = service.config(EmbeddingDomain::Graph);

        assert_eq!(code_cfg.domain, EmbeddingDomain::Code);
        assert_eq!(general_cfg.domain, EmbeddingDomain::General);
        assert_eq!(graph_cfg.domain, EmbeddingDomain::Graph);
    }

    #[test]
    fn test_embedding_service_preserves_text_variation() {
        let service = MockEmbeddingService::new();

        let short = service.embed("x", EmbeddingDomain::Code).unwrap();
        let long_text = "x".repeat(100);
        let long = service.embed(&long_text, EmbeddingDomain::Code).unwrap();

        // Text length affects embedding (text-specific variation)
        assert_ne!(short[1], long[1]);
    }

    #[test]
    fn test_embedding_service_code_domain_consistent() {
        let service = MockEmbeddingService::new();

        let vec1 = service.embed("test", EmbeddingDomain::Code).unwrap();
        let vec2 = service.embed("test", EmbeddingDomain::Code).unwrap();

        // Same text + same domain = same embedding (deterministic)
        assert_eq!(vec1, vec2);
    }

    #[test]
    fn test_embedding_service_general_domain_consistent() {
        let service = MockEmbeddingService::new();

        let vec1 = service.embed("test", EmbeddingDomain::General).unwrap();
        let vec2 = service.embed("test", EmbeddingDomain::General).unwrap();

        // Same text + same domain = same embedding (deterministic)
        assert_eq!(vec1, vec2);
    }

    #[test]
    fn test_embedding_service_graph_domain_consistent() {
        let service = MockEmbeddingService::new();

        let vec1 = service.embed("test", EmbeddingDomain::Graph).unwrap();
        let vec2 = service.embed("test", EmbeddingDomain::Graph).unwrap();

        // Same text + same domain = same embedding (deterministic)
        assert_eq!(vec1, vec2);
    }

    #[test]
    fn test_embedding_service_graph_domain_unique_marker() {
        let service = MockEmbeddingService::new();

        let graph_vec = service.embed("node relationship", EmbeddingDomain::Graph).unwrap();

        // GRAPH domain should have unique marker 0.25
        assert_eq!(graph_vec[0], 0.25);
    }

    #[test]
    fn test_embedding_service_all_domains_different() {
        let service = MockEmbeddingService::new();

        let code_vec = service.embed("test", EmbeddingDomain::Code).unwrap();
        let general_vec = service.embed("test", EmbeddingDomain::General).unwrap();
        let graph_vec = service.embed("test", EmbeddingDomain::Graph).unwrap();

        // All three domains should produce different embeddings
        assert_ne!(code_vec, general_vec);
        assert_ne!(code_vec, graph_vec);
        assert_ne!(general_vec, graph_vec);
    }

    #[test]
    fn test_embedding_service_graph_config_valid() {
        let service = MockEmbeddingService::new();

        let graph_cfg = service.config(EmbeddingDomain::Graph);

        // Verify GRAPH domain has correct configuration
        assert_eq!(graph_cfg.domain, EmbeddingDomain::Graph);
        assert!(!graph_cfg.model_name.is_empty());
        assert!(graph_cfg.index_path.contains("graph"));
        assert_eq!(graph_cfg.dimension, 384);
    }
}
