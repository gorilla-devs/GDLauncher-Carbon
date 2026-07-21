//! Schema-diff down-generation tests (spec §10.1, CI matrix T2/T3).
//!
//! Synthetic fixtures exercise the generator against every diff shape it must
//! invert — created/dropped table, added/dropped column, added/dropped index,
//! trigger, and the rename/DML touchpoints it must refuse to auto-invert — and
//! each generated down is round-tripped: applying `up` then the generated `down`
//! must reproduce the prior schema byte-for-byte. A real-schema case proves the
//! generator works against the committed migrations' actual DDL, and a census
//! walks every committed migration that carries a down and round-trips it.

use carbon_repos::downgen::{
    analyze_up, detect_dml_on_existing_tables, detect_rename, generate_down, verify_round_trip,
    HumanAction,
};
use carbon_repos::schema_dump::dump_schema;
use rusqlite::Connection;

/// Builds a fresh in-memory DB, applies `ups` (FKs off), then applies `extra`,
/// and returns the normalized schema dump — the reference a round-trip targets.
fn dump_after(ups: &[&str], extra: &[&str]) -> String {
    let conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
    for u in ups.iter().chain(extra.iter()) {
        conn.execute_batch(u).unwrap();
    }
    dump_schema(&conn).unwrap()
}

/// Asserts the generated down for `up` (after `prev`) reproduces the prior
/// schema, and returns the generated SQL for further inspection.
fn gen_ok(prev: &[&str], up: &str) -> String {
    let down = generate_down(prev, up).unwrap_or_else(|e| panic!("generate_down failed: {e}"));
    // generate_down self-verifies, but assert it explicitly against an
    // independently-built reference too.
    let before = dump_after(prev, &[]);
    let conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
    for u in prev {
        conn.execute_batch(u).unwrap();
    }
    conn.execute_batch(up).unwrap();
    conn.execute_batch(&down).unwrap();
    assert_eq!(
        dump_schema(&conn).unwrap(),
        before,
        "generated down did not restore prior schema; down was:\n{down}"
    );
    down
}

const BASE: &str =
    "CREATE TABLE \"A\" (id INTEGER PRIMARY KEY, name TEXT NOT NULL);\
     CREATE TABLE \"B\" (id INTEGER PRIMARY KEY, aid INTEGER, note TEXT);\
     CREATE INDEX \"idx_b_aid\" ON \"B\" (aid);";

#[test]
fn created_table_is_dropped() {
    let down = gen_ok(&[BASE], "CREATE TABLE \"C\" (id INTEGER PRIMARY KEY, v TEXT);");
    assert!(down.to_uppercase().contains("DROP TABLE"), "down: {down}");
}

#[test]
fn dropped_table_is_recreated_from_old_ddl() {
    let down = gen_ok(&[BASE], "DROP INDEX \"idx_b_aid\"; DROP TABLE \"B\";");
    let up = down.to_uppercase();
    assert!(up.contains("CREATE TABLE"), "must recreate B: {down}");
    assert!(up.contains("CREATE INDEX"), "must recreate B's index: {down}");
}

#[test]
fn added_column_is_dropped() {
    let down = gen_ok(&[BASE], "ALTER TABLE \"A\" ADD COLUMN age INTEGER;");
    assert!(
        down.to_uppercase().contains("DROP COLUMN"),
        "unindexed column add should invert via DROP COLUMN: {down}"
    );
}

#[test]
fn added_indexed_column_round_trips_via_rebuild() {
    // The column is indexed, so SQLite forbids DROP COLUMN; the generator must
    // fall back to the rebuild dance. Round-trip is the assertion.
    gen_ok(
        &[BASE],
        "ALTER TABLE \"A\" ADD COLUMN slug TEXT;\
         CREATE UNIQUE INDEX \"idx_a_slug\" ON \"A\" (slug);",
    );
}

#[test]
fn column_drop_via_rebuild_dance_round_trips() {
    // The up drops A.name via the classic table-rebuild dance; the down must
    // rebuild A back to its two-column shape.
    let up = "CREATE TABLE \"A_new\" (id INTEGER PRIMARY KEY);\
              INSERT INTO \"A_new\" (id) SELECT id FROM \"A\";\
              DROP TABLE \"A\";\
              ALTER TABLE \"A_new\" RENAME TO \"A\";";
    let down = gen_ok(&[BASE], up);
    assert!(
        down.to_uppercase().contains("INSERT INTO"),
        "rebuild down should copy shared columns: {down}"
    );
}

#[test]
fn added_index_is_dropped() {
    let down = gen_ok(&[BASE], "CREATE INDEX \"idx_a_name\" ON \"A\" (name);");
    assert!(down.to_uppercase().contains("DROP INDEX"), "down: {down}");
}

#[test]
fn dropped_index_is_recreated() {
    let down = gen_ok(&[BASE], "DROP INDEX \"idx_b_aid\";");
    let up = down.to_uppercase();
    assert!(up.contains("CREATE INDEX"), "down: {down}");
    assert!(up.contains("IDX_B_AID"), "down: {down}");
}

