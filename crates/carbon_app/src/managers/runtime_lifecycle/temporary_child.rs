//! The temporary-child IO boundary is injectable without executing a process.
use super::RuntimeActivity;
use std::{
    io,
    pin::Pin,
    process::{ExitStatus, Output},
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncRead, AsyncReadExt};

type Reader = Pin<Box<dyn AsyncRead + Send>>;
#[async_trait::async_trait]
pub(super) trait Child: Send + 'static {
    fn stdout(&mut self) -> Option<Reader>;
    fn stderr(&mut self) -> Option<Reader>;
    fn start_kill(&mut self) -> io::Result<()>;
    async fn wait(&mut self) -> io::Result<ExitStatus>;
}
pub(super) struct NativeChild(pub tokio::process::Child);
#[async_trait::async_trait]
impl Child for NativeChild {
    fn stdout(&mut self) -> Option<Reader> {
        self.0.stdout.take().map(|r| Box::pin(r) as Reader)
    }
    fn stderr(&mut self) -> Option<Reader> {
        self.0.stderr.take().map(|r| Box::pin(r) as Reader)
    }
    fn start_kill(&mut self) -> io::Result<()> {
        self.0.start_kill()
    }
    async fn wait(&mut self) -> io::Result<ExitStatus> {
        self.0.wait().await
    }
}
const REAP_BUDGET: std::time::Duration = std::time::Duration::from_secs(5);
const OUTPUT_BUDGET: std::time::Duration = std::time::Duration::from_secs(2);
const MAX_CAPTURE: usize = 4 * 1024 * 1024;

struct CancelOnDrop(tokio_util::sync::CancellationToken);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

/// Output captured from one stream. Shared with the supervisor so that a
/// reader aborted after the child's exit (a descendant still holds the pipe)
/// does not take what it had already read with it.
type Captured = Arc<Mutex<Vec<u8>>>;

fn take_captured(captured: &Captured) -> Vec<u8> {
    std::mem::take(&mut *captured.lock().unwrap())
}

/// A reader stopped by our own abort just ended early; anything else it
/// returned (it may have finished in the meantime) keeps its meaning.
fn after_abort(result: Result<io::Result<()>, tokio::task::JoinError>) -> io::Result<()> {
    match result {
        Err(error) if error.is_cancelled() => Ok(()),
        result => result.map_err(io::Error::other).and_then(|r| r),
    }
}

async fn read(
    mut reader: Option<Reader>,
    progress: Option<tokio::sync::watch::Sender<super::ChildProgress>>,
    captured: Captured,
) -> io::Result<()> {
    let Some(reader) = reader.as_mut() else {
        return Ok(());
    };
    let mut chunk = [0; 8192];
    let mut line = Vec::new();
    let mut snapshot = super::ChildProgress::default();
    let mut truncated = false;
    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        {
            let mut bytes = captured.lock().unwrap();
            if bytes.len() + read > MAX_CAPTURE {
                let excess = bytes.len() + read - MAX_CAPTURE;
                bytes.drain(..excess);
                truncated = true;
            }
            bytes.extend_from_slice(&chunk[..read]);
        }
        if let Some(progress) = &progress {
            for byte in &chunk[..read] {
                if *byte == b'\n' {
                    let text = String::from_utf8_lossy(&line);
                    if text.starts_with("Processing:")
                        || text.starts_with("  Processing:")
                        || text.contains("] Processing:")
                    {
                        snapshot.processors = snapshot.processors.saturating_add(1);
                    }
                    progress.send_replace(snapshot.clone());
                    line.clear();
                } else if line.len() < 65536 {
                    line.push(*byte);
                }
            }
        }
    }
    if !line.is_empty() {
        if let Some(progress) = &progress {
            let text = String::from_utf8_lossy(&line);
            if text.starts_with("Processing:")
                || text.starts_with("  Processing:")
                || text.contains("] Processing:")
            {
                snapshot.processors = snapshot.processors.saturating_add(1);
            }
            progress.send_replace(snapshot);
        }
    }
    if truncated {
        tracing::warn!("Temporary child output capture retained only its last 4 MiB");
    }
    Ok(())
}

