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
//! - [`check_insert_datetime_columns`]: every registered `INSERT`'s explicit
//!   column list (parsed from the SQL text, not SQLite metadata) must name
//!   every `DATETIME`-typed column of its target table, so none can fall back
//!   to a DDL default silently.
//!
//! Exported so later plans/tasks can call these directly instead of redefining
//! them per test file.

use crate::registry::{QueryCheck, QueryClass};
use rusqlite::Connection;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use std::sync::{Arc, Mutex};

/// Checks the structural properties of every `QueryCheck` in `queries` against
/// `conn`'s schema, returning one human-readable violation string per problem
/// found. An empty result means every query passed.
pub fn check_module(conn: &Connection, queries: &[QueryCheck]) -> Vec<String> {
    let mut violations = Vec::new();
    for q in queries {
        // 0. leading whitespace must be ASCII: `is_write_sql` (which drives
        // pool routing) skips only ASCII whitespace as a const fn, falling
        // back conservatively to Write on anything else. Registered SQL with
        // leading Unicode whitespace would classify Write regardless of verb;
        // this rule makes that state unrepresentable instead of merely rare.
        // CENSUS-RULE: checker.sql-ascii-leading
        let ascii_trimmed = q.sql.trim_start_matches([' ', '\t', '\n', '\r']);
        if ascii_trimmed.len() != q.sql.trim_start().len() {
            violations.push(format!(
                "{}: SQL has leading non-ASCII whitespace — the const classifier cannot skip it",
                q.name
            ));
        }
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
                violations.push(format!(
                    "{}: SQL param {a} is not declared in the registry",
                    q.name
                ));
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
    /// Table names the statement inserts into.
    inserts: Vec<String>,
    /// Table names the statement deletes from.
    deletes: Vec<String>,
}

impl WriteManifest {
    /// True when the statement performs no write action at all (no insert,
    /// update, or delete) — the manifest-lock rule's definition of "actually a
    /// read".
    fn is_empty(&self) -> bool {
        self.updates.is_empty() && self.inserts.is_empty() && self.deletes.is_empty()
    }

    /// Human-readable summary of every write action recorded, for violation
    /// messages.
    fn describe(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for table in &self.inserts {
            parts.push(format!("INSERT into {table}"));
        }
        for table in &self.deletes {
            parts.push(format!("DELETE from {table}"));
        }
        for (table, column) in &self.updates {
            parts.push(format!("UPDATE {table}.{column}"));
        }
        parts.join(", ")
    }
}

/// Runs the authorizer over `sql`'s preparation to capture its write manifest.
/// Preparation errors are surfaced (they are separately reported by
/// [`check_module`], so callers skip on error).
fn build_manifest(conn: &Connection, sql: &str) -> rusqlite::Result<WriteManifest> {
    let collected = Arc::new(Mutex::new(WriteManifest::default()));
    let sink = collected.clone();
    conn.authorizer(Some(move |ctx: AuthContext<'_>| {
        if let Ok(mut m) = sink.lock() {
            match ctx.action {
                AuthAction::Update {
                    table_name,
                    column_name,
                } => {
                    m.updates
                        .push((table_name.to_string(), column_name.to_string()));
                }
                AuthAction::Insert { table_name } => {
                    m.inserts.push(table_name.to_string());
                }
                AuthAction::Delete { table_name } => {
                    m.deletes.push(table_name.to_string());
                }
                _ => {}
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

/// Manifest-locked classification rule: every `Read`-classified [`QueryCheck`]
/// must have an empty authorizer write-set. `class` is derived at compile time
/// from the SQL's leading verb (`SELECT`/`WITH` ⇒ `Read`), which the async
/// wrapper trusts to route the query to the read-only pool connection — but a
/// `WITH` CTE can wrap a data-modifying statement (`WITH x AS (DELETE FROM t
/// RETURNING *) SELECT * FROM x`) whose leading verb lies about what the
/// engine actually executes. This rule asks the authorizer instead of the SQL
/// text: any insert/update/delete action on a `Read`-classified statement is a
/// violation, because the wrapper would route it to the read-only pool where
/// it either fails loudly (best case) or, if the pool is not truly read-only in
/// some future configuration, silently corrupts data.
///
/// `Write`-classified queries are never checked here: a `Write` class routes to
/// the writer connection regardless of whether the statement happens to write
/// anything, so an empty write-set on a `Write`-classified query (a `Write`ish
/// `SELECT`-adjacent statement misclassified conservatively) is legal — the
/// conservative default can never be a violation of this rule.
pub fn check_classification(conn: &Connection, queries: &[QueryCheck]) -> Vec<String> {
    let mut violations = Vec::new();
    for q in queries {
        if q.class != QueryClass::Read {
            continue;
        }
        let manifest = match build_manifest(conn, q.sql) {
            Ok(m) => m,
            Err(_) => continue,
        };
        // CENSUS-RULE: checker.read-class-no-writes
        if !manifest.is_empty() {
            violations.push(format!(
                "{}: classified Read but the engine reports writes ({}) — the wrapper would route it to the read-only pool",
                q.name,
                manifest.describe()
            ));
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

/// Finds the first case-insensitive whole-word occurrence of `word` in `text`,
/// returning its byte offset. "Whole word" means neither neighbor is an
/// identifier character, so a keyword embedded in a longer identifier is never
/// mistaken for the keyword itself.
fn find_word(text: &str, word: &str) -> Option<usize> {
    let upper = text.to_ascii_uppercase();
    let word_upper = word.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    let is_ident = |c: u8| (c as char).is_ascii_alphanumeric() || c == b'_';
    let mut from = 0;
    while let Some(rel) = upper[from..].find(&word_upper) {
        let at = from + rel;
        let end = at + word_upper.len();
        let before_ok = at == 0 || !is_ident(bytes[at - 1]);
        let after_ok = end >= bytes.len() || !is_ident(bytes[end]);
        if before_ok && after_ok {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

/// Consumes one SQL identifier (bare, or quoted with `"…"`, `` `…` ``, or
/// `[…]`) from the start of `s`, returning the identifier text (quotes
/// included) and the remainder of `s` after it.
fn take_identifier(s: &str) -> Option<(&str, &str)> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    match bytes[0] {
        b'"' | b'`' => {
            let close = s[1..].find(bytes[0] as char)? + 1;
            Some((&s[..=close], &s[close + 1..]))
        }
        b'[' => {
            let close = s.find(']')?;
            Some((&s[..=close], &s[close + 1..]))
        }
        _ => {
            let end = s
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(s.len());
            if end == 0 {
                None
            } else {
                Some((&s[..end], &s[end..]))
            }
        }
    }
}

/// Strips a single layer of `"…"`, `` `…` ``, or `[…]` quoting from a SQL
/// identifier, if present; otherwise returns it unchanged.
fn strip_ident_quotes(s: &str) -> &str {
    let s = s.trim();
    for (open, close) in [("\"", "\""), ("`", "`"), ("[", "]")] {
        if let Some(inner) = s.strip_prefix(open).and_then(|r| r.strip_suffix(close)) {
            return inner;
        }
    }
    s
}

/// Parses `INSERT [OR …] INTO <table> (<col1>, <col2>, …)`, returning the
/// (unquoted) table name and column list. Returns `None` when the statement
/// has no explicit column list (`INSERT INTO t VALUES …`) — the DDL defaults
/// then apply silently, which [`check_insert_datetime_columns`] treats as an
/// outright violation since it can't be verified safe without the list.
fn parse_insert_target(sql: &str) -> Option<(String, Vec<String>)> {
    let into_at = find_word(sql, "INTO")?;
    let rest = sql[into_at + 4..].trim_start();
    let (table_raw, after_table) = take_identifier(rest)?;
    let table = strip_ident_quotes(table_raw).to_string();
    let after_table = after_table.trim_start();
    let after_table = after_table.strip_prefix('(')?;
    let close = after_table.find(')')?;
    let columns: Vec<String> = after_table[..close]
        .split(',')
        .map(|c| strip_ident_quotes(c.trim()).to_string())
        .filter(|c| !c.is_empty())
        .collect();
    Some((table, columns))
}

/// INSERT-datetime lint: every registered query whose SQL is an `INSERT` must
/// list every `DATETIME`-typed column of its target table (per `PRAGMA
/// table_info`) explicitly in its column list. Every migration DDL declares
/// its `DATETIME` columns with `DEFAULT CURRENT_TIMESTAMP` (a TEXT string) or
/// leaves them nullable with no default — either way, a bare `INSERT` that
/// omits the column relies on the DDL rather than [`crate::dbtypes::DbDateTime`],
/// so a column meant to hold epoch-millis silently gets a `CURRENT_TIMESTAMP`
/// text value the moment a future migration adds a default to it. Listing the
/// column explicitly (bound to `DbDateTime`, or a literal `NULL` when the value
/// is not yet known) is required no matter what the current DDL default is,
/// so the query stays correct even if the DDL default later changes.
/// A column-list-less `INSERT INTO t VALUES …` is flagged outright: without a
/// column list there is nothing to verify against.
pub fn check_insert_datetime_columns(conn: &Connection, queries: &[QueryCheck]) -> Vec<String> {
    let mut violations = Vec::new();
    for q in queries {
        let first_word: String = q
            .sql
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        if !first_word.eq_ignore_ascii_case("INSERT") {
            continue;
        }
        // CENSUS-RULE: checker.insert-datetime-explicit
        let Some((table, columns)) = parse_insert_target(q.sql) else {
            violations.push(format!(
                "{}: INSERT has no explicit column list (`INSERT INTO <table> VALUES …`) — \
                 a DATETIME column relying on its DDL default could silently receive a \
                 CURRENT_TIMESTAMP text value instead of DbDateTime's epoch-millis; list every \
                 column explicitly",
                q.name
            ));
            continue;
        };
        let mut info_stmt = match conn.prepare(&format!("PRAGMA table_info(\"{table}\")")) {
            Ok(st) => st,
            // Unknown table is already reported by check_module.
            Err(_) => continue,
        };
        let table_cols: rusqlite::Result<Vec<(String, String)>> = info_stmt
            .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
            .and_then(Iterator::collect);
        let Ok(table_cols) = table_cols else { continue };
        for (col_name, decltype) in &table_cols {
            if decltype.eq_ignore_ascii_case("DATETIME")
                && !columns.iter().any(|c| c.eq_ignore_ascii_case(col_name))
            {
                violations.push(format!(
                    "{}: INSERT into {table} omits DATETIME column '{col_name}' from its column \
                     list — list it explicitly (bind DbDateTime, or a literal NULL if the value \
                     is not yet known) so it never falls back to the DDL default",
                    q.name
                ));
            }
        }
    }
    violations
}

/// CENSUS-RULE: checker.handwritten-sql-registered — every hand-written SQL
/// statement in a repos module must receive its SQL through an UPPER_SNAKE
/// const that a `QueryCheck` in the same file also references (`sql: CONST`),
/// so the checker provably validates the exact string the code executes.
/// Inline string literals, `format!` expressions, and other non-const
/// arguments are violations. Receivers `st`/`stmt` are prepared-statement
/// method calls (their first argument is params, not SQL) and are skipped.
pub fn check_handwritten_sql(files: &[(String, String)]) -> Vec<String> {
    let mut violations = Vec::new();
    for (name, src) in files {
        let mut used_consts: Vec<String> = Vec::new();
        for (token, receiver_sensitive) in [
            ("prepare_cached(", false),
            (".prepare(", false),
            ("execute_batch(", false),
            (".execute(", true),
            (".query_row(", true),
            (".query_map(", true),
        ] {
            let mut from = 0;
            while let Some(pos) = src[from..].find(token) {
                let at = from + pos;
                from = at + token.len();
                if receiver_sensitive {
                    let recv: String = src[..at]
                        .chars()
                        .rev()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                    if recv == "st" || recv == "stmt" {
                        continue;
                    }
                }
                let arg = src[at + token.len()..]
                    .trim_start_matches(|c: char| c.is_whitespace())
                    .trim_start_matches('&');
                let line_no = src[..at].lines().count();
                if arg.starts_with('"') || arg.starts_with("r\"") || arg.starts_with("r#\"") {
                    violations.push(format!(
                        "{name}:{line_no}: inline SQL literal at `{token}` — extract an UPPER_SNAKE const shared with a QueryCheck"
                    ));
                } else if arg.starts_with("format!") {
                    violations.push(format!(
                        "{name}:{line_no}: format!-built SQL at `{token}` — dynamic SQL must go through DynamicQuery"
                    ));
                } else {
                    let ident: String = arg
                        .chars()
                        .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
                        .collect();
                    if ident.len() >= 2 && arg[ident.len()..].starts_with([',', ')']) {
                        used_consts.push(ident);
                    } else {
                        violations.push(format!(
                            "{name}:{line_no}: non-const SQL argument at `{token}` — pass an UPPER_SNAKE const shared with a QueryCheck"
                        ));
                    }
                }
            }
        }
        for c in used_consts {
            if !src.contains(&format!("sql: {c}")) {
                violations.push(format!(
                    "{name}: const `{c}` is executed but no QueryCheck references it (`sql: {c}` missing) — the checker cannot vouch for it"
                ));
            }
        }
    }
    violations
}
