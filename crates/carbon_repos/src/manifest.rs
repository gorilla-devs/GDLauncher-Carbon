//! Derived migration metadata: `kind` and lossiness (spec §10.2-10.3).
//!
//! Task 2's [`crate::downgen`] generates the `down.sql` and flags the rename /
//! DML touchpoints. This module derives — never trusts — the two remaining
//! metadata fields a migration carries, so a declaration in `get_migrations()`
//! that disagrees with what the engine actually attests is a CI failure:
//!
//! - **`kind`** ([`derive_kind`]): `Additive` iff the migration only adds
//!   objects an older binary ignores, runs no DML on pre-existing tables, and
//!   introduces no constraint capable of rejecting an old binary's writes (a
//!   new `UNIQUE` index on an existing table, a `NOT NULL` column added to one,
//!   a new trigger on one, a table rebuild). Anything else is `Breaking`.
//!   Kind is derived from the schema diff plus SQLite's own authorizer
//!   (reusing [`crate::downgen::detect_dml_on_existing_tables`]), never from
//!   SQL-text parsing.
//!
//! - **lossiness** ([`DataDown`], [`seeded_lost_fields`]): a fixed-seed
//!   boundary-value generator fills every table of the *before* schema `S(n-1)`
//!   in foreign-key-topological order, then applies `up` then the stored
//!   `down` and compares the old-schema-visible projection field by field. Any
//!   field whose value did not survive the round-trip is *lost*; the declared
//!   `data_down` must name exactly that set (`full` for none) — an undeclared
//!   lost field and a declared-but-actually-preserved field both fail, in both
//!   directions.
//!
//! Every schema comparison here routes through [`crate::schema_dump`]'s
//! normalizer, the single canonical form the snapshot test and the runtime
//! down-run verification also use.

use crate::compat::MigrationKind;
use crate::db_error::{DbError, DbResult};
use crate::downgen::{build, detect_dml_on_existing_tables};
use crate::schema_dump::normalize_ddl;
use rusqlite::Connection;
use rusqlite::types::Value;
use std::collections::{BTreeMap, BTreeSet};

// ------------------------------------------------------------------------
// Schema introspection
// ------------------------------------------------------------------------

/// One column of a table, as reported by `PRAGMA table_info`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Col {
    name: String,
    /// Declared type text (may be empty for a typeless column).
    ctype: String,
    not_null: bool,
    /// Default expression text exactly as stored, or `None`.
    dflt: Option<String>,
    /// Position in the primary key (`0` = not part of the PK).
    pk: i64,
}

/// A user table: its stored `CREATE TABLE` DDL and ordered columns.
#[derive(Clone, Debug)]
struct Tbl {
    ddl: String,
    cols: Vec<Col>,
}

/// A user index keyed by name: owning table, whether it enforces uniqueness,
/// and its origin (`c` = `CREATE INDEX`, `u` = `UNIQUE` constraint, `pk` =
/// primary key).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Idx {
    table: String,
    unique: bool,
    origin: String,
}

/// Reads every user table's columns, keyed by table name.
fn read_tables(conn: &Connection) -> DbResult<BTreeMap<String, Tbl>> {
    let names: Vec<(String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT name, sql FROM sqlite_master WHERE type = 'table' \
             AND name NOT LIKE 'sqlite_%' \
             AND name NOT IN ('_migrations', '_prisma_migrations')",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let mut tables = BTreeMap::new();
    for (name, ddl) in names {
        let cols = read_columns(conn, &name)?;
        tables.insert(name, Tbl { ddl, cols });
    }
    Ok(tables)
}

/// Reads one table's columns in `cid` order.
fn read_columns(conn: &Connection, table: &str) -> DbResult<Vec<Col>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", quote(table)))?;
    let rows = stmt.query_map([], |r| {
        Ok(Col {
            name: r.get::<_, String>(1)?,
            ctype: r.get::<_, String>(2)?,
            not_null: r.get::<_, i64>(3)? != 0,
            dflt: r.get::<_, Option<String>>(4)?,
            pk: r.get::<_, i64>(5)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Every user index across all user tables, keyed by index name. Uses
/// `PRAGMA index_list` so implicit unique/primary-key indexes (whose
/// `sqlite_master.sql` is `NULL`) are included with their `unique`/`origin`
/// attested by the engine.
fn read_indexes(
    conn: &Connection,
    tables: &BTreeMap<String, Tbl>,
) -> DbResult<BTreeMap<String, Idx>> {
    let mut out = BTreeMap::new();
    for table in tables.keys() {
        let mut stmt = conn.prepare(&format!("PRAGMA index_list({})", quote(table)))?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(1)?,   // name
                r.get::<_, i64>(2)? != 0, // unique
                r.get::<_, String>(3)?,   // origin
            ))
        })?;
        for row in rows {
            let (name, unique, origin) = row?;
            out.insert(
                name,
                Idx {
                    table: table.clone(),
                    unique,
                    origin,
                },
            );
        }
    }
    Ok(out)
}

/// User triggers keyed by name: `(owning table, stored DDL)`.
fn read_triggers(conn: &Connection) -> DbResult<BTreeMap<String, (String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT name, tbl_name, sql FROM sqlite_master WHERE type = 'trigger' \
         AND name NOT LIKE 'sqlite_%'",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (name, tbl, sql) = row?;
        out.insert(name, (tbl, sql));
    }
    Ok(out)
}

