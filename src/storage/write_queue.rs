use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot};

pub struct WriteJob {
    pub func: Box<dyn FnOnce(&Connection) -> anyhow::Result<serde_json::Value> + Send>,
    pub reply: oneshot::Sender<anyhow::Result<serde_json::Value>>,
}

pub struct WriteQueue {
    pub tx: mpsc::Sender<WriteJob>,
}

impl WriteQueue {
    pub fn start(db_path: String) -> Self {
        let (tx, mut rx) = mpsc::channel::<WriteJob>(128);

        tokio::spawn(async move {
            let conn = Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                    | rusqlite::OpenFlags::SQLITE_OPEN_FULL_MUTEX,
            )
            .unwrap();

            conn.pragma_update(None, "journal_mode", "WAL").unwrap();

            while let Some(job) = rx.recv().await {
                let result = (job.func)(&conn);
                let _ = job.reply.send(result);
            }
        });

        Self {
            tx,
        }
    }

    pub async fn execute<F>(&self, func: F) -> anyhow::Result<serde_json::Value>
    where
        F: FnOnce(&Connection) -> anyhow::Result<serde_json::Value> + Send + 'static,
    {
        let (reply_tx, reply_rx) = oneshot::channel();
        let job = WriteJob {
            func: Box::new(func),
            reply: reply_tx,
        };
        self.tx.send(job).await.map_err(|_| anyhow::anyhow!("Write queue channel closed"))?;
        reply_rx.await?
    }
}
