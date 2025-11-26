#[derive(Debug)]
pub enum DlrError {
    PluginNotFound(String),

    PluginStartFailed(String),

    IpcFailed(String),

    ExecutionFailed(String),

    InvalidResponse(String),

    PluginCrashed(String),

    IoError(std::io::Error),

    JsonError(serde_json::Error),

    RegistryError(String),
}

impl std::fmt::Display for DlrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DlrError::PluginNotFound(msg) => write!(f, "Plugin not found: {}", msg),
            DlrError::PluginStartFailed(msg) => write!(f, "Plugin failed to start: {}", msg),
            DlrError::IpcFailed(msg) => write!(f, "IPC communication failed: {}", msg),
            DlrError::ExecutionFailed(msg) => write!(f, "Plugin execution failed: {}", msg),
            DlrError::InvalidResponse(msg) => write!(f, "Invalid plugin response: {}", msg),
            DlrError::PluginCrashed(msg) => write!(f, "Plugin crashed: {}", msg),
            DlrError::IoError(err) => write!(f, "IO error: {}", err),
            DlrError::JsonError(err) => write!(f, "JSON serialization error: {}", err),
            DlrError::RegistryError(msg) => write!(f, "Plugin registry error: {}", msg),
        }
    }
}

impl std::error::Error for DlrError {}

impl From<std::io::Error> for DlrError {
    fn from(err: std::io::Error) -> Self {
        DlrError::IoError(err)
    }
}

impl From<serde_json::Error> for DlrError {
    fn from(err: serde_json::Error) -> Self {
        DlrError::JsonError(err)
    }
}
