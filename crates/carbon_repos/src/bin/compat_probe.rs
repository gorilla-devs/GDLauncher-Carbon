//! `compat_probe` — the cross-version harness's entrypoint into *this* binary's
//! compatibility machinery (spec §12 T7/T8).
//!
//! The cross-version tests (`tests/cross_version.rs`) drive an *old* checkout's
//! binary against a database written by a *new* checkout. Because this release
//! is the permanent compatibility floor, "the old binary" a future release
//! tests against is a checkout of this very code — so the entrypoints that
//! future harness needs must ship here, now. This bin is that stable surface:
//!
//! - `compat_probe open <db_path>` runs the bidirectional runner
//!   ([`carbon_repos::compat::MigrationSet::open`]) against `<db_path>` with this
//!   binary's own migration list, printing the verdict and exiting non-zero on
//!   any refusal. This is T8: the *old* release's runner performing a down-run
//!   from a database a *newer* release created.
//! - `compat_probe check <db_path>` opens `<db_path>` and runs the full query
//!   checker (structural checks + lints) for every registered query against the
//!   live on-disk schema, exiting non-zero on any violation. This is T7: the
//!   *old* release's own checker verifying its queries still compile against a
//!   *newer* release's overlaid schema.
//!
//! Both subcommands print a machine-greppable `PROBE:<RESULT>` line so the
//! harness can assert on the outcome without depending on exact prose.

use carbon_repos::checker::{check_manifests, check_module, check_nullability, check_query_plans};
use carbon_repos::compat::{OpenVerdict, RefusalKind};
use carbon_repos::registry::QueryCheck;
use rusqlite::Connection;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, db_path) = match (args.first(), args.get(1)) {
        (Some(cmd), Some(path)) => (cmd.as_str(), Path::new(path.as_str())),
        _ => {
            eprintln!("usage: compat_probe <open|check> <db_path>");
            return ExitCode::FAILURE;
        }
    };
    match cmd {
        "open" => probe_open(db_path),
        "check" => probe_check(db_path),
        other => {
            eprintln!("unknown subcommand `{other}` (expected `open` or `check`)");
            ExitCode::FAILURE
        }
    }
}

/// Runs the bidirectional runner against `db_path` and reports the verdict.
fn probe_open(db_path: &Path) -> ExitCode {
    let mut conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            println!("PROBE:OPEN_ERROR|{e}");
            return ExitCode::FAILURE;
        }
    };
    let (set, _count) = carbon_repos::get_migrations();
    match set.open(&mut conn, db_path) {
        Ok(OpenVerdict::Proceed) => {
            println!("PROBE:PROCEED");
            ExitCode::SUCCESS
        }
        Ok(OpenVerdict::Downgraded) => {
            println!("PROBE:DOWNGRADED");
            ExitCode::SUCCESS
        }
        Ok(OpenVerdict::Refuse(kind)) => {
            match kind {
                RefusalKind::BackwardsMigration => println!("PROBE:REFUSE|BACKWARDS_MIGRATION"),
                RefusalKind::Diverged { version } => println!("PROBE:REFUSE|DIVERGED|{version}"),
                RefusalKind::DowngradeFailed { snapshot_path } => println!(
                    "PROBE:REFUSE|DOWNGRADE_FAILED|{}",
                    snapshot_path
                        .as_deref()
                        .map(Path::display)
                        .map(|d| d.to_string())
                        .unwrap_or_default()
                ),
            }
            ExitCode::FAILURE
        }
        Err(e) => {
            println!("PROBE:OPEN_ERROR|{e}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the full query checker against the live on-disk schema at `db_path`.
fn probe_check(db_path: &Path) -> ExitCode {
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            println!("PROBE:OPEN_ERROR|{e}");
            return ExitCode::FAILURE;
        }
    };
    let queries = all_registered_queries();
    let mut violations = Vec::new();
    violations.extend(check_module(&conn, &queries));
    violations.extend(check_manifests(&conn, &queries));
    violations.extend(check_nullability(&conn, &queries));
    violations.extend(check_query_plans(&conn, &queries));

    if violations.is_empty() {
        println!("PROBE:CHECK_OK|{}", queries.len());
        ExitCode::SUCCESS
    } else {
        for v in &violations {
            eprintln!("  {v}");
        }
        println!("PROBE:CHECK_VIOLATIONS|{}", violations.len());
        ExitCode::FAILURE
    }
}

/// Every registered `QueryCheck` across every repo module — the same aggregation
/// the in-process checker tests use, so the probe covers the identical query set.
fn all_registered_queries() -> Vec<QueryCheck> {
    use carbon_repos::repos;
    let mut all: Vec<QueryCheck> = Vec::new();
    all.extend(repos::java::all_queries());
    all.extend(repos::app_configuration::all_queries());
    all.extend(repos::frontend_preference::all_queries());
    all.extend(repos::http_cache::all_queries());
    all.extend(repos::account::all_queries());
    all.extend(repos::skin::all_queries());
    all.extend(repos::active_downloads::all_queries());
    all.extend(repos::instance::all_queries());
    all.extend(repos::server::all_queries());
    all.extend(repos::version_meta::all_queries());
    all.extend(repos::mod_file_cache::all_queries());
    all.extend(repos::mod_metadata::all_queries());
    all.extend(repos::modpack_cache::all_queries());
    all
}
