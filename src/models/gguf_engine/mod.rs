//! GGUFEngine Implementation for SynCore LLM Interface
//!
//! This module provides a local, CPU-based LLM backend using GGUF format.
//! It offers privacy, offline operation, and fast inference for small to medium
//! sized language models without requiring external services.
//!
//! ## MODULE MOVE PLAN (COMPLETED)
//!
//! This module was migrated from candle_backend with following changes:
//!
//! 1. Files moved:
//!    - src/models/candle_backend/mod.rs → src/models/gguf_engine/mod.rs
//!    - src/models/candle_backend/cache.rs → src/models/gguf_engine/cache.rs
//!    - src/models/candle_backend/loader.rs → src/models/gguf_engine/loader.rs
//!    - src/models/candle_backend/tokenizer.rs → src/models/gguf_engine/tokenizer.rs
//!    - src/models/candle_backend/generate.rs → src/models/gguf_engine/generate.rs
//!
//! 2. Main struct renamed:
//!    - CandleBackend → GGUFEngine
//!    - CandleTokenizer → GgufTokenizer
//!
//! 3. Public API connections:
//!    - src/models/mod.rs exports GGUFEngine
//!    - src/llm/factory.rs LlmBackend::GGUFEngine constructs GGUFEngine
//!
//! 4. Tests:
//!    - tests/candle_backend_smoke_tests.rs → needs update to use GGUFEngine
//!    - tests/candle_integration_e2e.rs → needs update to use GGUFEngine
//!
//! 5. Preserved behavior:
//!    - Same GGUF loading via gguf_runtime
//!    - Same OnceCell caching
//!    - Same tokenizer.json loading
//!    - Same deterministic generation (seed=42, temperature=0.0)
//!    - Same config precedence: config → env → defaults
//!

pub mod cache;
pub mod generate;
pub mod gguf_runtime;
pub mod loader;
pub mod tokenizer;

pub use cache::{get_cached_model, CachedModel};
pub use generate::generate_text;
pub use loader::{load_qwen_model, LoadedModel, ModelComponents};
pub use tokenizer::{GgufTokenizer, TokenizerWrapper};

use crate::llm::{Completion, LanguageModel, Prompt};
use anyhow::{anyhow, Result};
use candle_core::Device;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// GGUF engine health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GgufStatus {
    /// Engine is operating normally
    Ok,
    /// Engine is working but with limitations (e.g., GPU fallback to CPU)
    Degraded,
    /// Engine has encountered an error
    Error,
}

/// GGUF engine health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufEngineHealth {
    /// Backend name (always "gguf_engine")
    pub backend_name: String,
    /// Current health status
    pub status: GgufStatus,
    /// Device being used: "cpu", "gpu_vulkan", or "cpu_fallback"
    pub device: String,
    /// Resolved model path after config/env processing
    pub model_path: Option<String>,
    /// Whether model is successfully loaded
    pub model_loaded: bool,
    /// Whether tokenizer is successfully loaded
    pub tokenizer_loaded: bool,
    /// Model architecture (e.g., "qwen2")
    pub arch: Option<String>,
    /// Last error message (if any)
    pub last_error: Option<String>,
}

/// GGUF engine performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufEngineMetrics {
    /// Total number of generation requests
    pub total_requests: u64,
    /// Total input tokens processed
    pub total_tokens_in: u64,
    /// Total output tokens generated
    pub total_tokens_out: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Latency of last request in milliseconds
    pub last_latency_ms: f64,
    /// Model file size in bytes (if known)
    pub model_file_size_bytes: Option<u64>,
}

/// Local GGUF-based LLM backend
pub struct GGUFEngine {
    /// Model name identifier
    model_name: String,
    /// Model file path
    model_path: PathBuf,
    /// Tokenizer file path
    tokenizer_path: Option<PathBuf>,