#[test]
fn added_trigger_is_dropped() {
    let up = "CREATE TRIGGER \"trg_a\" AFTER INSERT ON \"A\" \
              BEGIN UPDATE \"B\" SET note = 'x' WHERE aid = NEW.id; END;";
    let down = gen_ok(&[BASE], up);
    assert!(down.to_uppercase().contains("DROP TRIGGER"), "down: {down}");
}

#[test]
fn rename_column_is_flagged_and_not_auto_inverted() {
    let analysis = analyze_up(&[BASE], "ALTER TABLE \"A\" RENAME COLUMN name TO title;").unwrap();
    assert!(analysis.rename, "RENAME COLUMN must be flagged");
    assert!(detect_rename(&[BASE], "ALTER TABLE \"A\" RENAME COLUMN name TO title;").unwrap());
    assert_eq!(analysis.human_actions(), vec![HumanAction::Rename]);
}

#[test]
fn rename_shaped_column_change_is_flagged() {
    // Drop one column and add another in the same up on a pre-existing table:
    // the diff cannot tell this from a rename, so it must be flagged.
    let up = "CREATE TABLE \"A_new\" (id INTEGER PRIMARY KEY, label TEXT NOT NULL);\
              INSERT INTO \"A_new\" (id) SELECT id FROM \"A\";\
              DROP TABLE \"A\";\
              ALTER TABLE \"A_new\" RENAME TO \"A\";";
    assert!(
        detect_rename(&[BASE], up).unwrap(),
        "a same-table drop+add must be flagged as rename-shaped"
    );
}

#[test]
fn rename_keyword_inside_a_string_literal_is_not_flagged() {
    // A plain additive column whose default value happens to contain the word
    // RENAME must not trip the textual detector.
    let up = "ALTER TABLE \"A\" ADD COLUMN hint TEXT DEFAULT 'RENAME COLUMN please';";
    assert!(
        !detect_rename(&[BASE], up).unwrap(),
        "RENAME inside a string literal is not a rename"
    );
    // And it still auto-generates a valid down.
    gen_ok(&[BASE], up);
}

#[test]
fn dml_on_existing_table_is_flagged_but_rebuild_temp_writes_are_not() {
    // Direct DML on a pre-existing table is flagged.
    let dml =
        detect_dml_on_existing_tables(&[BASE], "UPDATE \"A\" SET name = 'x' WHERE id = 1;").unwrap();
    assert_eq!(dml, vec!["UPDATE A".to_string()]);

    let ins =
        detect_dml_on_existing_tables(&[BASE], "INSERT INTO \"A\" (id, name) VALUES (9, 'y');")
            .unwrap();
    assert_eq!(ins, vec!["INSERT A".to_string()]);

    // The rebuild dance's INSERT targets a brand-new table, so it is NOT flagged.
    let rebuild = "CREATE TABLE \"A_new\" (id INTEGER PRIMARY KEY);\
                   INSERT INTO \"A_new\" (id) SELECT id FROM \"A\";\
                   DROP TABLE \"A\";\
                   ALTER TABLE \"A_new\" RENAME TO \"A\";";
    let dml_rebuild = detect_dml_on_existing_tables(&[BASE], rebuild).unwrap();
    assert!(
        dml_rebuild.is_empty(),
        "writes into a migration-created table are not DML on existing tables: {dml_rebuild:?}"
    );
}

#[test]
fn a_wrong_hand_written_down_fails_round_trip_verification() {
    // Planted-failure self-test: the down is valid SQL but leaves a spurious
    // table, so it must not verify.
    let up = "CREATE TABLE \"C\" (id INTEGER PRIMARY KEY);";
    let bogus_down = "CREATE TABLE \"Bogus\" (x INTEGER);";
    let err = verify_round_trip(&[BASE], up, bogus_down).unwrap_err();
    assert!(
        matches!(err, carbon_repos::downgen::GenError::RoundTripFailed { .. }),
        "a down that does not restore the prior schema must fail: {err}"
    );

    // The correct hand-written down verifies.
    verify_round_trip(&[BASE], up, "DROP TABLE \"C\";").unwrap();
}

#[test]
fn generates_and_round_trips_against_the_real_committed_schema() {
    // Append a synthetic migration to the real 25-migration chain and prove the
    // generator inverts it against the actual shipped DDL.
    let (set, _n) = carbon_repos::get_migrations();
    let prev: Vec<&str> = set.migrations.iter().map(|d| d.up_sql).collect();

    let synthetic = "ALTER TABLE \"Server\" ADD COLUMN experimentalFlag TEXT;";
    let down = gen_ok(&prev, synthetic);
    assert!(down.to_uppercase().contains("DROP COLUMN"), "down: {down}");
}

#[test]
fn every_committed_migration_with_a_down_round_trips() {
    // Auto-derived over the whole list: every migration that carries a stored
    // down must round-trip against its own predecessors. Vacuous until the first
    // post-floor migration ships a down; wired now so it activates automatically.
    let (set, _n) = carbon_repos::get_migrations();
    for (i, def) in set.migrations.iter().enumerate() {
        if let Some(down) = def.down_sql {
            let prev: Vec<&str> = set.migrations[..i].iter().map(|d| d.up_sql).collect();
            verify_round_trip(&prev, def.up_sql, down)
                .unwrap_or_else(|e| panic!("migration {} down does not round-trip: {e}", def.name));
        }
    }
}
