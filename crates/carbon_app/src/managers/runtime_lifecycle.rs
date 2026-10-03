//! Relocation admission owns external writers independently of UI/map state.
//! Lock order: briefly acquire lifecycle mutex before instance/server operation gates;
//! never hold it across awaits. Abandoned external ownership remains a blocker.
use anyhow::{Context, Result, bail};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use sysinfo::{Pid, ProcessesToUpdate, System};

#[derive(Default)]
pub struct RuntimeLifecycle {
    state: Mutex<State>,
    child_records: Option<std::path::PathBuf>,
}
#[derive(Default)]
struct State {
    ready: bool,
    failure: Option<String>,
    relocation: Option<(String, bool)>,
    next_id: u64,
    activities: HashMap<u64, String>,
    adopted: HashMap<(u32, u64), String>,
}

pub struct RuntimeActivity {
    lifecycle: Arc<RuntimeLifecycle>,
    id: u64,
    retain_on_drop: bool,
    record: Option<std::path::PathBuf>,
}
impl RuntimeActivity {
    /// Call immediately before spawning an external writer. Drop/cancellation is
    /// no longer completion. Only observed exit (or a proven spawn failure) clears it.
    pub fn external(&mut self) {
        self.retain_on_drop = true;
    }
    /// All external writers retain a durable relocation record. Persist intent
    /// before spawn so a core crash cannot turn an orphan into apparent quiescence.
    pub fn external_child(&mut self) -> Result<()> {
        if let Some(root) = &self.lifecycle.child_records {
            std::fs::create_dir_all(root)?;
            let record = root.join(format!("{}.json", uuid::Uuid::new_v4()));
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&record)?;
            self.record = Some(record);
            file.sync_all()?; // empty record is explicitly Unknown, never completion
            #[cfg(unix)]
            {
                std::fs::File::open(root)?.sync_all()?;
                if let Some(parent) = root.parent() {
                    std::fs::File::open(parent)?.sync_all()?;
                }
            }
        }
        self.external();
        Ok(())
    }
    pub fn record_child_pid(&self, pid: Option<u32>) -> Result<()> {
        let Some(record) = &self.record else {
            return Ok(());
        };
        let Some(pid) = pid else {
            return Ok(());
        };
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));
        let start = system.process(Pid::from_u32(pid)).map(|p| p.start_time());
        {
            let start = start.unwrap_or(0);
            let bytes = serde_json::to_vec(&(pid, start))?;
            // A partial record remains Unknown on next startup; never removes authority.
            let mut file = std::fs::OpenOptions::new().write(true).open(record)?;
            std::io::Write::write_all(&mut file, &bytes)?;
            file.sync_all()?;
        }
        Ok(())
    }
    pub fn finish(mut self) {
        self.retain_on_drop = false;
    }
}
impl Drop for RuntimeActivity {
    fn drop(&mut self) {
        if !self.retain_on_drop {
            if let Some(record) = &self.record {
                if let Err(error) = std::fs::remove_file(record) {
                    if error.kind() != std::io::ErrorKind::NotFound {
                        tracing::warn!(
                            "Confirmed completion, but ownership record could not be removed: {error}"
                        );
                    }
                }
            }
            self.lifecycle
                .state
                .lock()
                .unwrap()
                .activities
                .remove(&self.id);
        }
    }
}

fn relocation_process_absent(observed_start: Option<u64>, _recorded_start: u64) -> bool {
    // Epoch start times may change after a wall-clock correction (Task22/M23).
    // A present PID, even a mismatched/reused one, is not relocation exit proof.
    observed_start.is_none()
}

fn confirmed_absent(pid: u32, observed_start: Option<u64>, recorded_start: u64) -> bool {
    if !relocation_process_absent(observed_start, recorded_start) {
        return false;
    }
    #[cfg(target_os = "linux")]
    {
        // A partial/permission-limited sysinfo refresh must not free a live owner.
        if std::fs::metadata("/proc/self").is_err() {
            return false;
        }
        matches!(std::fs::metadata(format!("/proc/{pid}")), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
    }
    #[cfg(windows)]
    {
        use std::ffi::c_void;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
            fn GetLastError() -> u32;
            fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
            fn CloseHandle(handle: *mut c_void) -> i32;
        }
        if pid == 0 {
            return false;
        }
        // Read-only process query: access denied means Unknown, never Exited.
        unsafe {
            let handle = OpenProcess(0x1000, 0, pid); // PROCESS_QUERY_LIMITED_INFORMATION
            if handle.is_null() {
                return GetLastError() == 87;
            } // ERROR_INVALID_PARAMETER: absent PID
            let mut code = 259;
            let queried = GetExitCodeProcess(handle, &mut code);
            CloseHandle(handle);
            queried != 0 && code != 259 // STILL_ACTIVE
        }
    }
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn kill(pid: i32, signal: i32) -> i32;
            fn __error() -> *mut i32;
        }
        if pid == 0 || pid > i32::MAX as u32 {
            return false;
        }
        // Signal zero only queries existence; EPERM and other errors remain Unknown.
        unsafe { kill(pid as i32, 0) == -1 && *__error() == 3 } // ESRCH
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        false
    }
}

