//! Fresh-install baseline equivalence test (spec §11, CI matrix T4).
//!
//! A truly fresh database — empty `sqlite_master`, never touched by any
//! binary — executes `baseline/baseline.sql` in one transaction instead of
//! replaying the 25-migration chain. This is the CI-fatal gate that makes
//! squash drift structurally impossible: the baseline path's end state must
//! equal the chain path's end state in every respect a caller could observe —
//! normalized schema, full `_migrations` metadata (downs included), and data
//! (empty, on both sides, on a fresh install).

use carbon_repos::compat::OpenVerdict;
use carbon_repos::schema_dump::dump_schema;
use rusqlite::Connection;
use std::time::Instant;

/// One `_migrations` row, `applied_at` excluded: the two paths run at
/// different wall-clock instants, so only the columns that must agree —
/// version, name, checksum, kind, down_sql, data_down — are compared.
#[derive(Debug, PartialEq, Eq)]
struct MigrationRow {
    version: i32,
    name: String,
    checksum: String,
    kind: String,
    down_sql: Option<String>,
    data_down: String,
}

fn migration_rows(conn: &Connection) -> Vec<MigrationRow> {
    let mut stmt = conn
        .prepare(
            "SELECT version, name, checksum, kind, down_sql, data_down \
             FROM _migrations ORDER BY version",
        )
        .unwrap();
    stmt.query_map([], |r| {
        Ok(MigrationRow {
            version: r.get(0)?,
            name: r.get(1)?,
            checksum: r.get(2)?,
            kind: r.get(3)?,
            down_sql: r.get(4)?,
            data_down: r.get(5)?,
        })
    })
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap()
}

