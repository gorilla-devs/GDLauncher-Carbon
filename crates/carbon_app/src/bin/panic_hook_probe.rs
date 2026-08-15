//! A dedicated binary that reproduces, on a real process's real main
//! thread, the panic shapes `crates/carbon_app/src/main.rs` and its
//! `tokio` tasks can hit: log a line, panic, then either let the process
//! die uncaught or catch the panic and keep running.
//!
//! This exists only for `logger::test::panic_hook_flushes_pending_log_line_before_unwind`
//! and `logger::test::panic_hook_rearms_file_logger_after_a_survived_panic`
//! (`../logger.rs`), which shell out to it. A panic inside a `#[test]` fires
//! on a libtest-spawned worker thread, never the process's own main thread —
//! the harness's own bookkeeping between that panic and the eventual
//! `process::exit` gives the async log writer ample time to flush on its
//! own, hook or no hook, so a test that panics in-process cannot actually
//! tell `install_panic_hook` apart from its absence. Only a panic on a
//! genuine main thread, with nothing else running, reproduces the races
//! `install_panic_hook` exists to close.
//!
//! Takes an optional second argument selecting which shape of panic to
//! reproduce:
//! - `fatal` (default): panics on the main thread and lets the panic
//!   propagate all the way up, uncaught — the process exits non-zero. This
//!   is the fatal-DB-error / `axum::serve(...).unwrap()` shape.
//! - `survivable`: panics on the main thread but immediately catches it
//!   with `catch_unwind`, logs one more line, and exits cleanly (code 0).
//!   This is the shape a `tokio::spawn`ed task's panic produces — the
//!   runtime surfaces it to the spawner as a `JoinError` rather than
//!   propagating it to `main`, so the process lives on. The panic hook
//!   still fires either way; only what happens after the panic differs.
//! - `format_panic`: panics *while formatting a logged event* — a field
//!   logged with `%` (`Display`) whose `Display::fmt` itself panics — then
//!   catches it and keeps going, same as `survivable`. This is the shape
//!   that broke an earlier version of `logger.rs` built on
//!   `tracing_subscriber::reload::Layer`: that layer's `on_event` held an
//!   internal read lock across the *entire* wrapped layer's `on_event`
//!   call, including formatting, so a panic here reached the panic hook
//!   with that lock still held on this thread — and the hook's own
//!   `reload::Handle::reload` call needed a write lock on the very same
//!   lock. Self-deadlock. See `logger::SlotWriter`'s doc comment for the
//!   full mechanism and why the current design (a permanent layer whose
//!   writer is read from a plain `Mutex` only *after* formatting
//!   completes) can't hit it.
//!
//! `logger.rs` is included by path rather than shared through a library
//! crate — `carbon_app` has none, only this binary and the main one — so
//! this runs the exact same `setup_logger`/`install_panic_hook`
//! implementation the real launcher does, not a reimplementation of it.
//! `cleanup_old_logs` comes along with the include but is unused here,
//! hence the blanket allow rather than picking the module apart.
#[path = "../logger.rs"]
#[allow(dead_code)]
mod logger;

/// Read by the wrapper tests after this process exits, to prove
/// `install_panic_hook` (or its absence) actually decided whether this line
/// reached disk.
const MARKER: &str = "PANIC_HOOK_PROBE_MARKER_9d3fa1c4";

/// Logged after the deliberate panic in `survivable` mode. Only reaches
/// disk if `install_panic_hook` re-armed file logging rather than leaving
/// it permanently retired after the first, survived panic.
const POST_PANIC_MARKER: &str = "PANIC_HOOK_PROBE_POST_PANIC_MARKER_7f0eaa2c";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let log_dir = args
        .get(1)
        .expect("usage: panic_hook_probe <log_dir> [fatal|survivable]");
    let mode = args.get(2).map(String::as_str).unwrap_or("fatal");

    tokio::runtime::Runtime::new()
        .expect("failed to build a tokio runtime")
        .block_on(logger::setup_logger(std::path::Path::new(log_dir)));

    logger::install_panic_hook();

    match mode {
        "fatal" => {
            tracing::error!("{}", MARKER);
            panic!("panic_hook_probe: deliberate panic on the real main thread");
        }
        "survivable" => {
            tracing::error!("{}", MARKER);

            // The panic hook runs here exactly as it would for an uncaught
            // panic — `catch_unwind` only stops the unwind from reaching
            // `main`, it doesn't suppress the hook, which fires at the
            // `panic!` call site before any unwinding starts.
            let result = std::panic::catch_unwind(|| {
                panic!("panic_hook_probe: deliberate survivable panic on the real main thread");
            });
            assert!(
                result.is_err(),
                "the deliberate panic should have been caught"
            );

            tracing::info!("{}", POST_PANIC_MARKER);

            // Flush and exit explicitly rather than letting `main` return
            // normally: `LOG_STATE` is a process-lifetime static, and
            // statics are never dropped on a normal return from `main` any
            // more than they are on `std::process::exit` (see
            // `logger::flush_and_exit`'s own doc comment) — without this,
            // the line above could still be sitting unflushed in the
            // non-blocking writer's channel when the wrapper test reads the
            // file, which would be flakiness this probe exists to avoid,
            // not exhibit.
            logger::flush_and_exit(0);
        }
        "format_panic" => {
            tracing::error!("{}", MARKER);

            struct PanicsOnDisplay;
            impl std::fmt::Display for PanicsOnDisplay {
                fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    panic!(
                        "panic_hook_probe: deliberate panic from inside a logged field's \
                         Display impl, invoked while tracing_subscriber's fmt layer is \
                         formatting the event"
                    );
                }
            }

            // `tracing::error!` dispatches synchronously: the macro call,
            // subscriber dispatch, `fmt::Layer::on_event`, and
            // `format_event` (which is what actually calls
            // `PanicsOnDisplay::fmt` below, via the `%bad` field) are all
            // one call stack, so the panic happens *inside* this
            // `tracing::error!` invocation, before `make_writer` would ever
            // be reached for this event.
            let result = std::panic::catch_unwind(|| {
                tracing::error!(bad = %PanicsOnDisplay, "this field's Display panics while formatting");
            });
            assert!(
                result.is_err(),
                "the deliberate Display-impl panic should have been caught"
            );

            tracing::info!("{}", POST_PANIC_MARKER);
            logger::flush_and_exit(0);
        }
        other => panic!("panic_hook_probe: unknown mode {other:?}"),
    }
}
