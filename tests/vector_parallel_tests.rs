use syncore::vector::{VectorStore, RealEmbeddings, SearchScope};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
// use tempfile::TempDir; // Not used in this test file

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_batch_insert_performance() {
        let mut store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

        // Create test data
        let test_data: Vec<_> = (0..100).map(|i| {
            (i as i64, Some(1), format!("Test document content {}", i))
        }).collect();

        let start = Instant::now();
        let result = store.insert_batch_parallel(test_data.clone());
        let duration = start.elapsed();

        assert!(result.is_ok(), "Batch insert should succeed");
        let inserted_ids = result.unwrap();
        assert_eq!(inserted_ids.len(), 100, "Should insert all 100 documents");

        // Verify all documents were inserted
        assert_eq!(store.len(), 100, "Store should contain 100 documents");

        // Performance check - should complete in reasonable time
        assert!(duration.as_millis() < 5000, "Batch insert should complete within 5 seconds");

        // Verify IDs are correct
        for (i, &id) in inserted_ids.iter().enumerate() {
            assert_eq!(id, i as i64, "ID should match expected value");
        }
    }

    #[test]
    fn test_parallel_search_accuracy() {
        let mut store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

        // Insert test documents
        let documents = vec![
            (1, Some(1), "Rust programming language".to_string()),
            (2, Some(1), "Python programming".to_string()),
            (3, Some(2), "JavaScript web development".to_string()),
            (4, Some(2), "Rust systems programming".to_string()),
            (5, None, "Go concurrency".to_string()),
        ];

        for (id, task_id, text) in documents.clone() {
            store.insert_text(id, task_id, &text, "test").unwrap();
        }

        // Test parallel search
        let results = store.search_parallel("Rust", 10, SearchScope::Global).unwrap();

        assert_eq!(results.len(), 2, "Should find 2 documents containing 'Rust'");

        // Results should be sorted by score (descending)
        let mut scores: Vec<f32> = results.iter().map(|r| r.score).collect();
        scores.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let actual_scores: Vec<f32> = results.iter().map(|r| r.score).collect();
        assert_eq!(scores, actual_scores, "Results should be sorted by score");

        // Verify correct document IDs
        let result_ids: Vec<i64> = results.iter().map(|r| r.id).collect();
        assert!(result_ids.contains(&1), "Should contain document ID 1");
        assert!(result_ids.contains(&4), "Should contain document ID 4");
    }

    #[test]
    fn test_parallel_search_task_scope() {
        let mut store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

        // Insert documents with different task IDs
        let documents = vec![
            (1, Some(1), "Rust programming task 1".to_string()),
            (2, Some(1), "Python programming task 1".to_string()),
            (3, Some(2), "JavaScript web task 2".to_string()),
            (4, Some(2), "Rust systems task 2".to_string()),
        ];

        for (id, task_id, text) in documents {
            store.insert_text(id, task_id, &text, "test").unwrap();
        }

        // Search within task 1
        let task1_results = store.search_parallel("Rust", 10, SearchScope::Task(1)).unwrap();
        assert_eq!(task1_results.len(), 1, "Should find 1 document in task 1");
        assert_eq!(task1_results[0].id, 1, "Should be document ID 1");
        assert_eq!(task1_results[0].task_id, Some(1), "Should belong to task 1");

        // Search within task 2
        let task2_results = store.search_parallel("Rust", 10, SearchScope::Task(2)).unwrap();
        assert_eq!(task2_results.len(), 1, "Should find 1 document in task 2");
        assert_eq!(task2_results[0].id, 4, "Should be document ID 4");
        assert_eq!(task2_results[0].task_id, Some(2), "Should belong to task 2");
    }

    #[test]
    fn test_parallel_vs_sequential_consistency() {
        let mut sequential_store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));
        let mut parallel_store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

        // Insert same documents into both stores
        let documents: Vec<_> = (0..50).map(|i| {
            (i as i64, Some(i % 5), format!("Document {} with unique content {}", i, i * 7))
        }).collect();

        // Sequential insert
        for (id, task_id, text) in documents.clone() {
            sequential_store.insert_text(id, task_id, &text, "test").unwrap();
        }

        // Parallel batch insert
        parallel_store.insert_batch_parallel(documents).unwrap();

        // Compare search results
        let query = "Document 23";
        let sequential_results = sequential_store.search(query, 10, SearchScope::Global).unwrap();
        let parallel_results = parallel_store.search_parallel(query, 10, SearchScope::Global).unwrap();

        assert_eq!(sequential_results.len(), parallel_results.len(),
                  "Both methods should return same number of results");

        if !sequential_results.is_empty() {
            assert_eq!(sequential_results[0].id, parallel_results[0].id,
                      "Top result ID should be the same");
        }
    }

    #[test]
    fn test_concurrent_vector_operations() {
        let store = Arc::new(Mutex::new(VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()))));
        let mut handles = vec![];

        // Spawn multiple threads performing concurrent operations
        for thread_id in 0..4 {
            let store_clone = Arc::clone(&store);
            let handle = thread::spawn(move || {
                for i in 0..25 {
                    let id = (thread_id * 25 + i) as i64;
                    let text = format!("Thread {} document {}", thread_id, i);

                    // Insert document
                    {
                        let mut store_guard = store_clone.lock().unwrap();
                        store_guard.insert_text(id, Some(thread_id as i64), &text, "concurrent_test").unwrap();
                    }

                    // Search
                    {
                        let store_guard = store_clone.lock().unwrap();
                        let results = store_guard.search_parallel("document", 5, SearchScope::Global).unwrap();
                        assert!(!results.is_empty(), "Search should return results");
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify final state
        let final_store = store.lock().unwrap();
        assert_eq!(final_store.len(), 100, "Should have 100 documents total");
    }

    #[test]
    fn test_parallel_search_empty_store() {
        let store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

        let results = store.search_parallel("any query", 10, SearchScope::Global).unwrap();
        assert!(results.is_empty(), "Empty store should return no results");
    }

    #[test]
    fn test_parallel_search_k_limiting() {
        let mut store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

        // Insert more documents than k limit
        for i in 0..20 {
            store.insert_text(i, Some(1), &format!("Document {}", i), "test").unwrap();
        }

        let results = store.search_parallel("Document", 5, SearchScope::Global).unwrap();
        assert_eq!(results.len(), 5, "Should return exactly k results");

        // Verify results are sorted
        for i in 1..results.len() {
            assert!(results[i-1].score >= results[i].score, "Results should be sorted by score");
        }
    }

    #[test]
    fn test_batch_insert_error_handling() {
        let mut store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

        // Test with empty batch
        let empty_batch: Vec<(i64, Option<i64>, String)> = vec![];
        let result = store.insert_batch_parallel(empty_batch);
        assert!(result.is_ok(), "Empty batch should succeed");
        assert!(result.unwrap().is_empty(), "Empty batch should return empty IDs");

        // Test with single document
        let single_doc = vec![(1, Some(1), "Single document".to_string())];
        let result = store.insert_batch_parallel(single_doc);
        assert!(result.is_ok(), "Single document batch should succeed");
        assert_eq!(result.unwrap().len(), 1, "Should return one ID");
    }

    #[test]
    fn test_parallel_search_performance_scaling() {
        let mut store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

        // Insert varying sizes of documents
        for size in [100, 500, 1000] {
            // Clear previous documents
            store = VectorStore::new(Box::new(RealEmbeddings::new(384).unwrap()));

            for i in 0..size {
                store.insert_text(i, None, &format!("Performance test document {}", i), "perf_test").unwrap();
            }

            let start = Instant::now();
            let results = store.search_parallel("performance", 10, SearchScope::Global).unwrap();
            let duration = start.elapsed();

            assert_eq!(results.len(), (size as usize).min(10), "Should return correct number of results");
            assert!(duration.as_millis() < 1000, "Search should complete within 1 second for {} documents", size);

            println!("Parallel search time for {} documents: {:?}", size, duration);
        }
    }
}
