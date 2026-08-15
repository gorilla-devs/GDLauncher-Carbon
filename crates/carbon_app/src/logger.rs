use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, TryLockError};

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::{
    EnvFilter, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

/// Hard cap for a single log file (launcher session logs and per-instance game logs).
/// Without it a log-spamming game can grow one session file past 100 GB; once the cap
/// is reached, further output is dropped after a single truncation notice.
pub const MAX_LOG_FILE_SIZE: u64 = 256 * 1024 * 1024;

pub const LOG_TRUNCATION_NOTICE: &[u8] = b"\n[log truncated: file size cap reached]\n";

/// `Write` wrapper that drops output past [`MAX_LOG_FILE_SIZE`].
#[cfg_attr(debug_assertions, allow(dead_code))]
struct SizeCappedWriter<W: std::io::Write> {
    inner: W,
    written: u64,
    truncated: bool,
}

impl<W: std::io::Write> SizeCappedWriter<W> {
    /// `already_written` seeds the cap counter from bytes the file already
    /// holds before this writer instance's first call. This matters because
    /// [`rearm_file_logger_after_panic`] reopens (appends to, never
    /// truncates — see `build_file_writer`) the very same on-disk file
    /// across a panic: starting a fresh writer generation's counter at `0`
    /// against a file that already holds content would let
    /// [`MAX_LOG_FILE_SIZE`] be exceeded by however much the file held
    /// going in. The cap bounds one log *file*, across every writer
    /// generation that ever reopens it — not one writer *instance*. A
    /// brand-new file passes `0` (its own size before any writer touches
    /// it).
    #[cfg_attr(debug_assertions, allow(dead_code))]
    fn new(inner: W, already_written: u64) -> Self {
        Self {
            inner,
            written: already_written,
            truncated: already_written >= MAX_LOG_FILE_SIZE,
        }
    }
}

impl<W: std::io::Write> std::io::Write for SizeCappedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.truncated {
            // Report the bytes as written so tracing doesn't treat this as an error.
            return Ok(buf.len());
        }

        if self.written + buf.len() as u64 > MAX_LOG_FILE_SIZE {
            self.truncated = true;
            let _ = self.inner.write_all(LOG_TRUNCATION_NOTICE);
            let _ = self.inner.flush();
            return Ok(buf.len());
        }

        let n = self.inner.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn generate_logs_filters() -> String {
    #[cfg(debug_assertions)]
    let app_level = "carbon_app=trace";
    #[cfg(not(debug_assertions))]
    let app_level = "carbon_app=debug";

    let filters = &[
        "debug",
        app_level,
        "hyper::client::pool=warn",
        "reqwest::connect=warn",
        "hyper::proto::h1::conn=warn",
        "hyper::proto::h1::io=warn",
        "hyper::proto::h1::decode=warn",
        "hyper_util::client::legacy::pool=warn",
        "hyper_util::client::legacy::connect::http=warn",
        "hyper_util::client::legacy::connect::dns=warn",
        "hyper_util::client::legacy::client=warn",
        "reqwest::async_impl::client=warn",
        "hyper::client::connect::http=warn",
        "hyper::client::connect::dns=warn",
        "rustls::client::hs=warn",
        "rustls::client::tls13=warn",
        "h2::client=warn",
        "rustls::client::common=warn",
        "h2::codec::framed_read=warn",
        "h2::codec::framed_write=warn",
        "h2::proto::settings=warn",
        "tungstenite::protocol=warn",
    ];

    filters.to_vec().join(",")
}

/// Cleanup old log files, keeping only the most recent `keep_count` files.
/// Reused by the cache-cleanup dialog so the launcher's "don't blanket-
/// wipe logs" policy is enforced from a single place.
pub fn cleanup_old_logs(logs_path: &Path, keep_count: usize) {
    let Ok(read_dir) = std::fs::read_dir(logs_path) else {
        return;
    };

    let mut entries: Vec<_> = read_dir
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "log"))
        .collect();

    // Sort by modified time, newest first
    entries.sort_by(|a, b| {
        let time_a = a.metadata().ok().and_then(|m| m.modified().ok());
        let time_b = b.metadata().ok().and_then(|m| m.modified().ok());
        time_b.cmp(&time_a)
    });

    // Delete all but the newest `keep_count` files
    for entry in entries.into_iter().skip(keep_count) {
        if let Err(e) = std::fs::remove_file(entry.path()) {
            eprintln!("Failed to delete old log file {:?}: {}", entry.path(), e);
        }
    }
}

