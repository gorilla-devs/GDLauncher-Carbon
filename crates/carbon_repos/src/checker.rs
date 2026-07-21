//! Test-time query checker ("SQLx at test time", fixture-free).
//!
//! Verifies every registered `QueryCheck` against a real migrated schema. The
//! verdicts are driven off SQLite's own statement metadata — an authorizer, the
//! `column_metadata` APIs, parameter enumeration, and `EXPLAIN QUERY PLAN` — so
//! each check reflects what SQLite actually compiles rather than what the SQL
//! string happens to spell:
//!
//! - [`check_module`]: the SQL prepares (unknown tables/columns/syntax fail),
//!   declared param names are exactly the statement's bound params (no extras,
//!   none missing), a multi-param query never uses a positional `?`, and every
//!   expected column appears in the result set.
//! - [`check_manifests`]: an authorizer records the write actions each statement
//!   performs; a write to a freshness table must set that table's freshness
//!   column.
//! - [`check_nullability`]: each result column's origin is resolved via
//!   `column_metadata`; a column whose source is nullable must be `Option`, and
//!   an expression column with no resolvable origin must be `Option` or carry an
//!   explicit `#[nullable(...)]` override.
//! - [`check_query_plans`]: `EXPLAIN QUERY PLAN` must not full-scan a guarded
//!   hot cache table unless the query is explicitly allowlisted.
//!
//! Exported so later plans/tasks can call these directly instead of redefining
//! them per test file.

use crate::registry::QueryCheck;
use rusqlite::Connection;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use std::sync::{Arc, Mutex};

/// Checks the structural properties of every `QueryCheck` in `queries` against
/// `conn`'s schema, returning one human-readable violation string per problem
/// found. An empty result means every query passed.
pub fn check_module(conn: &Connection, queries: &[QueryCheck]) -> Vec<String> {
    let mut violations = Vec::new();
    for q in queries {
        // 1. prepare: syntax, tables, columns, params must exist
        // CENSUS-RULE: checker.prepare
        let st = match conn.prepare(q.sql) {
            Ok(st) => st,
            Err(e) => {
                violations.push(format!("{}: does not prepare: {e}", q.name));
                continue;
            }
        };
        // 2. param-name enumeration: the declared param set must equal the set
        // of named parameters SQLite actually bound (exact equality — an extra
        // declared name or an undeclared SQL param are both violations).
        let count = st.parameter_count();
        let mut actual: Vec<&str> = Vec::new();
        for i in 1..=count {
            if let Some(name) = st.parameter_name(i) {
                actual.push(name);
            }
        }
        for p in q.params {
            // CENSUS-RULE: checker.declared-param-present
            if !actual.iter().any(|a| a == p) {
                violations.push(format!("{}: declared param {p} not present in SQL", q.name));
            }
        }
        for a in &actual {
            // CENSUS-RULE: checker.undeclared-param
            if !q.params.iter().any(|p| p == a) {
                violations.push(format!("{}: SQL param {a} is not declared in the registry", q.name));
            }
        }
        // 3. multi-param queries must use named params (no bare '?'), scanning
        // only outside string literals so a literal '?' in a text value is not
        // mistaken for a positional placeholder.
        // CENSUS-RULE: checker.positional-param
        if q.params.len() > 1 && sql_has_positional_param(q.sql) {
            violations.push(format!("{}: multi-param query uses positional '?'", q.name));
        }
        // 4. result shape vs COLUMNS metadata
        if let Some(cols) = q.columns {
            let actual_cols: Vec<String> =
                st.column_names().iter().map(|s| s.to_string()).collect();
            for spec in cols {
                // CENSUS-RULE: checker.result-column-present
                if !actual_cols.iter().any(|a| a == spec.name) {
                    violations.push(format!(
                        "{}: column '{}' missing from result set {actual_cols:?}",
                        q.name, spec.name
                    ));
                }
            }
        }
    }
    violations
}

/// True when `sql` contains a positional `?` placeholder outside any string
/// literal or quoted identifier. A tiny state machine skips `'...'` and `"..."`
/// spans (SQLite doubles the quote to escape it) so a literal `?` inside a text
/// value never reads as a placeholder.
fn sql_has_positional_param(sql: &str) -> bool {
    #[derive(PartialEq)]
    enum State {
        Normal,
        Single,
        Double,
    }
    let mut state = State::Normal;
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        match state {
            State::Normal => match c {
                '\'' => state = State::Single,
                '"' => state = State::Double,
                '?' => return true,
                _ => {}
            },
            State::Single => {
                if c == '\'' {
                    // A doubled '' is an escaped quote, not the end of the span.
                    if chars.peek() == Some(&'\'') {
                        chars.next();
                    } else {
                        state = State::Normal;
                    }
                }
            }
            State::Double => {
                if c == '"' {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                    } else {
                        state = State::Normal;
                    }
                }
            }
        }
    }
    false
}

