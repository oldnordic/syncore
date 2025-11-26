use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsJsConfig {
    pub tsserver_path: Option<String>,
    pub eslint_path: Option<String>,
    pub prettier_path: Option<String>,
    pub eslint_config: Option<String>,
    pub project_root: Option<String>,
}

impl Default for TsJsConfig {
    fn default() -> Self {
        Self {
            tsserver_path: None,
            eslint_path: Some("eslint".to_string()),
            prettier_path: Some("prettier".to_string()),
            eslint_config: None,
            project_root: None,
        }
    }
}

impl TsJsConfig {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(ref tsserver_path) = self.tsserver_path {
            if !PathBuf::from(tsserver_path).exists() {
                return Err(format!("tsserver_path does not exist: {}", tsserver_path));
            }
        }

        if let Some(ref eslint_path) = self.eslint_path {
            if eslint_path != "eslint" && !PathBuf::from(eslint_path).exists() {
                return Err(format!("eslint_path does not exist: {}", eslint_path));
            }
        }

        if let Some(ref prettier_path) = self.prettier_path {
            if prettier_path != "prettier" && !PathBuf::from(prettier_path).exists() {
                return Err(format!("prettier_path does not exist: {}", prettier_path));
            }
        }

        Ok(())
    }
}