use serde::{Deserialize, Serialize};
use std::fs;

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