/// Quotes an identifier for embedding in generated SQL.
fn quote(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

// ------------------------------------------------------------------------
// Kind derivation (spec §10.2)
// ------------------------------------------------------------------------

/// The derived kind plus the concrete reasons a migration is `Breaking` (empty
/// when `Additive`). The reasons drive readable CI failures and the generator's
/// printout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindDerivation {
    pub kind: MigrationKind,
    pub reasons: Vec<String>,
}

/// Derives whether an older binary may overlay `up` (applied after `prev_ups`)
/// or must down-run past it. See the module docs for the rule; the returned
/// kind is [`KindDerivation::kind`].
pub fn derive_kind(prev_ups: &[&str], up: &str) -> DbResult<MigrationKind> {
    Ok(derive_kind_explained(prev_ups, up)?.kind)
}

/// One foreign key whose `ON DELETE` action rejects, rather than resolves, a
/// delete of the parent row.
struct RestrictingFk {
    parent: String,
    /// The declared action, as SQLite reports it.
    action: String,
}

/// Reads `table`'s foreign keys and keeps those whose `ON DELETE` action refuses
/// the parent delete. `CASCADE`, `SET NULL` and `SET DEFAULT` resolve it and so
/// cannot reject an old binary; `RESTRICT` and `NO ACTION` (what SQLite reports
/// when the clause is omitted) refuse it.
///
/// `ON UPDATE` is deliberately not consulted. It restricts updates to the
/// *referenced key*, which here is always a synthetic primary key the app never
/// rewrites, and SQLite reports an omitted clause as `NO ACTION` — so treating
/// it as breaking would classify nearly every foreign key that way and route
/// routine migrations down the down-run path for a write nothing performs.
fn read_restricting_fks(conn: &Connection, table: &str) -> DbResult<Vec<RestrictingFk>> {
    let mut stmt = conn.prepare(&format!("PRAGMA foreign_key_list({})", quote(table)))?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(2)?, // parent table
                r.get::<_, String>(6)?, // on_delete
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .filter(|(_, on_delete)| matches!(on_delete.as_str(), "RESTRICT" | "NO ACTION"))
        .map(|(parent, action)| RestrictingFk { parent, action })
        .collect())
}