impl RuntimeLifecycle {
    pub fn with_runtime_path(root: &std::path::Path) -> Self {
        Self {
            state: Mutex::new(State::default()),
            child_records: Some(root.join("__gdl_runtime_writers")),
        }
    }
    pub async fn reconcile_children(&self) -> Result<()> {
        let Some(root) = &self.child_records else {
            return Ok(());
        };
        let mut entries = match tokio::fs::read_dir(root).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        while let Some(entry) = entries.next_entry().await? {
            let bytes = tokio::fs::read(entry.path()).await?;
            let (pid, start): (u32, u64) = serde_json::from_slice(&bytes)
                .with_context(|| format!("Unconfirmed external writer ownership at {}; after independently confirming the child has exited, remove this record and restart to retry", entry.path().display()))?;
            let mut system = System::new();
            system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));
            let observed = system.process(Pid::from_u32(pid)).map(|p| p.start_time());
            if confirmed_absent(pid, observed, start) {
                tokio::fs::remove_file(entry.path()).await?;
            } else {
                self.state
                    .lock()
                    .unwrap()
                    .adopted
                    .insert((pid, start), entry.path().display().to_string());
            }
        }
        Ok(())
    }
    pub fn admit(self: &Arc<Self>, owner: impl Into<String>) -> Result<RuntimeActivity> {
        let mut state = self.state.lock().unwrap();
        if state.relocation.is_some() {
            bail!("Runtime relocation is in progress; retry after restart");
        }
        state.next_id += 1;
        let id = state.next_id;
        state.activities.insert(id, owner.into());
        Ok(RuntimeActivity {
            lifecycle: self.clone(),
            id,
            retain_on_drop: false,
            record: None,
        })
    }
    pub fn reconciliation_finished(&self, result: Result<()>) {
        let mut state = self.state.lock().unwrap();
        match result {
            Ok(()) if state.failure.is_none() => state.ready = true,
            Ok(()) => {}
            Err(error) => {
                state.ready = false;
                state.failure = Some(format!("{error:#}"));
            }
        }
    }
    pub fn begin_relocation(&self, request: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if !state.ready {
            bail!(
                "Runtime ownership reconciliation is pending or failed: {}",
                state.failure.as_deref().unwrap_or("startup in progress")
            );
        }
        if state.relocation.is_some() {
            bail!("Another runtime relocation is in progress");
        }
        if !state.activities.is_empty() || !state.adopted.is_empty() {
            bail!(
                "Runtime is busy; finish games, preparations, servers and installers, then retry. For a failed or cancelled child, let it exit then restart to reconcile its record. Unconfirmed writers: {:?} {:?}",
                state.activities.values(),
                state.adopted.values()
            );
        }
        state.relocation = Some((request.to_owned(), false));
        Ok(())
    }
    pub fn cancel_relocation(&self, request: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if state
            .relocation
            .as_ref()
            .is_some_and(|(id, shutdown)| id == request && !shutdown)
        {
            state.relocation = None;
            Ok(())
        } else {
            bail!("No matching cancellable relocation")
        }
    }
    pub fn commit_shutdown(&self, request: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        match state.relocation.as_mut() {
            Some((id, shutdown)) if id == request => {
                *shutdown = true;
                Ok(())
            }
            _ => bail!("Shutdown requires the matching relocation lease"),
        }
    }
    pub fn relocation_shutdown(&self) -> bool {
        self.state
            .lock()
            .unwrap()
            .relocation
            .as_ref()
            .is_some_and(|(_, shutdown)| *shutdown)
    }
    /// Snapshot process identities outside the admission mutex. Removal only ever
    /// affects captured identities; a concurrent admission still blocks begin_relocation.
    pub fn refresh_adopted(&self) {
        let captured: Vec<_> = self.state.lock().unwrap().adopted.keys().copied().collect();
        let pids: Vec<_> = captured
            .iter()
            .map(|(pid, _)| Pid::from_u32(*pid))
            .collect();
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&pids));
        let mut state = self.state.lock().unwrap();
        for (pid, start) in captured {
            if confirmed_absent(
                pid,
                system.process(Pid::from_u32(pid)).map(|p| p.start_time()),
                start,
            ) {
                state.adopted.remove(&(pid, start));
            }
        }
    }
    fn persist_reconciled_process(&self, pid: u32, start: u64) -> Result<()> {
        let Some(root) = &self.child_records else {
            return Ok(());
        };
        std::fs::create_dir_all(root)?;
        let record = root.join(format!("reconciled-{pid}-{start}.json"));
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(record)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        std::io::Write::write_all(&mut file, &serde_json::to_vec(&(pid, start))?)?;
        file.sync_all()?;
        #[cfg(unix)]
        {
            std::fs::File::open(root)?.sync_all()?;
            if let Some(parent) = root.parent() {
                std::fs::File::open(parent)?.sync_all()?;
            }
        }
        Ok(())
    }

    /// Runs before best-effort UI reconciliation can discard corrupt pidfiles.
    pub async fn reconcile_root(&self, root: &std::path::Path, marker: &str) -> Result<()> {
        let mut entries = match tokio::fs::read_dir(root).await {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        while let Some(entry) = entries.next_entry().await? {
            if !entry.file_type().await?.is_dir() {
                continue;
            }
            let record = super::orphan_pid::read_pid_file(&entry.path(), marker)
                .await
                .with_context(|| format!("Cannot reconcile {}", entry.path().display()))?;
            if let Some((pid, expected)) = record {
                let mut system = System::new();
                system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));
                let observed = system.process(Pid::from_u32(pid)).map(|p| p.start_time());
                if !confirmed_absent(pid, observed, expected.unwrap_or(0)) {
                    // Best-effort UI reconciliation can discard legacy/mismatched
                    // pidfiles. Preserve relocation ownership before allowing it.
                    self.persist_reconciled_process(pid, expected.unwrap_or(0))?;
                    self.state.lock().unwrap().adopted.insert(
                        (pid, expected.unwrap_or(0)),
                        format!(
                            "{} (present or unconfirmed process)",
                            entry.path().display()
                        ),
                    );
                }
            }
        }
        Ok(())
    }
}

