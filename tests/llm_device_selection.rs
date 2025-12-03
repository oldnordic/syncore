//! TDD tests for LLM device selection (Phase 13)
//! Tests config + env precedence and device resolution

use std::env;
use syncore::config::{GgufDevice, LlmConfig};

#[test]
fn test_device_selection_config_cpu_no_env() {
    // Test: Config: device="cpu", no env → GGUFEngine uses Cpu
    let config = LlmConfig {
        backend: "gguf".to_string(),
        model: "test".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
        model_path: "test.gguf".to_string(),
        tokenizer_path: "test.json".to_string(),
        device: "cpu".to_string(),
    };

    let resolved = config.resolved_device();
    assert_eq!(resolved, GgufDevice::Cpu);
    assert_eq!(resolved.as_str(), "cpu");
}

#[test]
fn test_device_selection_config_gpu_no_env() {
    // Test: Config: device="gpu", no env → GGUFEngine uses GpuVulkan
    let config = LlmConfig {
        backend: "gguf".to_string(),
        model: "test".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
        model_path: "test.gguf".to_string(),
        tokenizer_path: "test.json".to_string(),
        device: "gpu".to_string(),
    };

    let resolved = config.resolved_device();
    assert_eq!(resolved, GgufDevice::GpuVulkan);
    assert_eq!(resolved.as_str(), "gpu_vulkan");
}

#[test]
fn test_device_selection_env_cpu_overrides_config_gpu() {
    // Test: Env: SYNC_LLM_DEVICE=cpu, config=device="gpu" → Env wins → Cpu
    env::set_var("SYNC_LLM_DEVICE", "cpu");

    let config = LlmConfig {
        backend: "gguf".to_string(),
        model: "test".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
        model_path: "test.gguf".to_string(),
        tokenizer_path: "test.json".to_string(),
        device: "gpu".to_string(), // Config says GPU
    };

    let resolved = config.resolved_device();
    assert_eq!(resolved, GgufDevice::Cpu);

    env::remove_var("SYNC_LLM_DEVICE");
}

#[test]
fn test_device_selection_env_gpu_overrides_config_cpu() {
    // Test: Env: SYNC_LLM_DEVICE=gpu, config=device="cpu" → Env wins → GpuVulkan
    env::set_var("SYNC_LLM_DEVICE", "gpu");

    let config = LlmConfig {
        backend: "gguf".to_string(),
        model: "test".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
        model_path: "test.gguf".to_string(),
        tokenizer_path: "test.json".to_string(),
        device: "cpu".to_string(), // Config says CPU
    };

    let resolved = config.resolved_device();
    assert_eq!(resolved, GgufDevice::GpuVulkan);

    env::remove_var("SYNC_LLM_DEVICE");
}

#[test]
fn test_device_selection_invalid_config_value() {
    // Test: Invalid value (device="foobar") → defaults to CPU with warning
    let config = LlmConfig {
        backend: "gguf".to_string(),
        model: "test".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
        model_path: "test.gguf".to_string(),
        tokenizer_path: "test.json".to_string(),
        device: "foobar".to_string(), // Invalid
    };

    let resolved = config.resolved_device();
    assert_eq!(resolved, GgufDevice::Cpu);
}

#[test]
fn test_device_selection_invalid_env_value() {
    // Test: Invalid env value → defaults to CPU with warning
    env::set_var("SYNC_LLM_DEVICE", "invalid");

    let config = LlmConfig {
        backend: "gguf".to_string(),
        model: "test".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
        model_path: "test.gguf".to_string(),
        tokenizer_path: "test.json".to_string(),
        device: "gpu".to_string(),
    };

    let resolved = config.resolved_device();
    assert_eq!(resolved, GgufDevice::Cpu);

    env::remove_var("SYNC_LLM_DEVICE");
}

#[test]
fn test_device_selection_case_insensitive() {
    // Test: Case insensitive handling
    let test_cases = vec![
        ("CPU", GgufDevice::Cpu),
        ("cpu", GgufDevice::Cpu),
        ("Cpu", GgufDevice::Cpu),
        ("GPU", GgufDevice::GpuVulkan),
        ("gpu", GgufDevice::GpuVulkan),
        ("Gpu", GgufDevice::GpuVulkan),
    ];

    for (device_str, expected) in test_cases {
        let config = LlmConfig {
            backend: "gguf".to_string(),
            model: "test".to_string(),
            url: "local".to_string(),
            timeout_seconds: 30,
            model_path: "test.gguf".to_string(),
            tokenizer_path: "test.json".to_string(),
            device: device_str.to_string(),
        };

        let resolved = config.resolved_device();
        assert_eq!(resolved, expected, "Failed for device_str: {}", device_str);
    }
}

#[test]
fn test_device_selection_env_precedence_over_config() {
    // Test comprehensive precedence: env > config > default
    env::set_var("SYNC_LLM_DEVICE", "gpu");

    let config = LlmConfig {
        backend: "gguf".to_string(),
        model: "test".to_string(),
        url: "local".to_string(),
        timeout_seconds: 30,
        model_path: "test.gguf".to_string(),
        tokenizer_path: "test.json".to_string(),
        device: "cpu".to_string(),
    };

    let resolved = config.resolved_device();
    assert_eq!(resolved, GgufDevice::GpuVulkan);

    env::remove_var("SYNC_LLM_DEVICE");

    // Without env, config should be used
    let resolved = config.resolved_device();
    assert_eq!(resolved, GgufDevice::Cpu);
}
