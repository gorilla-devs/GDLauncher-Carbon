//! Down-script generation by schema diff, the library half of the
//! `new_migration` tool.
//!
//! A migration's two surrounding schemas are both mechanically constructible:
//! applying the migrations before it yields the *before* schema `S(n-1)`, and
//! applying its `up` on top yields the *after* schema `S(n)`. Diffing the two
//! `sqlite_master` object sets gives the inverse DDL that turns `S(n)` back into
//! `S(n-1)` — created objects get dropped, dropped objects get recreated from
//! `S(n-1)`'s exact stored DDL, and a changed table is either walked back with
//! `ALTER TABLE … DROP COLUMN` (when SQLite permits) or rebuilt from its old
//! DDL. The generated down is then round-tripped in-process: applying `up` then
//! the generated `down` must reproduce `S(n-1)`'s normalized schema byte-for-byte
//! before the script is trusted.
//!
//! Two changes can't be inverted from the schema diff alone and are surfaced as
//! human touchpoints instead of silently mis-generated: a **rename** (the diff
//! can't tell a rename from an unrelated drop+add, so restoring data needs a
//! hand-written reverse rename) and **DML on a pre-existing table** (data
//! transforms aren't DDL and have no derivable inverse). Both are detected via
//! SQLite's own authorizer / column metadata rather than SQL text parsing, and
//! the tool refuses to auto-generate until the developer supplies a verified
//! `down.sql`.

use crate::db_error::{DbError, DbResult};
use crate::schema_dump::{dump_schema, normalize_ddl};
use rusqlite::Connection;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

/// Why the generator cannot produce a trustworthy `down.sql` on its own. The CLI
/// turns each into a non-zero exit with an instruction for the developer.
#[derive(Debug)]
pub enum GenError {
    /// A SQLite error while building either surrounding schema or applying the
    /// generated down.
    Sqlite(rusqlite::Error),
    /// Any other DB-layer failure (mirrors [`DbError`]).
    Db(DbError),
    /// The generated (or hand-written) down did not reproduce `S(n-1)`: the
    /// schema after `up`+`down` differs from the schema at `S(n-1)`.
    RoundTripFailed {
        /// Normalized dump the down should have reproduced (`S(n-1)`).
        expected: String,
        /// Normalized dump the down actually produced.
        actual: String,
    },
}

impl std::fmt::Display for GenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenError::Sqlite(e) => write!(f, "sqlite error during down generation: {e}"),
            GenError::Db(e) => write!(f, "db error during down generation: {e}"),
            GenError::RoundTripFailed { expected, actual } => write!(
                f,
                "round-trip failed: applying up then the down does not restore the prior schema.\n\
                 --- expected (before this migration) ---\n{expected}\n\
                 --- actual (after up + generated down) ---\n{actual}"
            ),
        }
    }
}

impl std::error::Error for GenError {}

impl From<rusqlite::Error> for GenError {
    fn from(e: rusqlite::Error) -> Self {
        GenError::Sqlite(e)
    }
}

impl From<DbError> for GenError {
    fn from(e: DbError) -> Self {
        GenError::Db(e)
    }
}

/// A human touchpoint the diff cannot resolve, flagged for developer action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HumanAction {
    /// The `up` renames a column, or changes a pre-existing table's columns with
    /// both a removal and an addition (indistinguishable from a rename). The
    /// developer must hand-write the reverse rename so data is preserved.
    Rename,
    /// The `up` runs DML (`INSERT`/`UPDATE`/`DELETE`) against tables that already
    /// existed before it. Each entry is a `"<OP> <table>"` label. Not auto-
    /// invertible: the developer must provide and confirm an inverse `down.sql`.
    Dml(Vec<String>),
}

/// What the generator found when analysing an `up` against its predecessors:
/// the flags that gate auto-generation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpAnalysis {
    /// Set when a rename (or rename-shaped column change) is present.
    pub rename: bool,
    /// DML operations the `up` performs on pre-existing tables (empty when none).
    pub dml_on_existing: Vec<String>,
}

impl UpAnalysis {
    /// The human touchpoints implied by this analysis, in a stable order.
    pub fn human_actions(&self) -> Vec<HumanAction> {
        let mut out = Vec::new();
        if self.rename {
            out.push(HumanAction::Rename);
        }
        if !self.dml_on_existing.is_empty() {
            out.push(HumanAction::Dml(self.dml_on_existing.clone()));
        }
        out
    }
}

