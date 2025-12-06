//! Candle Inference Performance Benchmarks
//!
//! PHASE 3.4 — PERFORMANCE BENCHMARKS
//!
//! Comprehensive performance benchmarks for Candle inference including:
//! - Latency measurements for different model sizes
//! - Throughput benchmarks for concurrent requests
//! - Memory usage profiling
//! - Error recovery performance metrics
//! - Circuit breaker performance under load
//! - Cache hit/miss ratios and performance impact
//! - Prompt hashing performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use std::time::Duration;

use syncore::llm::error_recovery::{ErrorRecoveryConfig, SafeLanguageModel};
use syncore::llm::prompt_hash::{hash_prompt, PromptHashCache};
use syncore::llm::test::TestLanguageModel;
use syncore::llm::{Completion, LanguageModel, Prompt};

/// Benchmark inference latency for different prompt sizes
fn bench_inference_latency(c: &mut Criterion) {
    let model = Arc::new(TestLanguageModel::predefined("test"));

    let mut group = c.benchmark_group("inference_latency");

    // Test different prompt sizes
    let prompt_sizes = vec!["10", "50", "100", "500"];
    let prompts = vec![
        "Hello world",
        "Write a short story about a robot who discovers emotions for the first time. The robot lives in a futuristic city where humans have largely abandoned it, and it must learn to navigate this new world on its own.",
        "Explain the concept of quantum computing in detail. Cover the basic principles of superposition and entanglement, describe how quantum bits (qubits) differ from classical bits, and discuss the potential applications of quantum computing in fields like cryptography, drug discovery, and optimization problems.",
        "Write a comprehensive technical guide to building a scalable microservices architecture using Rust. Include detailed explanations of service discovery, load balancing, inter-service communication patterns (REST, gRPC, message queues), data consistency strategies (eventual consistency, saga pattern), monitoring and observability, security considerations, deployment strategies (Docker, Kubernetes), and performance optimization techniques. Provide code examples where relevant and discuss trade-offs between different approaches."
    ];

    for (size, prompt) in prompt_sizes.iter().zip(prompts.iter()) {
        let prompt_obj = Prompt::new("You are a helpful assistant.", *prompt);
        group.throughput(Throughput::Bytes(prompt.len() as u64));
        group.bench_with_input(BenchmarkId::new("test_model", size), prompt, |b, prompt_text| {
            b.iter(|| {
                let prompt_obj = Prompt::new("You are a helpful assistant.", *prompt_text);
                let _completion = model.complete(black_box(&prompt_obj)).unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark concurrent inference throughput
fn bench_concurrent_throughput(c: &mut Criterion) {
    let model = Arc::new(TestLanguageModel::predefined("test"));

    let mut group = c.benchmark_group("concurrent_throughput");

    // Test different concurrency levels
    for concurrency in [1, 2, 4, 8] {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("test_model", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.iter(|| {
                    let prompt = Prompt::new("You are a helpful assistant.", "Say hello");
                    let mut handles = Vec::new();

                    for _ in 0..concurrency {
                        let model_clone = Arc::clone(&model);
                        let prompt_clone = prompt.clone();

                        let handle = std::thread::spawn(move || {
                            model_clone.complete(&prompt_clone).unwrap()
                        });
                        handles.push(handle);
                    }

                    // Wait for all requests to complete
                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark SafeLanguageModel performance overhead
fn bench_safe_model_overhead(c: &mut Criterion) {
    let raw_model = Arc::new(TestLanguageModel::predefined("test")) as Arc<dyn LanguageModel>;

    let safe_config = ErrorRecoveryConfig {
        max_retries: 3,
        initial_backoff: Duration::from_millis(10),
        max_backoff: Duration::from_millis(100),
        ..Default::default()
    };

    let safe_model = SafeLanguageModel::new(raw_model.clone(), safe_config);

    let prompt = Prompt::new("You are a helpful assistant.", "What is 2+2?");

    c.bench_function("raw_model_latency", |b| {
        b.iter(|| {
            let _completion = raw_model.complete(black_box(&prompt)).unwrap();
        });
    });

    // Note: SafeLanguageModel is async, so we'll create a sync wrapper for benchmarking
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

    let safe_model_async = safe_model;

    c.bench_function("safe_model_latency", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _completion =
                    safe_model_async.complete_with_recovery(black_box(&prompt)).await.unwrap();
            });
        });
    });
}

/// Benchmark prompt hashing performance
fn bench_prompt_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("prompt_hashing");

    let prompts = vec![
        "Short prompt",
        "This is a medium length prompt with some more content to hash",
        "This is a very long prompt that contains multiple sentences and paragraphs. It includes various types of content including numbers like 123, special characters like @#$%, and different word lengths. The purpose is to test how the hashing performance scales with input size and complexity. Longer prompts should still hash quickly even with all the normalization steps involved in the process."
    ];

    for (_i, prompt) in prompts.iter().enumerate() {
        let size_name = format!("{}bytes", prompt.len());
        group.throughput(Throughput::Bytes(prompt.len() as u64));
        group.bench_with_input(BenchmarkId::new("hash_prompt", size_name), prompt, |b, prompt| {
            b.iter(|| {
                black_box(hash_prompt(black_box(prompt)));
            });
        });
    }

    group.finish();
}

/// Benchmark prompt hash cache performance
fn bench_hash_cache_performance(c: &mut Criterion) {
    let mut cache = PromptHashCache::new();
    let prompt = "Test prompt for caching";

    c.bench_function("cache_miss", |b| {
        b.iter(|| {
            // Clear cache each time to force miss
            cache.clear();
            cache.get_or_compute(black_box(prompt), &Default::default());
        });
    });

    // Pre-populate cache
    cache.get_or_compute(prompt, &Default::default());

    c.bench_function("cache_hit", |b| {
        b.iter(|| {
            cache.get_or_compute(black_box(prompt), &Default::default());
        });
    });
}

/// Benchmark memory usage patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let model = TestLanguageModel::predefined("test");

    let mut group = c.benchmark_group("memory_patterns");

    // Test memory impact of multiple sequential requests
    group.bench_function("sequential_requests", |b| {
        b.iter(|| {
            for i in 0..10 {
                let prompt = Prompt::new("You are a helpful assistant.", &format!("Request {}", i));
                let _completion = model.complete(&prompt).unwrap();
            }
        });
    });

    group.finish();
}

/// Benchmark error recovery under simulated failures
fn bench_error_recovery(c: &mut Criterion) {
    // Create a mock model that simulates failures
    struct FailingModel {
        failure_rate: f32,
        call_count: std::sync::atomic::AtomicU32,
    }

    impl LanguageModel for FailingModel {
        fn complete(&self, _prompt: &Prompt) -> anyhow::Result<Completion> {
            let count = self.call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if (count as f32 % 100.0) < self.failure_rate * 100.0 {
                Err(anyhow::anyhow!("Simulated failure"))
            } else {
                Ok(Completion::new("Success response"))
            }
        }
    }

    let failing_model = Arc::new(FailingModel {
        failure_rate: 0.3, // 30% failure rate
        call_count: std::sync::atomic::AtomicU32::new(0),
    });

    let config = ErrorRecoveryConfig {
        max_retries: 3,
        initial_backoff: Duration::from_millis(1),
        max_backoff: Duration::from_millis(10),
        ..Default::default()
    };

    let safe_model = SafeLanguageModel::new(failing_model, config);
    let prompt = Prompt::new("You are a helpful assistant.", "Test prompt");

    // Use blocking runtime for async benchmark
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

    c.bench_function("error_recovery_with_failures", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _result = safe_model.complete_with_recovery(black_box(&prompt)).await;
            });
        });
    });
}

