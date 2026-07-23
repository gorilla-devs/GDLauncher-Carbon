//! Shared on-disk pidfile lifecycle and orphan-JVM reconciliation logic.
//!
//! Both the server manager and the instance (game) manager record the pid of
//! the java process they spawn in a small marker file next to that
//! server/instance's own data, so that if this core exits without shutting
//! the process down cleanly (crash, force-quit, Windows `TerminateProcess`)
//! the next startup can detect and kill the orphaned JVM instead of leaving
//! it running forever. The read/write/remove operations and the "is this
//! pid still a live java process" / "what should happen to this recorded
//! pid" decisions are security-relevant — a wrong call either leaks a
//! running JVM forever or kills an unrelated process that reused the pid —
//! and are identical between the two callers, so they live here once,
//! parameterized by the pidfile name each caller uses (`.gdl_server.pid` /
//! `.gdl_instance.pid`).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use sysinfo::{Pid, System};
use tracing::warn;

/// Path to a pidfile given its root directory and file name. Both the
/// writer (right after spawning the java process) and the reader (on core
/// startup) must compute this the same way so they always agree on the
/// location — see each caller's own pidfile-name constant for why the root
/// it's joined to is chosen the way it is.
pub fn pid_file_path(root: &Path, file_name: &str) -> PathBuf {
    root.join(file_name)
}

/// Record `pid` as the current java process for whatever lives at `root`.
/// Best-effort: a write failure is only logged. Losing the pidfile just
/// means an unclean exit during this run won't be auto-cleaned up on the
/// next launch — it must never block or fail the launch itself.
pub async fn write_pid_file(root: &Path, file_name: &str, pid: u32) {
    let path = pid_file_path(root, file_name);
    if let Err(e) = tokio::fs::write(&path, pid.to_string()).await {
        warn!("Failed to write pidfile at {}: {}", path.display(), e);
    }
}

/// Remove a pidfile. Best-effort: a missing file is not an error, and any
/// other failure is only logged.
pub async fn remove_pid_file(root: &Path, file_name: &str) {
    let path = pid_file_path(root, file_name);
    match tokio::fs::remove_file(&path).await {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warn!("Failed to remove pidfile at {}: {}", path.display(), e),
    }
}

/// Read and parse a pidfile. `Ok(None)` means no pidfile exists — there is
/// nothing to reconcile for whatever lives at `root`. Any other I/O or parse
/// failure comes back as `Err` so the caller can log it and fall back to
/// treating it exactly like "no pidfile"; this must never fail startup.
pub async fn read_pid_file(root: &Path, file_name: &str) -> Result<Option<u32>> {
    let path = pid_file_path(root, file_name);
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => {
            let pid = content
                .trim()
                .parse::<u32>()
                .with_context(|| format!("invalid pid recorded in {}", path.display()))?;
            Ok(Some(pid))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("failed to read pidfile at {}", path.display())),
    }
}

/// Outcome of reconciling one recorded pid against the live process table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidReconcileAction {
    /// No pidfile was present — nothing to do.
    NoPidFile,
    /// The recorded pid is dead, or alive but not a java process (the pid
    /// was reused by something unrelated) — the stale file is removed, but
    /// nothing is killed.
    RemoveStale,
    /// The recorded pid is alive and still a java process: an orphaned JVM
    /// left over from a session this core did not shut down cleanly.
    KillOrphan,
}

/// Decide what to do with a recorded pid. Deliberately split out from the
/// sysinfo lookup so this branch is unit-testable without a real process
/// table: `is_live_java` is the caller's answer to "is this pid currently
/// alive and running as java", however it determined that.
///
/// Killing only ever requires BOTH a pid that was actually recorded AND
/// sysinfo confirming it currently looks like a JVM — a name mismatch (pid
/// reused by an unrelated process) always falls back to `RemoveStale`.
pub fn reconcile_pid(recorded_pid: Option<u32>, is_live_java: bool) -> PidReconcileAction {
    match recorded_pid {
        None => PidReconcileAction::NoPidFile,
        Some(_) if is_live_java => PidReconcileAction::KillOrphan,
        Some(_) => PidReconcileAction::RemoveStale,
    }
}

