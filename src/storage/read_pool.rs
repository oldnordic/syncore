use rusqlite::{Connection, OpenFlags};
use deadpool::managed::{self, Metrics};

pub struct SqliteManager {
    pub path: String,
}

impl managed::Manager for SqliteManager {
    type Type = Connection;
    type Error = rusqlite::Error;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            Connection::open_with_flags(
                &path,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
            )
        })
        .await
        .expect("spawn_blocking failed")
    }

    async fn recycle(
        &self,
        _conn: &mut Self::Type,
        _metrics: &Metrics,
    ) -> managed::RecycleResult<Self::Error> {
        Ok(())
    }
}

pub type ReadPool = deadpool::managed::Pool<SqliteManager>;

pub fn create_read_pool(path: String, size: usize) -> ReadPool {
    let mgr = SqliteManager { path };
    ReadPool::builder(mgr)
        .max_size(size)
        .build()
        .unwrap()
}