/// Owns the release build's live file-log writer and its `WorkerGuard`
/// together, as one unit, for the rest of the process's life. The two are
/// always updated in the same instant (see
/// [`rearm_file_logger_after_panic`]'s single critical section) so nothing
/// can ever observe a writer with no guard behind it (meaning nothing owns
/// its worker thread for a final blocking flush) or a guard with no writer
/// in front of it (meaning [`SlotWriter`] can't reach the file it's
/// supposed to be serving).
///
/// `WorkerGuard::drop` is what actually blocks until the non-blocking
/// writer's worker thread has flushed pending lines to disk; a guard that
/// only lived in a local at the `main()` call site would never run that
/// `Drop` on a `std::process::exit` call elsewhere in the process, since
/// `exit` terminates immediately without unwinding. Keeping the only
/// owning reference in a process-lifetime static instead lets
/// [`flush_and_exit`] take it and drop it on demand from anywhere in the
/// crate.
///
/// Stays `None` in debug builds, where `setup_logger` never installs a
/// file writer, and is briefly `None` in a release build between the
/// flush and rebuild halves of [`rearm_file_logger_after_panic`] — during
/// that window [`SlotWriter::make_writer`] returns a no-op sink instead of
/// blocking or reaching a half-built writer.
static LOG_STATE: Mutex<Option<(NonBlocking, WorkerGuard)>> = Mutex::new(None);

/// The `MakeWriter` installed on the release build's file-log `fmt::Layer`.
/// The layer itself is registered exactly once, in `setup_logger`, and is
/// never rebuilt or reloaded; what changes across a panic is only which
/// `NonBlocking` writer this returns, read fresh out of [`LOG_STATE`] on
/// every event.
///
/// This exists instead of wrapping the whole `fmt::Layer` in a
/// `tracing_subscriber::reload::Layer` (the more obvious way to make a
/// layer's writer swappable) because `reload::Layer::on_event` holds its
/// own internal lock — a *read* lock, taken to reach the wrapped layer —
/// for the entire duration of the wrapped layer's `on_event` call, which
/// includes formatting the event, i.e. running every logged field's
/// `Display`/`Debug` impl. A panic raised by one of those impls reaches
/// this crate's panic hook with that read lock still held on this very
/// thread (a panic hook runs before any unwinding starts, so nothing on
/// the stack above the panic site has had a chance to drop yet); the hook
/// rebuilding and reloading the layer would then need
/// `reload::Handle::reload`'s *write* lock on that same `RwLock`, from the
/// thread that already holds a read lock on it — a real, reproducible
/// self-deadlock, and (since `std`'s `RwLock` queues new readers behind an
/// already-waiting writer) one that also blocks every other thread trying
/// to log at the same time, not just this one.
///
/// Keeping the swappable state behind our own lock, entirely outside
/// `on_event`'s call graph, avoids the hazard by construction:
/// `fmt::Layer::on_event` (`tracing_subscriber::fmt::fmt_layer`) calls
/// `make_writer` only *after* it has already formatted the event into a
/// private buffer, so `make_writer` below never runs while any
/// user-controlled formatting code is on the stack — there is nothing here
/// for a formatting panic to hold a lock across.
struct SlotWriter;

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SlotWriter {
    type Writer = SlotOrSink;

    fn make_writer(&'a self) -> Self::Writer {
        // The critical section here is exactly "clone an
        // `Option<NonBlocking>`" — no user code, nothing that can itself
        // panic — so there is nothing for a reentrant or concurrent caller
        // to deadlock against. `try_lock` still guards it because a
        // logging call (including one made from inside a panic hook mid
        // rebuild) must never block waiting for a lock, full stop.
        match LOG_STATE.try_lock() {
            Ok(state) => match &*state {
                Some((writer, _)) => SlotOrSink::Slot(writer.clone()),
                None => SlotOrSink::Sink(std::io::sink()),
            },
            Err(_) => SlotOrSink::Sink(std::io::sink()),
        }
    }
}

/// Either the release build's real file writer or a no-op sink, returned by
/// [`SlotWriter::make_writer`] depending on whether [`LOG_STATE`] currently
/// holds a writer and whether its lock was free. A logging call that lands
/// on the sink branch (writer slot empty, or its lock contended by a panic
/// hook mid-rebuild) drops that one event's output — never blocks, never
/// panics, and never queues up to retry once the writer comes back.
enum SlotOrSink {
    Slot(NonBlocking),
    Sink(std::io::Sink),
}

impl std::io::Write for SlotOrSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            SlotOrSink::Slot(writer) => writer.write(buf),
            SlotOrSink::Sink(sink) => sink.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            SlotOrSink::Slot(writer) => writer.flush(),
            SlotOrSink::Sink(sink) => sink.flush(),
        }
    }
}