/// The caller owns only a cancellation request and receiver. The supervisor owns
/// the child, readers and lifecycle ACK across caller loss and cleanup timeouts.
pub(super) async fn collect(activity: RuntimeActivity, child: impl Child) -> io::Result<Output> {
    collect_observed(activity, child, None).await
}

pub(super) async fn collect_observed(
    activity: RuntimeActivity,
    child: impl Child,
    progress: Option<tokio::sync::watch::Sender<super::ChildProgress>>,
) -> io::Result<Output> {
    let cancel = activity.cancellation();
    let _cancel_on_drop = CancelOnDrop(cancel.clone());
    tokio::spawn(supervise(activity, child, cancel, progress))
        .await
        .map_err(io::Error::other)?
}

async fn supervise(
    activity: RuntimeActivity,
    mut child: impl Child,
    cancel: tokio_util::sync::CancellationToken,
    progress: Option<tokio::sync::watch::Sender<super::ChildProgress>>,
) -> io::Result<Output> {
    let out_bytes = Captured::default();
    let err_bytes = Captured::default();
    let mut stdout = tokio::spawn(read(child.stdout(), progress, out_bytes.clone()));
    let mut stderr = tokio::spawn(read(child.stderr(), None, err_bytes.clone()));
    let mut out = None;
    let mut err = None;
    let mut cancelled = false;
    let status = loop {
        tokio::select! {
            result = child.wait() => break result,
            _ = cancel.cancelled() => { cancelled = true; break stop_and_wait(&activity, &mut child).await; }
            result = &mut stdout, if out.is_none() => {
                out = Some(result.map_err(io::Error::other).and_then(|r| r));
                if out.as_ref().unwrap().is_err() { break stop_and_wait(&activity, &mut child).await; }
            }
            result = &mut stderr, if err.is_none() => {
                err = Some(result.map_err(io::Error::other).and_then(|r| r));
                if err.as_ref().unwrap().is_err() { break stop_and_wait(&activity, &mut child).await; }
            }
        }
    };
    let status = match status {
        Ok(status) => status,
        Err(error) => {
            activity.unknown(format!("Temporary child wait failed: {error}"));
            let _ = child.start_kill();
            // A wait failure cannot release the handle/reservations. Keep its
            // owner until a later real wait confirms exit, never an accepted kill.
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if child.wait().await.is_ok() {
                    break;
                }
            }
            stdout.abort();
            stderr.abort();
            if out.is_none() {
                let _ = stdout.await;
            }
            if err.is_none() {
                let _ = stderr.await;
            }
            return Err(io::Error::other(super::ChildSettlementUnknown)); // Preserve unknown cleanup classification after later reap.
        }
    };
    // The child is reaped. Only a descendant that inherited its pipes can keep
    // them open now, and descendants were never owned here: past the budget,
    // stop reading and settle with the output captured so far. Unknown stays
    // reserved for a wait that failed or a reap that was never confirmed.
    let deadline = tokio::time::sleep(OUTPUT_BUDGET);
    tokio::pin!(deadline);
    while out.is_none() || err.is_none() {
        tokio::select! {
            result = &mut stdout, if out.is_none() => { out = Some(result.map_err(io::Error::other).and_then(|r| r)); }
            result = &mut stderr, if err.is_none() => { err = Some(result.map_err(io::Error::other).and_then(|r| r)); }
            _ = &mut deadline => {
                tracing::warn!(
                    "Temporary child output stayed open {OUTPUT_BUDGET:?} after exit; a descendant may still hold its pipes. Continuing with the output captured so far"
                );
                if out.is_none() { stdout.abort(); out = Some(after_abort((&mut stdout).await)); }
                if err.is_none() { stderr.abort(); err = Some(after_abort((&mut stderr).await)); }
            }
        }
    }
    let output = out.unwrap().and(err.unwrap());
    // A returned stream error is settled owned IO; preserve the failure while
    // releasing only after child wait and both reader tasks have returned.
    activity.finish();
    output?;
    let (stdout, stderr) = (take_captured(&out_bytes), take_captured(&err_bytes));
    if cancelled {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Temporary child cancelled and reaped",
        ));
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

