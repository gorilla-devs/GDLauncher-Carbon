//! Shared schema-dump normalizer (spec Plan 3 §T2).
//!
//! Dumps a migrated database's `sqlite_master` table into a deterministic,
//! whitespace-normalized text form so the same logical schema produces
//! byte-identical output across SQLite builds that reformat stored DDL text
//! differently. This is the acceptance test for the rusqlite/bundled-SQLite
//! version bump (Task 4): the schema snapshot test in `tests/` byte-compares
//! this dump against a committed baseline before and after the bump. Plan 4's
//! runtime verification reuses this same function against the same baseline.

use crate::db_error::DbResult;
use rusqlite::Connection;

/// Collapses `sql` down to single-spaced tokens with no leading/trailing
/// whitespace, and forces a space around `(`, `)` and `,` so punctuation
/// spacing is canonical too. Line breaks, indentation, and paren/comma
/// spacing baked into the stored DDL text never affect the dump while any
/// real schema change still does.
fn normalize_whitespace(sql: &str) -> String {
    let spaced = sql.replace('(', " ( ").replace(')', " ) ").replace(',', " , ");
    spaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Public entry to the same whitespace normalizer [`dump_schema`] applies to
/// each object's stored DDL. The down generator ([`crate::downgen`]) compares a
/// single table's before/after DDL through this so it uses the identical
/// canonical form the schema snapshot and down-run verification do — one
/// normalizer, every schema comparison.
pub fn normalize_ddl(sql: &str) -> String {
    normalize_whitespace(sql)
}

/// Dumps every `sqlite_master` row (tables, indexes, triggers, views —
/// including SQLite's own auto-indexes, whose `sql` column is `NULL`) as one
/// normalized line per object: `type|name|tbl_name|sql`. Ordered by `type`
/// then `name` so the output is stable regardless of creation order.
///
/// The migration bookkeeping tables (`_migrations`, written by the runner, and
/// the legacy `_prisma_migrations` shim) are excluded: they carry runner state,
/// not the logical schema, so schema comparisons — the committed snapshot and
/// the down-run verification in [`crate::compat`] alike — must not see them.
pub fn dump_schema(conn: &Connection) -> DbResult<String> {
    let mut stmt = conn.prepare(
        "SELECT type, name, tbl_name, sql FROM sqlite_master
         WHERE tbl_name NOT IN ('_migrations', '_prisma_migrations')
         ORDER BY type, name",
    )?;
    let rows = stmt.query_map([], |row| {
        let type_: String = row.get(0)?;
        let name: String = row.get(1)?;
        let tbl_name: String = row.get(2)?;
        let sql: Option<String> = row.get(3)?;
        Ok((type_, name, tbl_name, sql))
    })?;

    let mut out = String::new();
    for row in rows {
        let (type_, name, tbl_name, sql) = row?;
        let normalized = sql.as_deref().map(normalize_whitespace).unwrap_or_default();
        out.push_str(&format!("{type_}|{name}|{tbl_name}|{normalized}\n"));
    }
    Ok(out)
}

/// Reconstructs a dependency-safe, directly executable DDL sequence from a
/// normalized schema dump ([`dump_schema`]'s `type|name|tbl_name|sql` lines):
/// every `table` statement first, then `index`, then `trigger`, then `view`,
/// each restored to a semicolon-terminated statement. Table creation order
/// among themselves is inert (no `FOREIGN KEY` target is checked at `CREATE
/// TABLE` time regardless of the `foreign_keys` pragma), but an index or
/// trigger statement requires its owning table to already exist, hence the
/// grouping. Two kinds of line are skipped because the engine creates the
/// object itself and refuses to be told to: an empty `sql` field (SQLite's
/// implicit auto-indexes, created by their owning table's `PRIMARY KEY` /
/// `UNIQUE` constraint) and any `sqlite_`-prefixed name (`sqlite_sequence`,
/// created automatically the first time an `AUTOINCREMENT` table is created —
/// re-issuing its `CREATE TABLE` is a reserved-name error).
///
/// This is the fresh-install baseline's execution order (spec §11): the exact
/// dump the schema snapshot test byte-compares as text is, here, replayed as
/// DDL by [`crate::compat`] instead of the historical migration chain.
pub fn executable_statements(dump: &str) -> Vec<String> {
    let mut tables = Vec::new();
    let mut indexes = Vec::new();
    let mut triggers = Vec::new();
    let mut views = Vec::new();
    for line in dump.lines() {
        let mut parts = line.splitn(4, '|');
        let ty = parts.next().unwrap_or_default();
        let name = parts.next().unwrap_or_default();
        parts.next(); // tbl_name — not needed to reconstruct the statement
        let sql = parts.next().unwrap_or_default();
        if sql.is_empty() || name.starts_with("sqlite_") {
            continue;
        }
        let stmt = format!("{sql};");
        match ty {
            "table" => tables.push(stmt),
            "index" => indexes.push(stmt),
            "trigger" => triggers.push(stmt),
            "view" => views.push(stmt),
            _ => {}
        }
    }
    tables.into_iter().chain(indexes).chain(triggers).chain(views).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
        let (m, _n) = crate::get_migrations();
        m.to_latest(&mut conn).unwrap();
        (dir, conn)
    }

    #[test]
    fn dump_is_deterministic_and_ordered_by_type_then_name() {
        let (_d, conn) = migrated();
        let a = dump_schema(&conn).unwrap();
        let b = dump_schema(&conn).unwrap();
        assert_eq!(a, b, "dump must be deterministic across calls");
        assert!(a.contains("table|Account|Account|"), "must include known table:\n{a}");

        // type is non-decreasing and, within a type, name is non-decreasing.
        let mut prev: Option<(String, String)> = None;
        for line in a.lines() {
            let mut parts = line.splitn(4, '|');
            let ty = parts.next().unwrap().to_string();
            let name = parts.next().unwrap().to_string();
            if let Some((pty, pname)) = &prev {
                assert!(
                    (pty.as_str(), pname.as_str()) <= (ty.as_str(), name.as_str()),
                    "rows must be ordered by (type, name): {prev:?} then ({ty}, {name})"
                );
            }
            prev = Some((ty, name));
        }
    }

    #[test]
    fn whitespace_differences_in_stored_ddl_do_not_affect_the_dump() {
        let dir = tempfile::tempdir().unwrap();
        let conn_a = Connection::open(dir.path().join("a.db")).unwrap();
        conn_a
            .execute_batch("CREATE TABLE T (id INTEGER PRIMARY KEY, name TEXT)")
            .unwrap();
        let conn_b = Connection::open(dir.path().join("b.db")).unwrap();
        conn_b
            .execute_batch("CREATE TABLE T (\n  id   INTEGER PRIMARY KEY,\n  name TEXT\n)")
            .unwrap();

        assert_eq!(dump_schema(&conn_a).unwrap(), dump_schema(&conn_b).unwrap());
    }

    #[test]
    fn executable_statements_replay_into_the_identical_schema() {
        // The dump of the fully migrated chain, replayed through
        // `executable_statements`, must reproduce the exact same schema when
        // executed fresh — the property the fresh-install baseline path relies
        // on (spec §11).
        let (_d, conn) = migrated();
        let dump = dump_schema(&conn).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let replayed = Connection::open(dir.path().join("replayed.db")).unwrap();
        let statements = executable_statements(&dump);
        replayed.execute_batch(&statements.join("\n")).unwrap();

        assert_eq!(dump_schema(&replayed).unwrap(), dump, "replayed schema must be byte-identical");
    }

    #[test]
    fn executable_statements_order_tables_before_indexes_and_triggers() {
        // A hand-built dump listing an index before its table (the actual
        // alphabetical order `dump_schema` produces: "index" < "table") must
        // still execute successfully — tables come first regardless of the
        // dump's own line order.
        let dump = "index|T_name_idx|T|CREATE INDEX \"T_name_idx\" ON \"T\" ( \"name\" )\n\
                     table|T|T|CREATE TABLE \"T\" ( \"id\" INTEGER PRIMARY KEY , \"name\" TEXT )\n";
        let statements = executable_statements(dump);
        assert_eq!(statements.len(), 2);
        assert!(statements[0].starts_with("CREATE TABLE"), "table must come first: {statements:?}");
        assert!(statements[1].starts_with("CREATE INDEX"), "index must come after its table");

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&statements.join("\n")).unwrap();
    }

    #[test]
    fn executable_statements_skip_empty_auto_index_lines() {
        // SQLite's implicit auto-indexes dump with an empty `sql` field
        // (spec-visible in the committed baseline); nothing should be emitted
        // for them.
        let dump = "index|sqlite_autoindex_T_1|T|\n\
                     table|T|T|CREATE TABLE \"T\" ( \"id\" TEXT NOT NULL PRIMARY KEY )\n";
        let statements = executable_statements(dump);
        assert_eq!(statements, vec!["CREATE TABLE \"T\" ( \"id\" TEXT NOT NULL PRIMARY KEY );"]);
    }

    #[test]
    fn executable_statements_skip_sqlite_sequence() {
        // `sqlite_sequence` is created automatically by the engine the first
        // time an AUTOINCREMENT table is created; re-issuing its CREATE TABLE
        // is a reserved-name error, so it must never be emitted.
        let dump = "table|sqlite_sequence|sqlite_sequence|CREATE TABLE sqlite_sequence ( name , seq )\n\
                     table|T|T|CREATE TABLE \"T\" ( \"id\" INTEGER PRIMARY KEY AUTOINCREMENT )\n";
        let statements = executable_statements(dump);
        assert_eq!(
            statements,
            vec!["CREATE TABLE \"T\" ( \"id\" INTEGER PRIMARY KEY AUTOINCREMENT );"]
        );
    }
}
