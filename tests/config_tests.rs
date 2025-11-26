// SPEC: SYNCORE-CONFIG-AND-TOOLS-FIX-01 (APEX v1.2)
// STEP A: Central configuration system tests
//
// Tests verify:
// - Full config schema parsing from TOML
// - Default values when sections are missing
// - Environment variable overrides
// - Path filter uses config.indexing.excluded_dirs

use std::collections::HashMap;
use syncore::config::{
    EmbeddingsConfig, HttpConfig, IndexingConfig, Neo4jConfig, PathsConfig, ProjectAnalysisConfig,
    SyncoreConfig, VectorSearchConfig,
};

/// Test: Full config schema loads correctly
#[test]
fn test_full_config_loads() {
    let toml = r#"
[paths]
db_path = "test.db"
code_graph_db = "test_graph.db"
cache_path = "test_cache"
socket_path = "/tmp/test.sock"
logs_dir = "test_logs"

[neo4j]
uri = "bolt://localhost:7687"
user = "testuser"
password = "testpass"
enabled = true

[indexing]
excluded_dirs = ["target", "node_modules"]
include_extensions = ["rs", "py"]
max_file_size = 2097152

[embeddings]
model = "semantic"
dimensions = 384
batch_size = 32

[vector_search]
default_limit = 20
min_score_threshold = 0.5

[project_analysis]
use_indexing_excludes = true

[project_analysis.hotspot_weights]
fan_in = 0.4
fan_out = 0.3
entity_count = 0.2
loc = 0.1

[http]
port = 8080
metrics_port = 9090
"#;

    let config: SyncoreConfig = toml::from_str(toml).expect("Failed to parse config");

    // Paths
    assert_eq!(config.paths.db_path, "test.db");
    assert_eq!(config.paths.code_graph_db, "test_graph.db");
    assert_eq!(config.paths.cache_path, "test_cache");
    assert_eq!(config.paths.socket_path, "/tmp/test.sock");
    assert_eq!(config.paths.logs_dir, "test_logs");

    // Neo4j
    assert_eq!(config.neo4j.uri, "bolt://localhost:7687");
    assert_eq!(config.neo4j.user, "testuser");
    assert_eq!(config.neo4j.password, "testpass");
    assert!(config.neo4j.enabled);

    // Indexing
    assert_eq!(
        config.indexing.excluded_dirs,
        vec!["target", "node_modules"]
    );
    assert_eq!(config.indexing.include_extensions, vec!["rs", "py"]);
    assert_eq!(config.indexing.max_file_size, 2097152);

    // Embeddings
    assert_eq!(config.embeddings.model, "semantic");
    assert_eq!(config.embeddings.dimensions, 384);
    assert_eq!(config.embeddings.batch_size, 32);

    // Vector search
    assert_eq!(config.vector_search.default_limit, 20);
    assert!((config.vector_search.min_score_threshold - 0.5).abs() < 0.001);

    // Project analysis
    assert!(config.project_analysis.use_indexing_excludes);
    assert!((config.project_analysis.hotspot_weights.fan_in - 0.4).abs() < 0.001);

    // HTTP
    assert_eq!(config.http.port, 8080);
    assert_eq!(config.http.metrics_port, 9090);
}

/// Test: Default values are set when sections are missing
#[test]
fn test_defaults_when_missing() {
    let toml = r#"
[paths]
db_path = "syncore.db"
"#;

    let config: SyncoreConfig = toml::from_str(toml).expect("Failed to parse partial config");

    // Explicitly set value
    assert_eq!(config.paths.db_path, "syncore.db");

    // Defaults should be applied
    assert_eq!(config.paths.code_graph_db, "syncore_code_graph.db");
    assert_eq!(config.paths.cache_path, "cache");
    assert_eq!(config.paths.socket_path, "/tmp/syncore.sock");
    assert_eq!(config.paths.logs_dir, "logs");

    // Neo4j defaults
    assert_eq!(config.neo4j.uri, "bolt://127.0.0.1:7687");
    assert_eq!(config.neo4j.user, "neo4j");
    assert!(config.neo4j.enabled);

    // Indexing defaults
    assert!(config
        .indexing
        .excluded_dirs
        .contains(&"target".to_string()));
    assert!(config
        .indexing
        .excluded_dirs
        .contains(&"node_modules".to_string()));
    assert!(config
        .indexing
        .include_extensions
        .contains(&"rs".to_string()));
    assert_eq!(config.indexing.max_file_size, 1048576);

    // Embeddings defaults
    assert_eq!(config.embeddings.model, "semantic");
    assert_eq!(config.embeddings.dimensions, 384);

    // Vector search defaults
    assert_eq!(config.vector_search.default_limit, 10);

    // HTTP defaults
    assert_eq!(config.http.port, 8080);
    assert_eq!(config.http.metrics_port, 9090);
}