/// Whether `pid` is currently alive and its process name contains "java"
/// (case-insensitive). `system` must already have been refreshed for this
/// pid (a targeted `ProcessesToUpdate::Some` refresh) — this only reads
/// back what's already there.
pub fn is_live_java_process(system: &System, pid: u32) -> bool {
    system
        .process(Pid::from_u32(pid))
        .map(|p| {
            p.name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("java")
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysinfo::ProcessesToUpdate;

    /// Generic test-only pidfile name — the real callers each use their own
    /// (`.gdl_server.pid` / `.gdl_instance.pid`), exercised by their own
    /// modules' tests. These tests only exercise the name-agnostic logic.
    const TEST_PID_FILE_NAME: &str = ".gdl_test.pid";

    #[tokio::test]
    async fn pid_file_write_read_remove_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Nothing written yet.
        assert_eq!(read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap(), None);

        write_pid_file(root, TEST_PID_FILE_NAME, 4242).await;
        assert_eq!(
            read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap(),
            Some(4242)
        );
        assert!(pid_file_path(root, TEST_PID_FILE_NAME).exists());

        remove_pid_file(root, TEST_PID_FILE_NAME).await;
        assert_eq!(read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap(), None);
        assert!(!pid_file_path(root, TEST_PID_FILE_NAME).exists());

        // Removing an already-gone pidfile is not an error (best-effort).
        remove_pid_file(root, TEST_PID_FILE_NAME).await;
    }

    #[tokio::test]
    async fn write_pid_file_overwrites_a_stale_value() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        write_pid_file(root, TEST_PID_FILE_NAME, 111).await;
        write_pid_file(root, TEST_PID_FILE_NAME, 222).await;

        assert_eq!(
            read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap(),
            Some(222)
        );
    }

    #[tokio::test]
    async fn read_pid_file_errors_on_garbage_instead_of_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        tokio::fs::write(pid_file_path(root, TEST_PID_FILE_NAME), b"not-a-pid")
            .await
            .unwrap();

        assert!(read_pid_file(root, TEST_PID_FILE_NAME).await.is_err());
    }

    #[tokio::test]
    async fn write_pid_file_failure_does_not_panic() {
        // Point at a root whose parent doesn't exist, so the write fails —
        // this must be swallowed (logged) rather than panicking, since
        // `write_pid_file` is best-effort and must never block a launch.
        let bogus_root = std::path::Path::new("/nonexistent/gdl-test-pidfile-root");
        write_pid_file(bogus_root, TEST_PID_FILE_NAME, 123).await;
    }

    // --- orphan pid reconciliation ---------------------------------------

    #[test]
    fn reconcile_pid_decisions() {
        // No pidfile at all: nothing to do, regardless of what a liveness
        // check would have said.
        assert_eq!(reconcile_pid(None, false), PidReconcileAction::NoPidFile);
        assert_eq!(reconcile_pid(None, true), PidReconcileAction::NoPidFile);

        // A recorded pid that's dead, or alive but not java (reused by
        // something unrelated) — never kill, just drop the stale file.
        assert_eq!(
            reconcile_pid(Some(1234), false),
            PidReconcileAction::RemoveStale
        );

        // A recorded pid that's alive AND still java: an orphaned JVM.
        assert_eq!(
            reconcile_pid(Some(1234), true),
            PidReconcileAction::KillOrphan
        );
    }

    #[test]
    fn is_live_java_process_false_for_a_dead_pid() {
        // A pid this large will not be alive on any real system — this
        // exercises the "pid not found" branch of `System::process`.
        let dead_pid = u32::MAX - 100;
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(dead_pid)]));

        assert!(!is_live_java_process(&system, dead_pid));
    }

    #[test]
    fn is_live_java_process_false_for_a_live_non_java_pid() {
        // The test binary itself is alive but is not a java process — this
        // exercises the name-mismatch branch without spawning anything.
        let own_pid = std::process::id();
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(own_pid)]));

        // Confirm sysinfo actually found the process (proving the `false`
        // below comes from the name not matching, not from a lookup that
        // silently failed and would have returned `false` either way).
        assert!(
            system.process(Pid::from_u32(own_pid)).is_some(),
            "sysinfo did not find this test's own live process"
        );
        assert!(!is_live_java_process(&system, own_pid));
    }

    #[tokio::test]
    async fn orphan_reconciliation_removes_pidfile_for_a_dead_recorded_pid() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let dead_pid = u32::MAX - 100;
        write_pid_file(root, TEST_PID_FILE_NAME, dead_pid).await;

        let recorded = read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap();
        assert_eq!(recorded, Some(dead_pid));

        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(dead_pid)]));
        let is_live_java = is_live_java_process(&system, dead_pid);

        match reconcile_pid(recorded, is_live_java) {
            PidReconcileAction::RemoveStale => remove_pid_file(root, TEST_PID_FILE_NAME).await,
            other => panic!("expected RemoveStale for a dead pid, got {:?}", other),
        }

        assert_eq!(read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap(), None);
    }

    #[tokio::test]
    async fn orphan_reconciliation_removes_pidfile_for_a_live_non_java_pid() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Our own pid: alive for the duration of the test, but not java.
        let own_pid = std::process::id();
        write_pid_file(root, TEST_PID_FILE_NAME, own_pid).await;
        let recorded = read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap();

        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(own_pid)]));
        let is_live_java = is_live_java_process(&system, own_pid);
        assert!(
            !is_live_java,
            "test process must not be misidentified as java"
        );

        match reconcile_pid(recorded, is_live_java) {
            PidReconcileAction::RemoveStale => remove_pid_file(root, TEST_PID_FILE_NAME).await,
            other => panic!(
                "expected RemoveStale for a live non-java pid, got {:?}",
                other
            ),
        }

        assert_eq!(read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap(), None);
        // Confirms reconciliation never tried to kill anything here: this
        // test's own process is still alive to make this assertion at all.
        assert!(System::new_all().process(Pid::from_u32(own_pid)).is_some());
    }
}
