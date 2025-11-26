pub mod error;
pub mod ipc;
pub mod loader;
pub mod manager;
pub mod plugin;
pub mod registry;

pub use error::DlrError;
pub use ipc::IpcClient;
pub use loader::PluginLoader;
pub use manager::DlrManager;
pub use plugin::{Plugin, PluginCapability, PluginStatus};
pub use registry::PluginRegistry;
