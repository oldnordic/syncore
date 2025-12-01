use syncore::dlr::{DlrError, Plugin, PluginCapability, PluginRegistry, PluginStatus};

#[cfg(test)]
mod dlr_registry_tests {
    use super::*;

    fn create_test_plugin(name: &str, status: PluginStatus) -> Plugin {
        Plugin {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            path: format!("/fake/path/{}", name),
            status,
            capabilities: vec![PluginCapability::IndexDirectory, PluginCapability::LspIngest],
            process_id: None,
        }
    }

    #[test]
    fn test_registry_register_plugin() {
        let mut registry = PluginRegistry::new();
        let plugin = create_test_plugin("test_plugin", PluginStatus::Unloaded);

        let result = registry.register_plugin(plugin);
        assert!(result.is_ok(), "Should register plugin successfully");

        let retrieved_plugin = registry.get_plugin("test_plugin");
        assert!(retrieved_plugin.is_some(), "Should retrieve registered plugin");
        assert_eq!(retrieved_plugin.unwrap().name, "test_plugin");
    }

    #[test]
    fn test_registry_prevents_duplicate_registration() {
        let mut registry = PluginRegistry::new();
        let plugin1 = create_test_plugin("duplicate_plugin", PluginStatus::Unloaded);
        let plugin2 = create_test_plugin("duplicate_plugin", PluginStatus::Ready);

        registry.register_plugin(plugin1).unwrap();

        let result = registry.register_plugin(plugin2);
        assert!(result.is_err(), "Should prevent duplicate registration");
        assert!(matches!(result.unwrap_err(), DlrError::RegistryError(_)));
    }

    #[test]
    fn test_registry_unregister_plugin() {
        let mut registry = PluginRegistry::new();
        let plugin = create_test_plugin("test_plugin", PluginStatus::Unloaded);

        registry.register_plugin(plugin).unwrap();

        let result = registry.unregister_plugin("test_plugin");
        assert!(result.is_ok(), "Should unregister plugin successfully");

        let retrieved_plugin = registry.get_plugin("test_plugin");
        assert!(retrieved_plugin.is_none(), "Plugin should be removed after unregistration");
    }

    #[test]
    fn test_registry_unregister_nonexistent_plugin() {
        let mut registry = PluginRegistry::new();

        let result = registry.unregister_plugin("nonexistent_plugin");
        assert!(result.is_err(), "Should fail to unregister nonexistent plugin");
        assert!(matches!(result.unwrap_err(), DlrError::RegistryError(_)));
    }

    #[test]
    fn test_registry_list_plugins() {
        let mut registry = PluginRegistry::new();

        let plugin1 = create_test_plugin("plugin1", PluginStatus::Unloaded);
        let plugin2 = create_test_plugin("plugin2", PluginStatus::Ready);

        registry.register_plugin(plugin1).unwrap();
        registry.register_plugin(plugin2).unwrap();

        let plugins = registry.list_plugins();
        assert_eq!(plugins.len(), 2, "Should list all registered plugins");

        let plugin_names: Vec<String> = plugins.iter().map(|p| p.name.clone()).collect();
        assert!(plugin_names.contains(&"plugin1".to_string()));
        assert!(plugin_names.contains(&"plugin2".to_string()));
    }

    #[test]
    fn test_registry_find_plugins_by_capability() {
        let mut registry = PluginRegistry::new();

        let mut plugin1 = create_test_plugin("plugin1", PluginStatus::Ready);
        plugin1.capabilities = vec![PluginCapability::IndexDirectory];

        let mut plugin2 = create_test_plugin("plugin2", PluginStatus::Ready);
        plugin2.capabilities = vec![PluginCapability::LspIngest];

        let mut plugin3 = create_test_plugin("plugin3", PluginStatus::Ready);
        plugin3.capabilities = vec![PluginCapability::IndexDirectory, PluginCapability::LspIngest];

        registry.register_plugin(plugin1).unwrap();
        registry.register_plugin(plugin2).unwrap();
        registry.register_plugin(plugin3).unwrap();

        let index_plugins = registry.find_plugins_by_capability("IndexDirectory");
        assert_eq!(index_plugins.len(), 2, "Should find 2 plugins with IndexDirectory capability");

        let lsp_plugins = registry.find_plugins_by_capability("LspIngest");
        assert_eq!(lsp_plugins.len(), 2, "Should find 2 plugins with LspIngest capability");

        let lint_plugins = registry.find_plugins_by_capability("LintIngest");
        assert_eq!(lint_plugins.len(), 0, "Should find 0 plugins with LintIngest capability");
    }