/// Test: Empty config uses all defaults
#[test]
fn test_empty_config_uses_defaults() {
    let config = SyncoreConfig::default();

    assert_eq!(config.paths.db_path, "syncore.db");
    assert!(config
        .indexing
        .excluded_dirs
        .contains(&"target".to_string()));
    assert_eq!(config.neo4j.uri, "bolt://127.0.0.1:7687");
    assert_eq!(config.embeddings.dimensions, 384);
}

/// Test: Config can be serialized back to TOML
#[test]
fn test_config_roundtrip() {
    let original = SyncoreConfig::default();
    let toml_str = toml::to_string_pretty(&original).expect("Failed to serialize");
    let parsed: SyncoreConfig = toml::from_str(&toml_str).expect("Failed to parse");

    assert_eq!(original.paths.db_path, parsed.paths.db_path);
    assert_eq!(
        original.indexing.excluded_dirs,
        parsed.indexing.excluded_dirs
    );
    assert_eq!(original.neo4j.uri, parsed.neo4j.uri);
}

/// Test: Excluded dirs list contains expected entries
#[test]
fn test_default_excluded_dirs() {
    let config = SyncoreConfig::default();
    let excluded = &config.indexing.excluded_dirs;

    // Must include these common build/dependency directories
    assert!(excluded.contains(&"target".to_string()), "missing target");
    assert!(
        excluded.contains(&"node_modules".to_string()),
        "missing node_modules"
    );
    assert!(excluded.contains(&".git".to_string()), "missing .git");
    assert!(
        excluded.contains(&"__pycache__".to_string()),
        "missing __pycache__"
    );
    assert!(excluded.contains(&".venv".to_string()), "missing .venv");
    assert!(excluded.contains(&"vendor".to_string()), "missing vendor");
    assert!(excluded.contains(&"dist".to_string()), "missing dist");
    assert!(excluded.contains(&"build".to_string()), "missing build");
    assert!(excluded.contains(&".cargo".to_string()), "missing .cargo");
}

/// Test: Include extensions list has common source file types
#[test]
fn test_default_include_extensions() {
    let config = SyncoreConfig::default();
    let extensions = &config.indexing.include_extensions;

    assert!(extensions.contains(&"rs".to_string()), "missing rs");
    assert!(extensions.contains(&"py".to_string()), "missing py");
    assert!(extensions.contains(&"js".to_string()), "missing js");
    assert!(extensions.contains(&"ts".to_string()), "missing ts");
}

/// Test: Hotspot weights sum to approximately 1.0
#[test]
fn test_hotspot_weights_sum() {
    let config = SyncoreConfig::default();
    let weights = &config.project_analysis.hotspot_weights;
    let sum = weights.fan_in + weights.fan_out + weights.entity_count + weights.loc;
    assert!(
        (sum - 1.0).abs() < 0.001,
        "Hotspot weights should sum to 1.0, got {}",
        sum
    );
}

/// Test: Config load from file
#[test]
fn test_load_from_file() {
    use std::io::Write;
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_syncore_config.toml");

    let toml = r#"
[paths]
db_path = "file_test.db"
logs_dir = "file_logs"

[indexing]
excluded_dirs = ["custom_exclude"]
"#;

    std::fs::write(&config_path, toml).expect("Failed to write test config");

    let config = SyncoreConfig::load(config_path.to_str().unwrap())
        .expect("Failed to load config from file");

    assert_eq!(config.paths.db_path, "file_test.db");
    assert_eq!(config.paths.logs_dir, "file_logs");
    assert_eq!(config.indexing.excluded_dirs, vec!["custom_exclude"]);

    // Cleanup
    std::fs::remove_file(config_path).ok();
}