/// The `(directory, file_name)` pair the release build's file layer targets.
/// Recorded once by `setup_logger` so [`rearm_file_logger_after_panic`] can
/// rebuild an appender pointing at the exact same file it just flushed:
/// `tracing_appender`'s rolling appenders open with
/// `OpenOptions::append(true)` (never truncate), so reopening this pair
/// continues the same log file rather than starting a new one — the panic
/// and everything logged before it stay in one file alongside whatever is
/// logged after. Unlike [`LOG_STATE`], this never changes after
/// `setup_logger` sets it: every rebuild targets the same pair.
static LOG_FILE_LOCATION: OnceLock<(PathBuf, String)> = OnceLock::new();

/// Builds the release build's file-log writer plus its `WorkerGuard`,
/// always targeting the exact `(logs_path, file_name)` pair passed in. Used
/// both by `setup_logger` on startup and by
/// [`rearm_file_logger_after_panic`] to rebuild a fresh writer after
/// flushing.
///
/// Goes through `RollingFileAppender::builder()` rather than the
/// `tracing_appender::rolling::never` convenience function specifically so
/// a filesystem error surfaces as a `Result` instead of a panic —
/// `rolling::never` calls `RollingFileAppender::new`, which `.expect()`s
/// internally on the same error this function instead propagates. Called
/// from a panic hook, that internal `.expect()` would turn a filesystem
/// error (an unwritable log directory, say) into a second panic — fatal on
/// its own, since panicking from inside a panic hook aborts the process —
/// raised from inside the panic hook itself, which is exactly what
/// [`rearm_file_logger_after_panic`] must never do.
///
/// Seeds [`SizeCappedWriter`]'s byte counter from the target file's actual
/// on-disk size (`0` if it doesn't exist yet) rather than always starting
/// at `0`, so [`MAX_LOG_FILE_SIZE`] bounds one log file across every
/// generation of writer that ever reopens it, not just whatever one writer
/// instance wrote since the last panic. `RollingFileAppender::builder`'s
/// `build` opens (and creates, if needed) the file eagerly, so by the time
/// this reads the file's metadata the size it observes already reflects
/// this call, not a stale pre-open state.
fn build_file_writer(
    logs_path: &Path,
    file_name: &str,
) -> Result<(NonBlocking, WorkerGuard), tracing_appender::rolling::InitError> {
    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::NEVER)
        .filename_prefix(file_name.to_string())
        .build(logs_path)?;

    let already_written = std::fs::metadata(logs_path.join(file_name))
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    Ok(tracing_appender::non_blocking(SizeCappedWriter::new(
        file_appender,
        already_written,
    )))
}

/// Flushes the release-build file log (if any) and exits the process.
///
/// A bare `std::process::exit` skips every destructor, including the
/// `WorkerGuard` that would otherwise block until `tracing-appender`'s
/// background worker thread drains its channel and writes pending lines to
/// disk. On an idle machine that worker thread usually gets scheduled in time
/// anyway, but under CPU contention it does not: a `tracing::error!` call
/// made immediately before a fatal exit can be silently lost, taking with it
/// the one line that names why the process is exiting. Every call site that
/// needs to terminate right after logging a fatal error should exit through
/// here instead of calling `std::process::exit` directly.
pub fn flush_and_exit(code: i32) -> ! {
    // A poisoned mutex (the only lockers are this function and
    // `rearm_file_logger_after_panic`/`SlotWriter::make_writer`, none of
    // which panics while holding it, so this is not expected to actually
    // be poisoned) still yields its inner value rather than turning an
    // already-fatal exit into a second panic.
    let state = LOG_STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
    // Dropping the tuple drops its `WorkerGuard` half, which sends the
    // worker thread a shutdown message and blocks (bounded, see
    // `tracing_appender::non_blocking::WorkerGuard`) until it confirms the
    // channel is drained. The `NonBlocking` half's drop is inert.
    drop(state);
    std::process::exit(code);
}

