//! Performance regression tests for DashMap and parking_lot optimizations
//!
//! These tests ensure that the performance optimizations maintain correctness
//! while improving concurrent access patterns.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use syncore::ingestion::{
    IngestionEventKind, IngestionJob, IngestionKind, IngestionPriority, IngestionQueue,
    IngestionSource,
};
use syncore::message_bus::{
    message::{AgentId, Msg, MsgKind},
    MessageBus,
};
use tokio::task::JoinSet;

#[tokio::test]
async fn test_ingestion_queue_concurrent_dedup() -> Result<()> {
    let (queue, _main_rx, _low_rx) = IngestionQueue::new(1000, 1000);

    let mut tasks = JoinSet::new();

    // Spawn 100 concurrent tasks submitting the same job
    for i in 0..100 {
        let queue_clone = queue.clone();
        tasks.spawn(async move {
            let job = IngestionJob::new(
                PathBuf::from("/test/path.rs"),
                IngestionKind::CodeFile,
                IngestionEventKind::Modified,
                IngestionPriority::Normal,
                IngestionSource::Cli,
            );

            queue_clone.submit_job(job).await
        });
    }

    // Wait for all tasks to complete
    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        results.push(result);
    }

    // All should succeed
    assert_eq!(results.len(), 100);
    for result in results {
        assert!(result.is_ok());
    }

    // Check stats - should have deduped most jobs
    let stats = queue.get_stats();
    assert_eq!(stats.jobs_created, 100);
    assert!(stats.jobs_deduped > 90); // Most should be deduped

    Ok(())
}

#[tokio::test]
async fn test_message_bus_concurrent_agent_registration() -> Result<()> {
    let bus = Arc::new(MessageBus::new());
    let mut tasks = JoinSet::new();

    // Spawn 50 concurrent agent registrations
    for i in 0..50 {
        let bus_clone = bus.clone();
        tasks.spawn(async move {
            let agent_id = AgentId::Internal(format!("agent_{}", i));
            let agent_id_clone = agent_id.clone();
            let _rx = bus_clone.register_agent(agent_id);

            // Register agent info
            bus_clone.register_agent_info(
                agent_id_clone,
                format!("Agent {}", i),
                vec!["test_capability".to_string()],
            );
        });
    }

    // Wait for all registrations
    while let Some(_) = tasks.join_next().await {}

    // Verify all agents are registered
    let agents = bus.list_agents();
    assert_eq!(agents.len(), 50);

    // Verify capability index
    let capable_agents = bus.agents_with_capability("test_capability");
    assert_eq!(capable_agents.len(), 50);

    Ok(())
}

#[tokio::test]
async fn test_message_bus_concurrent_messaging() -> Result<()> {
    let bus = Arc::new(MessageBus::new());

    // Register agents
    let agent1_rx = bus.register_agent(AgentId::Internal("agent1".to_string()));
    let agent2_rx = bus.register_agent(AgentId::Internal("agent2".to_string()));

    let mut tasks = JoinSet::new();

    // Spawn 100 concurrent message sends
    for i in 0..100 {
        let bus_clone = bus.clone();
        tasks.spawn(async move {
            let msg = Msg {
                id: i,
                from: AgentId::Internal("sender".to_string()),
                to: Some(AgentId::Internal(
                    if i % 2 == 0 {
                        "agent1"
                    } else {
                        "agent2"
                    }
                    .to_string(),
                )),
                kind: MsgKind::Direct,
                payload: serde_json::json!({"message": i}),
                timestamp: std::time::SystemTime::now(),
            };

            bus_clone.send(msg);
        });
    }

    // Wait for all sends
    while let Some(_) = tasks.join_next().await {}

    // Verify message history
    let history = bus.message_history();
    assert_eq!(history.len(), 100);

    Ok(())
}

#[tokio::test]
async fn test_ingestion_queue_cleanup_performance() -> Result<()> {
    let (queue, _main_rx, _low_rx) = IngestionQueue::new(1000, 1000);

    // Submit many jobs with different timestamps
    let start = Instant::now();
    for i in 0..1000 {
        let job = IngestionJob::new(
            PathBuf::from(format!("/test/path_{}.rs", i)),
            IngestionKind::CodeFile,
            IngestionEventKind::Modified,
            IngestionPriority::Normal,
            IngestionSource::Cli,
        );

        queue.submit_job(job).await?;
    }

    let submit_time = start.elapsed();

    // Test cleanup performance
    let cleanup_start = Instant::now();
    queue.cleanup_dedup(10); // Clean up entries older than 10 seconds
    let cleanup_time = cleanup_start.elapsed();

    // Cleanup should be fast (< 10ms)
    assert!(cleanup_time < Duration::from_millis(10));

    // Submit should also be reasonable (< 100ms for 1000 jobs)
    assert!(submit_time < Duration::from_millis(100));

    Ok(())
}

#[test]
fn test_message_bus_id_generation_thread_safety() {
    let bus = Arc::new(MessageBus::new());
    let mut handles = Vec::new();

    // Spawn 10 threads each generating 100 IDs
    for _ in 0..10 {
        let bus_clone = bus.clone();
        let handle = std::thread::spawn(move || {
            let mut ids = Vec::new();
            for _ in 0..100 {
                ids.push(bus_clone.next_message_id());
            }
            ids
        });
        handles.push(handle);
    }

    // Collect all IDs
    let mut all_ids = Vec::new();
    for handle in handles {
        all_ids.extend(handle.join().unwrap());
    }

    // Verify uniqueness and ordering
    assert_eq!(all_ids.len(), 1000);

    let mut sorted_ids = all_ids.clone();
    sorted_ids.sort();

    // Should be exactly 1..1000
    for (i, &id) in sorted_ids.iter().enumerate() {
        assert_eq!(id, i as u64 + 1);
    }

    // Verify no duplicates
    let mut unique_ids = all_ids.clone();
    unique_ids.sort();
    unique_ids.dedup();
    assert_eq!(unique_ids.len(), 1000);
}
