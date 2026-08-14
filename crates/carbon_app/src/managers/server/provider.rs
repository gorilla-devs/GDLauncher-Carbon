use crate::domain::server::LaunchConfig;
use crate::managers::orphan_pid;
use anyhow::Result;
use async_trait::async_trait;
use carbon_rt_path::ServerPath;
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, watch};

#[derive(Debug)]
pub struct ServerHandle {
    pub process_id: u32,
    pub kill_tx: mpsc::Sender<()>,
    pub stdin_tx: mpsc::Sender<String>,
    /// Resolves once the server process has exited, gracefully or after a kill.
    pub exited: ExitSignal,
}

/// "The server process has exited", readable by any number of waiters.
///
/// Level-triggered, which is the whole point: waiters attach at unpredictable
/// times — the crash watcher spawns behind a servers-map write, a database
/// write and a full process-table scan, and a stop can be requested at any
/// moment — and a JVM that dies on a bad `-X` flag can be gone before any of
/// them. An edge-triggered wake delivered into that gap reaches nobody and is
/// never redelivered, leaving the server displayed as Running forever and
/// every later stop waiting out its full timeout.
#[derive(Debug, Clone)]
pub struct ExitSignal(watch::Receiver<bool>);

impl ExitSignal {
    /// Resolves once the process has exited, immediately if it already had.
    pub async fn wait(&mut self) {
        // `wait_for` inspects the current value before it waits, so an exit
        // that landed first still resolves this. Its only error is every
        // sender having been dropped, which happens when the supervising task
        // finishes — after it published the exit — so that means the same
        // thing.
        let _ = self.0.wait_for(|exited| *exited).await;
    }
}

/// Creates an [`ExitSignal`] and the sender that resolves it. The sender
/// belongs to whatever supervises the process; dropping it resolves every
/// waiter, so it must outlive the process it reports on.
pub fn exit_signal() -> (watch::Sender<bool>, ExitSignal) {
    let (tx, rx) = watch::channel(false);
    (tx, ExitSignal(rx))
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

/// Record `pid` and its `start_time` (seconds since epoch, from sysinfo
/// right after spawning it) as this server's current JVM. Best-effort: a
/// write failure is only logged. Losing the pidfile just means an unclean
/// exit during this run won't be auto-cleaned up on the next launch — it
/// must never block or fail the launch itself.
pub async fn write_pid_file(server_root: &Path, pid: u32, start_time: u64) {
    orphan_pid::write_pid_file(server_root, PID_FILE_NAME, pid, start_time).await;
}

/// Remove a server's pidfile. Best-effort: a missing file is not an error,
/// and any other failure is only logged.
pub async fn remove_pid_file(server_root: &Path) {
    orphan_pid::remove_pid_file(server_root, PID_FILE_NAME).await;
}

/// Read and parse a server's pidfile. `Ok(None)` means no pidfile exists —
/// there is nothing to reconcile for this server. Any other I/O or parse
/// failure comes back as `Err` so the caller can log it and fall back to
/// treating it exactly like "no pidfile"; this must never fail startup. The
/// inner `Option<u64>` is the recorded start time, `None` for a legacy
/// pid-only pidfile — see `orphan_pid::read_pid_file`.
pub async fn read_pid_file(server_root: &Path) -> Result<Option<(u32, Option<u64>)>> {
    orphan_pid::read_pid_file(server_root, PID_FILE_NAME).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn exit_signalled_before_a_waiter_attaches_is_still_observed() {
        let (exited_tx, mut exited) = exit_signal();

        // The JVM dies before anything waits on it. That window is real: the
        // crash watcher only attaches after a servers-map write, a database
        // write and a process-table scan, and a bad `-X` flag kills the JVM in
        // about the time the scan alone takes.
        exited_tx.send(true).unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(1), exited.wait())
            .await
            .expect("a waiter attaching after the exit must still observe it");
    }

    #[tokio::test]
    async fn every_waiter_is_released_by_one_exit() {
        // The crash watcher and an in-flight stop both wait on the same signal.
        let (exited_tx, exited) = exit_signal();
        let mut watcher = exited.clone();
        let mut stopper = exited;

        exited_tx.send(true).unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            watcher.wait().await;
            stopper.wait().await;
        })
        .await
        .expect("one exit must release every waiter, not just the first");
    }

    #[tokio::test]
    async fn a_running_process_does_not_resolve_the_signal() {
        let (_exited_tx, mut exited) = exit_signal();

        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(300), exited.wait())
                .await
                .is_err(),
            "the signal must stay pending while the process is alive"
        );
    }

    #[tokio::test]
    async fn pid_file_write_read_remove_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Nothing written yet.
        assert_eq!(read_pid_file(root).await.unwrap(), None);

        write_pid_file(root, 4242, 1_700_000_000).await;
        assert_eq!(
            read_pid_file(root).await.unwrap(),
            Some((4242, Some(1_700_000_000)))
        );
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

        write_pid_file(root, 111, 1000).await;
        write_pid_file(root, 222, 2000).await;

        assert_eq!(read_pid_file(root).await.unwrap(), Some((222, Some(2000))));
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
        write_pid_file(bogus_root, 123, 1000).await;
    }
}
