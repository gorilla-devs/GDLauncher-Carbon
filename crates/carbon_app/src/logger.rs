use std::path::Path;
use std::sync::{Mutex, OnceLock};

use tracing_appender::non_blocking::WorkerGuard;
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
    #[cfg_attr(debug_assertions, allow(dead_code))]
    fn new(inner: W) -> Self {
        Self {
            inner,
            written: 0,
            truncated: false,
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

/// Owns the release-build file log's `WorkerGuard` for the rest of the
/// process's life. `WorkerGuard::drop` is what actually blocks until the
/// non-blocking writer's worker thread has flushed pending lines to disk; a
/// guard that only lived in a local at the `main()` call site would never run
/// that `Drop` on a `std::process::exit` call elsewhere in the process, since
/// `exit` terminates immediately without unwinding. Keeping the only owning
/// reference in a process-lifetime static instead lets [`flush_and_exit`] take
/// it and drop it on demand from anywhere in the crate. Stays `None` in debug
/// builds, where `setup_logger` never installs a file writer.
static LOG_GUARD: OnceLock<Mutex<Option<WorkerGuard>>> = OnceLock::new();

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
    if let Some(lock) = LOG_GUARD.get() {
        // Dropping the guard sends the worker thread a shutdown message and
        // blocks (bounded, see `tracing_appender::non_blocking::WorkerGuard`)
        // until it confirms the channel is drained. A poisoned mutex (the
        // only locker is this function and `install_panic_hook`'s hook,
        // neither of which panics while holding it, so this is not expected
        // to actually be poisoned) still yields its inner guard rather than
        // turning an already-fatal exit into a second panic.
        drop(
            lock.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .take(),
        );
    }
    std::process::exit(code);
}

/// Installs a panic hook that flushes the release-build file log (see
/// [`LOG_GUARD`]) before delegating to whatever hook was previously
/// installed (the default one prints the panic message and location to
/// stderr).
///
/// Panics unwind rather than abort — no profile in the workspace root
/// `Cargo.toml` sets `panic = "abort"` — and unwinding never runs `Drop` on
/// a `'static`, so without this hook a `tracing::error!` line written
/// immediately before a `panic!` (the migration-failure branch in
/// `managers/mod.rs`, or `axum::serve(...).unwrap()`'s implicit one) is only
/// as likely to reach disk as the non-blocking writer's worker thread is to
/// win an unscheduled race against the unwind — exactly the CPU-contention
/// loss [`flush_and_exit`] exists to prevent on the deliberate-exit paths.
/// A panic hook runs synchronously at the `panic!` call site, before the
/// stack starts unwinding, so the flush here is unconditional rather than a
/// race: by the time this function returns, any pending log line is already
/// on disk.
///
/// A no-op in debug builds and in any release build reached before
/// `setup_logger` has run: `LOG_GUARD` is only ever populated by the
/// `#[cfg(not(debug_assertions))]` branch of `setup_logger`, so `LOG_GUARD.
/// get()` is `None` in both cases and this falls straight through to the
/// previous hook.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(lock) = LOG_GUARD.get() {
            let mut guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            drop(guard.take());
        }
        previous(info);
    }));
}

/// Installs the tracing subscriber. In release builds this also owns the
/// file writer's `WorkerGuard` for the rest of the process's life (see
/// [`LOG_GUARD`]) rather than returning it to the caller, so [`flush_and_exit`]
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
        let file_name = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let file_appender =
            tracing_appender::rolling::never(logs_path, format!("{}.log", file_name));

        let (non_blocking, guard) =
            tracing_appender::non_blocking(SizeCappedWriter::new(file_appender));

        let printer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_ansi(true)
            .pretty()
            .with_thread_names(false);

        tracing_subscriber::registry()
            .with(printer.with_writer(non_blocking))
            .with(filter)
            .init();

        tracing::trace!("Logger initialized");
        let _ = LOG_GUARD.set(Mutex::new(Some(guard)));
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
    /// installs the file writer and populates `LOG_GUARD` under
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
}