/// Kind derivation with the breaking reasons attached.
pub fn derive_kind_explained(prev_ups: &[&str], up: &str) -> DbResult<KindDerivation> {
    let old_conn = build(prev_ups)?;
    let mut new_ups: Vec<&str> = prev_ups.to_vec();
    new_ups.push(up);
    let new_conn = build(&new_ups)?;

    let old_tables = read_tables(&old_conn)?;
    let new_tables = read_tables(&new_conn)?;
    let old_idx = read_indexes(&old_conn, &old_tables)?;
    let new_idx = read_indexes(&new_conn, &new_tables)?;
    let old_trg = read_triggers(&old_conn)?;
    let new_trg = read_triggers(&new_conn)?;

    let mut reasons = Vec::new();

    // --- tables ---
    for name in old_tables.keys() {
        if !new_tables.contains_key(name) {
            reasons.push(format!("table `{name}` dropped"));
        }
    }
    for (name, old_tbl) in &old_tables {
        let Some(new_tbl) = new_tables.get(name) else {
            continue;
        };
        let old_names: BTreeSet<&str> = old_tbl.cols.iter().map(|c| c.name.as_str()).collect();
        let new_by_name: BTreeMap<&str, &Col> =
            new_tbl.cols.iter().map(|c| (c.name.as_str(), c)).collect();

        // removed columns
        for c in &old_tbl.cols {
            if !new_by_name.contains_key(c.name.as_str()) {
                reasons.push(format!("column `{name}.{}` dropped", c.name));
            }
        }
        // redefined shared columns (type / null / default / pk membership)
        for old_col in &old_tbl.cols {
            if let Some(new_col) = new_by_name.get(old_col.name.as_str()) {
                if *new_col != old_col {
                    reasons.push(format!("column `{name}.{}` redefined", old_col.name));
                }
            }
        }
        // added columns: a NOT NULL addition can reject an old binary's write
        for new_col in &new_tbl.cols {
            if !old_names.contains(new_col.name.as_str()) && new_col.not_null {
                reasons.push(format!(
                    "NOT NULL column `{name}.{}` added to a pre-existing table",
                    new_col.name
                ));
            }
        }
        // Any table-level change beyond plain column additions (a new CHECK /
        // UNIQUE / FK constraint or a full rebuild) is caught structurally:
        // reconstruct the new DDL by ADD COLUMN alone and compare.
        if normalize_ddl(&old_tbl.ddl) != normalize_ddl(&new_tbl.ddl)
            && !is_pure_column_add(prev_ups, name, old_tbl, new_tbl)?
        {
            reasons.push(format!(
                "table `{name}` altered beyond plain column additions (rebuild or new table-level constraint)"
            ));
        }
    }

    // --- foreign keys introduced by brand-new tables ---
    // A new table is invisible to an old binary, but a foreign key it declares
    // against a pre-existing parent is not: a restricting action makes the
    // engine reject the old binary's delete or key update on that parent. The
    // loop above only walks `old_tables`, so this is the one constraint class a
    // wholly-new object can impose on the old schema.
    for name in new_tables.keys() {
        if old_tables.contains_key(name) {
            continue;
        }
        for fk in read_restricting_fks(&new_conn, name)? {
            if old_tables.contains_key(&fk.parent) {
                reasons.push(format!(
                    "new table `{name}` declares ON DELETE {} against pre-existing table `{}`",
                    fk.action, fk.parent
                ));
            }
        }
    }

    // --- indexes ---
    for (name, old_i) in &old_idx {
        match new_idx.get(name) {
            Some(new_i) if new_i == old_i => {}
            _ => reasons.push(format!("index `{name}` dropped or changed")),
        }
    }
    for (name, new_i) in &new_idx {
        if !old_idx.contains_key(name) && new_i.unique && old_tables.contains_key(&new_i.table) {
            reasons.push(format!(
                "new UNIQUE index `{name}` on pre-existing table `{}`",
                new_i.table
            ));
        }
    }

    // --- triggers ---
    for (name, old_t) in &old_trg {
        match new_trg.get(name) {
            Some(new_t) if new_t == old_t => {}
            _ => reasons.push(format!("trigger `{name}` dropped or changed")),
        }
    }
    for (name, (tbl, _)) in &new_trg {
        if !old_trg.contains_key(name) && old_tables.contains_key(tbl) {
            reasons.push(format!(
                "new trigger `{name}` on pre-existing table `{tbl}`"
            ));
        }
    }

    // --- DML on pre-existing tables (engine-attested) ---
    for op in detect_dml_on_existing_tables(prev_ups, up)? {
        reasons.push(format!("DML on a pre-existing table: {op}"));
    }

    let kind = if reasons.is_empty() {
        MigrationKind::Additive
    } else {
        MigrationKind::Breaking
    };
    Ok(KindDerivation { kind, reasons })
}

/// True when the only difference between `old_tbl` and `new_tbl` is appended
/// columns: applying `ALTER TABLE … ADD COLUMN` (reconstructed from the new
/// pragma column list) to the old schema reproduces the new table's DDL
/// exactly. A hidden `CHECK`/`UNIQUE` clause or a rebuild makes the
/// reconstruction diverge (or the `ALTER` fail), so this returns false and the
/// caller classifies the migration `Breaking`.
fn is_pure_column_add(
    prev_ups: &[&str],
    table: &str,
    old_tbl: &Tbl,
    new_tbl: &Tbl,
) -> DbResult<bool> {
    let old_names: BTreeSet<&str> = old_tbl.cols.iter().map(|c| c.name.as_str()).collect();
    // Any removed column rules out a pure add.
    let new_names: BTreeSet<&str> = new_tbl.cols.iter().map(|c| c.name.as_str()).collect();
    if old_tbl
        .cols
        .iter()
        .any(|c| !new_names.contains(c.name.as_str()))
    {
        return Ok(false);
    }

    let conn = build(prev_ups)?;
    for col in &new_tbl.cols {
        if old_names.contains(col.name.as_str()) {
            continue;
        }
        let stmt = format!(
            "ALTER TABLE {} ADD COLUMN {}",
            quote(table),
            render_col_def(col)
        );
        if conn.execute_batch(&stmt).is_err() {
            return Ok(false);
        }
    }
    let got: Result<String, _> = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |r| r.get(0),
    );
    Ok(match got {
        Ok(ddl) => ddl_eq_ignoring_identifier_quotes(&ddl, &new_tbl.ddl),
        Err(_) => false,
    })
}