/// One table's stored DDL plus its ordered column names.
struct TableInfo {
    ddl: String,
    columns: Vec<String>,
}

/// The subset of `sqlite_master` the diff reasons over: user tables, and user
/// indexes/triggers (those with a non-null `sql`, i.e. not SQLite's implicit
/// auto-indexes, which follow their table's DDL). Bookkeeping tables are
/// excluded exactly as [`dump_schema`] excludes them.
struct Schema {
    tables: BTreeMap<String, TableInfo>,
    /// index name -> (owning table, stored DDL).
    indexes: BTreeMap<String, (String, String)>,
    /// trigger name -> (owning table, stored DDL).
    triggers: BTreeMap<String, (String, String)>,
}

/// Applies `ups` in order to a fresh in-memory connection with foreign keys
/// OFF (matching the migration connection's semantics), so the
/// resulting schema is exactly what the runner would produce.
///
/// Shared with [`crate::manifest`], which builds the same surrounding schemas
/// to derive migration kind and seed boundary-value data for the lossiness
/// round-trip — one builder, identical connection semantics everywhere a
/// scratch schema is constructed.
pub(crate) fn build(ups: &[&str]) -> DbResult<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "OFF")?;
    for up in ups {
        conn.execute_batch(up)?;
    }
    Ok(conn)
}

/// The ordered column names of `table`, by `cid`.
fn table_columns(conn: &Connection, table: &str) -> DbResult<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{table}\")"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Reads the diff-relevant schema of `conn`.
fn read_schema(conn: &Connection) -> DbResult<Schema> {
    let mut tables = BTreeMap::new();
    let mut indexes = BTreeMap::new();
    let mut triggers = BTreeMap::new();

    let mut stmt = conn.prepare(
        "SELECT type, name, tbl_name, sql FROM sqlite_master \
         WHERE sql IS NOT NULL \
           AND name NOT LIKE 'sqlite_%' \
           AND tbl_name NOT IN ('_migrations', '_prisma_migrations') \
           AND name NOT IN ('_migrations', '_prisma_migrations')",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for (type_, name, tbl_name, sql) in rows {
        match type_.as_str() {
            "table" => {
                let columns = table_columns(conn, &name)?;
                tables.insert(name, TableInfo { ddl: sql, columns });
            }
            "index" => {
                indexes.insert(name, (tbl_name, sql));
            }
            "trigger" => {
                triggers.insert(name, (tbl_name, sql));
            }
            // Views are not produced by any migration; ignore defensively.
            _ => {}
        }
    }

    Ok(Schema {
        tables,
        indexes,
        triggers,
    })
}

/// Quotes an identifier for embedding in generated SQL.
fn quote(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

/// Names in `a` but not in `b`, preserving `a`'s order.
fn difference<'a>(a: &'a [String], b: &[String]) -> Vec<&'a String> {
    let set: BTreeSet<&String> = b.iter().collect();
    a.iter().filter(|c| !set.contains(*c)).collect()
}

/// Names present in both `a` and `b`, in `a`'s order.
fn intersection<'a>(a: &'a [String], b: &[String]) -> Vec<&'a String> {
    let set: BTreeSet<&String> = b.iter().collect();
    a.iter().filter(|c| set.contains(*c)).collect()
}

/// True when `ALTER TABLE … DROP COLUMN` for every column in `added` is accepted
/// by SQLite on the real `S(n)` schema and leaves `table` with DDL equal to
/// `old_ddl`. SQLite refuses `DROP COLUMN` for PK/unique/indexed/generated-
/// referenced columns; when it does, this returns false and the caller rebuilds.
fn drop_column_feasible(all_ups: &[&str], table: &str, added: &[&String], old_ddl: &str) -> bool {
    let conn = match build(all_ups) {
        Ok(c) => c,
        Err(_) => return false,
    };
    for col in added {
        let stmt = format!("ALTER TABLE {} DROP COLUMN {}", quote(table), quote(col));
        if conn.execute_batch(&stmt).is_err() {
            return false;
        }
    }
    let got: Result<String, _> = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |r| r.get(0),
    );
    match got {
        Ok(ddl) => normalize_ddl(&ddl) == normalize_ddl(old_ddl),
        Err(_) => false,
    }
}

