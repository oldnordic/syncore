use crate::dlr::{
    DlrError, IpcClient, Plugin, PluginCapability, PluginLoader, PluginRegistry, PluginStatus,
};
use std::collections::HashMap;
use std::path::Path;

pub struct DlrManager {
    loader: PluginLoader,
    registry: PluginRegistry,
    plugins_dir: String,
}

impl DlrManager {
    pub fn new<P: AsRef<Path>>(plugins_dir: P) -> Self {
        Self {
            loader: PluginLoader::new(),
            registry: PluginRegistry::new(),
            plugins_dir: plugins_dir.as_ref().to_string_lossy().to_string(),
        }
    }

    pub fn discover_and_register_plugins(&mut self) -> Result<usize, DlrError> {
        let plugins = self.loader.discover_plugins(&self.plugins_dir)?;
        let mut count = 0;

        for plugin in plugins {
            self.registry.register_plugin(plugin)?;
            count += 1;
        }

        Ok(count)
    }

    pub fn load_plugin(&mut self, plugin_name: &str) -> Result<(), DlrError> {
        let plugin = self
            .registry
            .get_plugin_mut(plugin_name)
            .ok_or_else(|| DlrError::PluginNotFound(plugin_name.to_string()))?;

        self.loader.load_plugin(plugin)?;

        if let Some(child) = self.loader.remove_plugin_child(plugin_name) {
            let mut ipc_client = IpcClient::new(child)?;

            let init_response = ipc_client.init_plugin(&plugin.name, &plugin.version)?;
            if init_response.status != "ready" {
                return Err(DlrError::PluginStartFailed(format!(
                    "Plugin {} initialization failed",
                    plugin_name
                )));
            }

            let capabilities_response = ipc_client.get_capabilities()?;
            plugin.capabilities = capabilities_response
                .capabilities
                .into_iter()
                .filter_map(|cap| match cap.as_str() {
                    "index_directory" => Some(PluginCapability::IndexDirectory),
                    "lsp_ingest" => Some(PluginCapability::LspIngest),
                    "lint_ingest" => Some(PluginCapability::LintIngest),
                    "diagnostics_export" => Some(PluginCapability::DiagnosticsExport),
                    _ => None,
                })
                .collect();

            let child = ipc_client.into_child();
            self.loader.plugins.insert(plugin_name.to_string(), child);
        }

        Ok(())
    }

    pub fn unload_plugin(&mut self, plugin_name: &str) -> Result<(), DlrError> {
        let plugin = self
            .registry
            .get_plugin_mut(plugin_name)
            .ok_or_else(|| DlrError::PluginNotFound(plugin_name.to_string()))?;

        if let Some(child) = self.loader.plugins.remove(plugin_name) {
            let mut ipc_client = IpcClient::new(child)?;
            let _ = ipc_client.shutdown_plugin();
            let mut child = ipc_client.into_child();
            let _ = child.kill();
            let _ = child.wait();
        }

        self.loader.unload_plugin(plugin)?;
        Ok(())
    }

    pub fn execute_plugin_task(
        &mut self,
        plugin_name: &str,
        task: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, DlrError> {
        let plugin = self
            .registry
            .get_plugin(plugin_name)
            .ok_or_else(|| DlrError::PluginNotFound(plugin_name.to_string()))?;

        if !matches!(plugin.status, PluginStatus::Ready) {
            return Err(DlrError::ExecutionFailed(format!(
                "Plugin {} is not ready",
                plugin_name
            )));
        }

        if let Some(child) = self.loader.plugins.remove(plugin_name) {
            let mut ipc_client = IpcClient::new(child)?;

            let response = ipc_client.execute_task(task, params)?;

            let child = ipc_client.into_child();
            self.loader.plugins.insert(plugin_name.to_string(), child);

            let mut result = HashMap::new();
            result.insert("result".to_string(), response.result);
            result.insert(
                "diagnostics".to_string(),
                serde_json::Value::Array(response.diagnostics),
            );
            result.insert(
                "entities".to_string(),
                serde_json::Value::Array(response.entities),
            );

            Ok(result)
        } else {
            Err(DlrError::ExecutionFailed(format!(
                "Plugin {} process not found",
                plugin_name
            )))
        }
    }

    pub fn get_plugin(&self, plugin_name: &str) -> Option<&Plugin> {
        self.registry.get_plugin(plugin_name)
    }

    pub fn list_plugins(&self) -> Vec<&Plugin> {
        self.registry.list_plugins()
    }

    pub fn find_plugins_by_capability(&self, capability: &PluginCapability) -> Vec<&Plugin> {
        let cap_str = format!("{:?}", capability);
        self.registry.find_plugins_by_capability(&cap_str)
    }

    pub fn shutdown_all(&mut self) -> Result<(), DlrError> {
        let plugin_names: Vec<String> = self
            .registry
            .list_plugins()
            .iter()
            .map(|p| p.name.clone())
            .collect();

        for plugin_name in plugin_names {
            let _ = self.unload_plugin(&plugin_name);
        }

        Ok(())
    }
}

impl Drop for DlrManager {
    fn drop(&mut self) {
        let _ = self.shutdown_all();
    }
}
