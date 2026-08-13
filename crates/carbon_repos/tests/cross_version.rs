//! Cross-version harness with real old binaries.
//!
//! T7 and T8 verify compatibility against an *actual previous release binary*,
//! not a simulation:
//!
//! - **T7 (overlay):** the previous release's own query checker must still
//!   compile every one of its registered queries against a schema written by
//!   *this* (newer) release. If a new migration were anything other than a
//!   clean additive overlay, the old checker would fail to prepare a query.
//! - **T8 (down-run):** the previous release's runner, opening a database this
//!   (newer) release created, must step the schema back down to its own version
//!   through the stored downs — or cleanly overlay when every extra migration is
//!   additive — never crash or corrupt.
//!
//! This is impossible to run in CI *before the floor release exists*: there is
//! no previous post-floor tag to build. Until one does, the simulation-based
//! equivalents in `tests/compat_runner.rs` (an "old binary" = the real list, a
//! "new binary" = that list plus synthetic migrations) stand in, and this
//! harness is a no-op unless explicitly activated.
//!
//! ## Activation (post-release)
//!
//! Once a previous stable tag ships (it will carry the `compat_probe` bin, which
//! is part of the floor), run:
//!
//! ```sh
//! # 1. Check out the previous stable tag into a sibling directory.
//! git worktree add /tmp/gdl-prev <previous-stable-tag>
//! # 2. Point the harness at that checkout's repo root and run these tests.
//! GDL_OLD_CHECKOUT=/tmp/gdl-prev cargo test -p carbon_repos --test cross_version
//! ```
//!
//! The harness builds and runs `<GDL_OLD_CHECKOUT>/crates/carbon_repos`'s
//! `compat_probe` binary (spec: that bin is the stable cross-version entrypoint)
//! against a database this checkout creates. With `GDL_OLD_CHECKOUT` unset the
//! tests print the activation steps and pass, so normal CI stays green.
//!
//! The post-release CI wiring (a job that checks out the prior tag, sets the env
//! var, and runs this suite) is tracked in `ROADMAP.md`.

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The old checkout to test against, or `None` (unset) → the tests skip with
/// activation instructions and CI stays green.
fn old_checkout() -> Option<PathBuf> {
    match std::env::var("GDL_OLD_CHECKOUT") {
        Ok(p) if !p.trim().is_empty() => Some(PathBuf::from(p)),
        _ => None,
    }
}

/// Prints the activation instructions and returns `true` when the harness is not
/// active (env unset) so the caller can early-return a passing no-op.
fn skip_if_inactive(test: &str) -> Option<PathBuf> {
    match old_checkout() {
        Some(root) => Some(root),
        None => {
            println!(
                "[{test}] skipped: GDL_OLD_CHECKOUT is unset.\n\
                 This cross-version test needs a previous stable release checkout to run against,\n\
                 which does not exist until this floor release ships. Until then the simulation\n\
                 equivalents in tests/compat_runner.rs provide the coverage. To activate after a\n\
                 post-floor tag exists:\n\
                   git worktree add /tmp/gdl-prev <previous-stable-tag>\n\
                   GDL_OLD_CHECKOUT=/tmp/gdl-prev cargo test -p carbon_repos --test cross_version"
            );
            None
        }
    }
}

/// The `Cargo.toml` of the old checkout's `carbon_repos` crate. Accepts either a
/// repo-root checkout (`<root>/crates/carbon_repos/Cargo.toml`) or a path that
/// already points at the crate directory (`<root>/Cargo.toml`).
fn old_manifest(root: &Path) -> PathBuf {
    let nested = root.join("crates/carbon_repos/Cargo.toml");
    if nested.exists() {
        nested
    } else {
        root.join("Cargo.toml")
    }
}

/// Runs the old checkout's `compat_probe <subcmd> <db>` and returns
/// `(exit-success, combined stdout)`. Panics with a diagnostic if the process
/// could not be launched at all (a set `GDL_OLD_CHECKOUT` is a deliberate
/// activation, so an unrunnable probe is a hard failure, not a silent skip).
fn run_old_probe(root: &Path, subcmd: &str, db: &Path) -> (bool, String) {
    let manifest = old_manifest(root);
    assert!(
        manifest.exists(),
        "GDL_OLD_CHECKOUT={} has no carbon_repos Cargo.toml at {}",
        root.display(),
        manifest.display()
    );
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--manifest-path",
            &manifest.to_string_lossy(),
            "--bin",
            "compat_probe",
            "--",
            subcmd,
        ])
        .arg(db)
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "could not launch the old checkout's compat_probe (cargo run at {}): {e}",
                manifest.display()
            )
        });
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{stdout}{stderr}");
    // A build failure means the old checkout predates the compat_probe bin: it
    // is not a post-floor tag, so cross-version testing is not yet possible.
    assert!(
        !combined.contains("no bin target named `compat_probe`")
            && !combined.contains("error: no bin target"),
        "the old checkout at {} has no `compat_probe` bin — it predates the compatibility \
         floor, so cross-version testing cannot run against it yet:\n{combined}",
        root.display()
    );
    (output.status.success(), combined)
}

/// Builds a fresh, fully HEAD-migrated database at `path` using *this* checkout's
/// migration list — the "new release" database the old binary is pointed at.
fn head_migrated_db(path: &Path) {
    let mut conn = Connection::open(path).unwrap();
    let (set, _count) = carbon_repos::get_migrations();
    set.to_latest(&mut conn).unwrap();
}

#[test]
fn cross_version_overlay_old_checker_accepts_head_schema() {
    // T7: the previous release's checker vs this release's schema.
    let Some(root) = skip_if_inactive("cross_version_overlay_old_checker_accepts_head_schema")
    else {
        return;
    };

    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("gdl_conf.db");
    head_migrated_db(&db);

    let (ok, out) = run_old_probe(&root, "check", &db);
    assert!(
        ok && out.contains("PROBE:CHECK_OK"),
        "the old release's checker must accept this release's schema (clean overlay); \
         probe output:\n{out}"
    );
}

#[test]
fn cross_version_down_run_by_old_runner() {
    // T8: the previous release's runner opening a database this release created.
    let Some(root) = skip_if_inactive("cross_version_down_run_by_old_runner") else {
        return;
    };

    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("gdl_conf.db");
    head_migrated_db(&db);

    // The old runner must open the newer database without a fatal refusal:
    // PROCEED when every extra migration overlays additively, DOWNGRADED when a
    // breaking one forces a verified down-run back to the old version. Both exit
    // zero; a refusal (BACKWARDS_MIGRATION / DIVERGED / DOWNGRADE_FAILED) or a
    // crash is the failure this test catches.
    let (ok, out) = run_old_probe(&root, "open", &db);
    assert!(
        ok && (out.contains("PROBE:PROCEED") || out.contains("PROBE:DOWNGRADED")),
        "the old release's runner must open this release's database (overlay or verified \
         down-run), not refuse or crash; probe output:\n{out}"
    );
}
