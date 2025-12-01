// SPEC: SYNCORE-CONFIG-AND-TOOLS-FIX-01 (APEX v1.2)
// STEP A: Central configuration system
//
// Provides a unified configuration schema for all SynCore components:
// - Paths (databases, cache, logs)
// - Neo4j connection settings
// - Indexing (excluded directories, file extensions)
// - Embeddings and vector search
// - Project analysis weights
// - HTTP server settings

use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::OnceLock;

// APEX 2.0-M-FIX: Import MemoryConfig
use crate::memory::MemoryConfig;

/// Global configuration singleton
static GLOBAL_CONFIG: OnceLock<SyncoreConfig> = OnceLock::new();

/// Path configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_code_graph_db")]
    pub code_graph_db: String,
    #[serde(default = "default_vector_index")]
    pub vector_index_path: String,
    #[serde(default = "default_cache_path")]
    pub cache_path: String,
    #[serde(default = "default_socket_path")]
    pub socket_path: String,
    #[serde(default = "default_logs_dir")]
    pub logs_dir: String,
}

fn default_db_path() -> String {
    "syncore.db".to_string()
}
fn default_code_graph_db() -> String {
    "syncore_code_graph.db".to_string()
}
fn default_vector_index() -> String {
    "vector.index".to_string()
}
fn default_cache_path() -> String {
    "cache".to_string()
}
fn default_socket_path() -> String {
    "/tmp/syncore.sock".to_string()
}
fn default_logs_dir() -> String {
    "logs".to_string()
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            code_graph_db: default_code_graph_db(),
            vector_index_path: default_vector_index(),
            cache_path: default_cache_path(),
            socket_path: default_socket_path(),
            logs_dir: default_logs_dir(),
        }
    }
}

/// Neo4j database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jConfig {
    #[serde(default = "default_neo4j_uri")]
    pub uri: String,
    #[serde(default = "default_neo4j_user")]
    pub user: String,
    #[serde(default = "default_neo4j_password")]
    pub password: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_neo4j_uri() -> String {
    "bolt://127.0.0.1:7687".to_string()
}
fn default_neo4j_user() -> String {
    "neo4j".to_string()
}
fn default_neo4j_password() -> String {
    String::new()
}
fn default_true() -> bool {
    true
}

impl Default for Neo4jConfig {
    fn default() -> Self {
        Self {
            uri: default_neo4j_uri(),
            user: default_neo4j_user(),
            password: default_neo4j_password(),
            enabled: default_true(),
        }
    }
}

/// Indexing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    #[serde(default = "default_excluded_dirs")]
    pub excluded_dirs: Vec<String>,
    #[serde(default = "default_include_extensions")]
    pub include_extensions: Vec<String>,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: usize,
}

fn default_excluded_dirs() -> Vec<String> {
    vec![
        // Rust
        "target".to_string(),
        // JavaScript/Node.js
        "node_modules".to_string(),
        // Version control
        ".git".to_string(),
        // Python
        "__pycache__".to_string(),
        ".venv".to_string(),
        "venv".to_string(),
        ".tox".to_string(),
        ".eggs".to_string(),
        // Go
        "vendor".to_string(),
        // Java/JVM
        ".gradle".to_string(),
        ".m2".to_string(),
        // IDE/Editor
        ".vscode".to_string(),
        ".idea".to_string(),
        ".vs".to_string(),
        // Generic build output
        "dist".to_string(),
        "build".to_string(),
        "out".to_string(),
        // Coverage/Test output
        "coverage".to_string(),
        "htmlcov".to_string(),
        ".nyc_output".to_string(),
        // C/C++
        "cmake-build-debug".to_string(),
        "cmake-build-release".to_string(),
        // Cargo registry
        ".cargo".to_string(),
    ]
}

fn default_include_extensions() -> Vec<String> {
    vec![
        "rs".to_string(),
        "py".to_string(),
        "js".to_string(),
        "ts".to_string(),
        "tsx".to_string(),
        "jsx".to_string(),
        "go".to_string(),
        "java".to_string(),
        "c".to_string(),
        "cpp".to_string(),
        "h".to_string(),
        "hpp".to_string(),
    ]
}

fn default_max_file_size() -> usize {
    1048576
} // 1MB

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            excluded_dirs: default_excluded_dirs(),
            include_extensions: default_include_extensions(),
            max_file_size: default_max_file_size(),
        }
    }
}