/// Compares two table DDLs through the shared whitespace normalizer, ignoring
/// identifier double-quoting. The reconstruction quotes every added column name
/// (`"age"`) while a hand-written `ALTER TABLE ADD COLUMN age …` may not, so the
/// pure-column-add check must treat `"age"` and `age` as the same identifier;
/// single-quoted string defaults are unaffected since they use single quotes.
fn ddl_eq_ignoring_identifier_quotes(a: &str, b: &str) -> bool {
    normalize_ddl(a).replace('"', "") == normalize_ddl(b).replace('"', "")
}

/// Renders a column definition for an `ALTER TABLE … ADD COLUMN` from its
/// pragma metadata.
fn render_col_def(col: &Col) -> String {
    let mut out = quote(&col.name);
    if !col.ctype.is_empty() {
        out.push(' ');
        out.push_str(&col.ctype);
    }
    if col.not_null {
        out.push_str(" NOT NULL");
    }
    if let Some(dflt) = &col.dflt {
        out.push_str(" DEFAULT ");
        out.push_str(dflt);
    }
    out
}

// ------------------------------------------------------------------------
// Lossiness declaration + derivation (spec §10.3)
// ------------------------------------------------------------------------

/// The parsed `data_down` declaration a migration carries. `Full` means the
/// down restores every field; `Partial` names the exact `Table.column` fields
/// the down does not restore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataDown {
    Full,
    Partial(BTreeSet<String>),
}

impl DataDown {
    /// Parses the stored `data_down` text (`full` or `partial:Table.col,…`).
    pub fn parse(s: &str) -> Result<DataDown, String> {
        let s = s.trim();
        if s == "full" {
            return Ok(DataDown::Full);
        }
        if let Some(rest) = s.strip_prefix("partial:") {
            let fields: BTreeSet<String> = rest
                .split(',')
                .map(str::trim)
                .filter(|f| !f.is_empty())
                .map(str::to_string)
                .collect();
            if fields.is_empty() {
                return Err("`partial:` declares no fields".to_string());
            }
            return Ok(DataDown::Partial(fields));
        }
        Err(format!(
            "unrecognized data_down `{s}` (expected `full` or `partial:Table.col,…`)"
        ))
    }

    /// The stored text form of this declaration.
    pub fn to_declaration(&self) -> String {
        match self {
            DataDown::Full => "full".to_string(),
            DataDown::Partial(fields) => {
                format!(
                    "partial:{}",
                    fields.iter().cloned().collect::<Vec<_>>().join(",")
                )
            }
        }
    }

    /// The declared lost-field set (empty for `Full`).
    fn declared_fields(&self) -> BTreeSet<String> {
        match self {
            DataDown::Full => BTreeSet::new(),
            DataDown::Partial(f) => f.clone(),
        }
    }
}

/// A failure of the derived metadata against its declaration.
#[derive(Debug)]
pub enum VerifyError {
    /// The declared kind disagrees with the derived one.
    KindMismatch {
        declared: MigrationKind,
        derived: MigrationKind,
        reasons: Vec<String>,
    },
    /// The declared `data_down` does not match the fields the seeded round-trip
    /// proved lost. `missing` = lost but undeclared; `stale` = declared but
    /// preserved.
    DataDownMismatch {
        declared: BTreeSet<String>,
        actual: BTreeSet<String>,
        missing: BTreeSet<String>,
        stale: BTreeSet<String>,
    },
    /// The declaration text could not be parsed.
    BadDeclaration(String),
    /// A DB-layer failure while deriving.
    Db(DbError),
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifyError::KindMismatch {
                declared,
                derived,
                reasons,
            } => write!(
                f,
                "declared kind `{}` but derived `{}` ({})",
                declared.as_str(),
                derived.as_str(),
                if reasons.is_empty() {
                    "additions-only".to_string()
                } else {
                    reasons.join("; ")
                }
            ),
            VerifyError::DataDownMismatch {
                declared,
                actual,
                missing,
                stale,
            } => write!(
                f,
                "data_down mismatch: declared {declared:?}, round-trip lost {actual:?} \
                 (undeclared lost: {missing:?}; stale declared: {stale:?})"
            ),
            VerifyError::BadDeclaration(s) => write!(f, "bad data_down declaration: {s}"),
            VerifyError::Db(e) => write!(f, "db error deriving metadata: {e}"),
        }
    }
}

impl std::error::Error for VerifyError {}

impl From<DbError> for VerifyError {
    fn from(e: DbError) -> Self {
        VerifyError::Db(e)
    }
}

/// Verifies a migration's declared `kind` equals the derived kind (both
/// directions). Called by the CI walk over `get_migrations()` and by the
/// planted-failure self-tests.
pub fn verify_kind(
    prev_ups: &[&str],
    up: &str,
    declared: MigrationKind,
) -> Result<(), VerifyError> {
    let derivation = derive_kind_explained(prev_ups, up)?;
    // CENSUS-RULE: manifest.kind-derivation
    if derivation.kind == declared {
        Ok(())
    } else {
        Err(VerifyError::KindMismatch {
            declared,
            derived: derivation.kind,
            reasons: derivation.reasons,
        })
    }
}

