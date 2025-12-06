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

/// Graph database backend selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphBackend {
    Neo4j,
    SqliteGraph,
}

impl Default for GraphBackend {
    fn default() -> Self {
        Self::SqliteGraph
    }
}

/// Graph database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConfig {
    #[serde(default)]
    pub backend: GraphBackend,
    #[serde(default = "default_graph_path")]
    pub path: String,
    #[serde(default = "default_neo4j_uri")]
    pub uri: String,
    #[serde(default = "default_neo4j_user")]
    pub user: String,
    #[serde(default = "default_neo4j_password")]
    pub password: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_graph_path() -> String {
    "syncore_code_graph.db".to_string()
}

/// Neo4j database configuration (legacy compatibility)
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

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            backend: GraphBackend::default(),
            path: default_graph_path(),
            uri: default_neo4j_uri(),
            user: default_neo4j_user(),
            password: default_neo4j_password(),
            enabled: default_true(),
        }
    }
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
    1_048_576
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
    "gguf_engine".to_string()
}
fn default_llm_model() -> String {
    "qwen2.5-0.5b".to_string()
}
fn default_llm_url() -> String {
    "local".to_string()
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

/// Reasoning backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    #[serde(default = "default_reasoning_backend")]
    pub backend: String,
    #[serde(default = "default_reasoning_namespace")]
    pub namespace: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_reasoning_backend() -> String {
    "sqlite".to_string()
}

fn default_reasoning_namespace() -> String {
    "syncore_default".to_string()
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            backend: default_reasoning_backend(),
            namespace: default_reasoning_namespace(),
            enabled: default_true(),
        }
    }
}

/// Main SynCore configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncoreConfig {
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub graph: GraphConfig,
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
    /// PHASE ST-12: Reasoning configuration
    #[serde(default)]
    pub reasoning: ReasoningConfig,
}

impl SyncoreConfig {
    /// Load configuration from a TOML file with graceful fallback
    ///
    /// CONFIG PRECEDENCE (Task 4B):
    /// 1. config/syncore.toml (PRIMARY)
    /// 2. Built-in defaults (LAST RESORT)
    ///
    /// Behavior:
    /// - If config file exists and is valid → load it completely
    /// - If config file does NOT exist → return default config with SQLiteGraph
    /// - If config file is malformed → fall back to SQLiteGraph defaults
    /// - Environment variables are NOT applied here (use load_with_env for overrides)
    pub fn load(path: &str) -> anyhow::Result<Self> {
        match fs::read_to_string(path) {
            Ok(content) => {
                // Config file exists, try to parse it
                match toml::from_str::<SyncoreConfig>(&content) {
                    Ok(mut config) => {
                        // Validate backend configuration
                        if !Self::is_valid_backend(&config.graph.backend) {
                            eprintln!("Warning: Invalid graph backend in config file, falling back to SQLiteGraph");
                            config.graph.backend = GraphBackend::SqliteGraph;
                        }
                        Ok(config)
                    }
                    Err(e) => {
                        eprintln!("Warning: Malformed config file '{}': {}", path, e);
                        eprintln!("Falling back to default configuration with SQLiteGraph backend");
                        Ok(Self::default())
                    }
                }
            }
            Err(_) => {
                // Config file doesn't exist, use defaults
                eprintln!("Config file not found at {}, using defaults", path);
                Ok(Self::default())
            }
        }
    }

    /// Load configuration with environment variable overrides
    ///
    /// CONFIG PRECEDENCE (Task 4B):
    /// 1. config/syncore.toml (PRIMARY)
    /// 2. Environment variables (OPTIONAL OVERRIDES)
    /// 3. Built-in defaults (LAST RESORT)
    ///
    /// Environment variables ONLY override individual fields, never replace missing config files.
    pub fn load_with_env(path: &str) -> anyhow::Result<Self> {
        let mut config = Self::load(path)?;
        config.apply_env_overrides();
        Ok(config)
    }

    /// Validate graph backend value
    fn is_valid_backend(backend: &GraphBackend) -> bool {
        matches!(backend, GraphBackend::SqliteGraph | GraphBackend::Neo4j)
    }