/// Installs a panic hook that flushes the release-build file log (see
/// [`LOG_STATE`]) and re-arms it for further writes (see
/// [`rearm_file_logger_after_panic`]) before delegating to whatever hook was
/// previously installed (the default one prints the panic message and
/// location to stderr).
///
/// Panics unwind rather than abort — no profile in the workspace root
/// `Cargo.toml` sets `panic = "abort"` — and unwinding never runs `Drop` on
/// a `'static`, so without the flush half of this hook a `tracing::error!`
/// line written immediately before a `panic!` (the migration-failure branch
/// in `managers/mod.rs`, or `axum::serve(...).unwrap()`'s implicit one) is
/// only as likely to reach disk as the non-blocking writer's worker thread
/// is to win an unscheduled race against the unwind — exactly the
/// CPU-contention loss [`flush_and_exit`] exists to prevent on the
/// deliberate-exit paths. A panic hook runs synchronously at the `panic!`
/// call site, before the stack starts unwinding, so the flush here is
/// unconditional rather than a race: by the time this function returns, any
/// pending log line is already on disk.
///
/// A panic hook fires for every panic raised in the process, including ones
/// the process goes on to survive — a `catch_unwind` boundary anywhere, or a
/// `tokio::spawn`ed task's panic, which the runtime turns into a `JoinError`
/// for its caller rather than propagating it to `main`. Without the re-arm
/// half of this hook, the flush above would permanently retire the
/// non-blocking writer's worker thread on the *first* such survived panic,
/// silently dropping every `tracing::` call made for the rest of the
/// process's life (see `rearm_file_logger_after_panic`'s doc comment) —
/// including, should the process later crash for real, the log line that
/// would have named why.
///
/// A no-op in debug builds and in any release build reached before
/// `setup_logger` has run: `setup_logger` is the only place that ever
/// populates [`LOG_STATE`] or [`LOG_FILE_LOCATION`], and only from its
/// `#[cfg(not(debug_assertions))]` branch, so
/// `rearm_file_logger_after_panic` finds nothing to flush or re-arm in
/// either case and this falls straight through to the previous hook.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        rearm_file_logger_after_panic();
        previous(info);
    }));
}

