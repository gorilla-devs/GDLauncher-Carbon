//! A dedicated binary that reproduces, on a real process's real main
//! thread, the exact shape `crates/carbon_app/src/main.rs` hits on a fatal
//! DB error (`managers/mod.rs`'s migration-failure `panic!`) or an
//! `axum::serve(...).unwrap()` panic: log a line, panic, let the process
//! exit through the standard library's own uncaught-panic handling.
//!
//! This exists only for `logger::test::panic_hook_flushes_pending_log_line_before_unwind`
//! (`../logger.rs`), which shells out to it. A panic inside a `#[test]` fires
//! on a libtest-spawned worker thread, never the process's own main thread —
//! the harness's own bookkeeping between that panic and the eventual
//! `process::exit` gives the async log writer ample time to flush on its
//! own, hook or no hook, so a test that panics in-process cannot actually
//! tell `install_panic_hook` apart from its absence. Only a panic on a
//! genuine main thread, with nothing else running, reproduces the race
//! `install_panic_hook` exists to close.
//!
//! `logger.rs` is included by path rather than shared through a library
//! crate — `carbon_app` has none, only this binary and the main one — so
//! this runs the exact same `setup_logger`/`install_panic_hook`
//! implementation the real launcher does, not a reimplementation of it.
//! `flush_and_exit` and `cleanup_old_logs` come along with the include but
//! are unused here (this probe only ever exits via a real panic), hence the
//! blanket allow rather than picking the module apart.
#[path = "../logger.rs"]
#[allow(dead_code)]
mod logger;

/// Read by the wrapper test after this process exits, to prove
/// `install_panic_hook` (or its absence) actually decided whether this line
/// reached disk.
const MARKER: &str = "PANIC_HOOK_PROBE_MARKER_9d3fa1c4";

fn main() {
    let log_dir = std::env::args()
        .nth(1)
        .expect("usage: panic_hook_probe <log_dir>");

    tokio::runtime::Runtime::new()
        .expect("failed to build a tokio runtime")
        .block_on(logger::setup_logger(std::path::Path::new(&log_dir)));

    logger::install_panic_hook();

    tracing::error!("{}", MARKER);
    panic!("panic_hook_probe: deliberate panic on the real main thread");
}