/// Every user table's full row data, keyed by table name: each row rendered
/// as `column=value` pairs in column order via rusqlite's dynamic `ValueRef`
/// (so any column type is captured without a per-table schema), ordered by
/// `rowid`. This is the "full data dump" spec §11/T4 calls for, specialized
/// to the fresh-install case where every table is either empty or holds the
/// one documented historical seed. A future migration that smuggles seed data
/// into only one of the two paths would show up here as a mismatch.
fn table_data_dump(conn: &Connection) -> Vec<(String, Vec<String>)> {
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master \
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
             AND name NOT IN ('_migrations', '_prisma_migrations') \
             ORDER BY name",
        )
        .unwrap();
    let names: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    names
        .into_iter()
        .map(|name| {
            let mut stmt = conn
                .prepare(&format!("SELECT * FROM \"{name}\" ORDER BY rowid"))
                .unwrap();
            let col_count = stmt.column_count();
            let col_names: Vec<String> = (0..col_count)
                .map(|i| stmt.column_name(i).unwrap().to_string())
                .collect();
            let rows = stmt
                .query_map([], |row| {
                    let cells: Vec<String> = (0..col_count)
                        .map(|i| format!("{}={:?}", col_names[i], row.get_ref(i).unwrap()))
                        .collect();
                    Ok(cells.join(", "))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            (name, rows)
        })
        .collect()
}

#[test]
fn baseline_path_equals_chain_path() {
    let (set, count) = carbon_repos::get_migrations();

    // Chain path: a fresh DB migrated by replaying every historical `up`.
    let chain_dir = tempfile::tempdir().unwrap();
    let chain_path = chain_dir.path().join("chain.db");
    let mut chain_conn = Connection::open(&chain_path).unwrap();
    let chain_started = Instant::now();
    set.to_latest(&mut chain_conn).unwrap();
    let chain_elapsed = chain_started.elapsed();

    // Baseline path: an equally fresh DB opened through the production `open`
    // entrypoint, whose empty-`sqlite_master` branch executes baseline.sql.
    let baseline_dir = tempfile::tempdir().unwrap();
    let baseline_path = baseline_dir.path().join("gdl_conf.db");
    let mut baseline_conn = Connection::open(&baseline_path).unwrap();
    let baseline_started = Instant::now();
    let verdict = set.open(&mut baseline_conn, &baseline_path).unwrap();
    let baseline_elapsed = baseline_started.elapsed();

    assert_eq!(
        verdict,
        OpenVerdict::Proceed,
        "a fresh install must proceed, not refuse"
    );

    // user_version: both land at the binary's full migration count.
    let chain_uv: i32 = chain_conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    let baseline_uv: i32 = baseline_conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(chain_uv, count);
    assert_eq!(baseline_uv, count);

    // Schema: normalized dumps byte-equal (bookkeeping tables excluded by
    // `dump_schema` itself).
    assert_eq!(
        dump_schema(&chain_conn).unwrap(),
        dump_schema(&baseline_conn).unwrap(),
        "baseline-path schema must be byte-identical to the chain-path schema"
    );

    // `_migrations` metadata: every row identical apart from `applied_at`.
    assert_eq!(
        migration_rows(&chain_conn),
        migration_rows(&baseline_conn),
        "baseline-path _migrations metadata must match the chain-path backfill"
    );
    assert_eq!(migration_rows(&chain_conn).len(), count as usize);

    // Data: byte-identical full dump of every user table between the two
    // paths — the check that would catch a migration smuggling seed data into
    // only one of them. Every table is empty except the one documented
    // exception: migration `20260223000000_add_servers` seeds a default
    // `ServerGroup` row, which `HISTORICAL_SEED_DATA` in `src/compat.rs`
    // replays on the baseline path so the two stay equivalent.
    let chain_data = table_data_dump(&chain_conn);
    let baseline_data = table_data_dump(&baseline_conn);
    assert_eq!(
        chain_data, baseline_data,
        "full data dump must be identical between the two paths"
    );
    for (table, rows) in &chain_data {
        if table == "ServerGroup" {
            assert_eq!(
                rows.len(),
                1,
                "ServerGroup must hold the one documented default seed row"
            );
        } else {
            assert!(rows.is_empty(), "{table} must be empty on a fresh install");
        }
    }

    // Perf note (informational only, no assertion): the baseline path is one
    // transaction instead of `count` transactions replaying historical DML.
    println!(
        "baseline perf: chain path {:?}, baseline path {:?} ({count} migrations)",
        chain_elapsed, baseline_elapsed
    );
}

#[test]
fn reopening_a_baselined_db_is_a_pure_no_op() {
    // Once installed, re-opening with the same binary must not re-run
    // anything: same verdict, same user_version, same metadata.
    let (set, count) = carbon_repos::get_migrations();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gdl_conf.db");

    {
        let mut conn = Connection::open(&path).unwrap();
        assert_eq!(set.open(&mut conn, &path).unwrap(), OpenVerdict::Proceed);
    }

    let mut conn = Connection::open(&path).unwrap();
    assert_eq!(set.open(&mut conn, &path).unwrap(), OpenVerdict::Proceed);
    let uv: i32 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(uv, count);
    assert_eq!(migration_rows(&conn).len(), count as usize);
}

#[test]
fn a_stale_baseline_dump_is_caught_by_the_equivalence_check() {
    // Planted-failure self-test (spec §12 T11): the equivalence assertion
    // above must actually be sensitive to drift, not vacuously true. Drop one
    // table's line from a hand-built dump (simulating a `baseline.sql` that
    // fell behind the chain after a new migration) and confirm replaying it
    // produces a schema this build's real chain-path dump does NOT match.
    use carbon_repos::schema_dump::executable_statements;

    let (set, _count) = carbon_repos::get_migrations();
    let chain_dir = tempfile::tempdir().unwrap();
    let mut chain_conn = Connection::open(chain_dir.path().join("chain.db")).unwrap();
    set.to_latest(&mut chain_conn).unwrap();
    let real_dump = dump_schema(&chain_conn).unwrap();

    // Drop every line owned by the `Java` table (the table itself and its own
    // indexes), not just the `CREATE TABLE` line — a dangling index on a
    // missing table would fail outright rather than silently drift, which
    // would also prove the point but isn't what this test is demonstrating.
    let stale_dump: String = real_dump
        .lines()
        .filter(|line| line.splitn(4, '|').nth(2) != Some("Java"))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_ne!(
        stale_dump.lines().count(),
        real_dump.lines().count(),
        "must actually drop a line"
    );

    let stale_conn = Connection::open_in_memory().unwrap();
    let statements = executable_statements(&stale_dump);
    stale_conn.execute_batch(&statements.join("\n")).unwrap();

    assert_ne!(
        dump_schema(&stale_conn).unwrap(),
        real_dump,
        "a stale baseline missing a table must NOT dump equal to the real chain schema"
    );
}
