//! Bidirectional runner simulation tests (spec §9, CI matrix T10/T11).
//!
//! The runner takes the migration list as a parameter, so an "old binary" is
//! simulated by passing the real 25-migration list and a "new binary" by
//! passing that list extended with synthetic migrations. Cross-version behavior
//! — overlay, verified down-run, and every refusal — is exercised in-process
//! without any old binary present. The planted-failure cases (corrupt down,
//! tampered checksum, missing metadata, a down that "succeeds" into the wrong
//! schema) assert the enforcement machinery fails as it must.

use carbon_repos::compat::{MigrationDef, MigrationKind, MigrationSet, OpenVerdict, RefusalKind};
use carbon_repos::schema_dump::dump_schema;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// The real floor list (25 historical migrations), the "old binary".
fn base() -> MigrationSet {
    carbon_repos::get_migrations().0
}

/// Clones a set and appends synthetic migrations, the "new binary".
fn extend(base: &MigrationSet, extra: &[MigrationDef]) -> MigrationSet {
    let mut migrations = base.migrations.clone();
    migrations.extend_from_slice(extra);
    MigrationSet { migrations }
}

/// Additive migration 26: a brand-new table an older binary can ignore. Its
/// down drops the table (used when a later breaking migration forces the whole
/// range to be stepped back).
const ADD_WIDGET: MigrationDef = MigrationDef {
    name: "26_add_widget",
    up_sql: "CREATE TABLE Widget (id INTEGER PRIMARY KEY, label TEXT);",
    down_sql: Some("DROP TABLE Widget;"),
    kind: MigrationKind::Additive,
    data_down: "full",
};

/// Breaking migration 27: rebuilds `Widget` to drop the `label` column (the
/// classic table-rebuild). Its down restores `Widget` from the exact prior DDL,
/// as a generated down does (spec §10.1: recreate from the before-snapshot),
/// so the down-run result is byte-identical to replaying the ups to version 26.
const DROP_WIDGET_LABEL: MigrationDef = MigrationDef {
    name: "27_drop_widget_label",
    up_sql: "CREATE TABLE Widget_new (id INTEGER PRIMARY KEY);\
             INSERT INTO Widget_new (id) SELECT id FROM Widget;\
             DROP TABLE Widget;\
             ALTER TABLE Widget_new RENAME TO Widget;",
    down_sql: Some(
        "DROP TABLE Widget;\
         CREATE TABLE Widget (id INTEGER PRIMARY KEY, label TEXT);",
    ),
    kind: MigrationKind::Breaking,
    data_down: "full",
};

fn open_db(path: &Path) -> Connection {
    Connection::open(path).unwrap()
}

fn user_version(conn: &Connection) -> i32 {
    conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap()
}

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        [name],
        |r| r.get::<_, i64>(0),
    )
    .unwrap()
        > 0
}

/// A temp file DB with a stable path, so down-run snapshotting has a file to copy.
fn temp_db() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gdl_conf.db");
    (dir, path)
}

#[test]
fn fresh_install_backfills_all_metadata_rows() {
    let (_d, path) = temp_db();
    let mut conn = open_db(&path);
    let set = base();
    set.to_latest(&mut conn).unwrap();

    assert_eq!(user_version(&conn), 25);
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 25, "every applied migration is recorded");

    // Row 1 carries the real name, the sha256 of its up, and a conservative
    // breaking kind with no stored down.
    let (name, checksum, kind, down): (String, String, String, Option<String>) = conn
        .query_row(
            "SELECT name, checksum, kind, down_sql FROM _migrations WHERE version = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(name, "20240120134904_init");
    assert_eq!(checksum, set.checksum(1));
    assert_eq!(kind, "breaking");
    assert_eq!(down, None);
}

#[test]
fn upgrader_without_metadata_is_backfilled_then_reopen_is_noop() {
    // A pre-floor upgrader: schema already at 25, no `_migrations` table. The
    // legacy path leaves `user_version = 25`; the runner must backfill without
    // re-applying, then a second open must be a pure no-op.
    let (_d, path) = temp_db();
    let mut conn = open_db(&path);
    let set = base();

    // Simulate the pre-floor on-disk state: apply the ups directly and set the
    // version, but record no metadata.
    for def in &set.migrations {
        conn.execute_batch(def.up_sql).unwrap();
    }
    conn.pragma_update(None, "user_version", 25).unwrap();

    assert!(matches!(set.open(&mut conn, &path).unwrap(), OpenVerdict::Proceed));
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 25, "backfill records every already-applied migration");
    assert_eq!(user_version(&conn), 25);

    // Second open: rows present, checksums match, nothing pending.
    assert!(matches!(set.open(&mut conn, &path).unwrap(), OpenVerdict::Proceed));
}

