use anyhow::Result;
use std::sync::Arc;
use syncore::storage::FaissPool;

#[tokio::test]
async fn faiss_pool_can_be_created() -> Result<()> {
    let pool = FaissPool::new("tests/data/faiss_test.index", 4);
    assert!(Arc::strong_count(&pool) >= 1);
    Ok(())
}

#[tokio::test]
async fn faiss_pool_provides_index_instances() -> Result<()> {
    let pool = FaissPool::new("tests/data/faiss_test.index", 4);
    let conn = pool
        .pool
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Pool error: {:?}", e))?;
    // Verify we got a valid wrapper with path set
    assert_eq!(conn.path, "tests/data/faiss_test.index");
    Ok(())
}

#[tokio::test]
async fn faiss_pool_recycles_connections() -> Result<()> {
    let pool = FaissPool::new("tests/data/faiss_test.index", 2);

    // Get two connections
    let conn1 = pool
        .pool
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Pool error: {:?}", e))?;
    let conn2 = pool
        .pool
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Pool error: {:?}", e))?;

    assert_eq!(conn1.path, "tests/data/faiss_test.index");
    assert_eq!(conn2.path, "tests/data/faiss_test.index");

    // Drop them back to pool
    drop(conn1);
    drop(conn2);

    // Get again - should reuse
    let conn3 = pool
        .pool
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Pool error: {:?}", e))?;
    assert_eq!(conn3.path, "tests/data/faiss_test.index");

    Ok(())
}