/// The rebuild-dance statements that restore `table` to its `old` shape from its
/// current `new` shape: rename the current table aside, recreate it from the old
/// DDL, copy the columns the two shapes share, and drop the aside table. Columns
/// present only in `old` come back empty (an inherent property of rollback, not
/// of generation); columns present only in `new` are dropped.
fn rebuild_table(table: &str, old: &TableInfo, new: &TableInfo, out: &mut Vec<String>) {
    let tmp = format!("{table}_dgtmp");
    let shared = intersection(&old.columns, &new.columns);

    out.push(format!(
        "ALTER TABLE {} RENAME TO {};",
        quote(table),
        quote(&tmp)
    ));
    out.push(format!("{};", old.ddl.trim_end_matches(';')));
    if !shared.is_empty() {
        let cols = shared
            .iter()
            .map(|c| quote(c))
            .collect::<Vec<_>>()
            .join(", ");
        out.push(format!(
            "INSERT INTO {} ({}) SELECT {} FROM {};",
            quote(table),
            cols,
            cols,
            quote(&tmp)
        ));
    }
    out.push(format!("DROP TABLE {};", quote(&tmp)));
}

/// Builds the inverse-DDL statements turning `new` (`S(n)`) back into `old`
/// (`S(n-1)`). Statement order: drop new/changed triggers and indexes, resolve
/// table shape, then recreate the old indexes and triggers that a table rebuild
/// (or recreation) removed or that the migration itself changed.
fn diff_down(all_ups: &[&str], old: &Schema, new: &Schema) -> Vec<String> {
    let mut drop_triggers = Vec::new();
    let mut drop_indexes = Vec::new();
    let mut table_stmts = Vec::new();
    let mut create_indexes = Vec::new();
    let mut create_triggers = Vec::new();

    // Tables whose indexes/triggers are wiped by the down (rebuilt in place or
    // recreated from scratch) and must therefore be fully re-established.
    let mut wiped_tables: BTreeSet<String> = BTreeSet::new();

    // --- triggers added or changed by the up: drop them ---
    for (name, (_tbl, new_sql)) in &new.triggers {
        if old.triggers.get(name).map(|(_, s)| s) != Some(new_sql) {
            drop_triggers.push(format!("DROP TRIGGER IF EXISTS {};", quote(name)));
        }
    }

    // --- indexes added or changed by the up: drop them ---
    for (name, (_tbl, new_sql)) in &new.indexes {
        if old.indexes.get(name).map(|(_, s)| s) != Some(new_sql) {
            drop_indexes.push(format!("DROP INDEX IF EXISTS {};", quote(name)));
        }
    }

    // --- created tables: drop ---
    for name in new.tables.keys() {
        if !old.tables.contains_key(name) {
            table_stmts.push(format!("DROP TABLE IF EXISTS {};", quote(name)));
        }
    }

    // --- dropped tables: recreate from the old DDL ---
    for (name, info) in &old.tables {
        if !new.tables.contains_key(name) {
            table_stmts.push(format!("{};", info.ddl.trim_end_matches(';')));
            wiped_tables.insert(name.clone());
        }
    }

    // --- changed tables: DROP COLUMN when SQLite permits, else rebuild ---
    for (name, old_info) in &old.tables {
        let Some(new_info) = new.tables.get(name) else {
            continue;
        };
        if normalize_ddl(&old_info.ddl) == normalize_ddl(&new_info.ddl) {
            continue;
        }
        let added = difference(&new_info.columns, &old_info.columns);
        let removed = difference(&old_info.columns, &new_info.columns);
        if removed.is_empty()
            && !added.is_empty()
            && drop_column_feasible(all_ups, name, &added, &old_info.ddl)
        {
            for col in &added {
                table_stmts.push(format!(
                    "ALTER TABLE {} DROP COLUMN {};",
                    quote(name),
                    quote(col)
                ));
            }
        } else {
            rebuild_table(name, old_info, new_info, &mut table_stmts);
            wiped_tables.insert(name.clone());
        }
    }

    // --- recreate old indexes the down removed or the up changed ---
    for (name, (tbl, old_sql)) in &old.indexes {
        let unchanged_present = new.indexes.get(name).map(|(_, s)| s) == Some(old_sql);
        if wiped_tables.contains(tbl) || !unchanged_present {
            create_indexes.push(format!("{};", old_sql.trim_end_matches(';')));
        }
    }

    // --- recreate old triggers the down removed or the up changed ---
    for (name, (tbl, old_sql)) in &old.triggers {
        let unchanged_present = new.triggers.get(name).map(|(_, s)| s) == Some(old_sql);
        if wiped_tables.contains(tbl) || !unchanged_present {
            create_triggers.push(format!("{};", old_sql.trim_end_matches(';')));
        }
    }

    let mut out = Vec::new();
    out.extend(drop_triggers);
    out.extend(drop_indexes);
    out.extend(table_stmts);
    out.extend(create_indexes);
    out.extend(create_triggers);
    out
}