/// Owns an external command independently of the future invoking it. Cancellation
/// or wait failure leaves an explicit uncertain writer; timeout/kill is not reap.
pub async fn tracked_output(
    lifecycle: &Arc<RuntimeLifecycle>,
    command: &mut tokio::process::Command,
) -> std::io::Result<std::process::Output> {
    let mut activity = lifecycle
        .admit("installer or hook child")
        .map_err(std::io::Error::other)?;
    command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    activity.external_child().map_err(std::io::Error::other)?;
    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            activity.finish();
            return Err(error);
        }
    };
    activity
        .record_child_pid(child.id())
        .map_err(std::io::Error::other)?;
    let result = child.wait_with_output().await;
    if result.is_ok() {
        activity.finish();
    }
    result
}

#[cfg(test)]
pub(crate) mod tests_support {
    use std::cell::Cell;

    thread_local! {
        static CHILD_RECORD_FAULT: Cell<bool> = const { Cell::new(false) };
    }

    /// Until dropped, writing a started child's identity into its record
    /// fails on this thread, as on a drive that just ran out of space.
    pub(crate) struct ChildRecordsFail(());

    pub(crate) fn fail_child_records() -> ChildRecordsFail {
        CHILD_RECORD_FAULT.with(|fault| fault.set(true));
        ChildRecordsFail(())
    }

    impl Drop for ChildRecordsFail {
        fn drop(&mut self) {
            CHILD_RECORD_FAULT.with(|fault| fault.set(false));
        }
    }

    pub(super) fn child_record_fault() -> std::io::Result<()> {
        if CHILD_RECORD_FAULT.with(Cell::get) {
            return Err(std::io::ErrorKind::StorageFull.into());
        }
        Ok(())
    }

