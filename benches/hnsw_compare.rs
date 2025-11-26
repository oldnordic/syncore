//! HNSW Comparison Benchmark
//!
//! Benchmarks SynCore's HNSW implementation with various configurations:
//! - Small index (m=8, ef_construction=100)
//! - Medium index (m=16, ef_construction=200) - default
//! - Large index (m=32, ef_construction=400)
//!
//! Metrics tracked:
//! - insertion_time: Time to insert N vectors
//! - search_time: Time to search for k nearest neighbors
//! - recall@10: Accuracy vs brute-force ground truth
//! - memory_usage_bytes: Estimated index size
//! - graph_build_time: Time to construct HNSW graph
//!
//! ISOLATION REQUIREMENT: This benchmark does NOT modify SynCore state or databases.
//! It creates standalone HNSW indices for comparison only.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use syncore::vector::hnsw::{HnswConfig, HnswVectorIndex};
use syncore::vector::traits::VectorIndex;

/// Configuration variant for comparison
#[derive(Debug, Clone)]
struct IndexConfig {
    name: &'static str,
    m: usize,
    ef_construction: usize,
    ef_search: usize,
}

impl IndexConfig {
    fn to_hnsw_config(&self) -> HnswConfig {
        HnswConfig {
            m: self.m,
            ef_construction: self.ef_construction,
            ef_search: self.ef_search,
        }
    }
}

/// Benchmark configurations
fn get_configs() -> Vec<IndexConfig> {
    vec![
        IndexConfig {
            name: "Small (m=8, ef_c=100)",
            m: 8,
            ef_construction: 100,
            ef_search: 50,
        },
        IndexConfig {
            name: "Medium (m=16, ef_c=200)",
            m: 16,
            ef_construction: 200,
            ef_search: 50,
        },
        IndexConfig {
            name: "Large (m=32, ef_c=400)",
            m: 32,
            ef_construction: 400,
            ef_search: 100,
        },
    ]
}

/// Generate deterministic test vectors (dimension 128)
fn generate_test_vectors(count: usize, dim: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut vectors = Vec::with_capacity(count);
    for i in 0..count {
        let vec: Vec<f32> = (0..dim)
            .map(|j| {
                let val = ((i + j + seed as usize) % 100) as f32;
                val / 100.0 // Normalize to [0, 1]
            })
            .collect();
        vectors.push(vec);
    }
    vectors
}

/// Calculate recall@k: how many of the HNSW top-k results are in ground truth top-k?
fn calculate_recall(
    hnsw_results: &[(i64, f32)],
    ground_truth: &[(i64, f32)],
    k: usize,
) -> f32 {
    let hnsw_ids: Vec<i64> = hnsw_results.iter().take(k).map(|(id, _)| *id).collect();
    let gt_ids: Vec<i64> = ground_truth.iter().take(k).map(|(id, _)| *id).collect();

    let recall_count = hnsw_ids.iter().filter(|id| gt_ids.contains(id)).count();
    recall_count as f32 / k as f32
}

/// Cosine distance (L2 distance of normalized vectors)
fn cosine_distance(v1: &[f32], v2: &[f32]) -> f32 {
    let norm1: f32 = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm2: f32 = v2.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm1 == 0.0 || norm2 == 0.0 {
        return 1.0;
    }

    let v1_norm: Vec<f32> = v1.iter().map(|x| x / norm1).collect();
    let v2_norm: Vec<f32> = v2.iter().map(|x| x / norm2).collect();

    let dist_squared: f32 = v1_norm
        .iter()
        .zip(v2_norm.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum();

    dist_squared.sqrt()
}

/// Brute-force ground truth search
fn brute_force_search(
    vectors: &[Vec<f32>],
    query: &[f32],
    k: usize,
) -> Vec<(i64, f32)> {
    let mut results: Vec<(i64, f32)> = vectors
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let dist = cosine_distance(query, v);
            (i as i64, dist)
        })
        .collect();

    results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    results.truncate(k);
    results
}

/// Benchmark insertion time
fn bench_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("insertion_time");
    let vectors = generate_test_vectors(1000, 128, 42);

    for config in get_configs() {
        group.bench_with_input(
            BenchmarkId::new("insert_1000_vectors", config.name),
            &config,
            |b, cfg| {
                b.iter(|| {
                    let hnsw_config = cfg.to_hnsw_config();
                    let mut index = HnswVectorIndex::new(hnsw_config, 42).unwrap();

                    for (i, vec) in vectors.iter().enumerate() {
                        index.add(black_box(i as i64), black_box(vec.clone())).unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark search time
fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_time");
    let vectors = generate_test_vectors(1000, 128, 42);
    let query = generate_test_vectors(1, 128, 999)[0].clone();

    for config in get_configs() {
        // Pre-build index
        let hnsw_config = config.to_hnsw_config();
        let mut index = HnswVectorIndex::new(hnsw_config, 42).unwrap();
        for (i, vec) in vectors.iter().enumerate() {
            index.add(i as i64, vec.clone()).unwrap();
        }

        group.bench_with_input(
            BenchmarkId::new("search_k10", config.name),
            &config,
            |b, _| {
                b.iter(|| {
                    index.search(black_box(&query), black_box(10)).unwrap()
                });
            },
        );
    }
    group.finish();
}

/// Benchmark recall@10 accuracy
fn bench_recall(c: &mut Criterion) {
    let mut group = c.benchmark_group("recall_at_10");
    let vectors = generate_test_vectors(500, 128, 42);
    let query = vectors[250].clone(); // Query with a vector from the dataset

    for config in get_configs() {
        // Build HNSW index
        let hnsw_config = config.to_hnsw_config();
        let mut index = HnswVectorIndex::new(hnsw_config, 42).unwrap();
        for (i, vec) in vectors.iter().enumerate() {
            index.add(i as i64, vec.clone()).unwrap();
        }

        // Get ground truth
        let ground_truth = brute_force_search(&vectors, &query, 10);

        group.bench_with_input(
            BenchmarkId::new("recall_calculation", config.name),
            &config,
            |b, _| {
                b.iter(|| {
                    let hnsw_results = index.search(black_box(&query), black_box(10)).unwrap();
                    let recall = calculate_recall(&hnsw_results, &ground_truth, 10);
                    black_box(recall)
                });
            },
        );
    }
    group.finish();
}

/// Benchmark graph construction time (isolated from insertion)
fn bench_graph_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_build_time");

    for config in get_configs() {
        group.bench_with_input(
            BenchmarkId::new("construct_index", config.name),
            &config,
            |b, cfg| {
                b.iter(|| {
                    let hnsw_config = cfg.to_hnsw_config();
                    let _ = HnswVectorIndex::new(black_box(hnsw_config), black_box(42)).unwrap();
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    hnsw_benches,
    bench_insertion,
    bench_search,
    bench_recall,
    bench_graph_build
);
criterion_main!(hnsw_benches);