/// Generates the `down.sql` body that inverts `up` (applied after `prev_ups`),
/// self-verified: the returned SQL is guaranteed to round-trip `S(n)` back to a
/// byte-identical `S(n-1)` schema, or a [`GenError::RoundTripFailed`] is returned.
///
/// This is the pure schema inverse; rename/DML touchpoints are surfaced
/// separately by [`analyze_up`] and are the CLI's gate for whether auto-
/// generation is even attempted.
pub fn generate_down(prev_ups: &[&str], up: &str) -> Result<String, GenError> {
    let old_conn = build(prev_ups)?;
    let old = read_schema(&old_conn)?;

    let mut new_ups: Vec<&str> = prev_ups.to_vec();
    new_ups.push(up);
    let new_conn = build(&new_ups)?;
    let new = read_schema(&new_conn)?;

    let stmts = diff_down(&new_ups, &old, &new);
    let down = if stmts.is_empty() {
        // A no-op migration (e.g. data-only, handled via the DML touchpoint) has
        // an empty schema inverse.
        String::new()
    } else {
        format!("{}\n", stmts.join("\n"))
    };

    verify_round_trip(prev_ups, up, &down)?;
    Ok(down)
}

/// Applies `up` then `down` on a fresh `S(n-1)` and asserts the resulting
/// normalized schema equals `S(n-1)`'s. Used to self-check generated downs and
/// to verify hand-written ones without overwriting them.
pub fn verify_round_trip(prev_ups: &[&str], up: &str, down: &str) -> Result<(), GenError> {
    let before = build(prev_ups)?;
    let expected = dump_schema(&before)?;

    let after = build(prev_ups)?;
    after.execute_batch(up)?;
    after.execute_batch(down)?;
    let actual = dump_schema(&after)?;

    // CENSUS-RULE: downgen.round-trip
    if actual == expected {
        Ok(())
    } else {
        Err(GenError::RoundTripFailed { expected, actual })
    }
}

/// Builds the schema that results from applying `prev_ups` then `up`, dumped
/// through the shared normalizer. When `up` is the newest migration in the
/// chain, this is the full chain's schema — what `new_migration` writes to
/// `baseline/baseline.sql` after generating or verifying each migration's down
/// (the baseline is regenerated on every new migration).
pub fn full_schema_dump(prev_ups: &[&str], up: &str) -> DbResult<String> {
    let mut ups: Vec<&str> = prev_ups.to_vec();
    ups.push(up);
    let conn = build(&ups)?;
    dump_schema(&conn)
}

/// The exact anchor comment `get_migrations()` in `lib.rs` carries inside its
/// migration `vec!`. `new_migration` inserts each new `MigrationDef` entry
/// directly above this line instead of printing it for a manual paste.
pub const MIGRATION_LIST_ANCHOR: &str =
    "// new-migration:anchor — the tool inserts new MigrationDef entries directly above this line";

/// Why [`insert_migration_entry`] could not place `entry` in `lib_src`.
#[derive(Debug, PartialEq, Eq)]
pub enum InsertError {
    /// [`MIGRATION_LIST_ANCHOR`] does not appear in `lib_src` at all.
    AnchorMissing,
    /// [`MIGRATION_LIST_ANCHOR`] appears more than once — the insertion point
    /// is ambiguous.
    AnchorDuplicated,
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertError::AnchorMissing => write!(
                f,
                "no `{MIGRATION_LIST_ANCHOR}` marker found in lib.rs — add it directly above the \
                 closing `];` of get_migrations()'s migration list"
            ),
            InsertError::AnchorDuplicated => write!(
                f,
                "more than one `{MIGRATION_LIST_ANCHOR}` marker found in lib.rs — the insertion \
                 point is ambiguous; there must be exactly one"
            ),
        }
    }
}

impl std::error::Error for InsertError {}