#[test]
fn additive_ahead_overlays_and_leaves_schema_untouched() {
    let (_d, path) = temp_db();
    let l26 = extend(&base(), &[ADD_WIDGET]);
    let l25 = base();

    // New binary writes to version 26.
    {
        let mut conn = open_db(&path);
        l26.to_latest(&mut conn).unwrap();
        assert_eq!(user_version(&conn), 26);
        assert!(table_exists(&conn, "Widget"));
    }

    // Old binary opens it: overlay, touching nothing.
    {
        let mut conn = open_db(&path);
        let verdict = l25.open(&mut conn, &path).unwrap();
        assert_eq!(verdict, OpenVerdict::Proceed);
        assert_eq!(user_version(&conn), 26, "overlay leaves the newer version in place");
        assert!(table_exists(&conn, "Widget"), "overlay touches nothing");
    }

    // New binary re-opens: no-op.
    {
        let mut conn = open_db(&path);
        assert_eq!(l26.open(&mut conn, &path).unwrap(), OpenVerdict::Proceed);
        assert_eq!(user_version(&conn), 26);
    }
}

#[test]
fn breaking_ahead_down_runs_and_restores_byte_identical_schema() {
    let (_d, path) = temp_db();
    let l25 = base();
    let l27 = extend(&base(), &[ADD_WIDGET, DROP_WIDGET_LABEL]);

    // Reference: what the old binary's own schema-25 looks like.
    let reference = {
        let mut c = open_db(&path.with_extension("ref.db"));
        l25.to_latest(&mut c).unwrap();
        dump_schema(&c).unwrap()
    };

    // New binary writes to version 27 (additive 26 + breaking 27).
    {
        let mut conn = open_db(&path);
        l27.to_latest(&mut conn).unwrap();
        assert_eq!(user_version(&conn), 27);
    }

    // Old binary opens it: any breaking ahead forces a verified down-run over
    // the whole range back to 25.
    {
        let mut conn = open_db(&path);
        let verdict = l25.open(&mut conn, &path).unwrap();
        assert_eq!(verdict, OpenVerdict::Downgraded);
        assert_eq!(user_version(&conn), 25);
        assert!(!table_exists(&conn, "Widget"), "additive 26 was also stepped back");
        assert_eq!(dump_schema(&conn).unwrap(), reference, "schema is byte-identical to own 25");

        let ahead: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migrations WHERE version > 25", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ahead, 0, "stepped-back migrations' metadata rows are removed");
    }

    // The pre-downgrade snapshot is preserved.
    let snapshot = path.with_file_name("gdl_conf.pre-downgrade.db");
    assert!(snapshot.exists(), "pre-downgrade snapshot must be kept");

    // Re-upgrading from the downgraded state re-applies both migrations.
    {
        let mut conn = open_db(&path);
        assert_eq!(l27.open(&mut conn, &path).unwrap(), OpenVerdict::Proceed);
        assert_eq!(user_version(&conn), 27);
    }
}

#[test]
fn corrupt_down_rolls_back_and_leaves_the_database_intact() {
    // CENSUS-SELFTEST: compat.downgrade-corrupt-down
    let (_d, path) = temp_db();
    let bad_down = MigrationDef {
        name: "27_bad_down",
        up_sql: "CREATE TABLE Gizmo (id INTEGER PRIMARY KEY);",
        down_sql: Some("THIS IS NOT VALID SQL;"),
        kind: MigrationKind::Breaking,
        data_down: "full",
    };
    let l26 = extend(&base(), &[bad_down]);
    let l25 = base();

    {
        let mut conn = open_db(&path);
        l26.to_latest(&mut conn).unwrap();
        assert_eq!(user_version(&conn), 26);
    }

    {
        let mut conn = open_db(&path);
        let verdict = l25.open(&mut conn, &path).unwrap();
        match verdict {
            OpenVerdict::Refuse(RefusalKind::DowngradeFailed { snapshot_path }) => {
                assert!(snapshot_path.exists(), "snapshot preserved on failure");
            }
            other => panic!("expected DowngradeFailed, got {other:?}"),
        }
        // The database is untouched: still at 26 with the metadata and table.
        assert_eq!(user_version(&conn), 26);
        assert!(table_exists(&conn, "Gizmo"));
        let ahead: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migrations WHERE version = 26", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ahead, 1, "the ahead metadata row must remain after rollback");
    }
}

#[test]
fn a_down_that_succeeds_into_the_wrong_schema_is_caught() {
    // CENSUS-SELFTEST: compat.downgrade-schema-mismatch
    // Planted verifier self-test: the down runs cleanly but does not restore the
    // binary's schema. Verification must reject it and roll back.
    let (_d, path) = temp_db();
    let wrong_down = MigrationDef {
        name: "27_wrong_down",
        up_sql: "CREATE TABLE Gadget (id INTEGER PRIMARY KEY, extra TEXT);",
        // Valid SQL, but it leaves a spurious table and never drops Gadget, so
        // the resulting schema differs from the reference at 25.
        down_sql: Some("CREATE TABLE Bogus (x INTEGER);"),
        kind: MigrationKind::Breaking,
        data_down: "full",
    };
    let l26 = extend(&base(), &[wrong_down]);
    let l25 = base();

    {
        let mut conn = open_db(&path);
        l26.to_latest(&mut conn).unwrap();
    }

    {
        let mut conn = open_db(&path);
        let verdict = l25.open(&mut conn, &path).unwrap();
        assert!(
            matches!(verdict, OpenVerdict::Refuse(RefusalKind::DowngradeFailed { .. })),
            "a mismatched post-down schema must be refused, got {verdict:?}"
        );
        // Rolled back: original schema intact, Bogus never committed.
        assert_eq!(user_version(&conn), 26);
        assert!(table_exists(&conn, "Gadget"));
        assert!(!table_exists(&conn, "Bogus"), "the failed down must not persist");
    }
}

