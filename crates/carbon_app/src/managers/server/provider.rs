use anyhow::Result;
use async_trait::async_trait;
use carbon_rt_path::ServerPath;
use std::path::Path;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct ServerHandle {
    pub process_id: u32,
    pub kill_tx: mpsc::Sender<()>,
    pub stdin_tx: mpsc::Sender<String>,
}

#[async_trait]
pub trait ServerProvider: Send + Sync {
    async fn start(
        &self,
        java_path: &Path,
        server_path: &ServerPath,
        xmx: i32,
        xms: i32,
        extra_args: &str,
        log_tx: mpsc::UnboundedSender<String>,
    ) -> Result<ServerHandle>;

    async fn stop(&self, handle: &ServerHandle) -> Result<()>;

    async fn kill(&self, handle: &ServerHandle) -> Result<()>;

    async fn send_command(&self, handle: &ServerHandle, command: &str) -> Result<()>;
}