    #[test]
    fn test_registry_get_ready_plugins() {
        let mut registry = PluginRegistry::new();

        let plugin1 = create_test_plugin("ready_plugin1", PluginStatus::Ready);
        let plugin2 = create_test_plugin("ready_plugin2", PluginStatus::Ready);
        let plugin3 = create_test_plugin("busy_plugin", PluginStatus::Busy);
        let plugin4 = create_test_plugin("unloaded_plugin", PluginStatus::Unloaded);

        registry.register_plugin(plugin1).unwrap();
        registry.register_plugin(plugin2).unwrap();
        registry.register_plugin(plugin3).unwrap();
        registry.register_plugin(plugin4).unwrap();

        let ready_plugins = registry.get_ready_plugins();
        assert_eq!(ready_plugins.len(), 2, "Should find only ready plugins");

        let plugin_names: Vec<String> = ready_plugins.iter().map(|p| p.name.clone()).collect();
        assert!(plugin_names.contains(&"ready_plugin1".to_string()));
        assert!(plugin_names.contains(&"ready_plugin2".to_string()));
        assert!(!plugin_names.contains(&"busy_plugin".to_string()));
        assert!(!plugin_names.contains(&"unloaded_plugin".to_string()));
    }

    #[test]
    fn test_registry_update_plugin_status() {
        let mut registry = PluginRegistry::new();
        let plugin = create_test_plugin("test_plugin", PluginStatus::Unloaded);

        registry.register_plugin(plugin).unwrap();

        let result = registry.update_plugin_status("test_plugin", PluginStatus::Ready);
        assert!(result.is_ok(), "Should update plugin status successfully");

        let updated_plugin = registry.get_plugin("test_plugin").unwrap();
        assert_eq!(updated_plugin.status, PluginStatus::Ready);
    }

    #[test]
    fn test_registry_update_nonexistent_plugin_status() {
        let mut registry = PluginRegistry::new();

        let result = registry.update_plugin_status("nonexistent_plugin", PluginStatus::Ready);
        assert!(result.is_err(), "Should fail to update nonexistent plugin status");
        assert!(matches!(result.unwrap_err(), DlrError::RegistryError(_)));
    }

    #[test]
    fn test_registry_plugin_exists() {
        let mut registry = PluginRegistry::new();
        let plugin = create_test_plugin("test_plugin", PluginStatus::Unloaded);

        assert!(
            !registry.plugin_exists("test_plugin"),
            "Plugin should not exist before registration"
        );

        registry.register_plugin(plugin).unwrap();
        assert!(registry.plugin_exists("test_plugin"), "Plugin should exist after registration");

        assert!(
            !registry.plugin_exists("nonexistent_plugin"),
            "Nonexistent plugin should not exist"
        );
    }

    #[test]
    fn test_registry_count_plugins() {
        let mut registry = PluginRegistry::new();

        assert_eq!(registry.count_plugins(), 0, "Should have 0 plugins initially");

        let plugin1 = create_test_plugin("plugin1", PluginStatus::Unloaded);
        registry.register_plugin(plugin1).unwrap();
        assert_eq!(registry.count_plugins(), 1, "Should have 1 plugin after first registration");

        let plugin2 = create_test_plugin("plugin2", PluginStatus::Unloaded);
        registry.register_plugin(plugin2).unwrap();
        assert_eq!(registry.count_plugins(), 2, "Should have 2 plugins after second registration");

        registry.unregister_plugin("plugin1").unwrap();
        assert_eq!(registry.count_plugins(), 1, "Should have 1 plugin after unregistration");
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = PluginRegistry::new();

        let plugin1 = create_test_plugin("plugin1", PluginStatus::Unloaded);
        let plugin2 = create_test_plugin("plugin2", PluginStatus::Unloaded);

        registry.register_plugin(plugin1).unwrap();
        registry.register_plugin(plugin2).unwrap();
        assert_eq!(registry.count_plugins(), 2, "Should have 2 plugins before clear");

        registry.clear();
        assert_eq!(registry.count_plugins(), 0, "Should have 0 plugins after clear");
        assert!(registry.list_plugins().is_empty(), "Should have no plugins after clear");
    }

    #[test]
    fn test_registry_get_plugin_mut() {
        let mut registry = PluginRegistry::new();
        let plugin = create_test_plugin("test_plugin", PluginStatus::Unloaded);

        registry.register_plugin(plugin).unwrap();

        let plugin_mut = registry.get_plugin_mut("test_plugin");
        assert!(plugin_mut.is_some(), "Should get mutable reference to plugin");

        let plugin_ref = plugin_mut.unwrap();
        plugin_ref.version = "0.2.0".to_string();

        let updated_plugin = registry.get_plugin("test_plugin").unwrap();
        assert_eq!(updated_plugin.version, "0.2.0", "Should have updated version");
    }
}
