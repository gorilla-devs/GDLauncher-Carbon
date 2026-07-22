//! Derived kind + lossiness tests (spec §10.2-10.3, CI matrix T5/T6/T11).
//!
//! Kind derivation is exercised across every diff shape (additive column/table/
//! non-unique index vs breaking drop/rebuild/unique-index/trigger/DML), both
//! directions of the declared-equals-derived gate are checked, the fixed-seed
//! boundary-value round-trip proves lossless vs lossy downs, and a planted-
//! failure block asserts the enforcement machinery FAILS a mislabeled kind, an
//! undeclared lossy drop, and a stale partial declaration — the fence is
//! fence-tested. A real-schema smoke test seeds the full committed schema in
//! FK-topological order to prove the generator handles composite keys and FKs.

use carbon_repos::compat::MigrationKind;
use carbon_repos::downgen::generate_down;
use carbon_repos::manifest::{
    DataDown, VerifyError, derive_kind, seeded_lost_fields, verify_data_down, verify_kind,
};
use std::collections::BTreeSet;

const BASE: &str = "CREATE TABLE \"A\" (id INTEGER PRIMARY KEY, name TEXT NOT NULL, note TEXT);\
     CREATE TABLE \"B\" (id INTEGER PRIMARY KEY, aid INTEGER, label TEXT);\
     CREATE INDEX \"idx_b_aid\" ON \"B\" (aid);";

