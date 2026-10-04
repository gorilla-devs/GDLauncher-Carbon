use crate::domain::server::LaunchConfig;
use crate::managers::orphan_pid;
use anyhow::Result;
use async_trait::async_trait;
use carbon_rt_path::ServerPath;
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, watch};

#[derive(Debug, Clone)]
pub struct ServerHandle {
    pub generation: uuid::Uuid,
    pub log_drained: Option<ExitSignal>,
    pub marker_root: Option<PathBuf>,
    pub runtime_activity: std::sync::Arc<
        std::sync::Mutex<Option<crate::managers::runtime_lifecycle::RuntimeActivity>>,
    >,
    pub process_id: u32,
    pub kill_tx: mpsc::Sender<()>,
    pub stdin_tx: mpsc::Sender<String>,
    /// Resolves once the server process has exited, gracefully or after a kill.
    pub exited: ExitSignal,
}

impl ServerHandle {
    pub fn finish_publication(&self) {
        if let Some(activity) = self.runtime_activity.lock().unwrap().take() {
            activity.finish();
        }
    }
}

/// How long [`drain`] waits for the exit of a server it killed to be
/// confirmed. That confirmation also waits for the server's output to close
/// (`local::POST_EXIT_OUTPUT_GRACE`), which must stay shorter than this.
pub(crate) const FORCED_EXIT_CONFIRM: std::time::Duration = std::time::Duration::from_secs(5);

/// Bounded channels and confirmed completion; disappearance is never success.
///
/// A run whose exit is already confirmed is drained: its supervisor closes
/// the stdin and kill channels once it has published the exit, which can be
/// before the exit watcher clears the handle. Closed channels are an error
/// only while the exit is still unconfirmed.
pub async fn drain(
    handle: &ServerHandle,
    force: bool,
    graceful: std::time::Duration,
) -> Result<()> {
    let enqueue = std::time::Duration::from_secs(1);
    let mut exited = handle.exited.clone();
    if exited.is_confirmed() {
        return Ok(());
    }
    if !force {
        let sent = tokio::time::timeout(enqueue, handle.stdin_tx.send("stop".into())).await;
        if matches!(sent, Ok(Ok(()))) {
            match tokio::time::timeout(graceful, exited.wait_confirmed()).await {
                Ok(result) => return result,
                Err(_) => tracing::warn!(
                    "Server graceful stop timed out; escalating captured generation {}",
                    handle.generation
                ),
            }
        } else if exited.is_confirmed() {
            return Ok(());
        }
    }
    match tokio::time::timeout(enqueue, handle.kill_tx.send(())).await {
        Ok(Ok(())) => {}
        _ if exited.is_confirmed() => return Ok(()),
        Err(_) => anyhow::bail!("Server kill enqueue timed out; exit Unknown"),
        Ok(Err(_)) => anyhow::bail!("Server kill channel closed; exit Unknown"),
    }
    tokio::time::timeout(FORCED_EXIT_CONFIRM, exited.wait_confirmed())
        .await
        .map_err(|_| {
            anyhow::anyhow!("Server forced exit unconfirmed; files and ownership retained")
        })??;
    Ok(())
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
    pub async fn wait_confirmed(&mut self) -> anyhow::Result<()> {
        self.0
            .wait_for(|exited| *exited)
            .await
            .map(|_| ())
            .map_err(|_| anyhow::anyhow!("Server supervisor disappeared without confirmed exit"))
    }

    /// Compatibility alias; sender loss remains an explicit error.
    pub async fn wait(&mut self) -> anyhow::Result<()> {
        self.wait_confirmed().await
    }

    /// Whether the exit has already been confirmed, without waiting.
    pub fn is_confirmed(&self) -> bool {
        *self.0.borrow()
    }
}

/// Creates an [`ExitSignal`] and the sender that resolves it. The sender
/// belongs to the supervisor. Dropping it without true wakes waiters with an
/// error; only a true value confirms the child's exit (a successful wait).
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
        runtime_activity: crate::managers::runtime_lifecycle::RuntimeActivity,
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
    async fn task04_runtime_exit_requires_true_not_sender_loss_or_timeout() {
        let (tx, mut signal) = exit_signal();
        drop(tx);
        assert!(signal.wait_confirmed().await.is_err());
        let (tx, mut signal) = exit_signal();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(5), signal.wait_confirmed())
                .await
                .is_err()
        );
        tx.send(true).unwrap();
        signal.wait_confirmed().await.unwrap();
        drop(tx);
        signal.wait_confirmed().await.unwrap();
    }

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

#[cfg(test)]
mod task07_tests {
    use super::*;
    #[tokio::test]
    async fn task07_stop_sender_loss_is_unknown_and_escalation_is_captured() {
        let (kill_tx, mut kill_rx) = mpsc::channel(1);
        let (stdin_tx, mut stdin_rx) = mpsc::channel(1);
        let (exit_tx, exited) = exit_signal();
        let handle = ServerHandle {
            generation: uuid::Uuid::new_v4(),
            runtime_activity: Default::default(),
            log_drained: None,
            marker_root: None,
            process_id: 42,
            kill_tx,
            stdin_tx,
            exited,
        };
        let result = async {
            assert_eq!(stdin_rx.recv().await.as_deref(), Some("stop"));
            kill_rx.recv().await.unwrap();
            exit_tx.send(true).unwrap();
        };
        let (drained, ()) = tokio::join!(
            drain(&handle, false, std::time::Duration::from_millis(1)),
            result
        );
        drained.unwrap();
        let (tx, signal) = exit_signal();
        drop(tx);
        let mut unknown = handle.clone();
        unknown.exited = signal;
        assert!(
            drain(&unknown, false, std::time::Duration::from_millis(1))
                .await
                .is_err()
        );
    }

    fn handle_with_closed_channels(exited: ExitSignal) -> ServerHandle {
        let (kill_tx, kill_rx) = mpsc::channel(1);
        let (stdin_tx, stdin_rx) = mpsc::channel(1);
        drop((kill_rx, stdin_rx));
        ServerHandle {
            generation: uuid::Uuid::new_v4(),
            runtime_activity: Default::default(),
            log_drained: None,
            marker_root: None,
            process_id: 42,
            kill_tx,
            stdin_tx,
            exited,
        }
    }

    #[tokio::test]
    async fn drain_after_a_published_exit_is_ok_even_with_closed_channels() {
        // The supervisor closes stdin and kill once it has published the
        // exit. A Stop, Kill or quit landing before the watcher clears the
        // handle must not report "kill channel closed; exit Unknown".
        let (exit_tx, exited) = exit_signal();
        exit_tx.send(true).unwrap();
        drop(exit_tx);
        let handle = handle_with_closed_channels(exited);
        for force in [false, true] {
            tokio::time::timeout(
                std::time::Duration::from_secs(1),
                drain(&handle, force, std::time::Duration::from_secs(30)),
            )
            .await
            .expect("an exit already confirmed needs no waiting")
            .unwrap();
        }

        // Closed channels without a confirmed exit stay Unknown.
        let (exit_tx, exited) = exit_signal();
        drop(exit_tx);
        let error = drain(
            &handle_with_closed_channels(exited),
            false,
            std::time::Duration::from_secs(30),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("exit Unknown"), "{error:#}");
    }
}