/// Tables whose freshness column must be set explicitly on every write that can
/// update an existing row. Missing one breaks cache-expiry reads silently (the
/// two modpack `updatedAt` columns feed a 7-day freshness gate).
pub const FRESHNESS: &[(&str, &str)] = &[
    ("VersionInfoCache", "lastUpdatedAt"),
    ("PartialVersionInfoCache", "lastUpdatedAt"),
    ("LwjglMetaCache", "lastUpdatedAt"),
    ("AssetsMetaCache", "lastUpdatedAt"),
    ("ModFileCache", "lastUpdatedAt"),
    ("ServerModFileCache", "lastUpdatedAt"),
    ("ModMetadata", "lastUpdatedAt"),
    ("FrontendPreference", "updatedAt"),
    ("CurseForgeModpackCache", "updatedAt"),
    ("ModrinthModpackCache", "updatedAt"),
];

/// The write actions a single statement performs, as reported by SQLite's
/// authorizer during preparation.
#[derive(Default)]
struct WriteManifest {
    /// `(table, column)` pairs the statement updates. Covers both plain
    /// `UPDATE ... SET` and the `DO UPDATE SET` branch of an upsert.
    updates: Vec<(String, String)>,
}

/// Runs the authorizer over `sql`'s preparation to capture its write manifest.
/// Preparation errors are surfaced (they are separately reported by
/// [`check_module`], so callers skip on error).
fn build_manifest(conn: &Connection, sql: &str) -> rusqlite::Result<WriteManifest> {
    let collected = Arc::new(Mutex::new(WriteManifest::default()));
    let sink = collected.clone();
    conn.authorizer(Some(move |ctx: AuthContext<'_>| {
        if let Ok(mut m) = sink.lock() {
            if let AuthAction::Update { table_name, column_name } = ctx.action {
                m.updates.push((table_name.to_string(), column_name.to_string()));
            }
        }
        Authorization::Allow
    }))?;
    let prepared = conn.prepare(sql);
    // Always detach the authorizer, even when preparation failed, so it never
    // leaks onto the shared connection.
    conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>)?;
    prepared?;
    let manifest = std::mem::take(&mut *collected.lock().expect("manifest mutex poisoned"));
    Ok(manifest)
}

/// Manifest-based freshness lint: any statement that updates a freshness table
/// (a plain `UPDATE` or the `DO UPDATE SET` branch of an upsert) must set that
/// table's freshness column. Reads the write actions from the authorizer rather
/// than the SQL text, so it can't be fooled by the column name merely appearing
/// elsewhere in the statement.
pub fn check_manifests(conn: &Connection, queries: &[QueryCheck]) -> Vec<String> {
    let mut violations = Vec::new();
    for q in queries {
        let manifest = match build_manifest(conn, q.sql) {
            Ok(m) => m,
            Err(_) => continue,
        };
        for (table, fresh_col) in FRESHNESS {
            let updated_cols: Vec<&str> = manifest
                .updates
                .iter()
                .filter(|(t, _)| t == table)
                .map(|(_, c)| c.as_str())
                .collect();
            // CENSUS-RULE: checker.freshness
            if !updated_cols.is_empty() && !updated_cols.iter().any(|c| c == fresh_col) {
                violations.push(format!(
                    "{}: writes {} but does not set freshness column '{}'",
                    q.name, table, fresh_col
                ));
            }
        }
    }
    violations
}

