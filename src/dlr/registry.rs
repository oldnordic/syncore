use crate::dlr::{DlrError, Plugin, PluginStatus};
use std::collections::HashMap;

pub struct PluginRegistry {
    plugins: HashMap<String, Plugin>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn register_plugin(&mut self, plugin: Plugin) -> Result<(), DlrError> {
        if self.plugins.contains_key(&plugin.name) {
            return Err(DlrError::RegistryError(format!(
                "Plugin {} already registered",
                plugin.name
            )));
        }

        self.plugins.insert(plugin.name.clone(), plugin);
        Ok(())
    }

    pub fn unregister_plugin(&mut self, plugin_name: &str) -> Result<Plugin, DlrError> {
        self.plugins
            .remove(plugin_name)
            .ok_or_else(|| DlrError::RegistryError(format!("Plugin {} not found", plugin_name)))
    }

    pub fn get_plugin(&self, plugin_name: &str) -> Option<&Plugin> {
        self.plugins.get(plugin_name)
    }

    pub fn get_plugin_mut(&mut self, plugin_name: &str) -> Option<&mut Plugin> {
        self.plugins.get_mut(plugin_name)
    }

    pub fn list_plugins(&self) -> Vec<&Plugin> {
        self.plugins.values().collect()
    }

    pub fn find_plugins_by_capability(&self, capability: &str) -> Vec<&Plugin> {
        self.plugins
            .values()
            .filter(|p| {
                p.capabilities
                    .iter()
                    .any(|c| format!("{:?}", c) == capability)
            })
            .collect()
    }

    pub fn get_ready_plugins(&self) -> Vec<&Plugin> {
        self.plugins
            .values()
            .filter(|p| matches!(p.status, PluginStatus::Ready))
            .collect()
    }

    pub fn update_plugin_status(
        &mut self,
        plugin_name: &str,
        status: PluginStatus,
    ) -> Result<(), DlrError> {
        let plugin = self
            .plugins
            .get_mut(plugin_name)
            .ok_or_else(|| DlrError::RegistryError(format!("Plugin {} not found", plugin_name)))?;

        plugin.status = status;
        Ok(())
    }

    pub fn plugin_exists(&self, plugin_name: &str) -> bool {
        self.plugins.contains_key(plugin_name)
    }

    pub fn count_plugins(&self) -> usize {
        self.plugins.len()
    }

    pub fn clear(&mut self) {
        self.plugins.clear();
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