async fn stop_and_wait(
    activity: &RuntimeActivity,
    child: &mut impl Child,
) -> io::Result<ExitStatus> {
    if let Err(error) = child.start_kill() {
        tracing::warn!(
            "Temporary child termination request failed; retaining wait ownership: {error}"
        );
    }
    match tokio::time::timeout(REAP_BUDGET, child.wait()).await {
        Ok(result) => result,
        Err(_) => {
            activity.unknown("Temporary child did not confirm reap within 5 seconds".into());
            // Keep supervising beyond the reporting budget. The shutdown request
            // will report Unresolved; neither its timeout nor caller loss releases us.
            child.wait().await
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::managers::runtime_lifecycle::{ActivityPolicy, RuntimeLifecycle, ShutdownPurpose};
    use std::{
        os::unix::process::ExitStatusExt,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };
    struct ControlledChild {
        exit: tokio::sync::oneshot::Receiver<io::Result<ExitStatus>>,
        kills: Arc<AtomicUsize>,
        stdout: Option<Reader>,
        stderr: Option<Reader>,
    }
    #[async_trait::async_trait]
    impl Child for ControlledChild {
        fn stdout(&mut self) -> Option<Reader> {
            self.stdout.take()
        }
        fn stderr(&mut self) -> Option<Reader> {
            self.stderr.take()
        }
        fn start_kill(&mut self) -> io::Result<()> {
            self.kills.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn wait(&mut self) -> io::Result<ExitStatus> {
            (&mut self.exit).await.map_err(io::Error::other)?
        }
    }
    #[tokio::test]
    async fn task18_child_requester_drop_stops_but_wait_proves_settlement() {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        let mut owner = lifecycle
            .admit_owned("fixture child", ActivityPolicy::TemporaryChild)
            .unwrap();
        owner
            .spawn(ActivityPolicy::TemporaryChild, || Ok(()))
            .unwrap();
        let kills = Arc::new(AtomicUsize::new(0));
        let (exit, wait) = tokio::sync::oneshot::channel();
        let child = ControlledChild {
            exit: wait,
            kills: kills.clone(),
            stdout: None,
            stderr: None,
        };
        let requester = tokio::spawn(collect(owner, child));
        tokio::task::yield_now().await;
        requester.abort();
        let _ = requester.await;
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            kills.load(Ordering::SeqCst),
            1,
            "caller loss must request retained child cleanup"
        );
        assert!(
            lifecycle.begin_relocation("busy").is_err(),
            "kill acceptance is not wait proof"
        );
        exit.send(Ok(ExitStatus::from_raw(0))).unwrap();
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        lifecycle.begin_relocation("move").unwrap();
    }
    #[tokio::test]
    async fn task18_child_normal_output_and_nonzero_exit_are_reaped() {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        let owner = lifecycle
            .admit_owned("fixture child", ActivityPolicy::TemporaryChild)
            .unwrap();
        let kills = Arc::new(AtomicUsize::new(0));
        let (exit, wait) = tokio::sync::oneshot::channel();
        exit.send(Ok(ExitStatus::from_raw(256))).unwrap();
        let child = ControlledChild {
            exit: wait,
            kills: kills.clone(),
            stdout: Some(Box::pin(std::io::Cursor::new(b"stdout"))),
            stderr: Some(Box::pin(std::io::Cursor::new(b"stderr"))),
        };
        let output = collect(owner, child).await.unwrap();
        assert!(!output.status.success());
        assert_eq!(output.stdout, b"stdout");
        assert_eq!(output.stderr, b"stderr");
        assert_eq!(kills.load(Ordering::SeqCst), 0);
        lifecycle.begin_relocation("move").unwrap();
    }
    struct SequenceChild {
        waits: tokio::sync::mpsc::Receiver<io::Result<ExitStatus>>,
        kills: Arc<AtomicUsize>,
        stdout: Option<Reader>,
    }
    #[async_trait::async_trait]
    impl Child for SequenceChild {
        fn stdout(&mut self) -> Option<Reader> {
            self.stdout.take()
        }
        fn stderr(&mut self) -> Option<Reader> {
            None
        }
        fn start_kill(&mut self) -> io::Result<()> {
            self.kills.fetch_add(1, Ordering::SeqCst);
            Err(io::Error::other("controlled kill failure"))
        }
        async fn wait(&mut self) -> io::Result<ExitStatus> {
            self.waits.recv().await.expect("controlled wait script")
        }
    }
    fn owner_with_permit() -> (
        Arc<RuntimeLifecycle>,
        RuntimeActivity,
        Arc<tokio::sync::Semaphore>,
    ) {
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        let owner = lifecycle
            .admit_owned("reserved installer", ActivityPolicy::Installer)
            .unwrap();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        let permit = Arc::new(semaphore.clone().try_acquire_owned().unwrap());
        lifecycle
            .state
            .lock()
            .unwrap()
            .activities
            .get_mut(&owner.id)
            .unwrap()
            .reservation = Some(permit);
        (lifecycle, owner, semaphore)
    }
    #[tokio::test]
    async fn task18_child_wait_error_is_unknown_and_preserves_installation_cleanup_signal() {
        let (lifecycle, owner, semaphore) = owner_with_permit();
        let (events, waits) = tokio::sync::mpsc::channel(2);
        let kills = Arc::new(AtomicUsize::new(0));
        events
            .send(Err(io::Error::other("controlled wait failure")))
            .await
            .unwrap();
        let task = tokio::spawn(collect(
            owner,
            SequenceChild {
                waits,
                kills: kills.clone(),
                stdout: None,
            },
        ));
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert_eq!(semaphore.available_permits(), 0);
        assert!(lifecycle.begin_relocation("move").is_err());
        events.send(Ok(ExitStatus::from_raw(0))).await.unwrap();
        let error = task.await.unwrap().unwrap_err();
        assert!(
            super::super::child_settlement_unknown(&anyhow::Error::from(error)),
            "managed cleanup must receive the unconfirmed ownership marker"
        );
        assert_eq!(
            semaphore.available_permits(),
            0,
            "unknown evidence keeps reservation"
        );
        assert_eq!(kills.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn reaped_child_whose_pipes_stay_open_settles_with_the_output_read_so_far() {
        // A hook that starts `nohup helper &`, or a .bat using `start`: the
        // hook itself exits and is reaped, but the helper keeps the inherited
        // pipe open. That must not leave an Unknown activity blocking quit.
        let (lifecycle, owner, semaphore) = owner_with_permit();
        let (pipe, mut held_writer) = tokio::io::duplex(64);
        tokio::io::AsyncWriteExt::write_all(&mut held_writer, b"hook output\n")
            .await
            .unwrap();
        let (exit, wait) = tokio::sync::oneshot::channel();
        exit.send(Ok(ExitStatus::from_raw(0))).unwrap();
        let kills = Arc::new(AtomicUsize::new(0));
        let started = std::time::Instant::now();
        let output = tokio::time::timeout(
            OUTPUT_BUDGET + std::time::Duration::from_secs(3),
            collect(
                owner,
                ControlledChild {
                    exit: wait,
                    kills: kills.clone(),
                    stdout: Some(Box::pin(pipe)),
                    stderr: None,
                },
            ),
        )
        .await
        .expect("the output budget bounds the wait")
        .expect("a reaped child settles even though its pipe is still open");
        assert!(started.elapsed() >= OUTPUT_BUDGET);
        assert!(output.status.success());
        assert_eq!(output.stdout, b"hook output\n");
        assert!(output.stderr.is_empty());
        assert_eq!(
            kills.load(Ordering::SeqCst),
            0,
            "the child was already reaped"
        );
        assert_eq!(
            semaphore.available_permits(),
            1,
            "the activity is finished, not Unknown"
        );
        assert!(lifecycle.state.lock().unwrap().activities.is_empty());
        lifecycle
            .prepare_shutdown("quit", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while lifecycle
                .shutdown_status("quit", ShutdownPurpose::Quit)
                .unwrap()
                .phase
                == super::super::ShutdownPhase::Draining
            {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            lifecycle
                .shutdown_status("quit", ShutdownPurpose::Quit)
                .unwrap()
                .phase,
            super::super::ShutdownPhase::Ready
        );
        lifecycle
            .commit_prepared_shutdown("quit", ShutdownPurpose::Quit)
            .unwrap();
        drop(held_writer);
    }
    #[tokio::test]
    async fn task18_child_progress_is_bounded_and_counts_final_partial_line() {
        let mut bytes = vec![b'x'; MAX_CAPTURE + 8192];
        bytes.push(b'\n');
        bytes.extend_from_slice(b"Processing: first\n[worker] Processing: last");
        let (sender, receiver) =
            tokio::sync::watch::channel(super::super::ChildProgress::default());
        let captured = Captured::default();
        read(
            Some(Box::pin(std::io::Cursor::new(bytes))),
            Some(sender),
            captured.clone(),
        )
        .await
        .unwrap();
        let output = take_captured(&captured);
        assert_eq!(output.len(), MAX_CAPTURE);
        assert!(output.ends_with(b"Processing: last"));
        assert_eq!(
            receiver.borrow().processors,
            2,
            "EOF must publish a final non-newline progress record"
        );
    }
    #[tokio::test]
    async fn task18_child_cancel_kill_failure_waits_for_proof_and_releases_permit() {
        let (lifecycle, owner, semaphore) = owner_with_permit();
        let cancel = owner.cancellation();
        let (events, waits) = tokio::sync::mpsc::channel(1);
        let kills = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn(collect(
            owner,
            SequenceChild {
                waits,
                kills: kills.clone(),
                stdout: None,
            },
        ));
        cancel.cancel();
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert_eq!(kills.load(Ordering::SeqCst), 1);
        assert!(!task.is_finished());
        assert_eq!(semaphore.available_permits(), 0);
        events.send(Ok(ExitStatus::from_raw(0))).await.unwrap();
        assert_eq!(
            task.await.unwrap().unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(semaphore.available_permits(), 1);
        lifecycle.begin_relocation("move").unwrap();
    }

    #[tokio::test]
    async fn task18_child_reap_budget_reports_unknown_then_retains_late_wait() {
        let (lifecycle, owner, semaphore) = owner_with_permit();
        let cancel = owner.cancellation();
        let (events, waits) = tokio::sync::mpsc::channel(1);
        let kills = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn(collect(
            owner,
            SequenceChild {
                waits,
                kills: kills.clone(),
                stdout: None,
            },
        ));
        cancel.cancel();
        tokio::time::timeout(REAP_BUDGET + std::time::Duration::from_secs(2), async {
            loop {
                if lifecycle
                    .state
                    .lock()
                    .unwrap()
                    .activities
                    .values()
                    .any(|entry| entry.phase == super::super::ActivityPhase::Unknown)
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(!task.is_finished());
        assert_eq!(semaphore.available_permits(), 0);
        lifecycle
            .prepare_shutdown("budget", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert!(
            lifecycle
                .commit_prepared_shutdown("budget", ShutdownPurpose::Quit)
                .is_err()
        );
        events.send(Ok(ExitStatus::from_raw(0))).await.unwrap();
        assert_eq!(
            task.await.unwrap().unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(
            semaphore.available_permits(),
            1,
            "only later actual wait plus reader joins permits release"
        );
    }
    struct FailedReader;
    impl AsyncRead for FailedReader {
        fn poll_read(
            self: Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            _: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<io::Result<()>> {
            std::task::Poll::Ready(Err(io::Error::other("controlled reader error")))
        }
    }
    #[tokio::test]
    async fn task18_child_reader_failure_waits_for_reap_before_settled_error() {
        let (lifecycle, owner, semaphore) = owner_with_permit();
        let (events, waits) = tokio::sync::mpsc::channel(1);
        let kills = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn(collect(
            owner,
            SequenceChild {
                waits,
                kills: kills.clone(),
                stdout: Some(Box::pin(FailedReader)),
            },
        ));
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert_eq!(kills.load(Ordering::SeqCst), 1);
        assert!(!task.is_finished());
        assert_eq!(semaphore.available_permits(), 0);
        events.send(Ok(ExitStatus::from_raw(0))).await.unwrap();
        let error = task.await.unwrap().unwrap_err();
        assert!(error.to_string().contains("controlled reader error"));
        assert_eq!(semaphore.available_permits(), 1);
        lifecycle.begin_relocation("reader-settled").unwrap();
    }
}