/// Inserts `entry` (a fully formatted `MigrationDef { … },` block, indented as
/// it should appear in the source) directly above [`MIGRATION_LIST_ANCHOR`] in
/// `lib_src`, returning the updated source. Idempotent: `entry` is identified
/// by its `name: "…"` line, and if a line with that exact text is already
/// present in `lib_src`, the source is returned unchanged — rerunning the tool
/// for the same migration never duplicates the entry. Fails if the anchor is
/// missing or appears more than once, since there would then be no single
/// unambiguous insertion point.
pub fn insert_migration_entry(lib_src: &str, entry: &str) -> Result<String, InsertError> {
    let anchor_count = lib_src.matches(MIGRATION_LIST_ANCHOR).count();
    if anchor_count == 0 {
        return Err(InsertError::AnchorMissing);
    }
    if anchor_count > 1 {
        return Err(InsertError::AnchorDuplicated);
    }

    if let Some(name_line) = entry
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("name:"))
    {
        if lib_src.lines().any(|l| l.trim() == name_line) {
            return Ok(lib_src.to_string());
        }
    }

    let anchor_pos = lib_src
        .find(MIGRATION_LIST_ANCHOR)
        .expect("anchor_count == 1 checked above");
    let line_start = lib_src[..anchor_pos]
        .rfind('\n')
        .map(|i| i + 1)
        .unwrap_or(0);

    let mut out = String::with_capacity(lib_src.len() + entry.len() + 1);
    out.push_str(&lib_src[..line_start]);
    for line in entry.lines() {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(&lib_src[line_start..]);
    Ok(out)
}

/// Analyses `up` (applied after `prev_ups`) for the touchpoints the schema diff
/// cannot resolve: renames and DML on pre-existing tables.
pub fn analyze_up(prev_ups: &[&str], up: &str) -> DbResult<UpAnalysis> {
    let dml_on_existing = detect_dml_on_existing_tables(prev_ups, up)?;
    let rename = detect_rename(prev_ups, up)?;
    Ok(UpAnalysis {
        rename,
        dml_on_existing,
    })
}

/// The DML operations `up` performs against tables that already existed in
/// `S(n-1)`, as attested by SQLite's authorizer while the `up` runs. Writes into
/// tables the migration itself creates (the table-rebuild dance's temp table) are
/// not reported, because those tables are absent from the pre-existing set.
pub fn detect_dml_on_existing_tables(prev_ups: &[&str], up: &str) -> DbResult<Vec<String>> {
    // CENSUS-RULE: downgen.dml-flag
    let conn = build(prev_ups)?;
    let existing: BTreeSet<String> = read_schema(&conn)?.tables.into_keys().collect();

    // `(op, table)` DML the authorizer attributes to a pre-existing table, and
    // the set of tables the up drops. `DROP TABLE t` authorizes the implicit
    // removal of its rows as an `SQLITE_DELETE` on `t`, which is not developer
    // DML; those deletes are filtered out via the dropped-table set.
    let dml: Arc<Mutex<BTreeSet<(String, String)>>> = Arc::new(Mutex::new(BTreeSet::new()));
    let dropped: Arc<Mutex<BTreeSet<String>>> = Arc::new(Mutex::new(BTreeSet::new()));
    let dml_sink = dml.clone();
    let dropped_sink = dropped.clone();
    let existing_for_hook = existing.clone();
    conn.authorizer(Some(move |ctx: AuthContext<'_>| {
        match ctx.action {
            AuthAction::Insert { table_name } if existing_for_hook.contains(table_name) => {
                dml_sink
                    .lock()
                    .map(|mut s| s.insert(("INSERT".into(), table_name.to_string())))
                    .ok();
            }
            AuthAction::Delete { table_name } if existing_for_hook.contains(table_name) => {
                dml_sink
                    .lock()
                    .map(|mut s| s.insert(("DELETE".into(), table_name.to_string())))
                    .ok();
            }
            AuthAction::Update { table_name, .. } if existing_for_hook.contains(table_name) => {
                dml_sink
                    .lock()
                    .map(|mut s| s.insert(("UPDATE".into(), table_name.to_string())))
                    .ok();
            }
            AuthAction::DropTable { table_name } => {
                dropped_sink
                    .lock()
                    .map(|mut s| s.insert(table_name.to_string()))
                    .ok();
            }
            _ => {}
        }
        Authorization::Allow
    }))?;

    let applied = conn.execute_batch(up);
    // Always detach the authorizer before surfacing any error.
    conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>)?;
    applied?;

    let dropped = std::mem::take(&mut *dropped.lock().expect("dropped mutex poisoned"));
    let out = dml
        .lock()
        .expect("dml manifest mutex poisoned")
        .iter()
        .filter(|(_, table)| !dropped.contains(table))
        .map(|(op, table)| format!("{op} {table}"))
        .collect();
    Ok(out)
}