/// Flushes the release build's file log by taking and dropping the current
/// `(NonBlocking, WorkerGuard)` pair (see [`LOG_STATE`]), then rebuilds a
/// fresh pair and stores it back, so file logging stays live for the rest
/// of the process's life no matter how many panics it survives. Called
/// from the panic hook installed by [`install_panic_hook`], before that
/// hook delegates to whichever hook ran before it.
///
/// A no-op in debug builds and before `setup_logger` has run: neither ever
/// populates [`LOG_STATE`], so the very first `try_lock` below finds it
/// holding `None` and the function returns right after the (no-op) flush
/// step, since [`LOG_FILE_LOCATION`] is unset either way.
///
/// ## Locking and reentrancy
///
/// [`LOG_STATE`]'s lock is taken twice here, each time with `try_lock`,
/// and released before the next line of code that could itself panic
/// runs — never held across `build_file_writer` (filesystem I/O) in
/// between. A panic hook must never block indefinitely, and
/// `std::sync::Mutex` is not reentrant, so the two things this guards
/// against are:
///
/// - A panic raised by the rebuild itself (in practice the standard
///   library aborts the process outright on a panic raised from inside a
///   panic hook rather than re-running the hook, so this is a defense
///   against that changing, not an observed case). Because the lock is
///   *not* held while the rebuild runs, a reentrant call on this same
///   thread finds the lock free at every point it might try to take it —
///   there is no window in which this thread could contend with itself,
///   deadlock is structurally impossible rather than merely handled.
/// - A genuinely concurrent panic on a *different* thread (or any other
///   logging call — see [`SlotWriter::make_writer`], which races this
///   exact lock on every event) landing inside one of this function's two
///   short critical sections at the same instant. That caller's
///   `try_lock` observes `WouldBlock`, and either skips its own
///   flush/re-arm (this function) or falls back to a no-op sink for that
///   one event (`SlotWriter`) — a missed flush or a dropped line, never a
///   hang.
///
/// The two critical sections are independent — the first takes and drops
/// the current pair (the flush); the second, after the rebuild has already
/// succeeded, stores the new one — and each mutates `LOG_STATE` as a
/// single unit (writer and guard stored together, not through two separate
/// locks). That single-section property is what makes the store race-free:
/// a `try_lock` loss there can only mean "the new pair isn't installed at
/// all", never "the writer is live but nothing owns its guard" or "the
/// guard is stored but `SlotWriter` still can't see a writer" — the two
/// halves can't observably drift apart, so there is nothing to leak or
/// roll back if that second `try_lock` loses its race: the freshly built
/// pair is simply dropped, harmless because neither half was ever
/// published through `LOG_STATE` for `SlotWriter` to have handed out to an
/// in-flight event.
fn rearm_file_logger_after_panic() {
    // Phase 1 — flush: take and drop the current pair, if there is one.
    // `WorkerGuard::drop` (run via the explicit `drop` calls below) is what
    // blocks until the worker thread confirms its channel is drained, so
    // whatever was logged right before this panic is on disk by the time
    // this match returns. The lock is released immediately after.
    //
    // `had_writer` records whether `LOG_STATE` actually held a pair to
    // flush, as opposed to already being `None` — which is the normal case
    // in a debug build (`setup_logger` never installs a file writer there;
    // debug logging goes straight to a stdout layer this whole module
    // never touches) and in a release build reached before `setup_logger`
    // has run. This hook fires on *every* panic the process survives, so a
    // debug build under `pnpm watch:core` hits the "nothing to flush"
    // branch on every caught/`tokio::spawn` panic — that must stay silent,
    // not print a "logging is now dead" line that's false in debug (the
    // stdout layer is untouched by any of this) and misleading in release
    // (nothing was ever armed, so there is nothing to declare dead).
    let had_writer = match LOG_STATE.try_lock() {
        Ok(mut state) => {
            let taken = state.take();
            let had_writer = taken.is_some();
            drop(taken);
            had_writer
        }
        Err(TryLockError::Poisoned(poisoned)) => {
            let taken = poisoned.into_inner().take();
            let had_writer = taken.is_some();
            drop(taken);
            had_writer
        }
        Err(TryLockError::WouldBlock) => {
            eprintln!(
                "carbon_app: panic hook could not acquire the log state lock (held by a \
                 concurrent or reentrant panic) — skipping log flush/re-arm for this panic"
            );
            return;
        }
    };

    let Some((logs_path, file_name)) = LOG_FILE_LOCATION.get() else {
        if had_writer {
            // Genuine anomaly, not the common "never armed" case: there was
            // a live writer in `LOG_STATE` to flush, but no recorded file
            // location to rebuild against. `setup_logger` always sets
            // `LOG_STATE` and then `LOG_FILE_LOCATION` together in its
            // `#[cfg(not(debug_assertions))]` branch, so this should be
            // unreachable in practice — worth surfacing if it ever isn't.
            eprintln!(
                "carbon_app: panic hook flushed the log file but has no recorded log file \
                 location to re-arm; logging is now dead for the rest of this process"
            );
        }
        // Otherwise: nothing was ever armed (see `had_writer`'s doc above).
        // Silently do nothing — there is nothing to flush or re-arm, and
        // saying otherwise on every debug-build panic would be both false
        // and constant noise under `pnpm watch:core`.
        return;
    };

    // Phase 2 — rebuild: no lock held anywhere in this section.
    let new_pair = match build_file_writer(logs_path, file_name) {
        Ok(pair) => pair,
        // Degraded-but-safe: the flush above already happened, so the log
        // line that mattered for this panic is not lost. Re-arming failed
        // (e.g. the log directory became unwritable), so every log line
        // from here on is silently dropped — but returning normally here,
        // rather than propagating this error as a second panic, is what
        // keeps a panic hook from ever aborting the process on its own.
        Err(e) => {
            eprintln!(
                "carbon_app: panic hook flushed the log file but failed to recreate the file \
                 appender ({e}); logging is now dead for the rest of this process"
            );
            return;
        }
    };

    // Phase 3 — store: hand the new pair to `LOG_STATE` as a single unit.
    match LOG_STATE.try_lock() {
        Ok(mut state) => *state = Some(new_pair),
        Err(TryLockError::Poisoned(poisoned)) => *poisoned.into_inner() = Some(new_pair),
        Err(TryLockError::WouldBlock) => {
            eprintln!(
                "carbon_app: panic hook rebuilt the file logger but could not store it (lock \
                 held by a concurrent or reentrant panic); logging stays dead until a later \
                 panic's hook re-arms it"
            );
            // `new_pair` drops here — see this function's doc comment for
            // why that's harmless rather than a repeat of the original bug.
        }
    }
}

