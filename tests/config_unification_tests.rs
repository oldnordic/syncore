//! APEX 2.0-M-FIX: Config Unification Tests
//!
//! Tests for MemoryConfig integration into SyncoreConfig.

use anyhow::Result;
use syncore::config::SyncoreConfig;
use syncore::memory::MemoryConfig;
use std::env;
use tempfile::NamedTempFile;

#[test]
fn test_syncore_config_has_memory_field() {
    let config = SyncoreConfig::default();
    
    // Should have memory field with default namespace
    assert_eq!(config.memory.default_namespace, "default");
    assert_eq!(config.memory.enable_semantic_search, true);
}

#[test]
fn test_memory_config_serialization() -> Result<()> {
    // Create config with custom memory settings
    let config = MemoryConfig {
        enable_semantic_search: false,
        auto_summarize_threshold: 1000,
        consolidation_similarity: 0.85,
        default_namespace: "custom".to_string(),
    };

    // Should serialize/deserialize without error
    let json = serde_json::to_string(&config)?;
    let restored: MemoryConfig = serde_json::from_str(&json)?;

    assert_eq!(restored.default_namespace, "custom");
    assert_eq!(restored.enable_semantic_search, false);
    assert_eq!(restored.auto_summarize_threshold, 1000);

    Ok(())
}

#[test]
fn test_syncore_config_toml_memory_section() -> Result<()> {
    // Create temp TOML with [memory] section
    let mut temp = NamedTempFile::new()?;
    use std::io::Write;
    writeln!(temp, "[memory]")?;
    writeln!(temp, "default_namespace = \"test_ns\"")?;
    writeln!(temp, "enable_semantic_search = false")?;
    writeln!(temp, "auto_summarize_threshold = 999")?;
    temp.flush()?;

    let config = SyncoreConfig::load(temp.path().to_str().unwrap())?;

    assert_eq!(config.memory.default_namespace, "test_ns");
    assert_eq!(config.memory.enable_semantic_search, false);
    assert_eq!(config.memory.auto_summarize_threshold, 999);

    Ok(())
}

#[test]
fn test_env_override_memory_default_namespace() -> Result<()> {
    // Cleanup any residual env from other tests
    env::remove_var("SYNCORE_MEMORY_DEFAULT_NAMESPACE");

    // Set environment variable
    env::set_var("SYNCORE_MEMORY_DEFAULT_NAMESPACE", "env_override");

    // Load config (should apply env override)
    let mut config = SyncoreConfig::default();
    config.apply_env_overrides();

    assert_eq!(config.memory.default_namespace, "env_override");

    // Cleanup
    env::remove_var("SYNCORE_MEMORY_DEFAULT_NAMESPACE");

    Ok(())
}

#[test]
fn test_env_override_does_not_affect_other_fields() -> Result<()> {
    // Cleanup any residual env from other tests
    env::remove_var("SYNCORE_MEMORY_DEFAULT_NAMESPACE");

    env::set_var("SYNCORE_MEMORY_DEFAULT_NAMESPACE", "env_test");

    let mut config = SyncoreConfig::default();
    let original_threshold = config.memory.auto_summarize_threshold;
    let original_enable = config.memory.enable_semantic_search;

    config.apply_env_overrides();

    // Namespace changed, others unchanged
    assert_eq!(config.memory.default_namespace, "env_test");
    assert_eq!(config.memory.auto_summarize_threshold, original_threshold);
    assert_eq!(config.memory.enable_semantic_search, original_enable);

    env::remove_var("SYNCORE_MEMORY_DEFAULT_NAMESPACE");

    Ok(())
}

#[test]
fn test_toml_and_env_precedence() -> Result<()> {
    // Cleanup any residual env from other tests
    env::remove_var("SYNCORE_MEMORY_DEFAULT_NAMESPACE");

    // TOML says "toml_ns", ENV says "env_ns"
    // ENV should win

    let mut temp = NamedTempFile::new()?;
    use std::io::Write;
    writeln!(temp, "[memory]")?;
    writeln!(temp, "default_namespace = \"toml_ns\"")?;
    temp.flush()?;

    env::set_var("SYNCORE_MEMORY_DEFAULT_NAMESPACE", "env_ns");

    let config = SyncoreConfig::load_with_env(temp.path().to_str().unwrap())?;

    // ENV should override TOML
    assert_eq!(config.memory.default_namespace, "env_ns");

    env::remove_var("SYNCORE_MEMORY_DEFAULT_NAMESPACE");

    Ok(())
}