/// Embeddings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    #[serde(default = "default_embeddings_model")]
    pub model: String,
    #[serde(default = "default_dimensions")]
    pub dimensions: usize,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_embeddings_model() -> String {
    "semantic".to_string()
}
fn default_dimensions() -> usize {
    384
}
fn default_batch_size() -> usize {
    32
}

impl Default for EmbeddingsConfig {
    fn default() -> Self {
        Self {
            model: default_embeddings_model(),
            dimensions: default_dimensions(),
            batch_size: default_batch_size(),
        }
    }
}

/// Vector search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchConfig {
    #[serde(default = "default_vector_limit")]
    pub default_limit: usize,
    #[serde(default = "default_min_score")]
    pub min_score_threshold: f32,
}

fn default_vector_limit() -> usize {
    10
}
fn default_min_score() -> f32 {
    0.3
}

impl Default for VectorSearchConfig {
    fn default() -> Self {
        Self {
            default_limit: default_vector_limit(),
            min_score_threshold: default_min_score(),
        }
    }
}

/// Hotspot weight configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotWeights {
    #[serde(default = "default_fan_in_weight")]
    pub fan_in: f32,
    #[serde(default = "default_fan_out_weight")]
    pub fan_out: f32,
    #[serde(default = "default_entity_count_weight")]
    pub entity_count: f32,
    #[serde(default = "default_loc_weight")]
    pub loc: f32,
}

fn default_fan_in_weight() -> f32 {
    0.4
}
fn default_fan_out_weight() -> f32 {
    0.3
}
fn default_entity_count_weight() -> f32 {
    0.2
}
fn default_loc_weight() -> f32 {
    0.1
}

impl Default for HotspotWeights {
    fn default() -> Self {
        Self {
            fan_in: default_fan_in_weight(),
            fan_out: default_fan_out_weight(),
            entity_count: default_entity_count_weight(),
            loc: default_loc_weight(),
        }
    }
}

/// Project analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAnalysisConfig {
    #[serde(default = "default_true")]
    pub use_indexing_excludes: bool,
    #[serde(default)]
    pub hotspot_weights: HotspotWeights,
}

impl Default for ProjectAnalysisConfig {
    fn default() -> Self {
        Self {
            use_indexing_excludes: true,
            hotspot_weights: HotspotWeights::default(),
        }
    }
}

/// LLM backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_llm_backend")]
    pub backend: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default = "default_llm_url")]
    pub url: String,
    #[serde(default = "default_llm_timeout_seconds")]
    pub timeout_seconds: u64,
}

fn default_llm_backend() -> String {
    "ollama".to_string()
}
fn default_llm_model() -> String {
    "qwen2.5-coder:3b".to_string()
}
fn default_llm_url() -> String {
    "http://localhost:11434".to_string()
}
fn default_llm_timeout_seconds() -> u64 {
    30
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            backend: default_llm_backend(),
            model: default_llm_model(),
            url: default_llm_url(),
            timeout_seconds: default_llm_timeout_seconds(),
        }
    }
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    #[serde(default = "default_http_port")]
    pub port: u16,
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
}

fn default_http_port() -> u16 {
    8080
}
fn default_metrics_port() -> u16 {
    9090
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            port: default_http_port(),
            metrics_port: default_metrics_port(),
        }
    }
}

/// Main SynCore configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncoreConfig {
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub neo4j: Neo4jConfig,
    #[serde(default)]
    pub indexing: IndexingConfig,
    #[serde(default)]
    pub embeddings: EmbeddingsConfig,
    #[serde(default)]
    pub vector_search: VectorSearchConfig,
    #[serde(default)]
    pub project_analysis: ProjectAnalysisConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub http: HttpConfig,
    /// APEX 2.0-M-FIX: Memory configuration
    #[serde(default)]
    pub memory: MemoryConfig,
}