/// Verifies a migration's declared `data_down` names exactly the fields the
/// seeded round-trip proves the stored `down` fails to restore.
pub fn verify_data_down(
    prev_ups: &[&str],
    up: &str,
    down: &str,
    declared: &str,
) -> Result<(), VerifyError> {
    let declared = DataDown::parse(declared).map_err(VerifyError::BadDeclaration)?;
    let actual = seeded_lost_fields(prev_ups, up, down)?;
    let declared_fields = declared.declared_fields();

    // CENSUS-RULE: manifest.data-down-undeclared-lost
    let missing: BTreeSet<String> = actual.difference(&declared_fields).cloned().collect();
    // CENSUS-RULE: manifest.data-down-stale-declared
    let stale: BTreeSet<String> = declared_fields.difference(&actual).cloned().collect();
    if missing.is_empty() && stale.is_empty() {
        Ok(())
    } else {
        Err(VerifyError::DataDownMismatch {
            declared: declared_fields,
            actual,
            missing,
            stale,
        })
    }
}

// ------------------------------------------------------------------------
// Seeded boundary-value round-trip (spec §10.3 item 4 / CI T5)
// ------------------------------------------------------------------------

/// Seeds every table of `S(n-1)` with deterministic boundary-value rows in
/// foreign-key-topological order, applies `up` then `down`, and returns the set
/// of `Table.column` fields whose value did not survive the round-trip.
///
/// A field is *lost* when, for some seeded row (matched by primary key), the
/// value visible under the old schema after `up`+`down` differs from what was
/// seeded — or the row/table is gone entirely. An empty result is a
/// proven-lossless down.
pub fn seeded_lost_fields(prev_ups: &[&str], up: &str, down: &str) -> DbResult<BTreeSet<String>> {
    let conn = build(prev_ups)?;
    let old_tables = read_tables(&conn)?;
    let order = fk_topological_order(&conn, &old_tables)?;

    let mut counter: i64 = 0;
    let mut inserted: BTreeMap<String, Vec<BTreeMap<String, Value>>> = BTreeMap::new();
    for table in &order {
        let rows = seed_table(&conn, table, &old_tables[table], &inserted, &mut counter)?;
        inserted.insert(table.clone(), rows);
    }

    // Snapshot the seeded data (over the old schema) before mutating anything.
    let baseline = read_all_data(&conn, &old_tables)?;

    // Roll forward then back on the same connection.
    conn.execute_batch(up)?;
    conn.execute_batch(down)?;

    let after = read_all_data(&conn, &old_tables)?;

    let mut lost = BTreeSet::new();
    for (table, tbl) in &old_tables {
        let pk_cols = pk_column_names(tbl);
        let base_rows = &baseline[table];
        let after_rows = after.get(table);

        // Index the post-round-trip rows by primary key (or whole row when the
        // table has no declared PK).
        let after_by_key: BTreeMap<Vec<String>, &BTreeMap<String, Value>> = match after_rows {
            Some(rows) => rows
                .iter()
                .map(|r| (row_key(r, &pk_cols, tbl), r))
                .collect(),
            None => BTreeMap::new(),
        };

        for base in base_rows {
            let key = row_key(base, &pk_cols, tbl);
            match after_by_key.get(&key) {
                Some(after_row) => {
                    for col in &tbl.cols {
                        let b = base.get(&col.name);
                        let a = after_row.get(&col.name);
                        if b != a {
                            lost.insert(format!("{table}.{}", col.name));
                        }
                    }
                }
                None => {
                    // The whole seeded row is gone: every non-key field is lost.
                    for col in &tbl.cols {
                        if col.pk == 0 {
                            lost.insert(format!("{table}.{}", col.name));
                        }
                    }
                }
            }
        }
    }
    Ok(lost)
}

/// The primary-key column names of `tbl` in key order (empty when the table has
/// no declared primary key).
fn pk_column_names(tbl: &Tbl) -> Vec<String> {
    let mut pk: Vec<&Col> = tbl.cols.iter().filter(|c| c.pk > 0).collect();
    pk.sort_by_key(|c| c.pk);
    pk.into_iter().map(|c| c.name.clone()).collect()
}

