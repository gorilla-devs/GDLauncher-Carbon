//! Shared on-disk pidfile lifecycle and orphan-JVM reconciliation logic.
//!
//! Both the server manager and the instance (game) manager record the pid of
//! the java process they spawn in a small marker file next to that
//! server/instance's own data, so that a core which exits without shutting
//! the process down through its own launch task (crash, force-quit, Windows
//! `TerminateProcess`, or simply the user closing the launcher) leaves the
//! next startup able to find that JVM again.
//!
//! Finding it is shared; what to do about it is not. A local server is
//! infrastructure the launcher hosts, so `ServerManager` kills a server JVM
//! it finds still running. A game is the user's session, so
//! `InstanceManager` adopts it instead, showing the instance as running with
//! a working Stop. Everything up to and including "is this pid still a live
//! java process this launcher actually spawned" is the same either way, and
//! is security-relevant — a wrong call either leaks a running JVM forever or
//! acts on an unrelated process that reused the pid — so it lives here once,
//! parameterized by the pidfile name each caller uses (`.gdl_server.pid` /
//! `.gdl_instance.pid`).
//!
//! A pid alone is not a stable identity: the OS recycles pid numbers, and a
//! long-lived core (or a reboot) can easily outlive the JVM it recorded, so
//! by the time reconciliation runs the same pid may belong to a completely
//! unrelated process. The pidfile therefore also records the process's start
//! time (seconds since epoch, from `sysinfo::Process::start_time()`) as a
//! second line, and reconciliation only ever treats a pid as this
//! launcher's own JVM when the live process's start time matches the
//! recorded one within tolerance. A pidfile written before this check
//! existed has only the pid line (the "legacy" format) and can never be
//! verified, so it is always treated the same as a stranger's process:
//! removed, never killed, never adopted.

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

