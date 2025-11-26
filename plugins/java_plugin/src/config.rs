use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub javac_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classpath: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errorprone_jar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pmd_bin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruleset: Option<String>,
}

impl Default for JavaConfig {
    fn default() -> Self {
        Self {
            javac_path: None,
            classpath: None,
            errorprone_jar: None,
            pmd_bin: None,
            ruleset: Some("rulesets/java/quickstart.xml".to_string()),
        }
    }
}

impl JavaConfig {
    pub fn from_params(params: &HashMap<String, serde_json::Value>) -> Self {
        let mut config = JavaConfig::default();
        
        if let Some(javac_path) = params.get("javac_path").and_then(|v| v.as_str()) {
            config.javac_path = Some(javac_path.to_string());
        }
        
        if let Some(classpath) = params.get("classpath").and_then(|v| v.as_str()) {
            config.classpath = Some(classpath.to_string());
        }
        
        if let Some(errorprone_jar) = params.get("errorprone_jar").and_then(|v| v.as_str()) {
            config.errorprone_jar = Some(errorprone_jar.to_string());
        }
        
        if let Some(pmd_bin) = params.get("pmd_bin").and_then(|v| v.as_str()) {
            config.pmd_bin = Some(pmd_bin.to_string());
        }
        
        if let Some(ruleset) = params.get("ruleset").and_then(|v| v.as_str()) {
            config.ruleset = Some(ruleset.to_string());
        }
        
        config
    }
}