    /// Apply environment variable overrides (OPTIONAL overrides only)
    ///
    /// Environment variables MUST:
    /// - Only override fields individually
    /// - NEVER replace missing config files  
    /// - NEVER be required for default behavior
    /// - Gracefully handle invalid values with fallbacks
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

        // Graph overrides - support both old and new environment variable names
        // Graceful fallback for invalid values instead of panic
        if let Ok(val) = std::env::var("SYNC_GRAPH_BACKEND") {
            match val.to_lowercase().as_str() {
                "neo4j" => self.graph.backend = GraphBackend::Neo4j,
                "sqlite" | "sqlitegraph" => self.graph.backend = GraphBackend::SqliteGraph,
                _ => {
                    eprintln!(
                        "Warning: Invalid SYNC_GRAPH_BACKEND: '{}', falling back to SQLiteGraph",
                        val
                    );
                    self.graph.backend = GraphBackend::SqliteGraph;
                }
            }
        } else if let Ok(val) = std::env::var("GRAPH_BACKEND") {
            // Legacy support
            match val.to_lowercase().as_str() {
                "neo4j" => self.graph.backend = GraphBackend::Neo4j,
                "sqlitegraph" => self.graph.backend = GraphBackend::SqliteGraph,
                _ => {
                    eprintln!(
                        "Warning: Invalid GRAPH_BACKEND: '{}', falling back to SQLiteGraph",
                        val
                    );
                    self.graph.backend = GraphBackend::SqliteGraph;
                }
            }
        }

        if let Ok(val) = std::env::var("SYNC_SQLITE_DB_PATH") {
            self.graph.path = val;
        } else if let Ok(val) = std::env::var("GRAPH_PATH") {
            // Legacy support
            self.graph.path = val;
        }

        if let Ok(val) = std::env::var("SYNC_NEO4J_URI") {
            self.graph.uri = val.clone();
            self.neo4j.uri = val;
        } else if let Ok(val) = std::env::var("GRAPH_URI") {
            // Legacy support
            self.graph.uri = val.clone();
            self.neo4j.uri = val;
        }

        if let Ok(val) = std::env::var("SYNC_NEO4J_USER") {
            self.graph.user = val.clone();
            self.neo4j.user = val;
        } else if let Ok(val) = std::env::var("GRAPH_USER") {
            // Legacy support
            self.graph.user = val.clone();
            self.neo4j.user = val;
        }

        if let Ok(val) = std::env::var("SYNC_NEO4J_PASSWORD") {
            self.graph.password = val.clone();
            self.neo4j.password = val;
        } else if let Ok(val) = std::env::var("GRAPH_PASS") {
            // Legacy support
            self.graph.password = val.clone();
            self.neo4j.password = val;
        }

        // Neo4j overrides (legacy compatibility)
        if let Ok(val) = std::env::var("NEO4J_URI") {
            self.neo4j.uri = val.clone();
            self.graph.uri = val;
        }
        if let Ok(val) = std::env::var("NEO4J_USER") {
            self.neo4j.user = val.clone();
            self.graph.user = val;
        }
        if let Ok(val) = std::env::var("NEO4J_PASS") {
            self.neo4j.password = val.clone();
            self.graph.password = val;
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

    /// Default configuration for SQLite-only testing (Task 4 requirement)
    ///
    /// Creates a configuration optimized for testing with SQLiteGraph backend:
    /// - Backend: SQLiteGraph (embedded, no external dependencies)
    /// - In-memory database: :memory: for fast test isolation
    /// - All optional features disabled
    /// - Minimal logging
    ///
    /// # Returns
    ///
    /// SyncoreConfig with SQLiteGraph backend and test-optimized settings
    pub fn default_sqlite_test() -> Self {
        let mut config = Self::default();

        // Force SQLiteGraph backend
        config.graph.backend = GraphBackend::SqliteGraph;

        // Use in-memory database for fast test isolation
        config.graph.path = ":memory:".to_string();

        // Disable Neo4j features
        config.graph.enabled = false;
        config.neo4j.enabled = false;

        // Use in-memory database for main DB as well
        config.paths.db_path = ":memory:".to_string();
        config.paths.code_graph_db = ":memory:".to_string();

        // Minimal logging for tests
        config.paths.logs_dir = "/tmp/syncore_test_logs".to_string();

        // Disable optional features for clean testing
        config.llm.backend = "test".to_string();

        config
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