/// The comparison key for a row: its primary-key values, or — when the table
/// has no primary key — the whole row rendered in column order (so identical
/// rows still match). Each value is type-tagged into an orderable string so the
/// key works as a `BTreeMap` key ([`rusqlite::types::Value`] is not `Ord`).
fn row_key(row: &BTreeMap<String, Value>, pk_cols: &[String], tbl: &Tbl) -> Vec<String> {
    let cols: Vec<&String> = if pk_cols.is_empty() {
        tbl.cols.iter().map(|c| &c.name).collect()
    } else {
        pk_cols.iter().collect()
    };
    cols.into_iter()
        .map(|c| encode_value(row.get(c).unwrap_or(&Value::Null)))
        .collect()
}

/// A stable, orderable, type-tagged encoding of a value for use as a map key.
fn encode_value(v: &Value) -> String {
    match v {
        Value::Null => "N".to_string(),
        Value::Integer(i) => format!("I{i}"),
        Value::Real(f) => format!("R{:016x}", f.to_bits()),
        Value::Text(s) => format!("T{s}"),
        Value::Blob(b) => format!("B{}", hex::encode(b)),
    }
}

/// Reads every user table's rows as `column -> value` maps.
fn read_all_data(
    conn: &Connection,
    tables: &BTreeMap<String, Tbl>,
) -> DbResult<BTreeMap<String, Vec<BTreeMap<String, Value>>>> {
    let mut out = BTreeMap::new();
    for (name, tbl) in tables {
        // A table the down failed to restore reads as no rows; the caller then
        // treats every seeded row as lost.
        let rows = read_table_rows(conn, name, tbl).unwrap_or_default();
        out.insert(name.clone(), rows);
    }
    Ok(out)
}

