//! Shared on-disk pidfile lifecycle and orphan-JVM reconciliation logic.
//!
//! Both the server manager and the instance (game) manager record the pid of
//! the java process they spawn in a small marker file next to that
//! server/instance's own data, so that a core which exits without shutting
//! the process down through its own launch task (crash, force-quit, Windows
//! `TerminateProcess`, or simply the user closing the launcher) leaves the
//! next startup able to find that JVM again.
//!
//! Instance recovery uses the identity classification below for adoption.
//! Server recovery retains live or ambiguous ownership without signalling;
//! ServerManager retires only markers with proven absence or stable reuse. Save completion for a
//! recovered server is unknown and requires operator recovery. RuntimeLifecycle
//! retains unconfirmed ownership as a relocation blocker.
//!
//! A PID is reusable. New Linux records retain epoch metadata on their first
//! two lines and append `linux-boot-ticks-v1 <boot UUID> <start ticks>`. The
//! exact boot/ticks pair supplies ownership across wall-clock adjustments.
//! Legacy epochs require the previous conservative proof before upgrade;
//! missing/malformed identity or legacy clock mismatch stays unconfirmed.
//! Unknown identity never authorizes signalling or destroys recovery evidence.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use sysinfo::{Pid, System};
use tracing::warn;

/// Stable Linux identity; epoch time remains metadata, never Linux authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ProcessIdentity {
    LinuxBootTicks {
        boot_id: uuid::Uuid,
        start_ticks: u64,
    },
}
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessRecord {
    pub pid: u32,
    pub start_time: Option<u64>,
    pub identity: Option<ProcessIdentity>,
}
// Epoch time is metadata once a stable token exists. Keys/cleanup compare the
// full authoritative PID/boot/ticks identity; legacy keys still include epoch.
impl PartialEq for ProcessRecord {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
            && match (self.identity, other.identity) {
                (Some(a), Some(b)) => a == b,
                (None, None) => self.start_time == other.start_time,
                _ => false,
            }
    }
}
impl Eq for ProcessRecord {}
impl std::hash::Hash for ProcessRecord {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.pid, state);
        std::hash::Hash::hash(&self.identity, state);
        if self.identity.is_none() {
            std::hash::Hash::hash(&self.start_time, state);
        }
    }
}
impl ProcessRecord {
    pub(crate) fn validate_identity(self) -> Result<Self> {
        if let Some(ProcessIdentity::LinuxBootTicks { boot_id, .. }) = self.identity {
            anyhow::ensure!(!boot_id.is_nil(), "Invalid boot identity");
        }
        Ok(self)
    }
    pub fn legacy(pid: u32, start_time: u64) -> Self {
        Self {
            pid,
            start_time: Some(start_time),
            identity: None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessObservation {
    Present {
        process: LiveProc,
        identity: Option<ProcessIdentity>,
    },
    Absent,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityStatus {
    Same,
    GoneOrReused,
    Unknown,
}

/// Legacy clock mismatches cannot prove exit/reuse on Linux. A present stable
/// extension is authoritative and can never fall back to epoch matching.
pub fn identity_status(
    record: ProcessRecord,
    observed: ProcessObservation,
    require_java: bool,
) -> IdentityStatus {
    if record.validate_identity().is_err() {
        return IdentityStatus::Unknown;
    }
    match observed {
        ProcessObservation::Absent => IdentityStatus::GoneOrReused,
        ProcessObservation::Unknown => IdentityStatus::Unknown,
        ProcessObservation::Present { process, identity } => {
            if require_java && !process.is_java {
                return IdentityStatus::GoneOrReused;
            }
            match (record.identity, identity) {
                (Some(expected), Some(actual)) if expected == actual => IdentityStatus::Same,
                (Some(_), Some(_)) => IdentityStatus::GoneOrReused,
                (Some(_), None) => IdentityStatus::Unknown,
                (None, _)
                    if record
                        .start_time
                        .is_some_and(|start| start_times_match(start, process.start_time)) =>
                {
                    IdentityStatus::Same
                }
                (None, _) if record.start_time.is_none() => IdentityStatus::Unknown,
                (None, _) => {
                    #[cfg(target_os = "linux")]
                    {
                        IdentityStatus::Unknown
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        IdentityStatus::GoneOrReused
                    }
                }
            }
        }
    }
}
/// Capture a legacy upgrade only after its existing Java/epoch proof succeeds.
pub fn verified_record(
    record: ProcessRecord,
    observed: ProcessObservation,
    require_java: bool,
) -> Option<ProcessRecord> {
    if identity_status(record, observed, require_java) != IdentityStatus::Same {
        return None;
    }
    let ProcessObservation::Present { identity, .. } = observed else {
        return None;
    };
    Some(ProcessRecord {
        identity: record.identity.or(identity),
        ..record
    })
}

#[cfg(target_os = "linux")]
fn parse_linux_identity(pid: u32, boot: &str, stat: &str) -> Option<ProcessIdentity> {
    let boot = uuid::Uuid::parse_str(boot.trim()).ok()?;
    if boot.is_nil() {
        return None;
    }
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    if close <= open || stat[..open].trim().parse::<u32>().ok()? != pid {
        return None;
    }
    // comm may contain whitespace and closing parentheses. The final ')' ends
    // field 2; field 22 is the twentieth token starting with field 3.
    let mut fields = stat[close + 1..].split_ascii_whitespace();
    let state = fields.next()?;
    if state.len() != 1 || !state.as_bytes()[0].is_ascii_alphabetic() {
        return None;
    }
    let start_ticks = fields.nth(18)?.parse::<u64>().ok()?;
    Some(ProcessIdentity::LinuxBootTicks {
        boot_id: boot,
        start_ticks,
    })
}
#[cfg(target_os = "linux")]
fn read_linux_identity(pid: u32) -> std::io::Result<ProcessIdentity> {
    let boot = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?;
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))?;
    parse_linux_identity(pid, &boot, &stat)
        .ok_or_else(|| std::io::Error::other("Invalid Linux process identity"))
}
/// The snapshot is bracketed by stable observations: a recycled PID cannot
/// supply one generation's Java name and another generation's start token.
#[cfg(target_os = "linux")]
fn observe_bracketed(
    before: std::io::Result<ProcessIdentity>,
    snapshot: impl FnOnce() -> Option<LiveProc>,
    after: impl FnOnce() -> std::io::Result<ProcessIdentity>,
    absence_confirmed: impl FnOnce() -> bool,
) -> ProcessObservation {
    let before = match before {
        Ok(identity) => identity,
        Err(error) => {
            return if error.kind() == std::io::ErrorKind::NotFound && absence_confirmed() {
                ProcessObservation::Absent
            } else {
                ProcessObservation::Unknown
            };
        }
    };
    let process = snapshot();
    match (process, after()) {
        (Some(process), Ok(identity)) if before == identity => ProcessObservation::Present {
            process,
            identity: Some(identity),
        },
        _ => ProcessObservation::Unknown,
    }
}
pub fn observe_process(pid: u32) -> ProcessObservation {
    let snapshot = || {
        let mut system = System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));
        live_proc(&system, pid)
    };
    #[cfg(target_os = "linux")]
    {
        observe_bracketed(
            read_linux_identity(pid),
            snapshot,
            || read_linux_identity(pid),
            || {
                // ENOENT from an unavailable proc mount is not absence proof.
                std::fs::read_to_string("/proc/self/stat").is_ok()
                    && matches!(std::fs::metadata(format!("/proc/{pid}")), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
            },
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        match snapshot() {
            Some(process) => ProcessObservation::Present {
                process,
                identity: None,
            },
            None => ProcessObservation::Absent,
        }
    }
}
/// Called immediately at owned spawn, while the actual child is retained.
pub fn capture_process(pid: u32) -> Option<ProcessRecord> {
    capture_observed(pid, observe_process(pid))
}
pub(crate) fn capture_observed(pid: u32, observed: ProcessObservation) -> Option<ProcessRecord> {
    #[cfg(target_os = "linux")]
    if matches!(observed, ProcessObservation::Present { identity: None, .. }) {
        return None;
    }
    match observed {
        ProcessObservation::Present { process, identity } => Some(ProcessRecord {
            pid,
            start_time: Some(process.start_time),
            identity,
        }),
        _ => None,
    }
}

pub async fn write_process_file(root: &Path, file_name: &str, record: ProcessRecord) {
    let mut content = format!("{}\n{}", record.pid, record.start_time.unwrap_or(0));
    if let Some(ProcessIdentity::LinuxBootTicks {
        boot_id,
        start_ticks,
    }) = record.identity
    {
        content.push_str(&format!("\nlinux-boot-ticks-v1 {boot_id} {start_ticks}"));
    }
    if let Err(error) = tokio::fs::write(pid_file_path(root, file_name), content).await {
        warn!("Failed to write process marker: {error}");
    }
}
pub async fn read_process_file(root: &Path, file_name: &str) -> Result<Option<ProcessRecord>> {
    let path = pid_file_path(root, file_name);
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut lines = content.lines();
    let pid = lines.next().unwrap_or("").trim().parse::<u32>()?;
    let start_time = match lines.next().map(str::trim) {
        Some("") | None => None,
        Some(value) => Some(value.parse::<u64>()?),
    };
    let identity = match lines.next() {
        None => None,
        Some(extension) => {
            let fields: Vec<_> = extension.split_ascii_whitespace().collect();
            anyhow::ensure!(
                fields.len() == 3 && fields[0] == "linux-boot-ticks-v1",
                "Unknown process identity extension"
            );
            let boot_id = uuid::Uuid::parse_str(fields[1])?;
            anyhow::ensure!(!boot_id.is_nil(), "Invalid boot identity");
            Some(ProcessIdentity::LinuxBootTicks {
                boot_id,
                start_ticks: fields[2].parse()?,
            })
        }
    };
    anyhow::ensure!(
        lines.next().is_none(),
        "Unexpected process identity extension"
    );
    Ok(Some(ProcessRecord {
        pid,
        start_time,
        identity,
    }))
}
pub async fn remove_process_file_if_matches_strict(
    root: &Path,
    file_name: &str,
    expected: ProcessRecord,
) -> Result<()> {
    match read_process_file(root, file_name).await? {
        None => Ok(()),
        Some(actual) if actual == expected => Ok(remove_pid_file_strict(root, file_name).await?),
        Some(_) => anyhow::bail!("Process marker changed; replacement ownership retained"),
    }
}
/// Exit cleanup is bound to the full captured marker, not a reused bare PID.
pub async fn remove_process_file_if_matches(root: &Path, file_name: &str, expected: ProcessRecord) {
    if read_process_file(root, file_name).await.ok().flatten() == Some(expected) {
        remove_pid_file(root, file_name).await;
    }
}

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

/// Ownership transfer requires confirmed removal before publishing a copied
/// root. Unlike exit cleanup, unexpected unlink errors must abort publication.
pub async fn remove_pid_file_strict(root: &Path, file_name: &str) -> std::io::Result<()> {
    match tokio::fs::remove_file(pid_file_path(root, file_name)).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
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
    Ok(read_process_file(root, file_name)
        .await?
        .map(|record| (record.pid, record.start_time)))
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
    /// decision: `InstanceManager` may adopt it. `ServerManager` shows a
    /// recovered server as running from a previous session and signals it
    /// only when the user clicks Stop, after verifying it again.
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

/// For tests that run a copy of a program named `java` as a stand-in JVM:
/// waits until `pid` is seen under that name. `spawn` can return before the
/// child is: Linux lets the parent go on and closes the child's exec-error
/// pipe before it renames the child late in `exec`, so the child can still
/// be seen under the test binary's name, which is not Java.
#[cfg(test)]
pub(crate) fn wait_until_named_java(pid: u32) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let (_, name) = observe_process_named(pid);
        if name.as_deref() == Some("java") {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "process {pid} was not seen named java within 5 s; last seen as {name:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// For tests: starts a copy of `sleep` named `java` in `dir`, which the
/// launcher takes for a game or server JVM, and returns once it is seen
/// under that name. Dropping the child ends it.
#[cfg(all(test, unix))]
pub(crate) async fn spawn_fake_java(dir: &std::path::Path) -> tokio::process::Child {
    let fake = dir.join("java");
    tokio::fs::copy("/bin/sleep", &fake).await.unwrap();
    // Another test forking while the copy was open for writing can hold it
    // open in its child until that child execs: ETXTBSY.
    let mut attempts = 0;
    let child = loop {
        match tokio::process::Command::new(&fake)
            .arg("60")
            // Never outlive the test, however it ends.
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => break child,
            Err(error) if error.raw_os_error() == Some(26) && attempts < 50 => {
                attempts += 1;
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            Err(error) => panic!("cannot start the fake JVM: {error}"),
        }
    };
    wait_until_named_java(child.id().expect("the fake JVM has a pid"));
    child
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

    /// The name comes from the snapshot that observed the process, and only
    /// with a process that was observed.
    #[test]
    fn observe_process_named_names_only_an_observed_process() {
        let own_pid = std::process::id();
        let (observed, name) = observe_process_named(own_pid);
        assert!(matches!(observed, ProcessObservation::Present { .. }));
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(own_pid)]));
        let expected = system
            .process(Pid::from_u32(own_pid))
            .unwrap()
            .name()
            .to_string_lossy()
            .into_owned();
        assert_eq!(name.as_deref(), Some(expected.as_str()));
        assert_eq!(observe_process(own_pid), observed);

        let (observed, name) = observe_process_named(u32::MAX - 100);
        assert!(!matches!(observed, ProcessObservation::Present { .. }));
        assert_eq!(name, None);
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

#[cfg(test)]
mod task22_tests {
    use super::*;
    pub(super) fn record() -> ProcessRecord {
        ProcessRecord {
            pid: 4242,
            start_time: Some(1_700_000_000),
            identity: Some(ProcessIdentity::LinuxBootTicks {
                boot_id: uuid::Uuid::parse_str("cf2d86a1-9be4-41d2-94cb-1e2c71d7cc80").unwrap(),
                start_ticks: 123456,
            }),
        }
    }
    fn live(epoch: u64, identity: Option<ProcessIdentity>) -> ProcessObservation {
        ProcessObservation::Present {
            process: LiveProc {
                is_java: true,
                start_time: epoch,
            },
            identity,
        }
    }
    #[tokio::test]
    async fn task22_clock_steps_marker_roundtrip_retains_owned_game() {
        let root = tempfile::tempdir().unwrap();
        let record = record();
        write_process_file(root.path(), ".gdl_instance.pid", record).await;
        let read = read_process_file(root.path(), ".gdl_instance.pid")
            .await
            .unwrap()
            .unwrap();
        for epoch in [1_699_999_970, 1_700_000_030] {
            assert_eq!(
                identity_status(read, live(epoch, record.identity), true),
                IdentityStatus::Same
            );
            assert_eq!(
                verified_record(read, live(epoch, record.identity), true),
                Some(record)
            );
        }
        assert!(pid_file_path(root.path(), ".gdl_instance.pid").exists());
    }
    #[test]
    fn task22_reused_ticks_boot_and_non_java_never_authorize_stop() {
        let record = record();
        let ProcessIdentity::LinuxBootTicks {
            boot_id,
            start_ticks,
        } = record.identity.unwrap();
        for identity in [
            ProcessIdentity::LinuxBootTicks {
                boot_id,
                start_ticks: start_ticks + 1,
            },
            ProcessIdentity::LinuxBootTicks {
                boot_id: uuid::Uuid::new_v4(),
                start_ticks,
            },
        ] {
            assert_eq!(
                identity_status(
                    record,
                    live(record.start_time.unwrap(), Some(identity)),
                    true
                ),
                IdentityStatus::GoneOrReused
            );
            assert_eq!(
                verified_record(
                    record,
                    live(record.start_time.unwrap(), Some(identity)),
                    true
                ),
                None
            );
        }
        assert_eq!(
            identity_status(
                record,
                ProcessObservation::Present {
                    process: LiveProc {
                        is_java: false,
                        start_time: 1_700_000_000
                    },
                    identity: record.identity
                },
                true
            ),
            IdentityStatus::GoneOrReused
        );
        assert_eq!(
            identity_status(record, ProcessObservation::Absent, true),
            IdentityStatus::GoneOrReused
        );
    }
    #[test]
    fn task22_unknown_never_downgrades_stable_identity() {
        let record = record();
        for observation in [
            ProcessObservation::Unknown,
            live(record.start_time.unwrap(), None),
        ] {
            assert_eq!(
                identity_status(record, observation, true),
                IdentityStatus::Unknown
            );
            assert_eq!(verified_record(record, observation, true), None);
        }
    }
    #[test]
    fn task22_legacy_epoch_proof_and_pid_only_controls() {
        let mut record = ProcessRecord::legacy(4242, 1000);
        for epoch in [998, 1000, 1002] {
            assert_eq!(
                identity_status(record, live(epoch, self::record().identity), true),
                IdentityStatus::Same
            );
            assert_eq!(
                verified_record(record, live(epoch, self::record().identity), true)
                    .unwrap()
                    .identity,
                self::record().identity
            );
        }
        #[cfg(target_os = "linux")]
        let mismatch = IdentityStatus::Unknown;
        #[cfg(not(target_os = "linux"))]
        let mismatch = IdentityStatus::GoneOrReused;
        for epoch in [997, 1003, 1030] {
            assert_eq!(
                identity_status(record, live(epoch, self::record().identity), true),
                mismatch
            );
            assert!(verified_record(record, live(epoch, self::record().identity), true).is_none());
        }
        record.start_time = None;
        assert_eq!(
            identity_status(record, live(1000, self::record().identity), true),
            IdentityStatus::Unknown
        );
    }
    #[tokio::test]
    async fn task22_malformed_present_extensions_are_retained_without_epoch_fallback() {
        let root = tempfile::tempdir().unwrap();
        let path = pid_file_path(root.path(), "marker");
        for extension in [
            "unknown 1 2",
            "linux-boot-ticks-v1 invalid 123",
            "linux-boot-ticks-v1 cf2d86a1-9be4-41d2-94cb-1e2c71d7cc80 nope",
            "",
            "linux-boot-ticks-v1 cf2d86a1-9be4-41d2-94cb-1e2c71d7cc80 123\nextra",
        ] {
            tokio::fs::write(&path, format!("4242\n1700000000\n{extension}\n"))
                .await
                .unwrap();
            assert!(
                read_process_file(root.path(), "marker").await.is_err(),
                "{extension}"
            );
            assert!(read_pid_file(root.path(), "marker").await.is_err());
            assert!(path.exists());
        }
    }
    #[tokio::test]
    async fn task22_exit_cleanup_cannot_remove_replacement_full_identity() {
        let root = tempfile::tempdir().unwrap();
        let captured = record();
        let mut replacement = captured;
        replacement.identity = Some(ProcessIdentity::LinuxBootTicks {
            boot_id: uuid::Uuid::new_v4(),
            start_ticks: 123456,
        });
        write_process_file(root.path(), "marker", replacement).await;
        remove_process_file_if_matches(root.path(), "marker", captured).await;
        assert_eq!(
            read_process_file(root.path(), "marker").await.unwrap(),
            Some(replacement)
        );
        remove_process_file_if_matches(root.path(), "marker", replacement).await;
        assert_eq!(
            read_process_file(root.path(), "marker").await.unwrap(),
            None
        );
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn task22_linux_stat_handles_command_parentheses_and_whitespace() {
        let stat = format!(
            "4242 (java (worker) ) name) S {} 123456 0",
            vec!["0"; 18].join(" ")
        );
        assert_eq!(
            parse_linux_identity(4242, "cf2d86a1-9be4-41d2-94cb-1e2c71d7cc80\n", &stat),
            record().identity
        );
        assert!(
            parse_linux_identity(4243, "cf2d86a1-9be4-41d2-94cb-1e2c71d7cc80", &stat).is_none()
        );
        assert!(parse_linux_identity(4242, "invalid", &stat).is_none());
        assert!(
            parse_linux_identity(
                4242,
                "cf2d86a1-9be4-41d2-94cb-1e2c71d7cc80",
                "4242 (java) S 0"
            )
            .is_none()
        );
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn task22_bracketing_refuses_mixed_generations_and_unproven_absence() {
        let identity = record().identity.unwrap();
        let snapshot = || {
            Some(LiveProc {
                is_java: true,
                start_time: 1000,
            })
        };
        assert!(matches!(
            observe_bracketed(Ok(identity), snapshot, || Ok(identity), || false),
            ProcessObservation::Present { .. }
        ));
        let changed = ProcessIdentity::LinuxBootTicks {
            boot_id: uuid::Uuid::new_v4(),
            start_ticks: 123456,
        };
        assert_eq!(
            observe_bracketed(Ok(identity), snapshot, || Ok(changed), || false),
            ProcessObservation::Unknown
        );
        assert_eq!(
            observe_bracketed(Ok(identity), || None, || Ok(identity), || false),
            ProcessObservation::Unknown
        );
        for absent in [true, false] {
            assert_eq!(
                observe_bracketed(
                    Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
                    || panic!("No lookup after failed token"),
                    || panic!("No lookup after failed token"),
                    || absent
                ),
                if absent {
                    ProcessObservation::Absent
                } else {
                    ProcessObservation::Unknown
                }
            );
        }
        assert_eq!(
            observe_bracketed(
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
                snapshot,
                || Ok(identity),
                || true
            ),
            ProcessObservation::Unknown
        );
    }
}

#[cfg(test)]
mod task22_strict_tests {
    use super::*;
    #[tokio::test]
    async fn task22_duplicate_strict_cleanup_absence_file_and_failure_controls() {
        let root = tempfile::tempdir().unwrap();
        remove_pid_file_strict(root.path(), "marker").await.unwrap();
        tokio::fs::write(
            root.path().join("marker"),
            b"4242\n1700000000\nlinux-boot-ticks-v1 cf2d86a1-9be4-41d2-94cb-1e2c71d7cc80 123456",
        )
        .await
        .unwrap();
        remove_pid_file_strict(root.path(), "marker").await.unwrap();
        assert!(!root.path().join("marker").exists());
        tokio::fs::create_dir(root.path().join("marker"))
            .await
            .unwrap();
        assert!(remove_pid_file_strict(root.path(), "marker").await.is_err());
        assert!(root.path().join("marker").is_dir());
    }
}