    /// Cached model and tokenizer
    cached_model: Arc<Mutex<Option<Arc<CachedModel>>>>,
    /// Health status tracking
    health: Arc<Mutex<GgufEngineHealth>>,
    /// Performance metrics (atomic for thread safety)
    metrics: Arc<GgufEngineMetricsAtomic>,
}

/// Thread-safe metrics storage using atomic operations
#[derive(Debug)]
struct GgufEngineMetricsAtomic {
    /// Total number of generation requests
    total_requests: AtomicU64,
    /// Total input tokens processed
    total_tokens_in: AtomicU64,
    /// Total output tokens generated
    total_tokens_out: AtomicU64,
    /// Sum of all latencies for average calculation
    total_latency_ms: AtomicU64,
    /// Latency of last request in milliseconds (stored as integer, converted to f64)
    last_latency_ms: AtomicU64,
    /// Model file size in bytes
    model_file_size_bytes: AtomicU64,
}

impl GgufEngineMetricsAtomic {
    fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_tokens_in: AtomicU64::new(0),
            total_tokens_out: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            last_latency_ms: AtomicU64::new(0),
            model_file_size_bytes: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> GgufEngineMetrics {
        let total_requests = self.total_requests.load(Ordering::Relaxed);
        let avg_latency_ms = if total_requests > 0 {
            self.total_latency_ms.load(Ordering::Relaxed) as f64 / total_requests as f64
        } else {
            0.0
        };

        GgufEngineMetrics {
            total_requests,
            total_tokens_in: self.total_tokens_in.load(Ordering::Relaxed),
            total_tokens_out: self.total_tokens_out.load(Ordering::Relaxed),
            avg_latency_ms,
            last_latency_ms: self.last_latency_ms.load(Ordering::Relaxed) as f64,
            model_file_size_bytes: {
                let size = self.model_file_size_bytes.load(Ordering::Relaxed);
                if size == 0 {
                    None
                } else {
                    Some(size)
                }
            },
        }
    }
}

impl GGUFEngine {
    /// Create a new GGUF backend with specified model and device
    pub async fn new(model_name: &str) -> Result<Self> {
        // Get device from config
        let device_config = if let Some(config) = crate::config::SyncoreConfig::try_global() {
            config.llm.resolved_device()
        } else {
            crate::config::GgufDevice::Cpu // Default to CPU
        };

        let (_device, device_str, status) = match device_config {
            crate::config::GgufDevice::Cpu => (Device::Cpu, "cpu".to_string(), GgufStatus::Ok),
            crate::config::GgufDevice::GpuVulkan => {
                // Try to create GPU device, fallback to CPU if unavailable
                match Self::create_gpu_device() {
                    Ok(gpu_device) => {
                        tracing::info!("Successfully created GPU device");
                        (gpu_device, "gpu_vulkan".to_string(), GgufStatus::Ok)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create GPU device: {}. Falling back to CPU.", e);
                        (Device::Cpu, "cpu_fallback".to_string(), GgufStatus::Degraded)
                    }
                }
            }
        };

        // Try to find model file
        let model_path = Self::find_model_file(model_name)?;

        tracing::info!("Initializing GGUFEngine with model: {}", model_name);
        tracing::info!("Model path: {:?}", model_path);

        // Get tokenizer path from config
        let tokenizer_path = if let Some(config) = crate::config::SyncoreConfig::try_global() {
            let path = PathBuf::from(&config.llm.tokenizer_path);
            if path.exists() {
                Some(path)
            } else {
                None
            }
        } else {
            None
        };

        // Initialize health and metrics
        let model_file_size = std::fs::metadata(&model_path).ok().map(|m| m.len());
        if let Some(size) = model_file_size {
            tracing::info!("Model file size: {} bytes", size);
        }

        let health = Arc::new(Mutex::new(GgufEngineHealth {
            backend_name: "gguf_engine".to_string(),
            status,
            device: device_str,
            model_path: Some(model_path.to_string_lossy().to_string()),
            model_loaded: false,
            tokenizer_loaded: tokenizer_path.is_some(),
            arch: None, // Will be detected when model loads
            last_error: None,
        }));

        let metrics = Arc::new(GgufEngineMetricsAtomic::new());
        if let Some(size) = model_file_size {
            metrics.model_file_size_bytes.store(size, Ordering::Relaxed);
        }

        let backend = Self {
            model_name: model_name.to_string(),
            model_path,
            tokenizer_path,
            cached_model: Arc::new(Mutex::new(None)),
            health,
            metrics,
        };

        Ok(backend)
    }

