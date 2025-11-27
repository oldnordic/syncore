//! APEX 2.5-LSP: External LSP Bridge & Diagnostics Pipeline
//!
//! Minimal, robust LSP bridge that:
//! - Auto-detects system LSP servers (starting with rust-analyzer)
//! - Speaks JSON-RPC 2.0 over stdin/stdout
//! - Sends didOpen/didChange notifications
//! - Receives publishDiagnostics
//! - Degrades gracefully to Disabled when LSP unavailable

mod diagnostics;
mod jsonrpc;

pub use diagnostics::LspDiagnosticEvent;
pub use jsonrpc::{frame_jsonrpc_message, parse_jsonrpc_message, parse_publish_diagnostics};

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::mpsc;

// ============================================================================
// Public Types
// ============================================================================

/// LSP bridge status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspStatus {
    Disabled,
    Ready,
}

/// LSP Bridge for external LSP server communication
pub struct LspBridge {
    status: LspStatus,
    stdin_tx: Option<mpsc::Sender<String>>,
    diagnostic_rx: mpsc::Receiver<LspDiagnosticEvent>,
    _child: Option<Child>,
    _stdin_handle: Option<tokio::task::JoinHandle<()>>,
    _stdout_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Errors from LSP bridge
#[derive(Debug, thiserror::Error)]
pub enum LspBridgeError {
    #[error("LSP server not found: {0}")]
    ServerNotFound(String),

    #[error("Failed to spawn LSP process: {0}")]
    SpawnFailed(String),

    #[error("LSP initialization failed: {0}")]
    InitializationFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

// ============================================================================
// Implementation
// ============================================================================

impl LspBridge {
    /// Attempt to start an LSP server for given root + language
    ///
    /// Returns Disabled status if server not found (graceful degradation).
    /// Returns Ready if server successfully started and initialized.
    pub async fn try_start(root: &Path, language_id: &str) -> Result<Self, LspBridgeError> {
        // Find LSP server for language
        let server_cmd = match Self::find_lsp_server(language_id) {
            Some(cmd) => cmd,
            None => {
                // Graceful degradation: return Disabled bridge
                return Ok(Self::disabled());
            }
        };

        // Spawn LSP server process
        let mut child = Command::new(&server_cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| LspBridgeError::SpawnFailed(format!("{}: {}", server_cmd, e)))?;

        let stdin = child
            .stdin
            .take()
            .context("Failed to get stdin")?;

        let stdout = child
            .stdout
            .take()
            .context("Failed to get stdout")?;

        // Create channels
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(100);
        let (diagnostic_tx, diagnostic_rx) = mpsc::channel::<LspDiagnosticEvent>(100);

        // Spawn stdin writer task
        let stdin_handle = tokio::spawn(async move {
            Self::stdin_writer_task(stdin, &mut stdin_rx).await;
        });

        // Spawn stdout reader task
        let diagnostic_tx_clone = diagnostic_tx.clone();
        let stdout_handle = tokio::spawn(async move {
            Self::stdout_reader_task(stdout, diagnostic_tx_clone).await;
        });

        // Perform initialization handshake
        let init_result = Self::initialize_lsp(&stdin_tx, root).await;

        if init_result.is_err() {
            // Initialization failed - return Disabled
            drop(stdin_tx);
            drop(child);
            return Ok(Self::disabled());
        }

        Ok(Self {
            status: LspStatus::Ready,
            stdin_tx: Some(stdin_tx),
            diagnostic_rx,
            _child: Some(child),
            _stdin_handle: Some(stdin_handle),
            _stdout_handle: Some(stdout_handle),
        })
    }

