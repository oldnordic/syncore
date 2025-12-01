use crate::protocol::{SynCoreMsg, SynCoreTool};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

pub struct Client {
    stream: UnixStream,
}

impl Client {
    pub async fn connect() -> anyhow::Result<Self> {
        let stream = UnixStream::connect("/tmp/syncore.sock").await?;
        Ok(Self {
            stream,
        })
    }

    pub async fn store(&mut self, key: &str, value: &str) -> anyhow::Result<String> {
        let args = rmp_serde::to_vec(&(key.to_string(), value.to_string()))?;
        let msg = SynCoreMsg {
            tool: SynCoreTool::MemoryStore,
            args,
        };

        let msg_bytes = rmp_serde::to_vec(&msg)?;
        self.stream.write_all(&msg_bytes).await?;

        let mut response = Vec::new();
        self.stream.read_to_end(&mut response).await?;

        let result: String = rmp_serde::from_slice(&response)?;
        Ok(result)
    }

    pub async fn query(&mut self, key: &str) -> anyhow::Result<Option<String>> {
        let args = rmp_serde::to_vec(&key.to_string())?;
        let msg = SynCoreMsg {
            tool: SynCoreTool::MemoryQuery,
            args,
        };

        let msg_bytes = rmp_serde::to_vec(&msg)?;
        self.stream.write_all(&msg_bytes).await?;

        let mut response = Vec::new();
        self.stream.read_to_end(&mut response).await?;

        let result: Option<String> = rmp_serde::from_slice(&response)?;
        Ok(result)
    }
}
