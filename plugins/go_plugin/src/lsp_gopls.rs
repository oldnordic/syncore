use anyhow::{Context, Result};
use crate::plugin_api::{Entity, EntityKind, Edge, EdgeKind, PluginResult, Span};
use serde_json::Value;
use std::process::Command;

pub struct GoplsClient {
    available: bool,
}

impl GoplsClient {
    pub fn new() -> Result<Self> {
        let available = Command::new("gopls")
            .arg("version")
            .output()
            .map(|_| true)
            .unwrap_or(false);

        Ok(Self { available })
    }

    pub fn is_available(&self) -> bool {
        self.available
    }

    pub fn get_symbols(&self, file_path: &str) -> Result<PluginResult> {
        if !self.available {
            return Ok(PluginResult {
                entities: Some(Vec::new()),
                edges: Some(Vec::new()),
                diagnostics: None,
                meta: None,
            });
        }

        let output = Command::new("gopls")
            .arg("symbols")
            .arg(file_path)
            .output()
            .context("Failed to run gopls symbols")?;

        if !output.status.success() {
            return Ok(PluginResult {
                entities: Some(Vec::new()),
                edges: Some(Vec::new()),
                diagnostics: None,
                meta: None,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json_value: Value = serde_json::from_str(&stdout)
            .context("Failed to parse gopls symbols output")?;

        let mut entities = Vec::new();
        let edges = Vec::new();

        if let Some(symbols) = json_value.as_array() {
            for symbol in symbols {
                if let Some(entity) = self.parse_symbol(symbol, file_path) {
                    entities.push(entity);
                }
            }
        }

        Ok(PluginResult {
            entities: Some(entities),
            edges: Some(edges),
            diagnostics: None,
            meta: None,
        })
    }

    pub fn find_references(&self, file_path: &str, line: u32, column: u32) -> Result<PluginResult> {
        if !self.available {
            return Ok(PluginResult {
                entities: None,
                edges: Some(Vec::new()),
                diagnostics: None,
                meta: None,
            });
        }

        let output = Command::new("gopls")
            .arg("references")
            .arg(format!("{}:{}:{}", file_path, line, column))
            .output()
            .context("Failed to run gopls references")?;

        if !output.status.success() {
            return Ok(PluginResult {
                entities: None,
                edges: Some(Vec::new()),
                diagnostics: None,
                meta: None,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json_value: Value = serde_json::from_str(&stdout)
            .context("Failed to parse gopls references output")?;

        let mut edges = Vec::new();

        if let Some(references) = json_value.as_array() {
            for reference in references {
                if let Some(edge) = self.parse_reference(reference, file_path) {
                    edges.push(edge);
                }
            }
        }

        Ok(PluginResult {
            entities: None,
            edges: Some(edges),
            diagnostics: None,
            meta: None,
        })
    }

    pub fn workspace_symbol(&self, query: &str, workspace_path: &str) -> Result<PluginResult> {
        if !self.available {
            return Ok(PluginResult {
                entities: Some(Vec::new()),
                edges: Some(Vec::new()),
                diagnostics: None,
                meta: None,
            });
        }

        let output = Command::new("gopls")
            .arg("workspace_symbol")
            .arg(query)
            .current_dir(workspace_path)
            .output()
            .context("Failed to run gopls workspace_symbol")?;

        if !output.status.success() {
            return Ok(PluginResult {
                entities: Some(Vec::new()),
                edges: Some(Vec::new()),
                diagnostics: None,
                meta: None,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json_value: Value = serde_json::from_str(&stdout)
            .context("Failed to parse gopls workspace_symbol output")?;

        let mut entities = Vec::new();

        if let Some(symbols) = json_value.as_array() {
            for symbol in symbols {
                if let Some(entity) = self.parse_workspace_symbol(symbol) {
                    entities.push(entity);
                }
            }
        }

        Ok(PluginResult {
            entities: Some(entities),
            edges: Some(Vec::new()),
            diagnostics: None,
            meta: None,
        })
    }

    fn parse_symbol(&self, symbol: &Value, file_path: &str) -> Option<Entity> {
        let name = symbol.get("name")?.as_str()?;
        let kind = symbol.get("kind")?.as_u64()?;
        let location = symbol.get("location")?;
        let span = location.get("span")?;

        let entity_kind = self.map_symbol_kind(kind)?;
        let start = span.get("start")?;
        let end = span.get("end")?;

        Some(Entity {
            file_path: file_path.to_string(),
            name: name.to_string(),
            kind: entity_kind,
            signature: symbol.get("detail").and_then(|d| d.as_str()).map(|s| s.to_string()),
            span: Some(Span {
                start_line: start.get("line")?.as_u64()? as u32 + 1,
                start_col: start.get("column")?.as_u64()? as u32,
                end_line: end.get("line")?.as_u64()? as u32 + 1,
                end_col: end.get("column")?.as_u64()? as u32,
            }),
            extra: None,
        })
    }

    fn parse_workspace_symbol(&self, symbol: &Value) -> Option<Entity> {
        let name = symbol.get("name")?.as_str()?;
        let kind = symbol.get("kind")?.as_u64()?;
        let location = symbol.get("location")?;
        let uri = location.get("uri")?.as_str()?;
        let span = location.get("span")?;

        let entity_kind = self.map_symbol_kind(kind)?;
        let file_path = uri.strip_prefix("file://")?;
        let start = span.get("start")?;
        let end = span.get("end")?;

        Some(Entity {
            file_path: file_path.to_string(),
            name: name.to_string(),
            kind: entity_kind,
            signature: symbol.get("detail").and_then(|d| d.as_str()).map(|s| s.to_string()),
            span: Some(Span {
                start_line: start.get("line")?.as_u64()? as u32 + 1,
                start_col: start.get("column")?.as_u64()? as u32,
                end_line: end.get("line")?.as_u64()? as u32 + 1,
                end_col: end.get("column")?.as_u64()? as u32,
            }),
            extra: None,
        })
    }

    fn parse_reference(&self, reference: &Value, file_path: &str) -> Option<Edge> {
        let uri = reference.get("uri")?.as_str()?;
        let span = reference.get("span")?;
        let start = span.get("start")?;
        
        Some(Edge {
            from: format!("{}:{}", file_path, start.get("line")?.as_u64()? + 1),
            to: uri.strip_prefix("file://")?.to_string(),
            kind: EdgeKind::References,
        })
    }

    fn map_symbol_kind(&self, kind: u64) -> Option<EntityKind> {
        match kind {
            1 => Some(EntityKind::File),
            2 => Some(EntityKind::Module),
            3 => Some(EntityKind::Namespace),
            4 => Some(EntityKind::Package),
            5 => Some(EntityKind::Class),
            6 => Some(EntityKind::Method),
            7 => Some(EntityKind::Property),
            8 => Some(EntityKind::Field),
            9 => Some(EntityKind::Constructor),
            10 => Some(EntityKind::Enum),
            11 => Some(EntityKind::Interface),
            12 => Some(EntityKind::Function),
            13 => Some(EntityKind::Variable),
            14 => Some(EntityKind::Constant),
            15 => Some(EntityKind::String),
            16 => Some(EntityKind::Number),
            17 => Some(EntityKind::Boolean),
            18 => Some(EntityKind::Array),
            19 => Some(EntityKind::Object),
            20 => Some(EntityKind::Key),
            21 => Some(EntityKind::Null),
            22 => Some(EntityKind::EnumMember),
            23 => Some(EntityKind::Struct),
            24 => Some(EntityKind::Event),
            25 => Some(EntityKind::Operator),
            26 => Some(EntityKind::TypeParameter),
            _ => None,
        }
    }
}