fn fields(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// ------------------------------------------------------------------------
// Kind derivation (spec §10.2)
// ------------------------------------------------------------------------

#[test]
fn new_table_is_additive() {
    let kind = derive_kind(
        &[BASE],
        "CREATE TABLE \"C\" (id INTEGER PRIMARY KEY, v TEXT);",
    )
    .unwrap();
    assert_eq!(kind, MigrationKind::Additive);
}

#[test]
fn nullable_column_add_is_additive() {
    let kind = derive_kind(&[BASE], "ALTER TABLE \"A\" ADD COLUMN age INTEGER;").unwrap();
    assert_eq!(kind, MigrationKind::Additive);
}

#[test]
fn non_unique_index_on_existing_table_is_additive() {
    // A non-unique index does not reject any write, so it overlays cleanly.
    let kind = derive_kind(&[BASE], "CREATE INDEX \"idx_a_name\" ON \"A\" (name);").unwrap();
    assert_eq!(kind, MigrationKind::Additive);
}

#[test]
fn new_table_with_a_unique_index_is_additive() {
    // The unique index is on a brand-new table an old binary never writes.
    let up = "CREATE TABLE \"C\" (id INTEGER PRIMARY KEY, slug TEXT);\
              CREATE UNIQUE INDEX \"idx_c_slug\" ON \"C\" (slug);";
    assert_eq!(derive_kind(&[BASE], up).unwrap(), MigrationKind::Additive);
}

#[test]
fn new_table_with_restricting_fk_to_existing_table_is_breaking() {
    // RESTRICT on a new child table rejects the old binary's DELETE against the
    // pre-existing parent, which is exactly the constraint class `Additive`
    // promises is absent. The existing schema already uses RESTRICT.
    let kind = derive_kind(
        &[BASE],
        "CREATE TABLE \"C\" (id INTEGER PRIMARY KEY, aid INTEGER NOT NULL, \
         CONSTRAINT \"c_aid_fkey\" FOREIGN KEY (aid) REFERENCES \"A\" (id) ON DELETE RESTRICT);",
    )
    .unwrap();
    assert_eq!(kind, MigrationKind::Breaking);
}

#[test]
fn new_table_with_default_no_action_fk_to_existing_table_is_breaking() {
    // Omitting ON DELETE leaves NO ACTION, which rejects the parent delete just
    // as RESTRICT does at the point the statement completes.
    let kind = derive_kind(
        &[BASE],
        "CREATE TABLE \"C\" (id INTEGER PRIMARY KEY, aid INTEGER NOT NULL, \
         CONSTRAINT \"c_aid_fkey\" FOREIGN KEY (aid) REFERENCES \"A\" (id));",
    )
    .unwrap();
    assert_eq!(kind, MigrationKind::Breaking);
}

#[test]
fn restricting_on_update_alone_does_not_make_a_new_table_breaking() {
    // Only ON DELETE is consulted. ON UPDATE restricts rewrites of the
    // referenced key, which is always a synthetic primary key nothing updates,
    // and SQLite reports an omitted clause as NO ACTION — so honouring it would
    // classify nearly every foreign key as breaking.
    let kind = derive_kind(
        &[BASE],
        "CREATE TABLE \"C\" (id INTEGER PRIMARY KEY, aid INTEGER NOT NULL, \
         CONSTRAINT \"c_aid_fkey\" FOREIGN KEY (aid) REFERENCES \"A\" (id) \
         ON UPDATE RESTRICT ON DELETE CASCADE);",
    )
    .unwrap();
    assert_eq!(kind, MigrationKind::Additive);
}

#[test]
fn new_table_with_non_restricting_fk_to_existing_table_is_additive() {
    // CASCADE and SET NULL resolve the parent delete instead of rejecting it,
    // so an old binary's writes still succeed.
    for action in ["CASCADE", "SET NULL"] {
        let up = format!(
            "CREATE TABLE \"C\" (id INTEGER PRIMARY KEY, aid INTEGER, \
             CONSTRAINT \"c_aid_fkey\" FOREIGN KEY (aid) REFERENCES \"A\" (id) ON DELETE {action});"
        );
        let kind = derive_kind(&[BASE], &up).unwrap();
        assert_eq!(kind, MigrationKind::Additive, "ON DELETE {action}");
    }
}

#[test]
fn new_table_with_fk_to_another_new_table_is_additive() {
    // Both tables are invisible to the old binary, so no constraint can reject
    // a write it is capable of making.
    let kind = derive_kind(
        &[BASE],
        "CREATE TABLE \"C\" (id INTEGER PRIMARY KEY); \
         CREATE TABLE \"D\" (id INTEGER PRIMARY KEY, cid INTEGER NOT NULL, \
         CONSTRAINT \"d_cid_fkey\" FOREIGN KEY (cid) REFERENCES \"C\" (id) ON DELETE RESTRICT);",
    )
    .unwrap();
    assert_eq!(kind, MigrationKind::Additive);
}

#[test]
fn not_null_column_on_existing_table_is_breaking() {
    let kind = derive_kind(
        &[BASE],
        "ALTER TABLE \"A\" ADD COLUMN flag INTEGER NOT NULL DEFAULT 0;",
    )
    .unwrap();
    assert_eq!(kind, MigrationKind::Breaking);
}

#[test]
fn unique_index_on_existing_column_is_breaking() {
    // The canonical spec example: an addition that diffs as additive but can
    // reject an old binary's duplicate write.
    let kind = derive_kind(
        &[BASE],
        "CREATE UNIQUE INDEX \"idx_a_name\" ON \"A\" (name);",
    )
    .unwrap();
    assert_eq!(kind, MigrationKind::Breaking);
}

#[test]
fn unique_column_add_on_existing_table_is_breaking() {
    // An inline UNIQUE on a new column of a pre-existing table creates an
    // implicit unique index — breaking despite being a column addition.
    let up = "ALTER TABLE \"A\" ADD COLUMN slug TEXT;\
              CREATE UNIQUE INDEX \"idx_a_slug\" ON \"A\" (slug);";
    assert_eq!(derive_kind(&[BASE], up).unwrap(), MigrationKind::Breaking);
}

#[test]
fn trigger_on_existing_table_is_breaking() {
    let up = "CREATE TRIGGER \"trg_a\" AFTER INSERT ON \"A\" \
              BEGIN UPDATE \"B\" SET label = 'x' WHERE aid = NEW.id; END;";
    assert_eq!(derive_kind(&[BASE], up).unwrap(), MigrationKind::Breaking);
}

#[test]
fn dropped_table_is_breaking() {
    assert_eq!(
        derive_kind(&[BASE], "DROP INDEX \"idx_b_aid\"; DROP TABLE \"B\";").unwrap(),
        MigrationKind::Breaking
    );
}

#[test]
fn dropped_column_via_rebuild_is_breaking() {
    let up = "CREATE TABLE \"A_new\" (id INTEGER PRIMARY KEY, name TEXT NOT NULL);\
              INSERT INTO \"A_new\" (id, name) SELECT id, name FROM \"A\";\
              DROP TABLE \"A\";\
              ALTER TABLE \"A_new\" RENAME TO \"A\";";
    assert_eq!(derive_kind(&[BASE], up).unwrap(), MigrationKind::Breaking);
}

#[test]
fn dml_on_existing_table_is_breaking() {
    assert_eq!(
        derive_kind(&[BASE], "UPDATE \"A\" SET note = 'x';").unwrap(),
        MigrationKind::Breaking
    );
}

#[test]
fn dropped_index_is_breaking() {
    assert_eq!(
        derive_kind(&[BASE], "DROP INDEX \"idx_b_aid\";").unwrap(),
        MigrationKind::Breaking
    );
}

// ------------------------------------------------------------------------
// declared == derived, both directions (spec §10.2, CI T6)
// ------------------------------------------------------------------------

#[test]
fn verify_kind_accepts_matching_declaration() {
    verify_kind(
        &[BASE],
        "CREATE TABLE \"C\" (id INTEGER PRIMARY KEY);",
        MigrationKind::Additive,
    )
    .unwrap();
    verify_kind(&[BASE], "DROP TABLE \"B\";", MigrationKind::Breaking).unwrap();
}

#[test]
fn verify_kind_rejects_additive_labeled_breaking() {
    // Declared-but-wrong in the "too conservative" direction still fails: the
    // gate is exact equality, both directions.
    let err = verify_kind(
        &[BASE],
        "CREATE TABLE \"C\" (id INTEGER PRIMARY KEY);",
        MigrationKind::Breaking,
    )
    .unwrap_err();
    assert!(matches!(err, VerifyError::KindMismatch { .. }), "{err}");
}

#[test]
fn verify_kind_rejects_breaking_labeled_additive() {
    let err = verify_kind(
        &[BASE],
        "CREATE UNIQUE INDEX \"idx_a_name\" ON \"A\" (name);",
        MigrationKind::Additive,
    )
    .unwrap_err();
    assert!(matches!(err, VerifyError::KindMismatch { .. }), "{err}");
}

// ------------------------------------------------------------------------
// DataDown parsing
// ------------------------------------------------------------------------

#[test]
fn data_down_parses_full_and_partial() {
    assert_eq!(DataDown::parse("full").unwrap(), DataDown::Full);
    assert_eq!(
        DataDown::parse("partial:A.name,A.note").unwrap(),
        DataDown::Partial(fields(&["A.name", "A.note"]))
    );
    assert!(DataDown::parse("partial:").is_err());
    assert!(DataDown::parse("bogus").is_err());
}

// ------------------------------------------------------------------------
// Seeded boundary-value round-trip (spec §10.3, CI T5)
// ------------------------------------------------------------------------

#[test]
fn additive_column_add_loses_nothing() {
    // Add a nullable column; the generated down drops it. No S(n-1) field is
    // touched, so the round-trip is proven lossless.
    let up = "ALTER TABLE \"A\" ADD COLUMN age INTEGER;";
    let down = generate_down(&[BASE], up).unwrap();
    assert!(seeded_lost_fields(&[BASE], up, &down).unwrap().is_empty());
    verify_data_down(&[BASE], up, &down, "full").unwrap();
}

#[test]
fn new_table_migration_loses_nothing() {
    let up = "CREATE TABLE \"C\" (id INTEGER PRIMARY KEY, v TEXT);";
    let down = generate_down(&[BASE], up).unwrap();
    assert!(seeded_lost_fields(&[BASE], up, &down).unwrap().is_empty());
}

#[test]
fn column_drop_loses_exactly_that_column() {
    // The up drops A.note via a rebuild; the generated down recreates note
    // empty, so the seeded values in note do not survive — note is lost, name
    // and id are not.
    let up = "CREATE TABLE \"A_new\" (id INTEGER PRIMARY KEY, name TEXT NOT NULL);\
              INSERT INTO \"A_new\" (id, name) SELECT id, name FROM \"A\";\
              DROP TABLE \"A\";\
              ALTER TABLE \"A_new\" RENAME TO \"A\";";
    let down = generate_down(&[BASE], up).unwrap();
    let lost = seeded_lost_fields(&[BASE], up, &down).unwrap();
    assert_eq!(lost, fields(&["A.note"]), "only A.note is lost");

    // Declared full → fails; declared partial:A.note → passes.
    assert!(verify_data_down(&[BASE], up, &down, "full").is_err());
    verify_data_down(&[BASE], up, &down, "partial:A.note").unwrap();
}

#[test]
fn dml_transform_without_inverse_is_lossy() {
    // The up overwrites A.note for every row; the down (empty) cannot restore
    // the prior values, so A.note is lost.
    let up = "UPDATE \"A\" SET note = 'overwritten';";
    let down = ""; // no inverse DML
    let lost = seeded_lost_fields(&[BASE], up, down).unwrap();
    assert!(
        lost.contains("A.note"),
        "note overwrite must be detected: {lost:?}"
    );
    assert!(verify_data_down(&[BASE], up, down, "full").is_err());
}

#[test]
fn dml_transform_with_inverse_down_round_trips_losslessly() {
    // A transform reversible for every seeded boundary value: up appends a
    // marker to a TEXT field, down strips it. This survives the empty string
    // and unicode-nasty strings alike (unlike an integer *2 // 2, which loses
    // precision at i64::MAX/MIN — the seeder now exercises those extremes).
    let prev = "CREATE TABLE \"N\" (id INTEGER PRIMARY KEY, v TEXT NOT NULL);";
    let up = "UPDATE \"N\" SET v = v || '<<';";
    let down = "UPDATE \"N\" SET v = substr(v, 1, length(v) - 2);";
    let lost = seeded_lost_fields(&[prev], up, down).unwrap();
    assert!(
        lost.is_empty(),
        "reversible transform must lose nothing: {lost:?}"
    );
    verify_data_down(&[prev], up, down, "full").unwrap();
}

#[test]
fn dropped_integer_and_blob_columns_are_both_detected_lost() {
    // Loss detection must compare every value type, not just text: drop an
    // INTEGER and a BLOB column via rebuild and confirm both are flagged (the
    // seeder fills them with boundary ints and 0x00-containing blobs).
    let prev = "CREATE TABLE \"T\" (id INTEGER PRIMARY KEY, n INTEGER, b BLOB);";
    let up = "CREATE TABLE \"T_new\" (id INTEGER PRIMARY KEY);\
              INSERT INTO \"T_new\" (id) SELECT id FROM \"T\";\
              DROP TABLE \"T\";\
              ALTER TABLE \"T_new\" RENAME TO \"T\";";
    let down = generate_down(&[prev], up).unwrap();
    let lost = seeded_lost_fields(&[prev], up, &down).unwrap();
    assert_eq!(
        lost,
        fields(&["T.b", "T.n"]),
        "both non-id columns are lost: {lost:?}"
    );
}

// ------------------------------------------------------------------------
// Planted-failure self-tests (spec §10.2-10.3 item 5, CI T11)
// ------------------------------------------------------------------------

#[test]
fn planted_mislabeled_kind_fails() {
    // CENSUS-SELFTEST: manifest.kind-derivation
    // A genuinely breaking migration declared additive must be caught.
    let up = "CREATE UNIQUE INDEX \"idx_a_name\" ON \"A\" (name);";
    assert!(
        verify_kind(&[BASE], up, MigrationKind::Additive).is_err(),
        "a breaking migration labeled additive must fail the gate"
    );
}

#[test]
fn planted_undeclared_lossy_drop_fails() {
    // CENSUS-SELFTEST: manifest.data-down-undeclared-lost
    // Dropping an old column and declaring the down lossless must be caught.
    let up = "CREATE TABLE \"A_new\" (id INTEGER PRIMARY KEY, name TEXT NOT NULL);\
              INSERT INTO \"A_new\" (id, name) SELECT id, name FROM \"A\";\
              DROP TABLE \"A\";\
              ALTER TABLE \"A_new\" RENAME TO \"A\";";
    let down = generate_down(&[BASE], up).unwrap();
    let err = verify_data_down(&[BASE], up, &down, "full").unwrap_err();
    match err {
        VerifyError::DataDownMismatch { missing, .. } => {
            assert!(
                missing.contains("A.note"),
                "undeclared lost field must be reported: {missing:?}"
            );
        }
        other => panic!("expected DataDownMismatch, got {other}"),
    }
}

#[test]
fn planted_stale_partial_declaration_fails() {
    // CENSUS-SELFTEST: manifest.data-down-stale-declared
    // A lossless additive migration declaring a partial loss must be caught as
    // stale (declared-but-preserved).
    let up = "ALTER TABLE \"A\" ADD COLUMN age INTEGER;";
    let down = generate_down(&[BASE], up).unwrap();
    let err = verify_data_down(&[BASE], up, &down, "partial:A.name").unwrap_err();
    match err {
        VerifyError::DataDownMismatch { stale, .. } => {
            assert!(
                stale.contains("A.name"),
                "stale declared field must be reported: {stale:?}"
            );
        }
        other => panic!("expected DataDownMismatch, got {other}"),
    }
}

// ------------------------------------------------------------------------
// Real committed schema (spec §10.3 item 4 against the shipped DDL)
// ------------------------------------------------------------------------

#[test]
fn seeds_and_round_trips_the_real_schema_losslessly_under_a_noop() {
    // Seeding the full committed 25-migration schema (composite PKs, foreign
    // keys, blobs, datetimes) in FK-topological order must succeed, and a
    // no-op up/down must lose nothing — a smoke test for the generator against
    // the real schema.
    let (set, _n) = carbon_repos::get_migrations();
    let prev: Vec<&str> = set.migrations.iter().map(|d| d.up_sql).collect();
    let lost = seeded_lost_fields(&prev, "", "").unwrap();
    assert!(
        lost.is_empty(),
        "a no-op over the real schema must lose nothing: {lost:?}"
    );
}

#[test]
fn derives_additive_for_a_nullable_add_on_the_real_schema() {
    let (set, _n) = carbon_repos::get_migrations();
    let prev: Vec<&str> = set.migrations.iter().map(|d| d.up_sql).collect();
    let kind = derive_kind(
        &prev,
        "ALTER TABLE \"Server\" ADD COLUMN experimentalFlag TEXT;",
    )
    .unwrap();
    assert_eq!(kind, MigrationKind::Additive);
}

// ------------------------------------------------------------------------
// Whole-list enforcement over get_migrations() (spec §10.2-10.3, CI T6)
// ------------------------------------------------------------------------

#[test]
fn every_post_floor_migration_matches_its_declared_metadata() {
    // Auto-derived ∀n: every migration that carries a stored down (post-floor)
    // must have its declared kind equal the derived kind and its declared
    // data_down equal the seeded round-trip's lost-field set. The 25 historical
    // migrations carry no down and are conservatively Breaking — exempt.
    // Vacuous today; activates automatically when the first post-floor
    // migration ships a down.
    let (set, _n) = carbon_repos::get_migrations();
    for (i, def) in set.migrations.iter().enumerate() {
        let Some(down) = def.down_sql else { continue };
        let prev: Vec<&str> = set.migrations[..i].iter().map(|d| d.up_sql).collect();
        verify_kind(&prev, def.up_sql, def.kind)
            .unwrap_or_else(|e| panic!("migration {} kind: {e}", def.name));
        verify_data_down(&prev, def.up_sql, down, def.data_down)
            .unwrap_or_else(|e| panic!("migration {} data_down: {e}", def.name));
    }
}
