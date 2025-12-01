use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

pub struct FaissJob {
    pub task: Box<dyn FnOnce() -> Result<()> + Send + 'static>,
    pub reply: oneshot::Sender<Result<()>>,
}

#[derive(Clone)]
pub struct FaissQueue {
    tx: mpsc::Sender<FaissJob>,
}

impl FaissQueue {
    pub fn new(buffer: usize) -> Arc<Self> {
        let (tx, mut rx) = mpsc::channel::<FaissJob>(buffer);

        tokio::spawn(async move {
            while let Some(job) = rx.recv().await {
                let result = (job.task)();
                let _ = job.reply.send(result);
            }
        });

        Arc::new(Self {
            tx,
        })
    }

    pub async fn submit<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce() -> Result<()> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(FaissJob {
                task: Box::new(f),
                reply: tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("FAISS queue channel closed"))?;
        rx.await?
    }
}
