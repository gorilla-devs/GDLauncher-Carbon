use crate::domain::server::LaunchConfig;
use crate::managers::orphan_pid;
use anyhow::Result;
use async_trait::async_trait;
use carbon_rt_path::ServerPath;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Notify, mpsc};

#[derive(Debug)]
pub struct ServerHandle {
    pub process_id: u32,
    pub kill_tx: mpsc::Sender<()>,
    pub stdin_tx: mpsc::Sender<String>,
    /// Notified when the server process exits (gracefully or after kill).
    pub exit_notify: Arc<Notify>,
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
        launch_config: &LaunchConfig,
        modloader_type: Option<&str>,
        log_tx: mpsc::UnboundedSender<String>,
    ) -> Result<ServerHandle>;

    async fn stop(&self, handle: &ServerHandle) -> Result<()>;

    async fn kill(&self, handle: &ServerHandle) -> Result<()>;

    async fn send_command(&self, handle: &ServerHandle, command: &str) -> Result<()>;
}

/// Filename of the on-disk marker recording a server's current JVM pid.
/// Lives directly under the server's root directory (`ServerPath::get_root`,
/// the parent of the `server/` data dir) rather than inside the data dir, so
/// it sits outside anything the modloader installer or world save touches.
pub const PID_FILE_NAME: &str = ".gdl_server.pid";

/// Path to a server's pidfile given its root directory. Both the writer
/// (`local::start`, right after spawning the JVM) and the reader
/// (`ServerManager::load_servers`, on every core startup) compute this from
/// the same `ServerPath::get_root()`, so they always agree on the location.
pub fn pid_file_path(server_root: &Path) -> PathBuf {
    orphan_pid::pid_file_path(server_root, PID_FILE_NAME)
}

/// Record `pid` as this server's current JVM. Best-effort: a write failure
/// is only logged. Losing the pidfile just means an unclean exit during this
/// run won't be auto-cleaned up on the next launch — it must never block or
/// fail the launch itself.
pub async fn write_pid_file(server_root: &Path, pid: u32) {
    orphan_pid::write_pid_file(server_root, PID_FILE_NAME, pid).await;
}

/// Remove a server's pidfile. Best-effort: a missing file is not an error,
/// and any other failure is only logged.
pub async fn remove_pid_file(server_root: &Path) {
    orphan_pid::remove_pid_file(server_root, PID_FILE_NAME).await;
}

/// Read and parse a server's pidfile. `Ok(None)` means no pidfile exists —
/// there is nothing to reconcile for this server. Any other I/O or parse
/// failure comes back as `Err` so the caller can log it and fall back to
/// treating it exactly like "no pidfile"; this must never fail startup.
pub async fn read_pid_file(server_root: &Path) -> Result<Option<u32>> {
    orphan_pid::read_pid_file(server_root, PID_FILE_NAME).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pid_file_write_read_remove_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Nothing written yet.
        assert_eq!(read_pid_file(root).await.unwrap(), None);

        write_pid_file(root, 4242).await;
        assert_eq!(read_pid_file(root).await.unwrap(), Some(4242));
        assert!(pid_file_path(root).exists());

        remove_pid_file(root).await;
        assert_eq!(read_pid_file(root).await.unwrap(), None);
        assert!(!pid_file_path(root).exists());

        // Removing an already-gone pidfile is not an error (best-effort).
        remove_pid_file(root).await;
    }

    #[tokio::test]
    async fn write_pid_file_overwrites_a_stale_value() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        write_pid_file(root, 111).await;
        write_pid_file(root, 222).await;

        assert_eq!(read_pid_file(root).await.unwrap(), Some(222));
    }

    #[tokio::test]
    async fn read_pid_file_errors_on_garbage_instead_of_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        tokio::fs::write(pid_file_path(root), b"not-a-pid")
            .await
            .unwrap();

        assert!(read_pid_file(root).await.is_err());
    }

    #[tokio::test]
    async fn write_pid_file_failure_does_not_panic() {
        // Point at a root whose parent doesn't exist, so the write fails —
        // this must be swallowed (logged) rather than panicking, since
        // `write_pid_file` is best-effort and must never block a launch.
        let bogus_root = std::path::Path::new("/nonexistent/gdl-test-pidfile-root");
        write_pid_file(bogus_root, 123).await;
    }
}