    /// What a Java probe gets from [`super::tracked_output`] when the data
    /// folder refuses its process record.
    pub(crate) fn bookkeeping_error() -> anyhow::Error {
        let error = super::ChildBookkeepingFailed::before_spawn(std::io::Error::from(
            std::io::ErrorKind::StorageFull,
        ));
        anyhow::Error::from(std::io::Error::other(error))
            .context("Cannot execute java checker command")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ready() -> Arc<RuntimeLifecycle> {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        lifecycle
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn runtime_lifecycle_missing_snapshot_is_not_absence_when_proc_exists() {
        assert!(!confirmed_absent(std::process::id(), None, 0));
        assert!(confirmed_absent(u32::MAX, None, 0));
    }
    #[test]
    fn runtime_lifecycle_pre_spawn_record_failure_releases_admission() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("__gdl_runtime_writers"), "not a directory").unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        lifecycle.reconciliation_finished(Ok(()));
        let mut activity = lifecycle.admit("test").unwrap();
        assert!(activity.external_child().is_err());
        drop(activity);
        lifecycle.begin_relocation("move").unwrap();
    }

    /// A data folder on a volume that does not sync folders (a VirtualBox
    /// shared folder answers EINVAL, network mounts ENOTSUP or ENOTTY) still
    /// gets process records, so Play, servers and Java checks work there as
    /// they did before records existed. The records folder's own entry in
    /// the data folder is synced only when the folder is new, so a Java
    /// check does not pay for it every time. A real I/O error still refuses
    /// the start.
    #[cfg(unix)]
    #[test]
    fn process_records_work_where_folders_cannot_be_synced() {
        use crate::util::durable::test_hooks::FolderSyncs;
        let root = tempfile::tempdir().unwrap();
        let writers = root.path().join("__gdl_runtime_writers");
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        lifecycle.reconciliation_finished(Ok(()));

        let syncs = FolderSyncs::failing_with(root.path(), libc::EINVAL);
        let mut first = lifecycle.admit("java check").unwrap();
        first.external_child().unwrap();
        first.record_child_pid(Some(std::process::id())).unwrap();
        let record = first.record.clone().unwrap();
        assert_eq!(
            decode_child_record(&std::fs::read(&record).unwrap())
                .unwrap()
                .pid,
            std::process::id()
        );
        let mut second = lifecycle.admit("java check").unwrap();
        second.external_child().unwrap();
        assert_eq!(
            syncs.synced(),
            [writers.clone(), root.path().to_path_buf(), writers.clone()],
            "the data folder is synced once, when the records folder is new"
        );
        drop(syncs);
        first.finish();
        second.finish();
        assert!(!record.exists());

        let _syncs = FolderSyncs::failing_with(root.path(), libc::EIO);
        let mut refused = lifecycle.admit("server").unwrap();
        let error = anyhow::Error::from(refused.external_child().unwrap_err());
        assert!(child_bookkeeping_failed(&error), "{error:#}");
        refused.finish();
        lifecycle.begin_relocation("move").unwrap();
    }

    /// A game or server whose identity cannot be recorded once it started
    /// stays tracked: its activity keeps running with the identity in
    /// memory, nothing reads it as unconfirmed, and its exit settles it as
    /// usual. The empty record stays behind as Unknown evidence in case the
    /// launcher crashes before that, and the log says why it stayed empty.
    #[tracing_test::traced_test]
    #[test]
    fn a_started_program_whose_record_cannot_be_written_stays_tracked() {
        let root = tempfile::tempdir().unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        lifecycle.reconciliation_finished(Ok(()));
        let mut activity = lifecycle.admit("game 1").unwrap();
        activity.external_child().unwrap();
        activity
            .spawn(ActivityPolicy::Game, || Ok::<_, std::io::Error>(()))
            .unwrap();
        let record = activity.record.clone().unwrap();
        assert!(activity.child_identity().is_none());
        {
            let _fault = tests_support::fail_child_records();
            activity.record_spawned_child(Some(std::process::id()));
        }
        assert!(logs_contain(
            "could not record the started program in the data folder"
        ));
        assert!(logs_contain(
            &std::io::Error::from(std::io::ErrorKind::StorageFull).to_string()
        ));
        assert_eq!(
            activity.child_identity().map(|identity| identity.pid),
            Some(std::process::id())
        );
        assert!(std::fs::read(&record).unwrap().is_empty());
        assert_eq!(
            lifecycle.state.lock().unwrap().activities[&activity.id].phase,
            ActivityPhase::Running
        );
        activity.finish();
        assert!(!record.exists());
        lifecycle.begin_relocation("move").unwrap();
    }