impl SyncoreConfig {
    /// Load configuration from a TOML file
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: SyncoreConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration with environment variable overrides
    pub fn load_with_env(path: &str) -> anyhow::Result<Self> {
        let mut config = Self::load(path)?;
        config.apply_env_overrides();
        Ok(config)
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) {
        // Path overrides
        if let Ok(val) = std::env::var("DB_PATH") {
            self.paths.db_path = val;
        }
        if let Ok(val) = std::env::var("SYNCORE_CODE_GRAPH_DB") {
            self.paths.code_graph_db = val;
        }
        if let Ok(val) = std::env::var("SYNCORE_CACHE_PATH") {
            self.paths.cache_path = val;
        }
        if let Ok(val) = std::env::var("SYNCORE_LOGS_DIR") {
            self.paths.logs_dir = val;
        }

        // Neo4j overrides
        if let Ok(val) = std::env::var("NEO4J_URI") {
            self.neo4j.uri = val;
        }
        if let Ok(val) = std::env::var("NEO4J_USER") {
            self.neo4j.user = val;
        }
        if let Ok(val) = std::env::var("NEO4J_PASS") {
            self.neo4j.password = val;
        }

        // LLM overrides
        if let Ok(val) = std::env::var("LLM_BACKEND") {
            self.llm.backend = val;
        }
        if let Ok(val) = std::env::var("LLM_MODEL") {
            self.llm.model = val;
        }
        if let Ok(val) = std::env::var("LLM_URL") {
            self.llm.url = val;
        }
        if let Ok(val) = std::env::var("LLM_TIMEOUT") {
            if let Ok(timeout) = val.parse() {
                self.llm.timeout_seconds = timeout;
            }
        }

        // HTTP overrides
        if let Ok(val) = std::env::var("HTTP_PORT") {
            if let Ok(port) = val.parse() {
                self.http.port = port;
            }
        }
        if let Ok(val) = std::env::var("METRICS_PORT") {
            if let Ok(port) = val.parse() {
                self.http.metrics_port = port;
            }
        }

        // APEX 2.0-M-FIX: Memory overrides
        if let Ok(val) = std::env::var("SYNCORE_MEMORY_DEFAULT_NAMESPACE") {
            self.memory.default_namespace = val;
        }
    }

    /// Save configuration to a TOML file
    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Initialize the global configuration singleton
    pub fn init_global(config: SyncoreConfig) {
        let _ = GLOBAL_CONFIG.set(config);
    }

    /// Get the global configuration (panics if not initialized)
    pub fn global() -> &'static SyncoreConfig {
        GLOBAL_CONFIG.get().expect("Config not initialized. Call SyncoreConfig::init_global first.")
    }

    /// Try to get the global configuration (returns None if not initialized)
    pub fn try_global() -> Option<&'static SyncoreConfig> {
        GLOBAL_CONFIG.get()
    }

    /// Check if a path should be excluded based on indexing.excluded_dirs
    pub fn should_exclude_path(&self, path: &str) -> bool {
        if path.is_empty() {
            return true;
        }

        let normalized = path.trim_start_matches("./");
        let components: Vec<&str> =
            normalized.split(['/', '\\']).filter(|s| !s.is_empty()).collect();

        for component in &components {
            for excluded in &self.indexing.excluded_dirs {
                if *component == excluded {
                    return true;
                }
                // Handle cmake-build-* variants
                if excluded.starts_with("cmake-build-") && component.starts_with("cmake-build-") {
                    return true;
                }
            }
        }

        false
    }
}

// Legacy Config type for backwards compatibility
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub socket_path: String,
    pub db_path: String,
    pub cache_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            socket_path: "/tmp/syncore.sock".to_string(),
            db_path: "syncore.db".to_string(),
            cache_path: "cache".to_string(),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SyncoreConfig::default();
        assert_eq!(config.paths.db_path, "syncore.db");
        assert!(config.indexing.excluded_dirs.contains(&"target".to_string()));
    }

    #[test]
    fn test_should_exclude_path() {
        let config = SyncoreConfig::default();

        // Should exclude
        assert!(config.should_exclude_path("target/debug/build/foo.rs"));
        assert!(config.should_exclude_path("node_modules/lodash/index.js"));
        assert!(config.should_exclude_path(".git/objects/abc"));
        assert!(config.should_exclude_path("src/vendor/lib.rs"));

        // Should not exclude
        assert!(!config.should_exclude_path("src/main.rs"));
        assert!(!config.should_exclude_path("src/target_parser.rs"));
        assert!(!config.should_exclude_path("tests/integration.rs"));
    }

    #[test]
    fn test_legacy_config_compat() {
        let config = Config::default();
        assert_eq!(config.socket_path, "/tmp/syncore.sock");
        assert_eq!(config.db_path, "syncore.db");
    }
}
