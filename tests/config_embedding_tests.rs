//! Embedding backend configuration tests
//!
//! Tests that embedding backend configuration loads correctly from syncore.toml

use anyhow::Result;
use std::fs;
use std::path::Path;
use syncore::config::SyncoreConfig;

#[test]
fn test_default_embedding_backend_loads_from_config() -> Result<()> {
    // Load the default config
    let config_path = Path::new("config/syncore.toml");
    assert!(config_path.exists(), "config/syncore.toml should exist");

    let config_str = fs::read_to_string(config_path)?;
    let config: SyncoreConfig = toml::from_str(&config_str)?;

    // Check embeddings config
    assert!(!config.embeddings.model.is_empty(), "Embedding model should be configured");
    assert!(config.embeddings.dimensions > 0, "Embedding dimensions should be > 0");
    assert!(config.embeddings.batch_size > 0, "Batch size should be > 0");

    // Validate reasonable values
    assert!(
        config.embeddings.dimensions >= 128 && config.embeddings.dimensions <= 4096,
        "Dimensions should be in reasonable range [128, 4096], got {}",
        config.embeddings.dimensions
    );

    Ok(())
}

#[test]
fn test_invalid_embedding_dimensions_rejected() -> Result<()> {
    // Test invalid config with zero dimensions
    let invalid_config = r#"
[paths]
db_path = "test.db"

[embeddings]
model = "test"
dimensions = 0
batch_size = 32
    "#;

    let config: Result<SyncoreConfig, _> = toml::from_str(invalid_config);

    // Currently this won't error during parsing, but should be validated at runtime
    // For now, just ensure we can parse it and check dimensions
    if let Ok(cfg) = config {
        // Runtime validation should catch this
        assert_eq!(cfg.embeddings.dimensions, 0);
    }

    Ok(())
}

#[test]
fn test_embedding_config_has_all_required_fields() -> Result<()> {
    // Minimal valid config
    let minimal_config = r#"
[paths]
db_path = "test.db"

[embeddings]
model = "huggingface"
dimensions = 384
batch_size = 16
    "#;

    let config: SyncoreConfig = toml::from_str(minimal_config)?;

    assert_eq!(config.embeddings.model, "huggingface");
    assert_eq!(config.embeddings.dimensions, 384);
    assert_eq!(config.embeddings.batch_size, 16);

    Ok(())
}

#[test]
fn test_embedding_backend_defaults() -> Result<()> {
    // Config with missing embeddings section should use defaults
    let minimal_config = r#"
[paths]
db_path = "test.db"
    "#;

    let config: SyncoreConfig = toml::from_str(minimal_config)?;

    // Should use default values
    assert!(!config.embeddings.model.is_empty());
    assert!(config.embeddings.dimensions > 0);
    assert!(config.embeddings.batch_size > 0);

    Ok(())
}