/// True when `up` renames a column or restructures a pre-existing table in a way
/// the schema diff cannot distinguish from a rename:
///
/// - a `RENAME COLUMN` appears in the `up` (matched outside string/identifier
///   literals and comments), or
/// - a table present in both `S(n-1)` and `S(n)` gains at least one column *and*
///   loses at least one column (a drop+add the diff cannot tell from a rename),
///   or
/// - a table is dropped whose ordered column names exactly match those of a
///   table the migration adds (a table rename expressed as drop+create).
pub fn detect_rename(prev_ups: &[&str], up: &str) -> DbResult<bool> {
    // CENSUS-RULE: downgen.rename-flag
    if contains_rename_column(up) {
        return Ok(true);
    }

    let old = read_schema(&build(prev_ups)?)?;
    let mut new_ups: Vec<&str> = prev_ups.to_vec();
    new_ups.push(up);
    let new = read_schema(&build(&new_ups)?)?;

    // A shared table with both an addition and a removal is rename-shaped.
    for (name, old_info) in &old.tables {
        if let Some(new_info) = new.tables.get(name) {
            let added = !difference(&new_info.columns, &old_info.columns).is_empty();
            let removed = !difference(&old_info.columns, &new_info.columns).is_empty();
            if added && removed {
                return Ok(true);
            }
        }
    }

    // A dropped table whose column signature matches an added table is a table
    // rename expressed as drop+create.
    let dropped: Vec<&TableInfo> = old
        .tables
        .iter()
        .filter(|(n, _)| !new.tables.contains_key(*n))
        .map(|(_, info)| info)
        .collect();
    let added: Vec<&TableInfo> = new
        .tables
        .iter()
        .filter(|(n, _)| !old.tables.contains_key(*n))
        .map(|(_, info)| info)
        .collect();
    for d in &dropped {
        if added.iter().any(|a| a.columns == d.columns) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// True when `sql` contains the keyword pair `RENAME COLUMN` outside any string
/// literal, quoted identifier, or comment. A small scanner skips `'…'`, `"…"`,
/// `[…]`, `` `…` ``, `-- …` line comments and `/* … */` block comments so the
/// words matter only where SQLite would read them as syntax.
fn contains_rename_column(sql: &str) -> bool {
    let stripped = strip_sql_noise(sql).to_ascii_uppercase();
    let mut tokens = stripped.split_whitespace().peekable();
    while let Some(tok) = tokens.next() {
        if tok == "RENAME" && tokens.peek() == Some(&"COLUMN") {
            return true;
        }
    }
    false
}

/// Replaces every string literal, quoted identifier, and comment span in `sql`
/// with a single space, leaving only the bare SQL syntax for keyword scanning.
fn strip_sql_noise(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        match c {
            '\'' | '"' | '`' => {
                let close = c;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] as char == close {
                        // A doubled delimiter is an escape, not the end.
                        if i + 1 < bytes.len() && bytes[i + 1] as char == close {
                            i += 2;
                            continue;
                        }
                        break;
                    }
                    i += 1;
                }
                out.push(' ');
            }
            '[' => {
                while i < bytes.len() && bytes[i] as char != ']' {
                    i += 1;
                }
                out.push(' ');
            }
            '-' if i + 1 < bytes.len() && bytes[i + 1] as char == '-' => {
                while i < bytes.len() && bytes[i] as char != '\n' {
                    i += 1;
                }
                out.push(' ');
                continue;
            }
            '/' if i + 1 < bytes.len() && bytes[i + 1] as char == '*' => {
                i += 2;
                while i + 1 < bytes.len()
                    && !(bytes[i] as char == '*' && bytes[i + 1] as char == '/')
                {
                    i += 1;
                }
                i += 1; // land on the '/'
                out.push(' ');
            }
            _ => out.push(c),
        }
        i += 1;
    }
    out
}
