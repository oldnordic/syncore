//! Dual-Embedding Service Implementation
//!
//! Provides domain-aware routing between CODE and GENERAL embedding models.
//! This is the production implementation of the EmbeddingService trait.
//!
//! ## Architecture
//!
//! - Maintains TWO separate VectorStores (CODE domain, GENERAL domain)
//! - Routes embedding operations based on EmbeddingDomain
//! - Uses separate HNSW indices per domain
//! - Thread-safe with Arc<Mutex<VectorStore>>

use anyhow::Result;
use std::sync::{Arc, Mutex};

use super::domain::{EmbeddingConfig, EmbeddingDomain, EmbeddingService};
use super::{Embeddings, HuggingFaceEmbeddings, VectorStore};

/// Production dual-embedding service
///
/// Routes between CODE-optimized and GENERAL-purpose embeddings based on domain.
pub struct DualEmbeddingService {
    /// CODE domain store (code entities)
    code_store: Arc<Mutex<VectorStore>>,

    /// GENERAL domain store (documents, tasks, notes)
    general_store: Arc<Mutex<VectorStore>>,

    /// CODE domain configuration
    code_config: EmbeddingConfig,

    /// GENERAL domain configuration
    general_config: EmbeddingConfig,
}

impl DualEmbeddingService {
    /// Create new dual-embedding service with default configurations
    ///
    /// APEX 1.9-E: CODE domain uses BGE-small-en-v1.5 for code-specific embeddings,
    /// GENERAL domain uses all-MiniLM-L6-v2 for general text.
    pub fn new() -> Result<Self> {
        let code_config = EmbeddingConfig::for_code();
        let general_config = EmbeddingConfig::for_general();

        Self::with_configs(code_config, general_config)
    }

    /// Create with explicit configurations per domain
    pub fn with_configs(
        code_config: EmbeddingConfig,
        general_config: EmbeddingConfig,
    ) -> Result<Self> {
        // Validate configs
        code_config.validate()?;
        general_config.validate()?;

        // Create embeddings for CODE domain - APEX 1.9-E: Uses BGE-small for code
        let code_embeddings = Box::new(HuggingFaceEmbeddings::new_bge()?);
        let mut code_store = VectorStore::new(code_embeddings);
        code_store.set_index_path(code_config.index_path.clone());

        // Create embeddings for GENERAL domain - Uses all-MiniLM for general text
        let general_embeddings = Box::new(HuggingFaceEmbeddings::new()?);
        let mut general_store = VectorStore::new(general_embeddings);
        general_store.set_index_path(general_config.index_path.clone());

        Ok(Self {
            code_store: Arc::new(Mutex::new(code_store)),
            general_store: Arc::new(Mutex::new(general_store)),
            code_config,
            general_config,
        })
    }

    /// Create from pre-existing VectorStore instances
    ///
    /// This is the preferred constructor when VectorStores are already initialized
    /// (e.g., in SynCoreState). Avoids creating duplicate stores.
    pub fn from_stores(
        code_store: Arc<Mutex<VectorStore>>,
        general_store: Arc<Mutex<VectorStore>>,
    ) -> Self {
        Self {
            code_store,
            general_store,
            code_config: EmbeddingConfig::for_code(),
            general_config: EmbeddingConfig::for_general(),
        }
    }

    /// Get CODE domain VectorStore (for direct operations)
    pub fn code_store(&self) -> Arc<Mutex<VectorStore>> {
        Arc::clone(&self.code_store)
    }

    /// Get GENERAL domain VectorStore (for direct operations)
    pub fn general_store(&self) -> Arc<Mutex<VectorStore>> {
        Arc::clone(&self.general_store)
    }

    /// Get VectorStore for specific domain
    pub fn store_for_domain(&self, domain: EmbeddingDomain) -> Arc<Mutex<VectorStore>> {
        match domain {
            EmbeddingDomain::Code => self.code_store(),
            EmbeddingDomain::General => self.general_store(),
        }
    }
}

impl EmbeddingService for DualEmbeddingService {
    fn embed(&self, text: &str, domain: EmbeddingDomain) -> Result<Vec<f32>> {
        // APEX 1.9-E: Use domain-specific embedding models
        // CODE domain: BGE-small-en-v1.5 (code-optimized)
        // GENERAL domain: all-MiniLM-L6-v2 (general text)
        let embeddings: Box<dyn Embeddings> = match domain {
            EmbeddingDomain::Code => Box::new(HuggingFaceEmbeddings::new_bge()?),
            EmbeddingDomain::General => Box::new(HuggingFaceEmbeddings::new()?),
        };
        embeddings.embed(text)
    }

    fn dimension(&self, domain: EmbeddingDomain) -> usize {
        match domain {
            EmbeddingDomain::Code => self.code_config.dimension,
            EmbeddingDomain::General => self.general_config.dimension,
        }
    }

    fn config(&self, domain: EmbeddingDomain) -> &EmbeddingConfig {
        match domain {
            EmbeddingDomain::Code => &self.code_config,
            EmbeddingDomain::General => &self.general_config,
        }
    }
}