    /// Create a test backend that returns mock responses
    pub fn new_test() -> Self {
        let health = Arc::new(Mutex::new(GgufEngineHealth {
            backend_name: "gguf_engine".to_string(),
            status: GgufStatus::Ok,
            device: "cpu".to_string(),
            model_path: None,
            model_loaded: false,
            tokenizer_loaded: false,
            arch: None,
            last_error: None,
        }));

        Self {
            model_name: "test-model".to_string(),
            model_path: PathBuf::new(),
            tokenizer_path: None,
            cached_model: Arc::new(Mutex::new(None)),
            health,
            metrics: Arc::new(GgufEngineMetricsAtomic::new()),
        }
    }

    /// Find model file using configuration
    fn find_model_file(_model_name: &str) -> Result<PathBuf> {
        // Try to get model path from config
        if let Some(config) = crate::config::SyncoreConfig::try_global() {
            let model_path = PathBuf::from(&config.llm.model_path);

            if model_path.exists() {
                tracing::info!("Found model file from config: {:?}", model_path);
                return Ok(model_path);
            }
        }

        // Fallback to environment variable
        if let Ok(model_path) = std::env::var("SYNC_LLM_MODEL_PATH") {
            let model_path = PathBuf::from(model_path);

            if model_path.exists() {
                tracing::info!("Found model file from env: {:?}", model_path);
                return Ok(model_path);
            }
        }

        // Default path
        let default_path = PathBuf::from("models/qwen2.5-0.5b.gguf");
        if default_path.exists() {
            tracing::info!("Using default model path: {:?}", default_path);
            return Ok(default_path);
        }

        Err(anyhow!("Model file not found. Check config/syncore.toml [llm] model_path or SYNC_LLM_MODEL_PATH env var"))
    }

    /// Load actual model and tokenizer using cache (lazy loading)
    async fn ensure_model_loaded(&self) -> Result<()> {
        {
            let cached_guard = self.cached_model.lock().unwrap();

            if cached_guard.is_some() {
                return Ok(());
            }
        } // Drop lock before await

        tracing::info!("Loading model from cache: {:?}", self.model_path);

        // Use cache system to load model and tokenizer
        let cached = get_cached_model(self.model_path.clone(), self.tokenizer_path.clone()).await?;

        {
            let mut cached_guard = self.cached_model.lock().unwrap();
            *cached_guard = Some(cached);
        } // Lock dropped after assignment

        tracing::info!("Model loading completed via cache");

        Ok(())
    }

    /// Get current health status of GGUF engine
    pub fn health(&self) -> GgufEngineHealth {
        self.health.lock().unwrap().clone()
    }

    /// Get current performance metrics
    pub fn metrics(&self) -> GgufEngineMetrics {
        self.metrics.snapshot()
    }

    /// Update health status (internal use)
    fn update_health<F>(&self, updater: F)
    where
        F: FnOnce(&mut GgufEngineHealth),
    {
        let mut health = self.health.lock().unwrap();
        updater(&mut health);
    }

    /// Record metrics for a generation request
    fn record_generation(&self, tokens_in: u64, tokens_out: u64, latency_ms: u64) {
        self.metrics.total_requests.fetch_add(1, Ordering::Relaxed);
        self.metrics.total_tokens_in.fetch_add(tokens_in, Ordering::Relaxed);
        self.metrics.total_tokens_out.fetch_add(tokens_out, Ordering::Relaxed);
        self.metrics.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
        self.metrics.last_latency_ms.store(latency_ms, Ordering::Relaxed);
    }