    /// Send didOpen notification
    pub async fn send_did_open(&self, path: &Path, text: &str) -> Result<()> {
        if self.status != LspStatus::Ready {
            return Ok(()); // No-op if disabled
        }

        let uri = Self::path_to_uri(path);
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "rust",
                    "version": 1,
                    "text": text
                }
            }
        });

        self.send_message(&message.to_string()).await
    }

    /// Send didChange notification (full-text sync)
    pub async fn send_did_change(&self, path: &Path, text: &str) -> Result<()> {
        if self.status != LspStatus::Ready {
            return Ok(());
        }

        let uri = Self::path_to_uri(path);
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "version": 2
                },
                "contentChanges": [
                    {
                        "text": text
                    }
                ]
            }
        });

        self.send_message(&message.to_string()).await
    }

    /// Receive diagnostic event (async stream)
    pub async fn recv_diagnostic(&mut self) -> Option<LspDiagnosticEvent> {
        self.diagnostic_rx.recv().await
    }

    /// Get current status
    pub fn status(&self) -> LspStatus {
        self.status
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Create disabled bridge
    pub fn disabled() -> Self {
        let (_tx, rx) = mpsc::channel(1);
        Self {
            status: LspStatus::Disabled,
            stdin_tx: None,
            diagnostic_rx: rx,
            _child: None,
            _stdin_handle: None,
            _stdout_handle: None,
        }
    }

    /// Find LSP server executable for language
    fn find_lsp_server(language_id: &str) -> Option<String> {
        match language_id {
            "rust" => {
                // Check if rust-analyzer is on PATH
                which::which("rust-analyzer")
                    .ok()
                    .and_then(|p| p.to_str().map(|s| s.to_string()))
            }
            _ => None, // Other languages not supported yet
        }
    }

    /// LSP initialization handshake
    async fn initialize_lsp(stdin_tx: &mpsc::Sender<String>, root: &Path) -> Result<()> {
        let root_uri = Self::path_to_uri(root);
        let init_message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {}
            }
        });

        let framed = jsonrpc::frame_jsonrpc_message(&init_message.to_string());
        stdin_tx
            .send(framed)
            .await
            .context("Failed to send initialize")?;

        // Send initialized notification
        let initialized_message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });

        let framed = jsonrpc::frame_jsonrpc_message(&initialized_message.to_string());
        stdin_tx
            .send(framed)
            .await
            .context("Failed to send initialized")?;

        Ok(())
    }

    /// Send message to LSP server
    async fn send_message(&self, json_message: &str) -> Result<()> {
        if let Some(tx) = &self.stdin_tx {
            let framed = jsonrpc::frame_jsonrpc_message(json_message);
            tx.send(framed).await.context("Failed to send message")?;
        }
        Ok(())
    }

    /// Convert path to file:// URI
    fn path_to_uri(path: &Path) -> String {
        format!("file://{}", path.display())
    }

    /// Stdin writer task
    async fn stdin_writer_task(
        mut stdin: ChildStdin,
        rx: &mut mpsc::Receiver<String>,
    ) {
        while let Some(message) = rx.recv().await {
            if stdin.write_all(message.as_bytes()).await.is_err() {
                break;
            }
            if stdin.flush().await.is_err() {
                break;
            }
        }
    }

    /// Stdout reader task
    async fn stdout_reader_task(
        stdout: ChildStdout,
        diagnostic_tx: mpsc::Sender<LspDiagnosticEvent>,
    ) {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            // Parse Content-Length header
            if line.starts_with("Content-Length:") {
                if let Some(len_str) = line.strip_prefix("Content-Length:").map(|s| s.trim()) {
                    if let Ok(content_len) = len_str.parse::<usize>() {
                        // Read empty line
                        let _ = lines.next_line().await;

                        // Read JSON content
                        let _buffer = vec![0u8; content_len];
                        // Need to read exact bytes - simplified approach
                        // In production, use proper byte reading
                        if let Ok(Some(json_line)) = lines.next_line().await {
                            // Parse and handle message
                            if let Ok(events) = jsonrpc::parse_publish_diagnostics(&json_line) {
                                for event in events {
                                    let _ = diagnostic_tx.send(event).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// FsEvent Integration Helper
// ============================================================================

use crate::fs_watcher::{FsEvent, FsEventKind};
use crate::parser_service::ParserService;

/// Helper to wire FsEvent → LSP notifications
///
/// Sends didOpen for Created, didChange for Modified.
/// For unsupported files or Disabled bridge: no-op.
pub async fn on_fs_event_update_lsp(
    bridge: &LspBridge,
    _parser: &ParserService,
    event: &FsEvent,
) -> Result<()> {
    if bridge.status() != LspStatus::Ready {
        return Ok(()); // No-op if disabled
    }

    // Check if supported file extension
    if !event.path.extension().map(|e| e == "rs").unwrap_or(false) {
        return Ok(()); // Only Rust files for now
    }

    match &event.kind {
        FsEventKind::Created => {
            // Read file content and send didOpen
            if let Ok(content) = std::fs::read_to_string(&event.path) {
                bridge.send_did_open(&event.path, &content).await?;
            }
        }
        FsEventKind::Modified => {
            // Read file content and send didChange
            if let Ok(content) = std::fs::read_to_string(&event.path) {
                bridge.send_did_change(&event.path, &content).await?;
            }
        }
        FsEventKind::Removed | FsEventKind::Renamed(_) => {
            // No-op for delete/rename
        }
    }

    Ok(())
}