/// Reads one table's rows over its declared columns.
fn read_table_rows(
    conn: &Connection,
    table: &str,
    tbl: &Tbl,
) -> DbResult<Vec<BTreeMap<String, Value>>> {
    let col_list = tbl
        .cols
        .iter()
        .map(|c| quote(&c.name))
        .collect::<Vec<_>>()
        .join(", ");
    let mut stmt = conn.prepare(&format!("SELECT {col_list} FROM {}", quote(table)))?;
    let names: Vec<String> = tbl.cols.iter().map(|c| c.name.clone()).collect();
    let rows = stmt.query_map([], |r| {
        let mut map = BTreeMap::new();
        for (i, name) in names.iter().enumerate() {
            map.insert(name.clone(), r.get::<_, Value>(i)?);
        }
        Ok(map)
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Orders `tables` so every table sorts after all tables it references by a
/// foreign key (parents before children). Self-references are ignored; a
/// reference cycle falls back to name order for the tables it entangles (the
/// seeder tolerates a best-effort order because it inserts with FKs OFF).
fn fk_topological_order(
    conn: &Connection,
    tables: &BTreeMap<String, Tbl>,
) -> DbResult<Vec<String>> {
    // parents[t] = set of tables t references.
    let mut parents: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for name in tables.keys() {
        let mut stmt = conn.prepare(&format!("PRAGMA foreign_key_list({})", quote(name)))?;
        let refs = stmt
            .query_map([], |r| r.get::<_, String>(2))? // parent table
            .collect::<Result<Vec<_>, _>>()?;
        let set = parents.entry(name.clone()).or_default();
        for parent in refs {
            if &parent != name && tables.contains_key(&parent) {
                set.insert(parent);
            }
        }
    }

    // Kahn's algorithm over the parents relation, breaking ties (and any cycle)
    // by name for determinism.
    let mut ordered = Vec::new();
    let mut placed: BTreeSet<String> = BTreeSet::new();
    while ordered.len() < tables.len() {
        let ready: Option<String> = tables
            .keys()
            .filter(|t| !placed.contains(*t))
            .find(|t| parents[*t].iter().all(|p| placed.contains(p)))
            .cloned();
        let next = match ready {
            Some(t) => t,
            // A cycle: take the lowest-named unplaced table to make progress.
            None => tables
                .keys()
                .find(|t| !placed.contains(*t))
                .cloned()
                .expect("unplaced table exists while loop runs"),
        };
        placed.insert(next.clone());
        ordered.push(next);
    }
    Ok(ordered)
}

/// Inserts the boundary-value rows for one table, drawing foreign-key values
/// from already-inserted parent rows. Returns the inserted rows as
/// `column -> value` maps for later children to reference.
fn seed_table(
    conn: &Connection,
    table: &str,
    tbl: &Tbl,
    inserted: &BTreeMap<String, Vec<BTreeMap<String, Value>>>,
    counter: &mut i64,
) -> DbResult<Vec<BTreeMap<String, Value>>> {
    // Foreign-key groups: parent table + positionally-paired (from, to) columns.
    let fks = read_fk_groups(conn, table)?;
    // Columns that must be unique (primary key or any unique index member).
    let unique_cols = unique_column_set(conn, table, tbl)?;

    // Five rows so a NOT NULL column sees every integer boundary (0, -1,
    // i64::MAX, i64::MIN) and every string/blob boundary, plus a fifth row
    // exercising a mixed-case ASCII string and, for DATETIME-declared columns,
    // a TEXT-shaped datetime value; every table gets the same count, so a
    // child in FK-topological order always finds enough distinct parent rows
    // to key against.
    let rows_per_table = 5;
    let mut rows = Vec::new();
    for row_i in 0..rows_per_table {
        let mut row: BTreeMap<String, Value> = BTreeMap::new();

        // Foreign-key columns first: reference a distinct parent row per row_i.
        for fk in &fks {
            let parent_rows = inserted.get(&fk.parent);
            match parent_rows.filter(|r| !r.is_empty()) {
                Some(parent_rows) => {
                    let parent = &parent_rows[row_i % parent_rows.len()];
                    for (from, to) in fk.pairs.iter() {
                        let v = parent.get(to).cloned().unwrap_or(Value::Null);
                        row.insert(from.clone(), v);
                    }
                }
                None => {
                    // No parent row to point at: null the reference (valid only
                    // when all its child columns are nullable; otherwise the
                    // insert surfaces the problem).
                    for (from, _to) in fk.pairs.iter() {
                        row.insert(from.clone(), Value::Null);
                    }
                }
            }
        }

        // Remaining columns.
        for col in &tbl.cols {
            if row.contains_key(&col.name) {
                continue;
            }
            let must_be_unique = unique_cols.contains(&col.name);
            let value = boundary_value(col, row_i, must_be_unique, counter);
            row.insert(col.name.clone(), value);
        }

        insert_row(conn, table, tbl, &row)?;
        rows.push(row);
    }
    Ok(rows)
}

/// A foreign-key constraint's parent table and its `(child_from, parent_to)`
/// column pairs.
struct FkGroup {
    parent: String,
    pairs: Vec<(String, String)>,
}

/// Reads `table`'s foreign keys grouped by constraint id.
fn read_fk_groups(conn: &Connection, table: &str) -> DbResult<Vec<FkGroup>> {
    let mut stmt = conn.prepare(&format!("PRAGMA foreign_key_list({})", quote(table)))?;
    let raw = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,            // id
                r.get::<_, String>(2)?,         // parent table
                r.get::<_, String>(3)?,         // from
                r.get::<_, Option<String>>(4)?, // to (null => parent PK, but our schema names it)
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut by_id: BTreeMap<i64, FkGroup> = BTreeMap::new();
    for (id, parent, from, to) in raw {
        let entry = by_id.entry(id).or_insert_with(|| FkGroup {
            parent: parent.clone(),
            pairs: Vec::new(),
        });
        // `to` is only null when the parent is referenced by its rowid; the
        // schema always names the referenced column, so fall back to `from`.
        entry.pairs.push((from.clone(), to.unwrap_or(from)));
    }
    Ok(by_id.into_values().collect())
}

/// The set of columns that must be unique in `table`: every primary-key column
/// and every column participating in a unique index. A per-row unique value in
/// any of these keeps the seed's primary keys and unique constraints
/// collision-free.
fn unique_column_set(conn: &Connection, table: &str, tbl: &Tbl) -> DbResult<BTreeSet<String>> {
    let mut set: BTreeSet<String> = tbl
        .cols
        .iter()
        .filter(|c| c.pk > 0)
        .map(|c| c.name.clone())
        .collect();

    // Unique indexes (explicit and constraint-implicit) and their columns.
    let unique_index_names: Vec<String> = {
        let mut stmt = conn.prepare(&format!("PRAGMA index_list({})", quote(table)))?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(1)?, r.get::<_, i64>(2)? != 0))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|(_, unique)| *unique)
            .map(|(name, _)| name)
            .collect()
    };
    for index in unique_index_names {
        let mut stmt = conn.prepare(&format!("PRAGMA index_info({})", quote(&index)))?;
        let cols = stmt
            .query_map([], |r| r.get::<_, Option<String>>(2))? // column name (null for expressions)
            .collect::<Result<Vec<_>, _>>()?;
        for col in cols.into_iter().flatten() {
            set.insert(col);
        }
    }
    Ok(set)
}

