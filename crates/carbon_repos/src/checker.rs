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
//!   none missing), no query uses a positional `?`, and every expected column
//!   appears in the result set.
//! - [`check_manifests`]: an authorizer records the write actions each statement
//!   performs; a write to a freshness table must set that table's freshness
//!   column.
//! - [`check_nullability`]: each result column's origin is resolved via
//!   `column_metadata`; a column whose source is nullable must be `Option`, and
//!   an expression column with no resolvable origin must be `Option` or carry an
//!   explicit `#[nullable(...)]` override. It also catches the one case where a
//!   `NOT NULL`-by-schema column must still be `Option`: sourced from a table
//!   this query joins via an *unambiguous* `LEFT [OUTER] JOIN` (see
//!   [`unambiguous_left_join_tables`] for exactly what that covers and does
//!   not — a deliberately conservative, self-join-safe subset of general
//!   outer-join nullability, not a full join-position analysis).
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
        // 3. every param must be named (no bare '?'), scanning only outside
        // string literals so a literal '?' in a text value is not mistaken for a
        // positional placeholder. The generated wrappers bind a name/value slice,
        // which routes to rusqlite's named binding and never fills a positional
        // slot — an unbound placeholder then reads as NULL rather than failing —
        // so this holds however many named params the query also declares.
        // CENSUS-RULE: checker.positional-param
        if sql_has_positional_param(q.sql) {
            violations.push(format!("{}: query uses positional '?'", q.name));
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
            // 5. duplicate result-column names (case-insensitive, matching
            // rusqlite's own case-insensitive by-name column resolution): the
            // generated `FromRow::from_row` reads every field by name, and
            // that lookup resolves to the first matching column index — an
            // unaliased join exposing two same-named columns (e.g. both sides
            // having an `id`) silently binds the left one, with the right one
            // never reachable by name despite appearing in the result set.
            let mut seen: Vec<String> = Vec::new();
            for name in &actual_cols {
                let lower = name.to_ascii_lowercase();
                // CENSUS-RULE: checker.duplicate-result-column
                if seen.contains(&lower) {
                    violations.push(format!(
                        "{}: result set has a duplicate column name '{name}' (columns: \
                         {actual_cols:?}) — FromRow's by-name lookup would silently bind \
                         the first (left) one; alias one side of the join distinctly",
                        q.name
                    ));
                } else {
                    seen.push(lower);
                }
            }
        }
    }
    violations
}

/// Strips SQL `-- ...` line comments and `/* ... */` block comments from
/// `sql`, replacing each with a single space (never nothing — `foo--bar`
/// stripped to `foobar` would wrongly merge two tokens into one) so callers
/// that tokenize or scan the result never mistake commented-out SQL for live
/// SQL. Quote-aware: a `--` or `/*` inside a `'...'` string literal or a
/// `"..."` / `` `...` `` / `[...]` quoted identifier is left untouched — a
/// literal like `'a--b'` survives intact — matching the same span rules
/// [`sql_tokens`] itself understands (`''` doubles an embedded quote inside a
/// `'...'` string; the other quote kinds have no doubling here, same as
/// `sql_tokens`'s own reader). Used by both [`sql_tokens`] and
/// [`sql_has_positional_param`] so a comment can neither feed a phantom
/// LEFT JOIN/table reference into the former nor trip a false positional-`?`
/// hit in the latter.
fn strip_sql_comments(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                out.push(c);
                while let Some(n) = chars.next() {
                    out.push(n);
                    if n == '\'' {
                        if chars.peek() == Some(&'\'') {
                            out.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                }
            }
            '"' | '`' | '[' => {
                let close = if c == '[' { ']' } else { c };
                out.push(c);
                for n in chars.by_ref() {
                    out.push(n);
                    if n == close {
                        break;
                    }
                }
            }
            '-' if chars.peek() == Some(&'-') => {
                chars.next(); // consume the second '-'
                for n in chars.by_ref() {
                    if n == '\n' {
                        break;
                    }
                }
                out.push(' ');
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next(); // consume the '*'
                let mut prev = '\0';
                for n in chars.by_ref() {
                    if prev == '*' && n == '/' {
                        break;
                    }
                    prev = n;
                }
                out.push(' ');
            }
            other => out.push(other),
        }
    }
    out
}

