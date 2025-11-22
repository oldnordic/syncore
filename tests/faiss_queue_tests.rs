use anyhow::Result;
use std::sync::Arc;
use syncore::storage::FaissQueue;

#[tokio::test]
async fn faiss_queue_executes_single_job() -> Result<()> {
    let queue = FaissQueue::new(16);

    let executed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = executed.clone();

    queue
        .submit(move || {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
        .await?;

    assert!(executed.load(std::sync::atomic::Ordering::SeqCst));
    Ok(())
}

#[tokio::test]
async fn faiss_queue_serializes_multiple_jobs() -> Result<()> {
    let queue = FaissQueue::new(16);

    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let c1 = counter.clone();
    let c2 = counter.clone();

    queue
        .submit(move || {
            let v = c1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            assert_eq!(v, 0);
            Ok(())
        })
        .await?;

    queue
        .submit(move || {
            let v = c2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            assert_eq!(v, 1);
            Ok(())
        })
        .await?;

    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
    Ok(())
}

#[tokio::test]
async fn faiss_queue_propagates_errors() {
    let queue = FaissQueue::new(16);

    let result = queue
        .submit(|| -> Result<()> { anyhow::bail!("boom") })
        .await;

    assert!(result.is_err());
}