/// Record `pid` and its `start_time` (seconds since epoch, as reported by
/// sysinfo right after spawning it) as the current java process for
/// whatever lives at `root`. The file is two lines, `"{pid}\n{start_time}"`
/// — the start time is what lets a future reconcile pass prove this pid
/// still refers to the same process rather than one that reused the number.
///
/// Best-effort: a write failure is only logged. Losing the pidfile just
/// means an unclean exit during this run won't be auto-cleaned up on the
/// next launch — it must never block or fail the launch itself. If the
/// caller could not determine a start time at all (the process had already
/// exited by the time it looked), it must not call this: a pidfile with no
/// provable identity is worse than no pidfile, since a legacy/unverifiable
/// entry is always refused rather than acted on anyway.
pub async fn write_pid_file(root: &Path, file_name: &str, pid: u32, start_time: u64) {
    let path = pid_file_path(root, file_name);
    let content = format!("{pid}\n{start_time}");
    if let Err(e) = tokio::fs::write(&path, content).await {
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
///
/// The inner `Option<u64>` is the recorded start time: `Some` for a current
/// two-line pidfile, `None` for a legacy single-line (pid-only) pidfile
/// written before start-time recording existed. A missing second line is
/// not a parse error — it is exactly how a legacy pidfile is told apart
/// from a current one, so reconciliation can refuse to verify it instead of
/// trusting it.
pub async fn read_pid_file(root: &Path, file_name: &str) -> Result<Option<(u32, Option<u64>)>> {
    let path = pid_file_path(root, file_name);
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => {
            let mut lines = content.lines();
            let pid = lines
                .next()
                .unwrap_or("")
                .trim()
                .parse::<u32>()
                .with_context(|| format!("invalid pid recorded in {}", path.display()))?;
            let start_time = match lines.next().map(str::trim) {
                Some(line) if !line.is_empty() => Some(line.parse::<u64>().with_context(|| {
                    format!("invalid start time recorded in {}", path.display())
                })?),
                _ => None,
            };
            Ok(Some((pid, start_time)))
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
    /// The recorded pid is alive and looks like java, but its identity
    /// cannot be proven: either the pidfile is the legacy single-line
    /// format (no recorded start time to check) or the live process's start
    /// time does not match the recorded one within tolerance (the pid was
    /// reused by an unrelated process after this launcher's own JVM
    /// exited). The file is removed, but the pid is never killed or
    /// adopted — an unverifiable match is treated exactly like a
    /// stranger's process.
    NotOurs,
    /// The recorded pid is alive, still a java process, and its start time
    /// matches the one recorded when this launcher spawned it (within
    /// tolerance): a JVM left over from a session this core did not shut
    /// down through its own launch task. Acting on it is the caller's
    /// decision and differs by kind — `ServerManager` kills it,
    /// `InstanceManager` adopts it (see this module's own doc comment for
    /// why they differ).
    StillRunning,
}

/// A recorded pid's current state in the live process table, as far as
/// reconciliation needs it: whether it currently looks like a java process,
/// and when the OS reports it started (seconds since epoch). `None` in
/// place of this struct (rather than a struct with `is_java: false`) means
/// the pid is not in the process table at all — see `live_proc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveProc {
    pub is_java: bool,
    pub start_time: u64,
}

/// How far a recorded start time may differ from what the live process
/// table reports and still count as the same process. Not zero: sysinfo
/// reports start times in whole seconds, so a process recorded right before
/// a second boundary and checked again right after can legitimately read
/// one second later. The extra second of slack absorbs that and ordinary
/// scheduling jitter without weakening the check against an actual pid
/// reuse, whose start time differs from the recorded one by however long
/// the original process ran for — never just a second or two.
const START_TIME_TOLERANCE_SECS: u64 = 2;

/// Whether two start times (seconds since epoch) are close enough to count
/// as the same process, within `START_TIME_TOLERANCE_SECS`. Shared by
/// `reconcile_pid` (verifying a freshly-read pidfile against the live
/// process table) and by anything re-checking a pid it already verified
/// once — an adopted game's Stop button and its liveness poller both need
/// the exact same tolerance, or a pid reused right after the game the
/// launcher adopted exits could pass one check and fail the other.
pub fn start_times_match(a: u64, b: u64) -> bool {
    a.abs_diff(b) <= START_TIME_TOLERANCE_SECS
}

/// Decide what a recorded pid represents, given the live process table's
/// answer for that same pid (`None` if it isn't alive at all). Deliberately
/// split out from the sysinfo lookup so this branch is unit-testable
/// without a real process table.
///
/// `StillRunning` — the only outcome either caller ever acts on rather than
/// just cleaning up after — requires all three: a live process sysinfo
/// confirms currently looks like a JVM, a start time recorded for that pid
/// (ruling out a legacy pidfile), and that recorded start time matching the
/// live process's within `START_TIME_TOLERANCE_SECS`. A name mismatch (pid
/// reused by something that isn't java at all) falls back to `RemoveStale`;
/// a java-looking live process behind a legacy or start-time-mismatched
/// pidfile falls back to `NotOurs`. Either way neither the kill nor the
/// adoption can land on a stranger's process or on a pid this launcher
/// cannot actually prove it owns.
pub fn reconcile_pid(
    recorded: Option<(u32, Option<u64>)>,
    live: Option<LiveProc>,
) -> PidReconcileAction {
    let Some((_, recorded_start)) = recorded else {
        return PidReconcileAction::NoPidFile;
    };

    let Some(live) = live else {
        return PidReconcileAction::RemoveStale;
    };

    if !live.is_java {
        return PidReconcileAction::RemoveStale;
    }

    match recorded_start {
        Some(recorded_start) if start_times_match(recorded_start, live.start_time) => {
            PidReconcileAction::StillRunning
        }
        _ => PidReconcileAction::NotOurs,
    }
}

/// Look up `pid`'s current state in the process table for reconciliation.
/// `system` must already have been refreshed for this pid (a targeted
/// `ProcessesToUpdate::Some` refresh) — this only reads back what's already
/// there. `None` means the pid is not currently alive.
pub fn live_proc(system: &System, pid: u32) -> Option<LiveProc> {
    system.process(Pid::from_u32(pid)).map(|p| LiveProc {
        is_java: p
            .name()
            .to_string_lossy()
            .to_ascii_lowercase()
            .contains("java"),
        start_time: p.start_time(),
    })
}

/// Whether `pid` is currently alive and its process name contains "java"
/// (case-insensitive). Same refresh requirement as `live_proc`, which this
/// is built on.
pub fn is_live_java_process(system: &System, pid: u32) -> bool {
    live_proc(system, pid).map(|p| p.is_java).unwrap_or(false)
}

/// Whether `pid` is still, right now, the exact process an adopted running
/// instance was verified against at adoption time: alive, java-looking, and
/// with a start time matching `expected_start_time`. Same refresh
/// requirement as `live_proc`.
///
/// `expected_start_time` is `None` for a session this core launched itself
/// (which never re-checks a pid this way — it holds the real child handle
/// instead) and always fails closed rather than falling back to a bare
/// liveness check: a caller with nothing to verify against has no basis to
/// call anything "the same process".
///
/// This is what closes the in-session pid-reuse window an adopted game's
/// Stop button and its liveness poller would otherwise have: adoption
/// verifies the pid once against its start time, but a poll (or a Stop
/// click) five seconds later is a fresh OS lookup on a bare pid number —
/// long enough after the real game exits for the OS to have handed that
/// number to something else entirely. Re-checking the start time every time,
/// not just once at adoption, is what keeps that later lookup honest.
pub fn is_verified_live_java_process(
    system: &System,
    pid: u32,
    expected_start_time: Option<u64>,
) -> bool {
    let Some(expected_start_time) = expected_start_time else {
        return false;
    };
    live_proc(system, pid)
        .map(|p| p.is_java && start_times_match(expected_start_time, p.start_time))
        .unwrap_or(false)
}

/// The start time (seconds since epoch) `system` currently reports for
/// `pid`, if it is alive. Same refresh requirement as `live_proc`. Each
/// writer site calls this immediately after spawning a process, to capture
/// the start time `write_pid_file` records for a later reconcile pass to
/// verify against.
pub fn process_start_time(system: &System, pid: u32) -> Option<u64> {
    system.process(Pid::from_u32(pid)).map(|p| p.start_time())
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

        write_pid_file(root, TEST_PID_FILE_NAME, 4242, 1_700_000_000).await;
        assert_eq!(
            read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap(),
            Some((4242, Some(1_700_000_000)))
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

        write_pid_file(root, TEST_PID_FILE_NAME, 111, 1000).await;
        write_pid_file(root, TEST_PID_FILE_NAME, 222, 2000).await;

        assert_eq!(
            read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap(),
            Some((222, Some(2000)))
        );
    }

    #[tokio::test]
    async fn legacy_single_line_pidfile_parses_as_pid_with_no_start_time() {
        // A pidfile written before start-time recording existed has only the
        // pid line. It must parse as legacy (pid, None), not error — that is
        // exactly the signal reconciliation uses to refuse to verify it.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        tokio::fs::write(pid_file_path(root, TEST_PID_FILE_NAME), b"4242")
            .await
            .unwrap();

        assert_eq!(
            read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap(),
            Some((4242, None))
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
        write_pid_file(bogus_root, TEST_PID_FILE_NAME, 123, 1000).await;
    }

    // --- orphan pid reconciliation ---------------------------------------

    #[test]
    fn reconcile_pid_decisions() {
        // No pidfile at all: nothing to do, regardless of what the live
        // process table would have said.
        assert_eq!(reconcile_pid(None, None), PidReconcileAction::NoPidFile);
        assert_eq!(
            reconcile_pid(
                None,
                Some(LiveProc {
                    is_java: true,
                    start_time: 1000
                })
            ),
            PidReconcileAction::NoPidFile
        );

        // A recorded pid that's dead — never kill, just drop the stale file.
        assert_eq!(
            reconcile_pid(Some((1234, Some(1000))), None),
            PidReconcileAction::RemoveStale
        );

        // A recorded pid that's alive but not java (reused by something
        // unrelated) — same: drop the file, never touch the process.
        assert_eq!(
            reconcile_pid(
                Some((1234, Some(1000))),
                Some(LiveProc {
                    is_java: false,
                    start_time: 1000
                })
            ),
            PidReconcileAction::RemoveStale
        );
    }

    #[test]
    fn reconcile_accepts_matching_start_time_within_tolerance() {
        // Exactly matching.
        assert_eq!(
            reconcile_pid(
                Some((1234, Some(1000))),
                Some(LiveProc {
                    is_java: true,
                    start_time: 1000
                })
            ),
            PidReconcileAction::StillRunning
        );

        // Within the +-2s tolerance, either direction.
        assert_eq!(
            reconcile_pid(
                Some((1234, Some(1000))),
                Some(LiveProc {
                    is_java: true,
                    start_time: 1002
                })
            ),
            PidReconcileAction::StillRunning
        );
        assert_eq!(
            reconcile_pid(
                Some((1234, Some(1000))),
                Some(LiveProc {
                    is_java: true,
                    start_time: 998
                })
            ),
            PidReconcileAction::StillRunning
        );
    }

    #[test]
    fn reconcile_refuses_pid_with_mismatched_start_time() {
        // Recorded start 1000, live start 5000: a JVM that started at a
        // wildly different time than what this launcher recorded is not the
        // process this launcher spawned, however alive and java-looking it
        // is now.
        assert_eq!(
            reconcile_pid(
                Some((1234, Some(1000))),
                Some(LiveProc {
                    is_java: true,
                    start_time: 5000
                })
            ),
            PidReconcileAction::NotOurs
        );

        // Just outside the +-2s tolerance, either direction.
        assert_eq!(
            reconcile_pid(
                Some((1234, Some(1000))),
                Some(LiveProc {
                    is_java: true,
                    start_time: 1003
                })
            ),
            PidReconcileAction::NotOurs
        );
        assert_eq!(
            reconcile_pid(
                Some((1234, Some(1000))),
                Some(LiveProc {
                    is_java: true,
                    start_time: 997
                })
            ),
            PidReconcileAction::NotOurs
        );
    }

    #[test]
    fn legacy_pidfile_without_start_time_is_dropped_not_killed() {
        // A legacy single-line pidfile carries no start time to verify
        // against. Even though the live pid is alive and looks like java,
        // its identity cannot be proven, so it must never be killed
        // (server) or adopted (instance) — only the stale file is dropped.
        assert_eq!(
            reconcile_pid(
                Some((1234, None)),
                Some(LiveProc {
                    is_java: true,
                    start_time: 1_700_000_000
                })
            ),
            PidReconcileAction::NotOurs
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

    #[test]
    fn start_times_match_within_tolerance_but_not_outside_it() {
        assert!(start_times_match(1000, 1000));
        assert!(start_times_match(1000, 1002));
        assert!(start_times_match(1000, 998));
        assert!(!start_times_match(1000, 1003));
        assert!(!start_times_match(1000, 997));
        assert!(!start_times_match(1000, 5000));
    }

    #[test]
    fn live_proc_none_for_a_dead_pid() {
        let dead_pid = u32::MAX - 100;
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(dead_pid)]));

        assert_eq!(live_proc(&system, dead_pid), None);
    }

    #[test]
    fn is_verified_live_java_process_false_with_no_expected_start_time() {
        // No start time to verify against (a launched-by-this-core session,
        // which never calls this at all in practice) must fail closed rather
        // than degrade to a bare liveness check.
        let own_pid = std::process::id();
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(own_pid)]));

        assert!(!is_verified_live_java_process(&system, own_pid, None));
    }

    #[test]
    fn is_verified_live_java_process_false_for_a_dead_pid() {
        let dead_pid = u32::MAX - 100;
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(dead_pid)]));

        assert!(!is_verified_live_java_process(
            &system,
            dead_pid,
            Some(1_700_000_000)
        ));
    }

    #[test]
    fn is_verified_live_java_process_false_for_a_live_non_java_pid_even_with_a_matching_start_time()
    {
        // The test binary itself is alive but not java — even a start time
        // that happens to match must not verify it: `is_java` is checked
        // independently of the start time, not inferred from it.
        let own_pid = std::process::id();
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(own_pid)]));
        let actual_start_time = live_proc(&system, own_pid).unwrap().start_time;

        assert!(!is_verified_live_java_process(
            &system,
            own_pid,
            Some(actual_start_time)
        ));
    }

    #[test]
    fn is_verified_live_java_process_false_for_a_mismatched_start_time() {
        // The crux of the in-session pid-reuse window this closes: a pid
        // that is alive and currently java-looking, but whose start time no
        // longer matches what was recorded when it was first verified — the
        // exact shape of the OS having handed the number to an unrelated
        // process since. This must never be treated as still-the-same
        // process, however java-looking it is right now.
        let own_pid = std::process::id();
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(own_pid)]));
        let actual_start_time = live_proc(&system, own_pid).unwrap().start_time;

        assert!(!is_verified_live_java_process(
            &system,
            own_pid,
            Some(actual_start_time.saturating_add(86_400))
        ));
    }

    #[test]
    fn process_start_time_none_for_a_dead_pid() {
        let dead_pid = u32::MAX - 100;
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(dead_pid)]));

        assert_eq!(process_start_time(&system, dead_pid), None);
    }

    #[test]
    fn process_start_time_matches_the_value_live_proc_reports() {
        let own_pid = std::process::id();
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(own_pid)]));

        let via_live_proc = live_proc(&system, own_pid)
            .expect("sysinfo did not find this test's own live process")
            .start_time;
        assert_eq!(process_start_time(&system, own_pid), Some(via_live_proc));
    }

    #[tokio::test]
    async fn orphan_reconciliation_removes_pidfile_for_a_dead_recorded_pid() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let dead_pid = u32::MAX - 100;
        write_pid_file(root, TEST_PID_FILE_NAME, dead_pid, 1_700_000_000).await;

        let recorded = read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap();
        assert_eq!(recorded, Some((dead_pid, Some(1_700_000_000))));

        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(dead_pid)]));
        let live = live_proc(&system, dead_pid);

        match reconcile_pid(recorded, live) {
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
        write_pid_file(root, TEST_PID_FILE_NAME, own_pid, 1_700_000_000).await;
        let recorded = read_pid_file(root, TEST_PID_FILE_NAME).await.unwrap();

        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(own_pid)]));
        let live = live_proc(&system, own_pid);
        assert!(
            !live.map(|p| p.is_java).unwrap_or(false),
            "test process must not be misidentified as java"
        );

        match reconcile_pid(recorded, live) {
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
