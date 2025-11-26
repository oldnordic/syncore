use crate::dlr::{DlrError, Plugin, PluginStatus};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub struct PluginLoader {
    pub plugins: HashMap<String, Child>,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn discover_plugins<P: AsRef<Path>>(
        &self,
        plugins_dir: P,
    ) -> Result<Vec<Plugin>, DlrError> {
        let mut plugins = Vec::new();
        let plugins_path = plugins_dir.as_ref();

        if !plugins_path.exists() {
            return Ok(plugins);
        }

        for entry in std::fs::read_dir(plugins_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let plugin_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let binary_path = path.join("target/release").join(&plugin_name);
                if binary_path.exists() {
                    plugins.push(Plugin {
                        name: plugin_name.clone(),
                        version: "0.1.0".to_string(),
                        path: binary_path.to_string_lossy().to_string(),
                        status: PluginStatus::Unloaded,
                        capabilities: vec![],
                        process_id: None,
                    });
                }
            }
        }

        Ok(plugins)
    }

    pub fn load_plugin(&mut self, plugin: &mut Plugin) -> Result<(), DlrError> {
        if plugin.status != PluginStatus::Unloaded {
            return Err(DlrError::PluginStartFailed(format!(
                "Plugin {} is not in unloaded state",
                plugin.name
            )));
        }

        plugin.status = PluginStatus::Loading;

        let mut child = Command::new(&plugin.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                DlrError::PluginStartFailed(format!(
                    "Failed to start plugin {}: {}",
                    plugin.name, e
                ))
            })?;

        let process_id = child.id();
        plugin.process_id = Some(process_id);

        self.plugins.insert(plugin.name.clone(), child);

        plugin.status = PluginStatus::Ready;
        Ok(())
    }

    pub fn unload_plugin(&mut self, plugin: &mut Plugin) -> Result<(), DlrError> {
        if plugin.process_id.is_none() {
            return Ok(());
        }

        if let Some(mut child) = self.plugins.remove(&plugin.name) {
            let _ = child.kill();
            let _ = child.wait();
        }

        plugin.process_id = None;
        plugin.status = PluginStatus::Shutdown;
        Ok(())
    }

    pub fn get_plugin_child(&mut self, plugin_name: &str) -> Option<&mut Child> {
        self.plugins.get_mut(plugin_name)
    }

    pub fn remove_plugin_child(&mut self, plugin_name: &str) -> Option<Child> {
        self.plugins.remove(plugin_name)
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}