/// Installs the tracing subscriber. In release builds this also owns the
/// file writer's `WorkerGuard` for the rest of the process's life (see
/// [`LOG_STATE`]) rather than returning it to the caller, so [`flush_and_exit`]
/// and [`install_panic_hook`] can reach it from any call site without
/// threading it through every function between `main()` and a fatal exit
/// deep in `managers::app`.
pub async fn setup_logger(runtime_path: &Path) {
    let logs_path = runtime_path.join("__gdl_logs__");

    println!("Logs path: {}", logs_path.display());

    if !logs_path.exists() {
        tokio::fs::create_dir_all(&logs_path).await.unwrap();
    }

    // Keep only the last 10 log files. Same retention as the cache-cleanup
    // dialog enforces — recent logs are useful for debugging crashes that
    // happened a few launches ago, so we don't blanket-wipe them.
    cleanup_old_logs(&logs_path, 10);

    let filter = EnvFilter::builder();

    // We need to check if the env is present, because, although
    // `EnvFilter::from_env()` says in it's docs that it will return an error
    // if the env is not set, reading the source of the method reveals this is
    // not true :(
    let filter = if std::env::var("RUST_LOG").is_ok() {
        println!("loaded logger directives from `RUST_LOG` env");

        filter.from_env().expect("logger directives are invalid")
    } else {
        let directives = generate_logs_filters();

        println!(
            "loaded default logger directives, to override, set `RUST_LOG` env var\n\
             RUST_LOG=\"{directives}\""
        );

        filter.parse(directives).unwrap()
    };

    // let processor = tracing_forest::Printer::new()
    //     .formatter(tracing_forest::printer::Pretty)
    //     // .formatter(serde_json::to_string_pretty)
    //     .writer(non_blocking);
    // let layer = tracing_forest::ForestLayer::from(processor);

    #[cfg(debug_assertions)]
    {
        let printer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_ansi(true)
            .pretty()
            .with_thread_names(true);

        tracing_subscriber::registry()
            .with(printer)
            .with(filter)
            .init();
    }
    #[cfg(not(debug_assertions))]
    {
        let file_name = format!("{}.log", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"));

        let (writer, guard) = build_file_writer(&logs_path, &file_name)
            .expect("failed to initialize the release build's file logger");

        let printer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_ansi(true)
            .pretty()
            .with_thread_names(false)
            .with_writer(SlotWriter);

        tracing_subscriber::registry()
            .with(printer)
            .with(filter)
            .init();

        // Populate `LOG_STATE` before the first log call below: `SlotWriter`
        // reads through it for every event, so a log line emitted while it's
        // still `None` would silently land on the no-op sink instead of disk.
        *LOG_STATE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((writer, guard));
        let _ = LOG_FILE_LOCATION.set((logs_path.clone(), file_name));

        tracing::trace!("Logger initialized");
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;

    #[test]
    fn size_capped_writer_stops_at_cap() {
        let mut writer = super::SizeCappedWriter {
            inner: Vec::new(),
            written: super::MAX_LOG_FILE_SIZE - 10,
            truncated: false,
        };

        writer.write_all(b"0123456789").unwrap();
        assert_eq!(writer.inner, b"0123456789");

        // This write crosses the cap: dropped, notice appended, no error reported
        writer.write_all(b"overflow").unwrap();
        assert!(writer.truncated);
        let expected = [b"0123456789" as &[u8], super::LOG_TRUNCATION_NOTICE].concat();
        assert_eq!(writer.inner, expected);

        // Subsequent writes are silently dropped
        writer.write_all(b"more").unwrap();
        assert_eq!(writer.inner, expected);
    }

    #[test]
    fn size_capped_writer_new_seeds_from_already_written() {
        // Simulates `build_file_writer` reopening a file that already holds
        // content from a prior writer generation: the cap must account for
        // those pre-existing bytes, not just what this instance writes.
        let mut writer = super::SizeCappedWriter::new(Vec::new(), super::MAX_LOG_FILE_SIZE - 5);

        writer.write_all(b"1234").unwrap();
        assert!(!writer.truncated, "5 bytes under the cap should still fit");

        writer.write_all(b"overflow").unwrap();
        assert!(
            writer.truncated,
            "the pre-existing bytes plus new writes should cross the cap"
        );
    }

    #[test]
    fn size_capped_writer_new_already_at_cap_starts_truncated() {
        // A file reopened at or past the cap (e.g. a previous generation
        // already appended the truncation notice) must stay truncated
        // immediately, not accept another `MAX_LOG_FILE_SIZE` worth of bytes.
        let writer = super::SizeCappedWriter::new(Vec::new(), super::MAX_LOG_FILE_SIZE);
        assert!(writer.truncated);
    }

    const PANIC_HOOK_PROBE_MARKER: &str = "PANIC_HOOK_PROBE_MARKER_9d3fa1c4";

    /// Resolves the compiled path of the `panic_hook_probe` binary
    /// (`src/bin/panic_hook_probe.rs`) sibling to this test binary.
    ///
    /// `CARGO_BIN_EXE_<name>` is Cargo's own documented way to hand a test
    /// the exact path of another target in the same package, but it is only
    /// populated for integration-test/bench targets — not for a unit test
    /// running inside a `bin` target's own harness, which is what this is.
    /// Falling back to deriving the path from `current_exe()` (this test
    /// binary lives at `target/<profile>/deps/carbon_app-<hash>`; the
    /// unhashed sibling binaries Cargo also places at `target/<profile>/`)
    /// covers exactly that gap.
    fn panic_hook_probe_path() -> std::path::PathBuf {
        if let Ok(path) = std::env::var("CARGO_BIN_EXE_panic_hook_probe") {
            return std::path::PathBuf::from(path);
        }
        let exe = std::env::current_exe().expect("failed to resolve the test binary's own path");
        let profile_dir = exe
            .parent() // .../target/<profile>/deps
            .and_then(|p| p.parent()) // .../target/<profile>
            .expect("test binary path has no target/<profile> ancestor");
        let name = if cfg!(windows) {
            "panic_hook_probe.exe"
        } else {
            "panic_hook_probe"
        };
        profile_dir.join(name)
    }

    /// Confirms `install_panic_hook` actually flushes a log line written
    /// immediately before a panic to disk, rather than losing it the way an
    /// unwind into a `'static` guard (never dropped) would on its own.
    ///
    /// Only meaningful in a release-shaped build: `setup_logger` only
    /// installs the file writer and populates `LOG_STATE` under
    /// `#[cfg(not(debug_assertions))]`; in a debug build tracing goes
    /// straight to a stdout layer with no guard to lose, so there would be
    /// nothing for this test to observe. Run with `cargo test --release` (or
    /// any profile with `debug-assertions = false`) to exercise it; under a
    /// plain debug `cargo test` this file is simply never written and the
    /// test is compiled out.
    ///
    /// Shells out to `panic_hook_probe` rather than panicking inline: a
    /// panic inside this test function fires on a libtest-spawned worker
    /// thread, never this process's own main thread, and the test harness's
    /// own bookkeeping between that panic and the eventual `process::exit`
    /// gives the async log writer ample time to flush on its own — hook or
    /// no hook. A test built that way cannot tell `install_panic_hook` apart
    /// from its absence (confirmed live: disabling the hook and panicking
    /// in-process here still passed 35/35, `taskset`-pinned included). Only
    /// a panic on a genuine main thread, with nothing else running,
    /// reproduces the race this hook exists to close — see
    /// `panic_hook_probe`'s own doc comment.
    #[cfg(not(debug_assertions))]
    #[test]
    fn panic_hook_flushes_pending_log_line_before_unwind() {
        let probe = panic_hook_probe_path();
        assert!(
            probe.exists(),
            "panic_hook_probe binary not found at {probe:?} — \
             `cargo build`/`cargo test` should have produced it alongside carbon_app"
        );

        let tmp = tempfile::tempdir().unwrap();
        let status = std::process::Command::new(&probe)
            .arg(tmp.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("failed to spawn panic_hook_probe");
        assert!(
            !status.success(),
            "panic_hook_probe should have exited non-zero: it panics deliberately"
        );

        let logs_dir = tmp.path().join("__gdl_logs__");
        let newest = std::fs::read_dir(&logs_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .max_by_key(|e| e.metadata().unwrap().modified().unwrap())
            .expect("setup_logger should have created a log file");
        let contents = std::fs::read_to_string(newest.path()).unwrap();
        assert!(
            contents.contains(PANIC_HOOK_PROBE_MARKER),
            "expected the panic hook to have flushed the pending log line to \
             disk before panic_hook_probe exited; log contents:\n{contents}"
        );
    }

    const POST_PANIC_MARKER: &str = "PANIC_HOOK_PROBE_POST_PANIC_MARKER_7f0eaa2c";

    /// Confirms `install_panic_hook` re-arms file logging after a panic the
    /// process survives, rather than leaving every later `tracing::` call
    /// silently dropped for the rest of the process's life.
    ///
    /// A panic hook fires for *every* panic, not just ones that reach the
    /// top of `main` uncaught — a `catch_unwind` boundary (or, in the real
    /// launcher, a `tokio::spawn`ed task's panic surfacing to its caller as
    /// a `JoinError` rather than propagating to `main`) still triggers it.
    /// Taking and dropping the `WorkerGuard` on that first, survived panic
    /// without ever replacing it permanently retires the non-blocking
    /// writer's worker thread: `NonBlocking::write` treats a disconnected
    /// channel as lossy by design (dropped writes are not reported as
    /// errors), so every `tracing::` call made afterward is silently
    /// swallowed. That includes the one line that would matter most — the
    /// next, *fatal* panic's own log line — so the launcher's on-disk crash
    /// log would end at the first benign panic instead of at the actual
    /// crash.
    ///
    /// See `panic_hook_flushes_pending_log_line_before_unwind` above for why
    /// this shells out to `panic_hook_probe` instead of panicking in-process,
    /// and why it only runs in a release-shaped build.
    #[cfg(not(debug_assertions))]
    #[test]
    fn panic_hook_rearms_file_logger_after_a_survived_panic() {
        let probe = panic_hook_probe_path();
        assert!(
            probe.exists(),
            "panic_hook_probe binary not found at {probe:?} — \
             `cargo build`/`cargo test` should have produced it alongside carbon_app"
        );

        let tmp = tempfile::tempdir().unwrap();
        let status = std::process::Command::new(&probe)
            .arg(tmp.path())
            .arg("survivable")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("failed to spawn panic_hook_probe");
        assert!(
            status.success(),
            "panic_hook_probe in survivable mode should have exited 0: it catches its \
             own deliberate panic and continues"
        );

        let logs_dir = tmp.path().join("__gdl_logs__");
        let newest = std::fs::read_dir(&logs_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .max_by_key(|e| e.metadata().unwrap().modified().unwrap())
            .expect("setup_logger should have created a log file");
        let contents = std::fs::read_to_string(newest.path()).unwrap();

        assert!(
            contents.contains(PANIC_HOOK_PROBE_MARKER),
            "expected the pre-panic line to have reached disk; log contents:\n{contents}"
        );
        assert!(
            contents.contains(POST_PANIC_MARKER),
            "expected the panic hook to have re-armed file logging after the survived \
             panic, so the line logged afterward would still reach disk; log \
             contents:\n{contents}"
        );
    }

    /// Confirms `install_panic_hook` survives a panic raised *during event
    /// formatting* — a field's `Display` impl panicking inside
    /// `fmt::Layer::on_event`'s call to `format_event` — without hanging.
    ///
    /// This is the exact shape that deadlocked an earlier version of
    /// `logger.rs` built on `tracing_subscriber::reload::Layer`: that
    /// layer's `on_event` held its own internal read lock across the whole
    /// wrapped layer's `on_event` call (formatting included), so a panic
    /// there reached the panic hook with that lock still held on this
    /// thread, and the hook's `reload::Handle::reload` call then needed a
    /// *write* lock on that same lock — a real, reproducible self-deadlock
    /// (and, since `std::sync::RwLock` queues a waiting writer ahead of new
    /// readers, one that blocks every other logging thread too, not just
    /// this one). See `SlotWriter`'s doc comment for why the current
    /// design — a permanent layer whose writer is read from a plain
    /// `Mutex` only *after* `format_event` returns — can't hit it: nothing
    /// here ever holds a lock across user-controlled formatting code.
    ///
    /// Spawns the probe with a generous but bounded timeout rather than a
    /// bare `.status()` wait: if this ever regresses back to the deadlock,
    /// the process hangs forever, and a test that just blocks on it would
    /// hang the whole suite (and CI) instead of failing.
    #[cfg(not(debug_assertions))]
    #[test]
    fn panic_hook_survives_a_panic_during_event_formatting() {
        let probe = panic_hook_probe_path();
        assert!(
            probe.exists(),
            "panic_hook_probe binary not found at {probe:?} — \
             `cargo build`/`cargo test` should have produced it alongside carbon_app"
        );

        let tmp = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new(&probe)
            .arg(tmp.path())
            .arg("format_panic")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("failed to spawn panic_hook_probe");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let status = loop {
            if let Some(status) = child.try_wait().expect("failed to poll panic_hook_probe") {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "panic_hook_probe in format_panic mode did not exit within 15s — this is \
                     the self-deadlock this test exists to catch, not a slow machine"
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        assert!(
            status.success(),
            "panic_hook_probe in format_panic mode should have exited 0: it catches its \
             own deliberate panic and continues"
        );

        let logs_dir = tmp.path().join("__gdl_logs__");
        let newest = std::fs::read_dir(&logs_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .max_by_key(|e| e.metadata().unwrap().modified().unwrap())
            .expect("setup_logger should have created a log file");
        let contents = std::fs::read_to_string(newest.path()).unwrap();

        assert!(
            contents.contains(PANIC_HOOK_PROBE_MARKER),
            "expected the pre-panic line to have reached disk; log contents:\n{contents}"
        );
        assert!(
            contents.contains(POST_PANIC_MARKER),
            "expected the panic hook to have re-armed file logging after the format-time \
             panic, so the line logged afterward would still reach disk; log \
             contents:\n{contents}"
        );
    }
}