/// A deterministic boundary value for `col` on row `row_i`. Unique columns get
/// a monotonically increasing value so primary keys and unique indexes never
/// collide; other columns get affinity-appropriate boundary values, with NULL
/// used on the second row where the column allows it.
fn boundary_value(col: &Col, row_i: usize, must_be_unique: bool, counter: &mut i64) -> Value {
    let affinity = type_affinity(&col.ctype);

    if must_be_unique {
        *counter += 1;
        let n = *counter;
        return match affinity {
            Affinity::Integer | Affinity::Real | Affinity::Numeric => Value::Integer(n),
            Affinity::Blob => Value::Blob(vec![0u8, (n & 0xff) as u8, 0xff]),
            Affinity::Text => Value::Text(format!("k{n}")),
        };
    }

    // Nullable, non-unique column: exercise NULL on the second row (the other
    // rows still carry boundary values so the column's data survival is tested).
    if !col.not_null && row_i == 1 {
        return Value::Null;
    }

    match affinity {
        Affinity::Integer | Affinity::Numeric => {
            // A DATETIME-declared column's 5th row is a TEXT datetime string
            // rather than another integer boundary: `DbDateTime` stores these
            // as epoch-millis integers, but a TEXT-shaped datetime value is a
            // real representation this column's declared type admits, and
            // nothing else here ever seeds one. A future down that only
            // handles the integer shape (e.g. an arithmetic rewrite) would
            // otherwise pass this round-trip proof by never encountering the
            // shape it mishandles.
            if row_i == 4 && is_datetime_declared(&col.ctype) {
                return Value::Text("2024-04-10 20:56:05".to_string());
            }
            let choices = [0i64, -1, i64::MAX, i64::MIN];
            Value::Integer(choices[row_i % choices.len()])
        }
        Affinity::Real => {
            let choices = [0.0f64, -1.5, 1e300, -1e-300];
            Value::Real(choices[row_i % choices.len()])
        }
        Affinity::Text => {
            // Empty string, unicode-nasty strings (no embedded NUL — SQLite
            // truncates TEXT at NUL; NUL bytes are exercised in BLOB columns),
            // and a mixed-case ASCII string: without it, every prior choice is
            // either already lowercase or has no case at all, so a future down
            // that silently lowercases (or uppercases) a TEXT column would
            // falsely verify as lossless.
            let choices = [
                "",
                "café🔥\u{1F9FF}'\"\\—",
                "  spaced  ",
                "𝕏𝕐𝕫",
                "AbC xYz 0Z",
            ];
            Value::Text(choices[row_i % choices.len()].to_string())
        }
        Affinity::Blob => {
            // Blobs including a 0x00 byte in the middle, plus an empty blob.
            let choices: [Vec<u8>; 4] = [
                vec![0u8, 1, 2, 0, 255],
                vec![],
                vec![0u8, 0, 0],
                vec![255u8, 0, 128],
            ];
            Value::Blob(choices[row_i % choices.len()].clone())
        }
    }
}

/// SQLite type affinity buckets the seeder distinguishes.
enum Affinity {
    Integer,
    Text,
    Blob,
    Real,
    Numeric,
}

/// The SQLite affinity of a declared column type (the standard rules).
fn type_affinity(ctype: &str) -> Affinity {
    let t = ctype.to_ascii_uppercase();
    if t.contains("INT") {
        Affinity::Integer
    } else if t.contains("CHAR") || t.contains("CLOB") || t.contains("TEXT") {
        Affinity::Text
    } else if t.is_empty() || t.contains("BLOB") {
        Affinity::Blob
    } else if t.contains("REAL") || t.contains("FLOA") || t.contains("DOUB") {
        Affinity::Real
    } else {
        // BOOLEAN, DATETIME, NUMERIC, DECIMAL, … → NUMERIC affinity.
        Affinity::Numeric
    }
}

/// True when `ctype`'s declared type names a DATE/TIME column, case-
/// insensitively — the `DATETIME` columns [`crate::dbtypes::DbDateTime`]
/// stores as epoch-millis integers, which [`type_affinity`] buckets under
/// `Numeric` (no `INT`/`CHAR`/`CLOB`/`TEXT`/`BLOB`/`REAL`/`FLOA`/`DOUB`
/// substring matches). Used only to pick the boundary seeder's 5th-row value
/// for such a column; it is not a distinct [`Affinity`] variant.
fn is_datetime_declared(ctype: &str) -> bool {
    let t = ctype.to_ascii_uppercase();
    t.contains("DATE") || t.contains("TIME")
}

/// Inserts one fully-specified row.
fn insert_row(
    conn: &Connection,
    table: &str,
    tbl: &Tbl,
    row: &BTreeMap<String, Value>,
) -> DbResult<()> {
    let cols: Vec<&Col> = tbl.cols.iter().collect();
    let col_list = cols
        .iter()
        .map(|c| quote(&c.name))
        .collect::<Vec<_>>()
        .join(", ");
    let placeholders = (1..=cols.len())
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let values: Vec<&dyn rusqlite::ToSql> = cols
        .iter()
        .map(|c| row.get(&c.name).unwrap() as &dyn rusqlite::ToSql)
        .collect();
    conn.execute(
        &format!(
            "INSERT INTO {} ({col_list}) VALUES ({placeholders})",
            quote(table)
        ),
        values.as_slice(),
    )?;
    Ok(())
}