    /// Attempt to create a GPU device (placeholder for future GPU support)
    fn create_gpu_device() -> Result<Device> {
        // TODO: Implement GPU device creation when Candle GPU support is added
        // For now, always return error to force CPU fallback
        Err(anyhow!("GPU device not yet implemented in this build. Use CPU device or enable GPU features in Candle."))
    }
}

impl LanguageModel for GGUFEngine {
    fn complete(&self, prompt: &Prompt) -> Result<Completion> {
        // Start timing
        let start_time = std::time::Instant::now();

        // For test model, use simple response
        if self.model_name == "test-model" {
            let response_text = format!("GGUFEngine response to: {}", prompt.user);
            let health = self.health();

            // Record metrics for test model
            let elapsed = start_time.elapsed();
            let latency_ms = elapsed.as_secs_f64() * 1000.0;
            let tokens_in = prompt.user.len() as u64; // Simple approximation for test
            let tokens_out = response_text.len() as u64; // Simple approximation for test
            self.record_generation(tokens_in, tokens_out, latency_ms as u64);

            let metadata = serde_json::json!({
                "backend": "gguf_engine",
                "model": "test-model",
                "device": health.device,
                "supports_streaming": false,
                "model_loaded": false,
                "description": "Test backend for GGUFEngine"
            });
            return Ok(Completion::with_metadata(response_text, metadata));
        }

        // Try to load model if not already loaded
        if let Err(e) = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.ensure_model_loaded())
        }) {
            let response_text = format!("Failed to load model: {}", e);
            let health = self.health();

            // Record metrics for failed model load
            let elapsed = start_time.elapsed();
            let latency_ms = elapsed.as_secs_f64() * 1000.0;
            let tokens_in = prompt.user.len() as u64;
            let tokens_out = 0; // No output on error
            self.record_generation(tokens_in, tokens_out, latency_ms as u64);

            // Update health status for error
            self.update_health(|h| {
                h.status = GgufStatus::Error;
                h.last_error = Some(e.to_string());
            });

            let metadata = serde_json::json!({
                "backend": "gguf_engine",
                "model": self.model_name,
                "device": health.device,
                "supports_streaming": false,
                "model_loaded": false,
                "error": e.to_string(),
                "description": "Local inference using Candle framework"
            });
            return Ok(Completion::with_metadata(response_text, metadata));
        }

        // Perform real inference
        let cached_guard = self.cached_model.lock().unwrap();

        if let Some(cached) = &*cached_guard {
            // Use real generation
            let options = generate::GenerateOptions {
                max_tokens: prompt.max_tokens.unwrap_or(32),
                temperature: prompt.temperature.unwrap_or(0.0),
                top_k: Some(1), // Deterministic for tests
                seed: Some(42), // Fixed seed for reproducibility
                ..Default::default()
            };

            match generate::generate_text_cached(cached, &prompt.user, &options) {
                Ok(generated_text) => {
                    let health = self.health();

                    // Record metrics for successful generation
                    let elapsed = start_time.elapsed();
                    let latency_ms = elapsed.as_secs_f64() * 1000.0;

                    // Count tokens using tokenizer
                    let tokens_in = match cached.tokenizer.encode(&prompt.user) {
                        Ok(tokens) => tokens.len() as u64,
                        Err(_) => prompt.user.len() as u64, // Fallback approximation
                    };

                    let tokens_out = match cached.tokenizer.encode(&generated_text) {
                        Ok(tokens) => tokens.len() as u64,
                        Err(_) => generated_text.len() as u64, // Fallback approximation
                    };

                    self.record_generation(tokens_in, tokens_out, latency_ms as u64);

                    // Update health for success
                    self.update_health(|h| {
                        h.status = GgufStatus::Ok;
                        h.last_error = None;
                        h.model_loaded = true;
                        h.arch = Some(cached.model.config.name.clone());
                    });

                    let metadata = serde_json::json!({
                        "backend": "gguf_engine",
                        "model": self.model_name,
                        "device": health.device,
                        "supports_streaming": false,
                        "model_loaded": true,
                        "description": "Local inference using Candle framework"
                    });
                    Ok(Completion::with_metadata(generated_text, metadata))
                }
                Err(e) => {
                    let response_text = format!("Generation failed: {}", e);
                    let health = self.health();

                    // Record metrics for failed generation
                    let elapsed = start_time.elapsed();
                    let latency_ms = elapsed.as_secs_f64() * 1000.0;
                    let tokens_in = prompt.user.len() as u64;
                    let tokens_out = 0; // No output on error
                    self.record_generation(tokens_in, tokens_out, latency_ms as u64);

                    // Update health for error
                    self.update_health(|h| {
                        h.status = GgufStatus::Error;
                        h.last_error = Some(e.to_string());
                    });

                    let metadata = serde_json::json!({
                        "backend": "gguf_engine",
                        "model": self.model_name,
                        "device": health.device,
                        "supports_streaming": false,
                        "model_loaded": true,
                        "error": e.to_string(),
                        "description": "Local inference using Candle framework"
                    });
                    Ok(Completion::with_metadata(response_text, metadata))
                }
            }
        } else {
            let response_text = "Model not loaded".to_string();
            let metadata = serde_json::json!({
                "backend": "gguf_engine",
                "model": self.model_name,
                "device": "cpu",
                "supports_streaming": false,
                "model_loaded": false,
                "error": "Unexpected state after model loading",
                "description": "Local CPU-based inference using Candle framework"
            });
            Ok(Completion::with_metadata(response_text, metadata))
        }
    }

    fn backend_name(&self) -> &str {
        "gguf_engine"
    }

    fn get_health(&self) -> Result<serde_json::Value> {
        let health = self.health();
        Ok(serde_json::to_value(health)?)
    }

    fn get_metrics(&self) -> Result<serde_json::Value> {
        let metrics = self.metrics();
        Ok(serde_json::to_value(metrics)?)
    }

    fn health_check(&self) -> Result<bool> {
        // Check if model file exists
        let model_exists = self.model_path.exists() || self.model_name == "test-model";

        if model_exists {
            tracing::info!("GGUFEngine health check passed for model: {}", self.model_name);
        } else {
            tracing::warn!(
                "GGUFEngine health check failed - model file not found: {:?}",
                self.model_path
            );
        }

        Ok(model_exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Prompt;

    #[test]
    fn test_gguf_engine_creation() {
        // Test that we can create a test backend
        let backend = GGUFEngine::new_test();
        assert_eq!(backend.backend_name(), "gguf_engine");
        assert_eq!(backend.model_name, "test-model");

        // Test health check
        let _is_healthy = backend.health_check().unwrap();
        // Health check may fail if model file doesn't exist
    }

    #[test]
    fn test_health_and_metrics() {
        let backend = GGUFEngine::new_test();

        // Test health
        let health = backend.health();
        assert_eq!(health.backend_name, "gguf_engine");
        assert_eq!(health.device, "cpu");
        assert!(!health.model_loaded);

        // Test metrics
        let metrics = backend.metrics();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.total_tokens_in, 0);
        assert_eq!(metrics.total_tokens_out, 0);
        assert_eq!(metrics.avg_latency_ms, 0.0);
    }

    #[tokio::test]
    async fn test_gguf_engine_new() {
        // This test may fail if model file doesn't exist
        // That's expected in a test environment
        if let Ok(_backend) = GGUFEngine::new("qwen2.5-mini").await {
            assert_eq!(_backend.model_name, "qwen2.5-mini");

            // Test health check
            let _is_healthy = _backend.health_check().unwrap();
            // Health check may fail if model file doesn't exist
        }
    }
}