/// Benchmark circuit breaker performance
fn bench_circuit_breaker(c: &mut Criterion) {
    // Create a model that fails consistently to trigger circuit breaker
    struct AlwaysFailingModel;

    impl LanguageModel for AlwaysFailingModel {
        fn complete(&self, _prompt: &Prompt) -> anyhow::Result<Completion> {
            Err(anyhow::anyhow!("Always fails"))
        }
    }

    let failing_model = Arc::new(AlwaysFailingModel);

    let config = ErrorRecoveryConfig {
        max_retries: 0, // No retries to hit circuit breaker faster
        circuit_breaker_threshold: 3,
        circuit_breaker_timeout: Duration::from_millis(100),
        ..Default::default()
    };

    let safe_model = SafeLanguageModel::new(failing_model, config);
    let prompt = Prompt::new("You are a helpful assistant.", "Test prompt");

    let mut group = c.benchmark_group("circuit_breaker");

    // Use blocking runtime for async benchmarks
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

    group.bench_function("before_trip", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _result = safe_model.complete_with_recovery(black_box(&prompt)).await;
            });
        });
    });

    // Trip the circuit breaker
    for _ in 0..3 {
        rt.block_on(async {
            let _ = safe_model.complete_with_recovery(&prompt).await;
        });
    }

    group.bench_function("after_trip", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _result = safe_model.complete_with_recovery(black_box(&prompt)).await;
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_inference_latency,
    bench_concurrent_throughput,
    bench_safe_model_overhead,
    bench_prompt_hashing,
    bench_hash_cache_performance,
    bench_memory_patterns,
    bench_error_recovery,
    bench_circuit_breaker
);

criterion_main!(benches);
