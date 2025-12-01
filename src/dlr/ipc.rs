use crate::dlr::{
    plugin::{
        PluginCapabilitiesResponse, PluginExecuteRequest, PluginExecuteResponse, PluginInitRequest,
        PluginInitResponse, PluginShutdownRequest, PluginShutdownResponse,
    },
    DlrError,
};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::Child;

pub struct IpcClient {
    child: Child,
}

impl IpcClient {
    pub fn new(child: Child) -> Result<Self, DlrError> {
        // Just verify that stdin and stdout are available, but don't take them yet
        if child.stdin.is_none() {
            return Err(DlrError::IpcFailed("Cannot access stdin".to_string()));
        }
        if child.stdout.is_none() {
            return Err(DlrError::IpcFailed("Cannot access stdout".to_string()));
        }

        Ok(Self {
            child,
        })
    }

    pub fn init_plugin(
        &mut self,
        plugin_name: &str,
        version: &str,
    ) -> Result<PluginInitResponse, DlrError> {
        let request = PluginInitRequest {
            event: "init".to_string(),
            plugin_name: plugin_name.to_string(),
            version: version.to_string(),
        };

        let response_str = self.send_request(&request)?;
        let response: PluginInitResponse = serde_json::from_str(&response_str).map_err(|e| {
            DlrError::InvalidResponse(format!("Failed to parse init response: {}", e))
        })?;

        Ok(response)
    }

    pub fn get_capabilities(&mut self) -> Result<PluginCapabilitiesResponse, DlrError> {
        let request = serde_json::json!({
            "task": "capabilities"
        });

        let response_str = self.send_json_request(&request)?;
        let response: PluginCapabilitiesResponse =
            serde_json::from_str(&response_str).map_err(|e| {
                DlrError::InvalidResponse(format!("Failed to parse capabilities response: {}", e))
            })?;

        Ok(response)
    }

    pub fn execute_task(
        &mut self,
        task: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<PluginExecuteResponse, DlrError> {
        let request = PluginExecuteRequest {
            task: task.to_string(),
            params,
        };

        let response_str = self.send_request(&request)?;
        let response: PluginExecuteResponse = serde_json::from_str(&response_str).map_err(|e| {
            DlrError::InvalidResponse(format!("Failed to parse execute response: {}", e))
        })?;

        Ok(response)
    }

    pub fn shutdown_plugin(&mut self) -> Result<PluginShutdownResponse, DlrError> {
        let request = PluginShutdownRequest {
            event: "shutdown".to_string(),
        };

        let response_str = self.send_request(&request)?;
        let response: PluginShutdownResponse =
            serde_json::from_str(&response_str).map_err(|e| {
                DlrError::InvalidResponse(format!("Failed to parse shutdown response: {}", e))
            })?;

        Ok(response)
    }

    fn send_request<T: serde::Serialize>(&mut self, request: &T) -> Result<String, DlrError> {
        let request_str = serde_json::to_string(request)
            .map_err(|e| DlrError::IpcFailed(format!("Failed to serialize request: {}", e)))?;

        self.send_json_request(&serde_json::from_str(&request_str).unwrap())
    }

    fn send_json_request(&mut self, request: &serde_json::Value) -> Result<String, DlrError> {
        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| DlrError::IpcFailed("Cannot access stdin".to_string()))?;
        let stdout = self
            .child
            .stdout
            .as_mut()
            .ok_or_else(|| DlrError::IpcFailed("Cannot access stdout".to_string()))?;

        let request_str = serde_json::to_string(request)
            .map_err(|e| DlrError::IpcFailed(format!("Failed to serialize request: {}", e)))?;

        writeln!(stdin, "{}", request_str)
            .map_err(|e| DlrError::IpcFailed(format!("Failed to write to stdin: {}", e)))?;
        stdin.flush().map_err(|e| DlrError::IpcFailed(format!("Failed to flush stdin: {}", e)))?;

        let mut reader = BufReader::new(stdout);
        let mut response_line = String::new();

        let bytes_read = reader
            .read_line(&mut response_line)
            .map_err(|e| DlrError::IpcFailed(format!("Failed to read from stdout: {}", e)))?;

        if bytes_read == 0 {
            return Err(DlrError::IpcFailed("Plugin process terminated unexpectedly".to_string()));
        }

        Ok(response_line.trim().to_string())
    }

    pub fn into_child(self) -> Child {
        self.child
    }
}