    #[test]
    fn runtime_lifecycle_wall_clock_mismatch_is_not_exit_proof() {
        assert!(
            !relocation_process_absent(Some(1_700_000_100), 1_700_000_000),
            "a still-present process can report a changed epoch start after a clock correction"
        );
        assert!(!relocation_process_absent(Some(0), 1_700_000_000));
        assert!(!relocation_process_absent(
            Some(1_700_000_000),
            1_700_000_000
        ));
        assert!(relocation_process_absent(None, 1_700_000_000));
    }

    #[tokio::test]
    async fn runtime_lifecycle_abandoned_child_record_survives_restart() {
        let root = tempfile::tempdir().unwrap();
        let l = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        let mut child = l.admit("installer").unwrap();
        child.external_child().unwrap();
        drop(child);
        let restarted = RuntimeLifecycle::with_runtime_path(root.path());
        let result = restarted.reconcile_children().await;
        assert!(result.is_err());
        restarted.reconciliation_finished(result);
        assert!(restarted.begin_relocation("one").is_err());
        assert_eq!(
            std::fs::read_dir(root.path().join("__gdl_runtime_writers"))
                .unwrap()
                .count(),
            1
        );
    }
    #[tokio::test]
    async fn runtime_lifecycle_confirmed_child_clears_durable_ownership() {
        let root = tempfile::tempdir().unwrap();
        let l = Arc::new(RuntimeLifecycle::with_runtime_path(root.path()));
        let mut child = l.admit("installer").unwrap();
        child.external_child().unwrap();
        child.finish();
        let restarted = RuntimeLifecycle::with_runtime_path(root.path());
        restarted.reconciliation_finished(restarted.reconcile_children().await);
        restarted.begin_relocation("one").unwrap();
    }
    #[tokio::test]
    async fn runtime_lifecycle_corrupt_pidfile_blocks_before_best_effort_cleanup() {
        let root = tempfile::tempdir().unwrap();
        let instance = root.path().join("instance");
        std::fs::create_dir(&instance).unwrap();
        std::fs::write(instance.join(".pid"), "malformed").unwrap();
        let l = RuntimeLifecycle::default();
        let result = l.reconcile_root(root.path(), ".pid").await;
        assert!(result.is_err());
        l.reconciliation_finished(result);
        assert!(l.begin_relocation("one").is_err());
        assert_eq!(
            std::fs::read_to_string(instance.join(".pid")).unwrap(),
            "malformed"
        );
    }

    #[test]
    fn runtime_lifecycle_startup_and_failure_block() {
        let l = RuntimeLifecycle::default();
        assert!(l.begin_relocation("one").is_err());
        l.reconciliation_finished(Err(anyhow::anyhow!("unreadable pidfile")));
        l.reconciliation_finished(Ok(()));
        assert!(l.begin_relocation("one").is_err());
    }
    #[test]
    fn runtime_lifecycle_external_drop_does_not_prove_exit() {
        let l = ready();
        let mut writer = l.admit("installer").unwrap();
        writer.external();
        drop(writer);
        assert!(l.begin_relocation("one").is_err());
    }
    #[test]
    fn runtime_lifecycle_confirmed_completion_and_reentry() {
        let l = ready();
        let mut writer = l.admit("game and post-exit hooks").unwrap();
        writer.external();
        assert!(l.begin_relocation("one").is_err());
        writer.finish();
        l.begin_relocation("one").unwrap();
        assert!(l.admit("queued preparation").is_err());
        assert!(l.cancel_relocation("stale").is_err());
        l.cancel_relocation("one").unwrap();
        drop(l.admit("another game").unwrap());
        l.begin_relocation("two").unwrap();
        l.commit_shutdown("two").unwrap();
        assert!(l.cancel_relocation("two").is_err());
        assert!(l.admit("automatic server restart").is_err());
    }
    #[test]
    fn runtime_lifecycle_launch_vs_relocation_has_one_winner() {
        let l = ready();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        std::thread::scope(|scope| {
            let launch = scope.spawn(|| {
                barrier.wait();
                l.admit("launch")
            });
            barrier.wait();
            let moved = l.begin_relocation("one").is_ok();
            let launch = launch.join().unwrap();
            assert_ne!(moved, launch.is_ok());
        });
    }
    #[test]
    fn runtime_lifecycle_adopted_remains_independent_of_ui_state() {
        let l = ready();
        l.state
            .lock()
            .unwrap()
            .adopted
            .insert((42, 100), "adopted".into());
        assert!(l.begin_relocation("one").is_err());
        assert!(l.commit_shutdown("one").is_err());
    }
}