/// Origin-based nullability lint. For every row-returning query, each expected
/// column is matched to its result column and its origin resolved via
/// `column_metadata`:
///
/// - a plain column whose source is nullable must be declared nullable
///   (`Option`), otherwise a NULL read panics at runtime;
/// - a column with no resolvable origin (a SQL expression / aggregate) must be
///   declared nullable or carry an explicit `#[nullable(...)]` override, since
///   its nullability can't be inferred.
///
/// The source-NOT-NULL direction is intentionally not enforced: a LEFT JOIN can
/// make a NOT-NULL source column NULL in the result, so declaring such a column
/// `Option` is correct, not a violation.
pub fn check_nullability(conn: &Connection, queries: &[QueryCheck]) -> Vec<String> {
    let mut violations = Vec::new();
    for q in queries {
        let cols = match q.columns {
            Some(c) => c,
            None => continue,
        };
        let st = match conn.prepare(q.sql) {
            Ok(st) => st,
            Err(_) => continue,
        };
        for spec in cols {
            // An explicit override takes the developer at their word.
            if spec.explicit_nullable {
                continue;
            }
            let idx = match st.column_index(spec.name) {
                Ok(i) => i,
                // A missing column is already reported by check_module.
                Err(_) => continue,
            };
            match st.column_metadata(idx) {
                Ok(Some((_, _, _, _, _, not_null, _, _))) => {
                    // CENSUS-RULE: checker.nullability-nullable-source
                    if !not_null && !spec.nullable {
                        violations.push(format!(
                            "{}: column '{}' maps a nullable source column but is declared non-null (use Option or #[nullable(true)])",
                            q.name, spec.name
                        ));
                    }
                }
                Ok(None) => {
                    // CENSUS-RULE: checker.nullability-expression-origin
                    if !spec.nullable {
                        violations.push(format!(
                            "{}: column '{}' is a SQL expression with no resolvable origin; declare it Option or add an explicit #[nullable(...)] override",
                            q.name, spec.name
                        ));
                    }
                }
                Err(_) => {}
            }
        }
    }
    violations
}

/// Hot cache tables that must never be full-scanned by a registered query: a
/// missing index here turns a per-key lookup into a table walk that grows with
/// the cache.
pub const SCAN_GUARDED_TABLES: &[&str] = &[
    "HTTPCache",
    "ModFileCache",
    "ServerModFileCache",
    "ModMetadata",
    "CurseForgeModCache",
    "ModrinthModCache",
    "LocalModImageCache",
    "CurseForgeModImageCache",
    "ModrinthModImageCache",
];

/// Registered queries whose full scan of a guarded table is intentional or
/// unavoidable on the frozen schema. Each entry is a query `name`.
pub const SCAN_ALLOWLIST: &[&str] = &[
    // A whole-table garbage-collection sweep: it deletes every ModMetadata row
    // with no referencing file-cache row, so it must visit every row (and the
    // correlated NOT-EXISTS probes walk the file caches by the unindexed
    // `metadataId`). Runs rarely, off the hot path.
    "gc_orphan_metadata",
    // Content-hash dedup lookup on ModMetadata by `(sha512, murmur2)`, neither
    // of which is indexed in the schema. The schema is frozen here, so an index
    // would need a future migration.
    "find_metadata_by_hashes",
];

/// EXPLAIN QUERY PLAN lint: every registered query's plan is inspected and any
/// full scan (`SCAN <table>` with no index/PK use) of a [`SCAN_GUARDED_TABLES`]
/// table fails, unless the query is named in [`SCAN_ALLOWLIST`]. Parameters are
/// bound to NULL — the plan's scan/search structure does not depend on their
/// values.
pub fn check_query_plans(conn: &Connection, queries: &[QueryCheck]) -> Vec<String> {
    let mut violations = Vec::new();
    for q in queries {
        if SCAN_ALLOWLIST.contains(&q.name) {
            continue;
        }
        let eqp_sql = format!("EXPLAIN QUERY PLAN {}", q.sql);
        let mut st = match conn.prepare(&eqp_sql) {
            Ok(st) => st,
            // Preparation failures are reported by check_module.
            Err(_) => continue,
        };
        let count = st.parameter_count();
        let nulls: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Null; count];
        let bound: Vec<&dyn rusqlite::ToSql> =
            nulls.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        let details: Result<Vec<String>, _> = st
            .query_map(bound.as_slice(), |r| r.get::<_, String>("detail"))
            .and_then(|rows| rows.collect());
        let details = match details {
            Ok(d) => d,
            Err(_) => continue,
        };
        for detail in &details {
            for table in SCAN_GUARDED_TABLES {
                // CENSUS-RULE: checker.query-plan-full-scan
                if plan_full_scans_table(detail, table) {
                    violations.push(format!(
                        "{}: query plan full-scans guarded table '{}' ({}); add an index/PK filter or allowlist it",
                        q.name,
                        table,
                        detail.trim()
                    ));
                }
            }
        }
    }
    violations
}

/// True when an EQP `detail` line describes a full table scan of `table` — a
/// `SCAN <table>` with no `USING (COVERING) INDEX` / `USING INTEGER PRIMARY KEY`
/// qualifier. `SEARCH ...` (indexed) and index scans are not full scans.
fn plan_full_scans_table(detail: &str, table: &str) -> bool {
    let rest = match detail.trim().strip_prefix("SCAN ") {
        Some(r) => r,
        None => return false,
    };
    let object = rest.split_whitespace().next().unwrap_or("");
    object == table && !rest.contains("USING")
}
