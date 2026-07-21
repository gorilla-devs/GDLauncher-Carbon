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
}