#[test]
fn tampered_checksum_is_refused_as_diverged() {
    // CENSUS-SELFTEST: compat.diverged-checksum
    let (_d, path) = temp_db();
    let set = base();
    {
        let mut conn = open_db(&path);
        set.to_latest(&mut conn).unwrap();
        conn.execute("UPDATE _migrations SET checksum = 'deadbeef' WHERE version = 5", [])
            .unwrap();
    }

    let mut conn = open_db(&path);
    match set.open(&mut conn, &path).unwrap() {
        OpenVerdict::Refuse(RefusalKind::Diverged { version }) => assert_eq!(version, 5),
        other => panic!("expected Diverged at version 5, got {other:?}"),
    }
}

#[test]
fn missing_metadata_above_own_count_is_backwards_migration() {
    // CENSUS-SELFTEST: compat.backwards-missing-metadata
    // A version ahead with no metadata row is a pre-floor database an old binary
    // cannot understand: today's refusal.
    let (_d, path) = temp_db();
    let l25 = base();
    {
        let mut conn = open_db(&path);
        l25.to_latest(&mut conn).unwrap();
        // Apply a synthetic 26 up and bump the version, but record no row.
        conn.execute_batch("CREATE TABLE Orphan (id INTEGER PRIMARY KEY);").unwrap();
        conn.pragma_update(None, "user_version", 26).unwrap();
    }

    let mut conn = open_db(&path);
    assert_eq!(
        l25.open(&mut conn, &path).unwrap(),
        OpenVerdict::Refuse(RefusalKind::BackwardsMigration)
    );
}

#[test]
fn breaking_ahead_without_a_stored_down_is_refused_and_snapshot_kept() {
    // CENSUS-SELFTEST: compat.downgrade-breaking-no-down
    // A breaking migration ahead that carries no stored down cannot be reversed:
    // the down-run must refuse (never overlay a breaking change) with the
    // pre-downgrade snapshot preserved and the database left intact.
    let (_d, path) = temp_db();
    let breaking_no_down = MigrationDef {
        name: "27_breaking_no_down",
        up_sql: "CREATE TABLE Sprocket (id INTEGER PRIMARY KEY, note TEXT);\
                 CREATE UNIQUE INDEX idx_sprocket_note ON Sprocket (note);",
        // A breaking migration deliberately shipped with no down (the historical/
        // pre-floor shape): the runner cannot step back through it.
        down_sql: None,
        kind: MigrationKind::Breaking,
        data_down: "full",
    };
    let l26 = extend(&base(), &[breaking_no_down]);
    let l25 = base();

    {
        let mut conn = open_db(&path);
        l26.to_latest(&mut conn).unwrap();
        assert_eq!(user_version(&conn), 26);
    }

    {
        let mut conn = open_db(&path);
        let verdict = l25.open(&mut conn, &path).unwrap();
        match verdict {
            OpenVerdict::Refuse(RefusalKind::DowngradeFailed { snapshot_path }) => {
                assert!(snapshot_path.exists(), "snapshot preserved when no down exists");
            }
            other => panic!("expected DowngradeFailed for a down-less breaking migration, got {other:?}"),
        }
        // Untouched: still at 26 with the table and its metadata row.
        assert_eq!(user_version(&conn), 26);
        assert!(table_exists(&conn, "Sprocket"));
        let ahead: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migrations WHERE version = 26", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ahead, 1, "the ahead metadata row must remain after refusal");
    }
}

#[test]
fn breaking_only_range_down_runs_from_intermediate_version() {
    // Opening with L26 against an L27 database steps back only the single
    // breaking migration 27, restoring the schema to 26 (Widget with its label).
    let (_d, path) = temp_db();
    let l26 = extend(&base(), &[ADD_WIDGET]);
    let l27 = extend(&base(), &[ADD_WIDGET, DROP_WIDGET_LABEL]);

    let reference_26 = {
        let mut c = open_db(&path.with_extension("ref.db"));
        l26.to_latest(&mut c).unwrap();
        dump_schema(&c).unwrap()
    };

    {
        let mut conn = open_db(&path);
        l27.to_latest(&mut conn).unwrap();
    }

    let mut conn = open_db(&path);
    assert_eq!(l26.open(&mut conn, &path).unwrap(), OpenVerdict::Downgraded);
    assert_eq!(user_version(&conn), 26);
    assert_eq!(dump_schema(&conn).unwrap(), reference_26);
}
