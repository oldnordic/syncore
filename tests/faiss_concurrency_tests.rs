use anyhow::Result;
use std::sync::Arc;
use syncore::router::SynCoreState;
use tokio::task;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn faiss_concurrent_reads_and_writes_do_not_deadlock() -> Result<()> {
    let state = Arc::new(SynCoreState::faiss_only("tests/data/faiss_test.index"));

    let mut tasks = Vec::new();

    // Spawn 4 write tasks
    for i in 0..4 {
        let st = state.clone();
        tasks.push(task::spawn(async move {
            st.faiss_queue
                .as_ref()
                .unwrap()
                .submit(move || {
                    // Simulate write work
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    eprintln!("Write task {} completed", i);
                    Ok(())
                })
                .await
        }));
    }

    // Spawn 8 read tasks
    for i in 0..8 {
        let st = state.clone();
        tasks.push(task::spawn(async move {
            let pool = st.faiss_pool.as_ref().unwrap().pool.clone();
            let conn = pool.get().await.map_err(|e| anyhow::anyhow!("Pool error: {:?}", e))?;
            let path = conn.path.clone();
            // Simulate read work using wrapper in spawn_blocking
            let res = tokio::task::spawn_blocking(move || {
                eprintln!("Read task {} using path: {}", i, path);
                std::thread::sleep(std::time::Duration::from_millis(5));
                Ok::<(), anyhow::Error>(())
            })
            .await;
            drop(conn);
            res.unwrap()
        }));
    }

    // Wait for all tasks to complete
    for t in tasks {
        t.await??;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn faiss_queue_handles_burst_writes() -> Result<()> {
    let state = Arc::new(SynCoreState::faiss_only("tests/data/faiss_test.index"));

    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut tasks = Vec::new();

    // Burst of 16 writes
    for _ in 0..16 {
        let st = state.clone();
        let cnt = counter.clone();
        tasks.push(task::spawn(async move {
            st.faiss_queue
                .as_ref()
                .unwrap()
                .submit(move || {
                    cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(())
                })
                .await
        }));
    }

    for t in tasks {
        t.await??;
    }

    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 16);
    Ok(())
}

#[tokio::test]
async fn faiss_pool_connections_are_reusable() -> Result<()> {
    let state = Arc::new(SynCoreState::faiss_only("tests/data/faiss_test.index"));

    let pool = state.faiss_pool.as_ref().unwrap().pool.clone();

    // Get and return connections multiple times
    for _ in 0..10 {
        let conn = pool.get().await.map_err(|e| anyhow::anyhow!("Pool error: {:?}", e))?;
        assert_eq!(conn.path, "tests/data/faiss_test.index");
        drop(conn);
    }

    Ok(())
}