// ============================================================================
// TDD TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::domain::EmbeddingDomain;

    #[test]
    fn test_dual_service_creation() {
        let service = DualEmbeddingService::new().unwrap();

        // Verify configs are set
        assert_eq!(service.code_config.domain, EmbeddingDomain::Code);
        assert_eq!(service.general_config.domain, EmbeddingDomain::General);
    }

    #[test]
    fn test_dual_service_separate_stores() {
        let service = DualEmbeddingService::new().unwrap();

        let code_ptr = Arc::as_ptr(&service.code_store());
        let general_ptr = Arc::as_ptr(&service.general_store());

        // Stores must be different instances
        assert_ne!(
            code_ptr, general_ptr,
            "CODE and GENERAL stores must be separate"
        );
    }

    #[test]
    fn test_dual_service_dimension_per_domain() {
        let service = DualEmbeddingService::new().unwrap();

        let code_dim = service.dimension(EmbeddingDomain::Code);
        let general_dim = service.dimension(EmbeddingDomain::General);

        assert_eq!(code_dim, 384);
        assert_eq!(general_dim, 384);
    }

    #[test]
    fn test_dual_service_config_per_domain() {
        let service = DualEmbeddingService::new().unwrap();

        let code_cfg = service.config(EmbeddingDomain::Code);
        let general_cfg = service.config(EmbeddingDomain::General);

        assert_eq!(code_cfg.domain, EmbeddingDomain::Code);
        assert_eq!(general_cfg.domain, EmbeddingDomain::General);
        assert_ne!(code_cfg.index_path, general_cfg.index_path);
    }

    #[test]
    fn test_dual_service_embed_code_domain() {
        let service = DualEmbeddingService::new().unwrap();

        let embedding = service
            .embed("fn main() {}", EmbeddingDomain::Code)
            .unwrap();

        assert_eq!(embedding.len(), 384);
        assert!(
            embedding.iter().any(|&x| x != 0.0),
            "Embedding should not be all zeros"
        );
    }

    #[test]
    fn test_dual_service_embed_general_domain() {
        let service = DualEmbeddingService::new().unwrap();

        let embedding = service
            .embed("This is a document", EmbeddingDomain::General)
            .unwrap();

        assert_eq!(embedding.len(), 384);
        assert!(
            embedding.iter().any(|&x| x != 0.0),
            "Embedding should not be all zeros"
        );
    }

    #[test]
    fn test_dual_service_store_for_domain() {
        let service = DualEmbeddingService::new().unwrap();

        let code_store = service.store_for_domain(EmbeddingDomain::Code);
        let general_store = service.store_for_domain(EmbeddingDomain::General);

        let code_ptr = Arc::as_ptr(&code_store);
        let general_ptr = Arc::as_ptr(&general_store);

        assert_ne!(
            code_ptr, general_ptr,
            "Domain routing must return different stores"
        );
    }

    #[test]
    fn test_dual_service_with_custom_configs() {
        let mut code_cfg = EmbeddingConfig::for_code();
        code_cfg.index_path = "custom_code.index".to_string();

        let mut general_cfg = EmbeddingConfig::for_general();
        general_cfg.index_path = "custom_general.index".to_string();

        let service =
            DualEmbeddingService::with_configs(code_cfg.clone(), general_cfg.clone()).unwrap();

        assert_eq!(
            service.config(EmbeddingDomain::Code).index_path,
            "custom_code.index"
        );
        assert_eq!(
            service.config(EmbeddingDomain::General).index_path,
            "custom_general.index"
        );
    }

    #[test]
    fn test_dual_service_rejects_invalid_config() {
        let mut invalid_cfg = EmbeddingConfig::for_code();
        invalid_cfg.dimension = 0; // Invalid

        let valid_cfg = EmbeddingConfig::for_general();

        let result = DualEmbeddingService::with_configs(invalid_cfg, valid_cfg);
        assert!(result.is_err(), "Should reject invalid config");
    }

    #[test]
    fn test_code_domain_uses_bge_model() {
        // APEX 1.9-E: Verify CODE domain uses BGE-small-en-v1.5 for code search
        let service = DualEmbeddingService::new().unwrap();
        let code_config = service.config(EmbeddingDomain::Code);

        assert_eq!(
            code_config.model_name, "BGE-small-en-v1.5",
            "CODE domain must use BGE-small-en-v1.5 for code-specific embeddings"
        );
        assert_eq!(code_config.dimension, 384);
    }

    #[test]
    fn test_general_domain_uses_minilm_model() {
        // APEX 1.9-E: Verify GENERAL domain continues using all-MiniLM-L6-v2
        let service = DualEmbeddingService::new().unwrap();
        let general_config = service.config(EmbeddingDomain::General);

        assert_eq!(
            general_config.model_name, "all-MiniLM-L6-v2",
            "GENERAL domain must continue using all-MiniLM-L6-v2"
        );
        assert_eq!(general_config.dimension, 384);
    }

    #[test]
    fn test_domain_model_separation() {
        // APEX 1.9-E: Verify different domains use different models
        let service = DualEmbeddingService::new().unwrap();
        let code_model = service.config(EmbeddingDomain::Code).model_name.clone();
        let general_model = service.config(EmbeddingDomain::General).model_name.clone();

        assert_ne!(
            code_model, general_model,
            "CODE and GENERAL domains must use different embedding models"
        );
    }
}
