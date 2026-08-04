//! Byte-identical schema baseline (spec Plan 3 §T2).
//!
//! Applies all 25 migrations to a fresh DB, dumps the normalized
//! `sqlite_master` schema through the shared [`carbon_repos::schema_dump`]
//! normalizer, and byte-compares it against the committed snapshot. This is
//! the acceptance test for the rusqlite/SQLite version bump: the bundled
//! SQLite jumps ~3.35 -> 3.4x, and this test proves the migrations still
//! produce the identical logical schema before and after.
//!
//! The committed snapshot lives at `baseline/baseline.sql` — the one file with
//! two consumers spec §11 calls for: this test byte-compares it as a dump,
//! and the fresh-install runner (`src/compat.rs`) replays it as executable
//! DDL. `tests/baseline.rs` is the CI-fatal equivalence test covering that
//! second consumer.

use carbon_repos::schema_dump::dump_schema;
use rusqlite::Connection;

const SNAPSHOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/baseline/baseline.sql");

/// Writes the current normalized dump straight to the committed snapshot
/// path so it can be generated and eyeballed. Not part of the regular suite;
/// run manually with `cargo test -p carbon_repos --test schema_snapshot --
/// --ignored generate_snapshot`.
#[test]
#[ignore]
fn generate_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("gen.db")).unwrap();
    let (migrations, _count) = carbon_repos::get_migrations();
    migrations.to_latest(&mut conn).unwrap();
    std::fs::write(SNAPSHOT, dump_schema(&conn).unwrap()).unwrap();
}

#[test]
fn migrated_schema_matches_committed_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("snapshot.db")).unwrap();
    let (migrations, count) = carbon_repos::get_migrations();
    assert_eq!(count, 25, "snapshot baseline assumes exactly 25 migrations");
    migrations.to_latest(&mut conn).unwrap();

    let dump = dump_schema(&conn).unwrap();
    let committed =
        std::fs::read_to_string(SNAPSHOT).expect("committed baseline/baseline.sql must exist");
    assert_eq!(
        dump, committed,
        "normalized schema dump no longer matches the committed baseline"
    );
}
