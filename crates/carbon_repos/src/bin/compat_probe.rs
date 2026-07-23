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

use carbon_repos::checker::{
    check_classification, check_insert_datetime_columns, check_manifests, check_module,
    check_nullability, check_pool_routing, check_query_plans,
};
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
    // Every rule shaped `(conn, queries) -> Vec<String>` runs here — the full
    // set `checker.rs` exports for validating registered queries against a
    // live schema (`tests/module_registration.rs` mechanically asserts this
    // list stays complete against checker.rs's exports). `check_handwritten_sql`
    // is deliberately excluded: it lints source files for un-registered SQL,
    // not registered queries against a schema, so it does not fit this probe's
    // "does this query still work against this database" purpose.
    violations.extend(check_module(&conn, &queries));
    violations.extend(check_manifests(&conn, &queries));
    violations.extend(check_nullability(&conn, &queries));
    violations.extend(check_query_plans(&conn, &queries));
    violations.extend(check_classification(&conn, &queries));
    violations.extend(check_insert_datetime_columns(&conn, &queries));
    // Schema-independent (no `Connection` needed), so it isn't part of the
    // mechanically-tracked set above, but it's cheap and worth running here
    // too: a shape/verb pool-routing mismatch is exactly the kind of static
    // fact an old binary's checker should still catch against a newer schema.
    violations.extend(check_pool_routing(&queries));

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

/// Every registered `QueryCheck` across every repo module — delegates to
/// `carbon_repos::repos::all_queries()`, the single shared source of truth
/// also used by the in-process checker tests, so the probe covers the
/// identical query set without hand-maintaining a second copy of the module
/// list (spec L9).
fn all_registered_queries() -> Vec<QueryCheck> {
    carbon_repos::repos::all_queries()
}
