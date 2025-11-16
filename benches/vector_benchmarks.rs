use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use syncore::vector::{VectorStore, RealEmbeddings, SearchScope};
use std::time::Duration;

fn bench_vector_search_sequential(c: &mut Criterion) {
    let mut store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

    // Setup test data
    for i in 0..1000 {
        store.insert_text(i, None, &format!("Test document {}", i), "benchmark").unwrap();
    }

    c.bench_function("search_sequential_1000_docs", |b| {
        b.iter(|| {
            store.search(black_box("test"), 10, SearchScope::Global).unwrap()
        })
    });
}

fn bench_vector_search_parallel(c: &mut Criterion) {
    let mut store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

    // Setup test data
    for i in 0..1000 {
        store.insert_text(i, None, &format!("Test document {}", i), "benchmark").unwrap();
    }

    c.bench_function("search_parallel_1000_docs", |b| {
        b.iter(|| {
            store.search_parallel(black_box("test"), 10, SearchScope::Global).unwrap()
        })
    });
}

fn bench_batch_insert_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_insert_sequential");

    for size in [10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::new("sequential", size), size, |b, &size| {
            b.iter(|| {
                let mut store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));
                for i in 0..size {
                    store.insert_text(i, None, &format!("Batch document {}", i), "benchmark").unwrap();
                }
            });
        });
    }
    group.finish();
}

fn bench_batch_insert_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_insert_parallel");
    group.measurement_time(Duration::from_secs(10));

    for size in [10, 50, 100, 500].iter() {
        let documents: Vec<_> = (0..*size).map(|i| {
            (i, None, format!("Batch document {}", i))
        }).collect();

        group.bench_with_input(BenchmarkId::new("parallel", size), &documents, |b, docs| {
            b.iter(|| {
                let mut store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));
                store.insert_batch_parallel(black_box(docs.clone())).unwrap()
            })
        });
    }
    group.finish();
}

fn bench_search_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_comparison");

    for size in [100, 500, 1000, 2000].iter() {
        // Setup stores for both sequential and parallel
        let mut sequential_store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));
        let mut parallel_store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

        for i in 0..*size {
            let text = format!("Comparison test document {} with unique content", i);
            sequential_store.insert_text(i, None, &text, "comparison").unwrap();
            parallel_store.insert_text(i, None, &text, "comparison").unwrap();
        }

        group.bench_with_input(BenchmarkId::new("sequential", size), size, |b, _| {
            b.iter(|| {
                sequential_store.search(black_box("comparison"), 10, SearchScope::Global).unwrap()
            })
        });

        group.bench_with_input(BenchmarkId::new("parallel", size), size, |b, _| {
            b.iter(|| {
                parallel_store.search_parallel(black_box("comparison"), 10, SearchScope::Global).unwrap()
            })
        });
    }
    group.finish();
}

fn bench_concurrent_operations(c: &mut Criterion) {
    let mut store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

    // Setup initial data
    for i in 0..500 {
        store.insert_text(i, None, &format!("Concurrent test document {}", i), "concurrent").unwrap();
    }

    c.bench_function("concurrent_search", |b| {
        b.iter(|| {
            // Simulate concurrent searches
            let queries = ["test", "concurrent", "document", "data"];
            for query in &queries {
                let _ = store.search_parallel(black_box(query), 5, SearchScope::Global);
            }
        })
    });
}

criterion_group!(
    benches,
    bench_vector_search_sequential,
    bench_vector_search_parallel,
    bench_batch_insert_sequential,
    bench_batch_insert_parallel,
    bench_search_comparison,
    bench_concurrent_operations
);
criterion_main!(benches);
