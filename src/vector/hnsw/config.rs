//! HNSW Configuration
//!
//! Tuning parameters for the Hierarchical Navigable Small World graph.

/// Configuration for HNSW index construction and search
#[derive(Debug, Clone)]
pub struct HnswConfig {
    /// Number of bi-directional links per node in each layer (except layer 0).
    /// Higher M = better recall, more memory, slower construction.
    /// Typical range: 8-64, default: 16
    pub m: usize,

    /// Size of the dynamic candidate list during graph construction.
    /// Higher ef_construction = better graph quality, slower construction.
    /// Typical range: 100-500, default: 200
    pub ef_construction: usize,

    /// Size of the dynamic candidate list during search.
    /// Higher ef_search = better recall, slower search.
    /// Typical range: 50-500, default: 100
    pub ef_search: usize,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            m: 16,
            ef_construction: 200,
            ef_search: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HnswConfig::default();
        assert_eq!(config.m, 16);
        assert_eq!(config.ef_construction, 200);
        assert_eq!(config.ef_search, 100);
    }

    #[test]
    fn test_custom_config() {
        let config = HnswConfig {
            m: 32,
            ef_construction: 400,
            ef_search: 200,
        };
        assert_eq!(config.m, 32);
        assert_eq!(config.ef_construction, 400);
        assert_eq!(config.ef_search, 200);
    }
}