/// True when `sql` contains a positional `?` placeholder outside any string
/// literal, quoted identifier, or SQL comment. Comments are stripped first
/// (see [`strip_sql_comments`]) so a literal `?` inside a `-- ...` or
/// `/* ... */` comment never reads as a placeholder. A tiny state machine then
/// skips `'...'` and `"..."` spans (SQLite doubles the quote to escape it) so
/// a literal `?` inside a text value doesn't either.
fn sql_has_positional_param(sql: &str) -> bool {
    #[derive(PartialEq)]
    enum State {
        Normal,
        Single,
        Double,
    }
    let sql = strip_sql_comments(sql);
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

/// Pool-routing rule: every registered query's [`QueryCheck::routes_write`]
/// must agree with its [`QueryCheck::class`] (`routes_write == (class ==
/// Write)`).
///
/// `class` is derived from the SQL's leading verb; `routes_write` is a
/// *second*, independently hard-coded fact — which `queries!` arm actually
/// fired, since only the `usize`/`execute` arm routes to the writer and every
/// row-returning arm (`Option<Row>`, `Vec<Row>`, `i64`, bare `Row`) routes to
/// the read-only pool regardless of what the SQL does. The two facts agree for
/// every query in this codebase today, but nothing stopped a future write
/// declared through a row-returning arm (`UPDATE … RETURNING id -> i64`) from
/// silently keying its runtime pool off the return-type shape rather than the
/// SQL verb: `check_classification` above only catches a *misclassified*
/// `class` (a `WITH`-wrapped write that lies about its own leading verb) — it
/// is structurally blind to a *correctly*-classified `Write` query that was
/// declared via a read-routing arm anyway, because that arm's `class_of($sql)`
/// call computes `Write` right alongside the very `routes_write: false` that
/// disagrees with it. This rule is the one place both facts are compared.
pub fn check_pool_routing(queries: &[QueryCheck]) -> Vec<String> {
    let mut violations = Vec::new();
    for q in queries {
        let expected = q.class == QueryClass::Write;
        // CENSUS-RULE: checker.routing-matches-class
        if q.routes_write != expected {
            violations.push(format!(
                "{}: classified {:?} but routes_write is {} — a write declared through a \
                 row-returning arm routes to the read-only pool and fails every call \
                 (SQLITE_READONLY), while a read declared through the usize/execute arm \
                 routes to the writer needlessly",
                q.name, q.class, q.routes_write
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
///   its nullability can't be inferred;
/// - a column sourced from a table this query joins via an unambiguous `LEFT
///   [OUTER] JOIN` must also be declared nullable, even when that table's own
///   schema marks the column `NOT NULL` — see [`unambiguous_left_join_tables`]
///   for exactly what "unambiguous" means and what is deliberately left
///   uncovered.
///
/// The source-NOT-NULL direction is intentionally not enforced *in general*: a
/// LEFT JOIN can make a NOT-NULL source column NULL in the result, so
/// declaring such a column `Option` is correct, not a violation. The
/// unambiguous-LEFT-JOIN case above is the one exception where this rule does
/// still enforce that direction, precisely because it can tell (from the SQL
/// text) that the column's table sits on the optional side.
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
        let left_join_tables = unambiguous_left_join_tables(q.sql);
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
                Ok(Some((_, table_name, _, _, _, not_null, _, _))) => {
                    // CENSUS-RULE: checker.nullability-nullable-source
                    if !not_null && !spec.nullable {
                        violations.push(format!(
                            "{}: column '{}' maps a nullable source column but is declared non-null (use Option or #[nullable(true)])",
                            q.name, spec.name
                        ));
                    }
                    // CENSUS-RULE: checker.nullability-outer-join-widening
                    if !spec.nullable
                        && left_join_tables
                            .contains(&table_name.to_string_lossy().to_ascii_uppercase())
                    {
                        violations.push(format!(
                            "{}: column '{}' is sourced from '{}', which this query LEFT JOINs — \
                             an unmatched row makes it NULL regardless of that table's own \
                             NOT NULL constraint; declare it Option or add an explicit \
                             #[nullable(...)] override",
                            q.name,
                            spec.name,
                            table_name.to_string_lossy()
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

/// The schema table names (upper-cased) this query joins via an *unambiguous*
/// `LEFT [OUTER] JOIN`: introduced exactly once in the whole query, and that
/// one introduction is a `LEFT`/`LEFT OUTER` join. Used by [`check_nullability`]
/// to catch a `NOT NULL`-by-schema column widened to nullable purely by
/// sitting on a LEFT JOIN's optional side — something `column_metadata` alone
/// can never see, since SQLite reports a column's schema constraint, not its
/// join position.
///
/// "Unambiguous" is deliberately conservative, trading recall for zero false
/// positives:
///
/// - **Self-joins are skipped.** A table introduced more than once (`FROM Foo
///   a LEFT JOIN Foo b ON …`) is dropped entirely: `column_metadata` reports
///   the bare schema name ("Foo") with no way to tell which occurrence a given
///   result column came from, so treating *any* occurrence as authoritative
///   for *every* occurrence would flag the preserved side (`a`) as if it were
///   the optional one (`b`).
/// - **Only `LEFT`/`LEFT OUTER JOIN` is recognised.** A `RIGHT JOIN` (which
///   widens the *preceding* tables instead) and a `FULL [OUTER] JOIN` (which
///   widens both sides) are not detected — neither appears anywhere in this
///   codebase's registered queries today, so the added complexity of tracking
///   them is deferred rather than risking an incorrect implementation.
/// - **No multi-hop propagation.** Only the table named directly after the
///   `LEFT JOIN` keyword is considered widened. A further `INNER JOIN` chained
///   onto that table's alias does not have its own widening modelled.
/// - **Derived tables and CTEs resolve safely, not necessarily precisely.** A
///   `LEFT JOIN (SELECT …) AS sub` has no bare identifier after `JOIN` (a `(`
///   token instead), so it never matches a real schema table name — the
///   heuristic silently does not apply rather than misfiring.
/// - **Old-style comma joins count too.** `FROM a, b` introduces `b` exactly
///   as an explicit `JOIN b` would; a comma seen while still inside a `FROM`
///   clause's table list is treated as another occurrence, so a table also
///   reached this way still correctly counts toward the ambiguity check
///   below.
///
/// None of these gaps can produce a false positive; they can only make the
/// rule miss a case, which is exactly the existing status quo this rule
/// improves on rather than regresses.
fn unambiguous_left_join_tables(sql: &str) -> std::collections::HashSet<String> {
    // Keywords/punctuation that close a `FROM` clause's comma-separated table
    // list once seen. `JOIN` itself is handled separately below (it starts its
    // own explicit join rather than continuing a comma list); plain
    // identifiers — bare table names, aliases, `AS` — never appear here, so
    // they never terminate the list before its next comma.
    const FROM_LIST_TERMINATORS: &[&str] = &[
        "WHERE",
        "INNER",
        "LEFT",
        "RIGHT",
        "FULL",
        "CROSS",
        "NATURAL",
        "GROUP",
        "ORDER",
        "LIMIT",
        "HAVING",
        "UNION",
        "EXCEPT",
        "INTERSECT",
        "WINDOW",
        "SET",
        "VALUES",
        "RETURNING",
        ")",
        ";",
    ];

    let tokens = sql_tokens(sql);
    let mut occurrences: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut seen_via_left_join: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let mut in_from_list = false;

    for (i, tok) in tokens.iter().enumerate() {
        let is_from = tok.eq_ignore_ascii_case("FROM");
        let is_join = tok.eq_ignore_ascii_case("JOIN");
        let is_from_list_comma = in_from_list && tok == ",";

        if is_join {
            in_from_list = false;
        } else if is_from {
            in_from_list = true;
        } else if in_from_list
            && FROM_LIST_TERMINATORS
                .iter()
                .any(|k| tok.eq_ignore_ascii_case(k))
        {
            in_from_list = false;
        }

        if !is_from && !is_join && !is_from_list_comma {
            continue;
        }
        // A bare identifier must follow; a `(` (derived table) or anything
        // else never matches a real schema table name later, so recording it
        // is harmless, but only an identifier can ever actually match.
        let Some(name) = tokens.get(i + 1) else {
            continue;
        };
        if !name
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
        {
            continue;
        }
        let key = name.to_ascii_uppercase();
        *occurrences.entry(key.clone()).or_insert(0) += 1;

        if is_join {
            // Walk back over join-modifier keywords immediately preceding
            // this `JOIN` to see whether `LEFT` (with an optional `OUTER`
            // between it and `JOIN`) introduced it.
            let mut j = i;
            let mut is_left = false;
            while j > 0 {
                let prev = &tokens[j - 1];
                if prev.eq_ignore_ascii_case("OUTER") {
                    j -= 1;
                    continue;
                }
                if prev.eq_ignore_ascii_case("LEFT") {
                    is_left = true;
                }
                break;
            }
            if is_left {
                seen_via_left_join.insert(key);
            }
        }
    }

    seen_via_left_join
        .into_iter()
        .filter(|name| occurrences.get(name) == Some(&1))
        .collect()
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
        // The plan names whatever the FROM/JOIN clause bound the table to, so
        // each guarded table is matched against its own name plus any alias the
        // query gives it.
        let guarded_names: Vec<(&str, Vec<String>)> = SCAN_GUARDED_TABLES
            .iter()
            .map(|table| {
                let mut names = vec![(*table).to_string()];
                names.extend(table_aliases(q.sql, table));
                (*table, names)
            })
            .collect();

        for detail in &details {
            for (table, names) in &guarded_names {
                // CENSUS-RULE: checker.query-plan-full-scan
                if names.iter().any(|n| plan_full_scans_table(detail, n)) {
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

/// Splits `sql` into identifier and single-character punctuation tokens,
/// unwrapping quoted identifiers and dropping string literals. Comments are
/// stripped first (see [`strip_sql_comments`]), so `-- ...` / `/* ... */` text
/// — including a commented-out `LEFT JOIN` or table reference — is never
/// tokenized as live SQL. Enough structure to tell an alias from the keyword
/// or punctuation that would otherwise follow a table name; not a SQL parser.
fn sql_tokens(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let sql = strip_sql_comments(sql);
    let mut chars = sql.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' | '`' | '[' => {
                let close = if c == '[' { ']' } else { c };
                let mut ident = String::new();
                for n in chars.by_ref() {
                    if n == close {
                        break;
                    }
                    ident.push(n);
                }
                out.push(ident);
            }
            '\'' => {
                // Skip the literal; '' is an embedded quote, not a terminator.
                while let Some(n) = chars.next() {
                    if n == '\'' {
                        if chars.peek() == Some(&'\'') {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
            }
            c if c.is_alphanumeric() || c == '_' => {
                let mut ident = String::from(c);
                while let Some(&n) = chars.peek() {
                    if n.is_alphanumeric() || n == '_' {
                        ident.push(n);
                        chars.next();
                    } else {
                        break;
                    }
                }
                out.push(ident);
            }
            c if c.is_whitespace() => {}
            other => out.push(other.to_string()),
        }
    }
    out
}

/// Names `sql` binds `table` to, so a plan naming the alias is still recognised
/// as scanning the guarded table. Only a bare or `AS`-introduced identifier
/// directly after the table name counts; a keyword or punctuation there means
/// the table was not aliased.
fn table_aliases(sql: &str, table: &str) -> Vec<String> {
    /// Words that may legally follow a table reference without being an alias.
    const NOT_AN_ALIAS: &[&str] = &[
        "AS",
        "ON",
        "USING",
        "WHERE",
        "GROUP",
        "ORDER",
        "LIMIT",
        "OFFSET",
        "HAVING",
        "JOIN",
        "INNER",
        "LEFT",
        "RIGHT",
        "FULL",
        "CROSS",
        "NATURAL",
        "OUTER",
        "UNION",
        "EXCEPT",
        "INTERSECT",
        "SET",
        "VALUES",
        "RETURNING",
        "WINDOW",
        "AND",
        "OR",
        "NOT",
        "SELECT",
        "FROM",
        "INDEXED",
        "BY",
        "WITH",
    ];

    let tokens = sql_tokens(sql);
    let mut aliases = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        if !token.eq_ignore_ascii_case(table) {
            continue;
        }
        let mut j = i + 1;
        if tokens.get(j).is_some_and(|t| t.eq_ignore_ascii_case("AS")) {
            j += 1;
        }
        let Some(candidate) = tokens.get(j) else {
            continue;
        };
        let is_identifier = candidate
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_');
        if is_identifier
            && !NOT_AN_ALIAS
                .iter()
                .any(|k| candidate.eq_ignore_ascii_case(k))
        {
            aliases.push(candidate.clone());
        }
    }

    aliases
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_sql_comments_strips_line_and_block_comments() {
        let sql = "SELECT id -- old: LEFT JOIN X\n  FROM Foo /* inline note */ WHERE id = 1";
        let stripped = strip_sql_comments(sql);
        assert!(
            !stripped.contains("LEFT JOIN X"),
            "line comment text must be gone: {stripped:?}"
        );
        assert!(
            !stripped.contains("inline note"),
            "block comment text must be gone: {stripped:?}"
        );
        assert!(
            stripped.contains("FROM Foo"),
            "live SQL must survive: {stripped:?}"
        );
        assert!(
            stripped.contains("WHERE id = 1"),
            "live SQL after a mid-statement block comment must survive: {stripped:?}"
        );
    }

    #[test]
    fn strip_sql_comments_preserves_string_literals_containing_comment_markers() {
        // A literal like 'a--b' or '/* not a comment */' must survive intact —
        // the scan must not mistake a marker inside a string for a real
        // comment start.
        let sql = "SELECT 'a--b', '/* not a comment */' FROM Foo";
        let stripped = strip_sql_comments(sql);
        assert!(
            stripped.contains("'a--b'"),
            "a string literal containing '--' must be untouched: {stripped:?}"
        );
        assert!(
            stripped.contains("'/* not a comment */'"),
            "a string literal containing '/*' must be untouched: {stripped:?}"
        );
    }

    #[test]
    fn sql_tokens_drops_a_commented_out_left_join() {
        let sql = "-- old: LEFT JOIN Bar b ON b.id = Foo.id\nSELECT Foo.id FROM Foo";
        let tokens = sql_tokens(sql);
        assert!(
            !tokens.iter().any(|t| t.eq_ignore_ascii_case("Bar")),
            "a commented-out table reference must not be tokenized: {tokens:?}"
        );
        assert!(
            !tokens.iter().any(|t| t.eq_ignore_ascii_case("LEFT")),
            "a commented-out LEFT keyword must not be tokenized: {tokens:?}"
        );
    }

    #[test]
    fn unambiguous_left_join_tables_ignores_a_phantom_left_join_in_a_comment() {
        // Before comment-stripping, this comment's "LEFT JOIN Bar" text alone
        // tokenized as a real, unambiguous (occurring exactly once) LEFT JOIN
        // of a table that never actually appears anywhere in the live SQL.
        let sql = "-- old shape: LEFT JOIN Bar b ON b.foo_id = Foo.id\nSELECT Foo.id FROM Foo";
        let widened = unambiguous_left_join_tables(sql);
        assert!(
            !widened.contains("BAR"),
            "a LEFT JOIN mentioned only in a comment must not count: {widened:?}"
        );
    }

    #[test]
    fn unambiguous_left_join_tables_counts_comma_introduced_tables() {
        // `Foo` is introduced twice: once by the old-style comma join in the
        // FROM list, once by the explicit LEFT JOIN — genuinely ambiguous,
        // like a self-join, and must be excluded from widening.
        let sql = "SELECT Foo.label FROM Bar, Foo LEFT JOIN Foo f2 ON f2.id = Foo.parent_id";
        let widened = unambiguous_left_join_tables(sql);
        assert!(
            !widened.contains("FOO"),
            "a table introduced both by a comma join and a LEFT JOIN must be treated as \
             ambiguous, not unambiguously widened: {widened:?}"
        );
    }

    #[test]
    fn sql_has_positional_param_ignores_a_question_mark_in_a_comment() {
        assert!(
            !sql_has_positional_param("SELECT id FROM Foo -- why?\nWHERE id = :id"),
            "a '?' inside a line comment must not read as a positional placeholder"
        );
        assert!(
            sql_has_positional_param("SELECT id FROM Foo WHERE id = ?"),
            "a genuine bare '?' outside any comment or literal must still be caught"
        );
    }
}
