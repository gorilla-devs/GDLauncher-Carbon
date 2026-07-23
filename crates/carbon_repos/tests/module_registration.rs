//! Mechanical completeness fences for cross-module query registration (spec
//! L9).
//!
//! Two independent places used to hand-maintain their own copy of the 13-repo-
//! module list (`tests/query_checker.rs` and `src/bin/compat_probe.rs`), and a
//! new repo module added to `src/repos/mod.rs` without also updating both
//! copies would silently escape the checker — nothing failed, the module's
//! queries were just never validated. Both call sites now delegate to
//! `carbon_repos::repos::all_queries()`, a single shared aggregator, which
//! closes most of the gap by construction (one list instead of two). But the
//! aggregator itself could still forget to `extend` a newly added module, so
//! this file mechanically compares its source against `repos/mod.rs`'s own
//! `pub mod` declarations — the one remaining place a module could be missed.
//!
//! It also asserts `compat_probe.rs`'s `probe_check` runs every checker rule
//! shaped `(conn: &Connection, queries: &[QueryCheck]) -> Vec<String>` that
//! `checker.rs` exports, so a new schema-validating rule added there is
//! mechanically forced into the cross-version probe too, not just the
//! in-process test suite.

use std::collections::BTreeSet;

fn repo_root() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

fn read(rel_path: &str) -> String {
    let path = format!("{}/{rel_path}", repo_root());
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("could not read {path}: {e}"))
}

/// The module names declared `pub mod <name>;` at the top of `src/repos/mod.rs`
/// — every repo module that exists, per its own directory listing of itself.
fn declared_repo_modules() -> BTreeSet<String> {
    let src = read("src/repos/mod.rs");
    let mut names = BTreeSet::new();
    for line in src.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed
            .strip_prefix("pub mod ")
            .and_then(|s| s.strip_suffix(';'))
        {
            names.insert(rest.to_string());
        }
    }
    names
}

/// The module names `repos::all_queries()` actually aggregates, parsed from its
/// own source: every `all.extend(<name>::all_queries());` line.
fn aggregated_repo_modules() -> BTreeSet<String> {
    let src = read("src/repos/mod.rs");
    let mut names = BTreeSet::new();
    for line in src.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("all.extend(") {
            if let Some(end) = rest.find("::all_queries())") {
                names.insert(rest[..end].to_string());
            }
        }
    }
    names
}

#[test]
fn all_queries_aggregates_every_declared_repo_module() {
    let declared = declared_repo_modules();
    let aggregated = aggregated_repo_modules();

    assert!(
        !declared.is_empty(),
        "found no `pub mod` declarations in src/repos/mod.rs — the source scan is misconfigured"
    );

    assert_eq!(
        declared,
        aggregated,
        "repos::all_queries() has drifted from repos/mod.rs's `pub mod` declarations.\n\
         Declared but not aggregated (its queries silently escape the checker): {:?}\n\
         Aggregated but not declared (a stale entry for a removed module): {:?}\n\
         Add or remove an `all.extend(<module>::all_queries());` line in \
         repos::all_queries() to match.",
        declared.difference(&aggregated).collect::<Vec<_>>(),
        aggregated.difference(&declared).collect::<Vec<_>>(),
    );
}

/// A planted-failure proof that the comparison above is not vacuous: feeding
/// it a `declared` set the `aggregated` set doesn't cover must disagree.
#[test]
fn module_set_comparison_flags_a_missing_module() {
    let declared: BTreeSet<String> = ["account", "instance", "server"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let aggregated: BTreeSet<String> = ["account", "instance"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_ne!(declared, aggregated);
    assert_eq!(
        declared.difference(&aggregated).collect::<Vec<_>>(),
        vec!["server"]
    );
}

/// Every `checker.rs` rule shaped exactly `(conn: &Connection, queries:
/// &[QueryCheck]) -> Vec<String>` — the schema-validating rules `compat_probe`
/// exists to run. `check_handwritten_sql` (`files: &[(String, String)]`) is a
/// source-file lint, not a registered-query-against-schema check, and is
/// deliberately excluded by not matching this signature.
const QUERY_SCHEMA_CHECKER_SIGNATURE: &str =
    "(conn: &Connection, queries: &[QueryCheck]) -> Vec<String>";

fn query_schema_checker_names() -> BTreeSet<String> {
    let src = read("src/checker.rs");
    let mut names = BTreeSet::new();
    for line in src.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("pub fn ") {
            if rest.contains(QUERY_SCHEMA_CHECKER_SIGNATURE) {
                let end = rest
                    .find('(')
                    .unwrap_or_else(|| panic!("malformed checker.rs fn line: {line}"));
                names.insert(rest[..end].to_string());
            }
        }
    }
    names
}

/// Drops everything from `//` onward on each line (a `//` inside a string
/// literal never occurs in these two source files, so this plain-text
/// approximation of "not a comment" is exact here). Without this, commenting
/// out a call site (`//violations.extend(check_x(&conn, ...));`) would still
/// satisfy a raw substring search over the whole file — the call's *text*
/// survives even though it no longer executes — silently defeating the
/// "is this rule actually called" check below.
fn strip_line_comments(src: &str) -> String {
    src.lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// True when `name(&conn` appears in `src` outside of a `//` comment.
fn calls_with_conn(src: &str, name: &str) -> bool {
    strip_line_comments(src).contains(&format!("{name}(&conn"))
}

#[test]
fn compat_probe_runs_every_query_schema_checker_rule() {
    let names = query_schema_checker_names();
    assert!(
        !names.is_empty(),
        "found no (conn, queries) -> Vec<String> checker rules in src/checker.rs — \
         the source scan is misconfigured"
    );

    let probe_src = read("src/bin/compat_probe.rs");
    let missing: Vec<&String> = names
        .iter()
        .filter(|name| !calls_with_conn(&probe_src, name))
        .collect();
    assert!(
        missing.is_empty(),
        "src/bin/compat_probe.rs's probe_check does not call these checker.rs rules, so an \
         old binary's cross-version probe would miss violations a newer schema introduces: \
         {missing:?}\n\
         Add a `violations.extend({{name}}(&conn, &queries));` line for each to probe_check."
    );
}

/// A planted-failure proof that the checker-rule scan above is not vacuous,
/// and specifically that a *commented-out* call site is caught rather than
/// matched as if it still executed.
#[test]
fn checker_rule_scan_flags_a_rule_missing_from_a_caller() {
    let names: BTreeSet<String> = ["check_module", "check_manifests"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let fake_caller_src = "violations.extend(check_module(&conn, &queries));\n\
        //violations.extend(check_manifests(&conn, &queries));";
    let missing: Vec<&String> = names
        .iter()
        .filter(|name| !calls_with_conn(fake_caller_src, name))
        .collect();
    assert_eq!(
        missing,
        vec!["check_manifests"],
        "a commented-out call must be treated as missing, not as still executing"
    );
}
