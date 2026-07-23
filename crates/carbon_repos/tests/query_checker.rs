use carbon_repos::checker::{
    check_classification, check_insert_datetime_columns, check_manifests, check_module,
    check_nullability, check_pool_routing, check_query_plans,
};
use carbon_repos::registry::QueryCheck;
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("check.db")).unwrap();
    let (migrations, _count) = carbon_repos::get_migrations();
    migrations.to_latest(&mut conn).unwrap();
    (dir, conn)
}

/// Every registered `QueryCheck` across every repo module. Every checker rule
/// runs over this same aggregated set, so every repo is covered automatically.
/// Delegates to `carbon_repos::repos::all_queries()` — the single shared
/// source of truth also used by `src/bin/compat_probe.rs` — rather than
/// hand-maintaining a second copy of the module list here (spec L9).
fn all_registered_queries() -> Vec<QueryCheck> {
    carbon_repos::repos::all_queries()
}

#[test]
fn all_registered_queries_pass_against_migrated_schema() {
    let (_d, conn) = migrated_db();
    let all = check_module(&conn, &all_registered_queries());
    assert!(
        all.is_empty(),
        "query checker violations:\n{}",
        all.join("\n")
    );
}

#[test]
fn manifest_freshness_lint_passes_for_all_registered_queries() {
    let (_d, conn) = migrated_db();
    let v = check_manifests(&conn, &all_registered_queries());
    assert!(v.is_empty(), "freshness lint violations:\n{}", v.join("\n"));
}

#[test]
fn nullability_lint_passes_for_all_registered_queries() {
    let (_d, conn) = migrated_db();
    let v = check_nullability(&conn, &all_registered_queries());
    assert!(
        v.is_empty(),
        "nullability lint violations:\n{}",
        v.join("\n")
    );
}

#[test]
fn query_plan_lint_passes_for_all_registered_queries() {
    let (_d, conn) = migrated_db();
    let v = check_query_plans(&conn, &all_registered_queries());
    assert!(
        v.is_empty(),
        "query plan lint violations:\n{}",
        v.join("\n")
    );
}

#[test]
fn insert_datetime_lint_passes_for_all_registered_queries() {
    let (_d, conn) = migrated_db();
    let v = check_insert_datetime_columns(&conn, &all_registered_queries());
    assert!(
        v.is_empty(),
        "insert datetime lint violations:\n{}",
        v.join("\n")
    );
}

#[test]
fn classification_lint_passes_for_all_registered_queries() {
    let (_d, conn) = migrated_db();
    let v = check_classification(&conn, &all_registered_queries());
    assert!(
        v.is_empty(),
        "read-class-no-writes violations:\n{}",
        v.join("\n")
    );
}

#[test]
fn pool_routing_lint_passes_for_all_registered_queries() {
    let v = check_pool_routing(&all_registered_queries());
    assert!(v.is_empty(), "pool routing violations:\n{}", v.join("\n"));
}

// ---------------------------------------------------------------------------
// Planted-failure self-tests: every rule is fence-tested with a query that must
// be flagged, so a broken checker can't silently pass everything.
// ---------------------------------------------------------------------------

#[test]
fn checker_catches_planted_structural_failures() {
    // CENSUS-SELFTEST: checker.prepare
    // CENSUS-SELFTEST: checker.declared-param-present
    // CENSUS-SELFTEST: checker.undeclared-param
    let (_d, conn) = migrated_db();
    let planted = [
        // Unknown table → does not prepare.
        QueryCheck {
            name: "bad_table",
            sql: "SELECT id FROM NotATable",
            params: &[],
            columns: None,
            class: carbon_repos::registry::class_of("SELECT id FROM NotATable"),
            routes_write: false,
        },
        // Declared param the SQL does not bind.
        QueryCheck {
            name: "bad_declared_param",
            sql: "SELECT id FROM Java WHERE id = :id",
            params: &[":wrong"],
            columns: None,
            class: carbon_repos::registry::class_of("SELECT id FROM Java WHERE id = :id"),
            routes_write: false,
        },
        // SQL param the registry does not declare (exact set equality flags a
        // bound parameter with no matching declaration).
        QueryCheck {
            name: "undeclared_param",
            sql: "SELECT id FROM Java WHERE id = :id",
            params: &[],
            columns: None,
            class: carbon_repos::registry::class_of("SELECT id FROM Java WHERE id = :id"),
            routes_write: false,
        },
    ];
    let v = check_module(&conn, &planted);
    assert!(
        v.iter().any(|m| m.contains("bad_table")),
        "must flag unknown table, got: {v:?}"
    );
    assert!(
        v.iter().any(|m| m.contains("bad_declared_param")),
        "must flag a declared param missing from SQL, got: {v:?}"
    );
    assert!(
        v.iter().any(|m| m.contains("undeclared_param")),
        "must flag an undeclared SQL param, got: {v:?}"
    );
}

#[test]
fn checker_does_not_flag_question_mark_in_string_literal() {
    // A multi-param query with a literal '?' inside a string value must NOT be
    // flagged for a positional placeholder: the scan skips string literals.
    let (_d, conn) = migrated_db();
    let ok = [QueryCheck {
        name: "literal_question_mark",
        sql: "SELECT id FROM Java WHERE major = :major OR os = 'what?'",
        params: &[":major", ":unused_but_present"],
        columns: None,
        class: carbon_repos::registry::class_of(
            "SELECT id FROM Java WHERE major = :major OR os = 'what?'",
        ),
        routes_write: false,
    }];
    // The '?'-in-literal rule must not fire; only the param-set mismatch for the
    // deliberately-unused declared name should show up.
    let v = check_module(&conn, &ok);
    assert!(
        !v.iter().any(|m| m.contains("positional")),
        "literal '?' must not be read as a positional placeholder, got: {v:?}"
    );
}

#[test]
fn checker_flags_a_positional_param_in_a_single_param_query() {
    // CENSUS-SELFTEST: checker.positional-param
    // The generated wrappers bind by name only, so a positional placeholder is
    // never bound and silently reads as NULL at runtime — independent of how
    // many named params the query also declares.
    let (_d, conn) = migrated_db();
    for (name, sql, params) in [
        (
            "positional_one_param",
            "SELECT id FROM Java WHERE major = :major AND arch = ?",
            &[":major"][..],
        ),
        (
            "positional_no_params",
            "SELECT id FROM Java WHERE arch = ?",
            &[][..],
        ),
    ] {
        let planted = [QueryCheck {
            name,
            sql,
            params,
            columns: None,
            class: carbon_repos::registry::class_of(sql),
            routes_write: false,
        }];
        let v = check_module(&conn, &planted);
        assert!(
            v.iter().any(|m| m.contains("positional")),
            "must flag the bare '?' in {sql:?}, got: {v:?}"
        );
    }
}

#[test]
fn checker_flags_real_positional_param_in_multiparam_query() {
    // CENSUS-SELFTEST: checker.positional-param
    let (_d, conn) = migrated_db();
    let planted = [QueryCheck {
        name: "positional_multiparam",
        sql: "SELECT id FROM Java WHERE major = :major AND arch = ?",
        params: &[":major", ":arch"],
        columns: None,
        class: carbon_repos::registry::class_of(
            "SELECT id FROM Java WHERE major = :major AND arch = ?",
        ),
        routes_write: false,
    }];
    let v = check_module(&conn, &planted);
    assert!(
        v.iter().any(|m| m.contains("positional")),
        "must flag a bare '?' in a multi-param query, got: {v:?}"
    );
}

#[test]
fn checker_flags_declared_column_missing_from_result_set() {
    // CENSUS-SELFTEST: checker.result-column-present
    // A `columns` spec naming a column the query never selects must be flagged:
    // the row decode would look up a name the result set does not carry.
    use carbon_repos::from_row::{ColumnSpec, TypeClass};
    const COLS: &[ColumnSpec] = &[ColumnSpec {
        name: "major",
        ty: TypeClass::Integer,
        nullable: false,
        explicit_nullable: false,
    }];
    let (_d, conn) = migrated_db();
    let planted = [QueryCheck {
        name: "missing_result_column",
        sql: "SELECT id FROM Java WHERE id = :id",
        params: &[":id"],
        columns: Some(COLS),
        class: carbon_repos::registry::class_of("SELECT id FROM Java WHERE id = :id"),
        routes_write: false,
    }];
    let v = check_module(&conn, &planted);
    assert!(
        v.iter()
            .any(|m| m.contains("missing_result_column") && m.contains("major")),
        "must flag a declared column absent from the result set, got: {v:?}"
    );
}

#[test]
fn duplicate_result_column_rule_catches_an_unaliased_join_collision() {
    // CENSUS-SELFTEST: checker.duplicate-result-column
    // An unaliased join exposing two same-named result columns (both sides
    // happen to have an `id`) silently binds the left one under rusqlite's
    // by-name row decoding — the right one is never reachable by name despite
    // appearing in the result set.
    use carbon_repos::from_row::{ColumnSpec, TypeClass};
    const COLS: &[ColumnSpec] = &[ColumnSpec {
        name: "id",
        ty: TypeClass::Text,
        nullable: false,
        explicit_nullable: false,
    }];
    let (_d, conn) = migrated_db();

    let dup_sql = "SELECT Java.id, Instance.id FROM Java, Instance";
    let planted = [QueryCheck {
        name: "unaliased_join_duplicate_id",
        sql: dup_sql,
        params: &[],
        columns: Some(COLS),
        class: carbon_repos::registry::class_of(dup_sql),
        routes_write: false,
    }];
    let v = check_module(&conn, &planted);
    assert!(
        v.iter()
            .any(|m| m.contains("unaliased_join_duplicate_id")
                && m.contains("duplicate column name")),
        "must flag a join exposing two same-named result columns, got: {v:?}"
    );

    // A distinctly-aliased join must not be flagged as a duplicate.
    let ok_sql = "SELECT Java.id AS java_id, Instance.id AS instance_id FROM Java, Instance";
    let ok = [QueryCheck {
        name: "aliased_join",
        sql: ok_sql,
        params: &[],
        columns: Some(COLS),
        class: carbon_repos::registry::class_of(ok_sql),
        routes_write: false,
    }];
    let ok_violations = check_module(&conn, &ok);
    assert!(
        !ok_violations
            .iter()
            .any(|m| m.contains("duplicate column name")),
        "distinctly-aliased columns must not be flagged as duplicates, got: {ok_violations:?}"
    );

    // Case-insensitive: `Id` and `id` must also collide, matching rusqlite's
    // own case-insensitive by-name column resolution.
    let case_sql = "SELECT Java.id AS Id, Instance.id AS id FROM Java, Instance";
    let case_variant = [QueryCheck {
        name: "case_insensitive_collision",
        sql: case_sql,
        params: &[],
        columns: Some(COLS),
        class: carbon_repos::registry::class_of(case_sql),
        routes_write: false,
    }];
    assert!(
        check_module(&conn, &case_variant)
            .iter()
            .any(|m| m.contains("duplicate column name")),
        "a case-insensitive duplicate ('Id' vs 'id') must also be flagged"
    );
}

#[test]
fn manifest_freshness_lint_catches_planted_failure() {
    // CENSUS-SELFTEST: checker.freshness
    // An UPDATE on a freshness table that never sets lastUpdatedAt must be
    // flagged by the authorizer-driven manifest lint.
    let (_d, conn) = migrated_db();
    let planted = [QueryCheck {
        name: "bad_freshness_update",
        sql: "UPDATE VersionInfoCache SET versionInfo = :v WHERE id = :id",
        params: &[":v", ":id"],
        columns: None,
        class: carbon_repos::registry::class_of(
            "UPDATE VersionInfoCache SET versionInfo = :v WHERE id = :id",
        ),
        routes_write: true,
    }];
    let v = check_manifests(&conn, &planted);
    assert_eq!(
        v.len(),
        1,
        "freshness lint must flag missing lastUpdatedAt, got: {v:?}"
    );
}

#[test]
fn manifest_freshness_lint_catches_upsert_missing_freshness() {
    // The DO UPDATE SET branch of an upsert must also carry the freshness
    // column — here it sets only `versionInfo`, so it must be flagged.
    let (_d, conn) = migrated_db();
    let planted = [QueryCheck {
        name: "bad_freshness_upsert",
        sql: "INSERT INTO VersionInfoCache (id, versionInfo, lastUpdatedAt) VALUES (:id, :v, :t) \
              ON CONFLICT(id) DO UPDATE SET versionInfo = excluded.versionInfo",
        params: &[":id", ":v", ":t"],
        columns: None,
        class: carbon_repos::registry::QueryClass::Write,
        routes_write: true,
    }];
    let v = check_manifests(&conn, &planted);
    assert_eq!(
        v.len(),
        1,
        "freshness lint must flag an upsert whose DO UPDATE omits lastUpdatedAt, got: {v:?}"
    );
}

mod planted_rows {
    use carbon_repos::queries;

    // A nullable source column (`Account.accessToken`) declared non-null.
    #[derive(carbon_macro::FromRow, Debug, PartialEq)]
    pub struct BadNullableRow {
        pub access_token: String,
    }

    // A SQL expression with no resolvable origin, declared non-null and without
    // an explicit override.
    #[derive(carbon_macro::FromRow, Debug, PartialEq)]
    pub struct BadExprRow {
        pub flag: bool,
    }

    queries! {
        fn bad_nullable(uuid: &str) -> Option<BadNullableRow> =
            "SELECT accessToken FROM Account WHERE uuid = :uuid";
        fn bad_expr(id: &str) -> Option<BadExprRow> =
            "SELECT (id IS NOT NULL) AS flag FROM Java WHERE id = :id";
    }
}

#[test]
fn nullability_lint_catches_nullable_source_declared_non_null() {
    // CENSUS-SELFTEST: checker.nullability-nullable-source
    let (_d, conn) = migrated_db();
    let v = check_nullability(&conn, planted_rows::QUERIES);
    assert!(
        v.iter()
            .any(|m| m.contains("bad_nullable") && m.contains("access")),
        "must flag a nullable source column declared non-null, got: {v:?}"
    );
}

#[test]
fn nullability_lint_catches_unmarked_expression_column() {
    // CENSUS-SELFTEST: checker.nullability-expression-origin
    let (_d, conn) = migrated_db();
    let v = check_nullability(&conn, planted_rows::QUERIES);
    assert!(
        v.iter()
            .any(|m| m.contains("bad_expr") && m.contains("expression")),
        "must flag an expression column that is neither Option nor overridden, got: {v:?}"
    );
}

#[test]
fn nullability_lint_catches_a_not_null_column_widened_by_an_unambiguous_left_join() {
    // CENSUS-SELFTEST: checker.nullability-outer-join-widening
    let (_d, conn) = migrated_db();
    conn.execute_batch(
        "CREATE TABLE OjParent (id INTEGER PRIMARY KEY);
         CREATE TABLE OjChild (id INTEGER PRIMARY KEY, parent_id INTEGER, label TEXT NOT NULL);
         INSERT INTO OjParent (id) VALUES (1), (2);
         INSERT INTO OjChild (id, parent_id, label) VALUES (1, 1, 'x');",
    )
    .unwrap();

    use carbon_repos::from_row::{ColumnSpec, TypeClass};
    const NON_OPTION: &[ColumnSpec] = &[ColumnSpec {
        name: "label",
        ty: TypeClass::Text,
        nullable: false,
        explicit_nullable: false,
    }];
    let sql =
        "SELECT OjChild.label FROM OjParent LEFT JOIN OjChild ON OjParent.id = OjChild.parent_id";
    let planted = [QueryCheck {
        name: "outer_join_widened",
        sql,
        params: &[],
        columns: Some(NON_OPTION),
        class: carbon_repos::registry::class_of(sql),
        routes_write: false,
    }];
    let v = check_nullability(&conn, &planted);
    assert!(
        v.iter()
            .any(|m| m.contains("outer_join_widened") && m.contains("LEFT JOIN")),
        "must flag a NOT NULL column widened to nullable by an unambiguous LEFT JOIN, got: {v:?}"
    );

    // The same column declared Option (or nullable) is accepted.
    const OPTION: &[ColumnSpec] = &[ColumnSpec {
        name: "label",
        ty: TypeClass::Text,
        nullable: true,
        explicit_nullable: false,
    }];
    let ok = [QueryCheck {
        name: "outer_join_widened_declared_option",
        sql,
        params: &[],
        columns: Some(OPTION),
        class: carbon_repos::registry::class_of(sql),
        routes_write: false,
    }];
    assert!(
        check_nullability(&conn, &ok).is_empty(),
        "a column correctly declared Option for the LEFT JOIN's optional side must not be flagged"
    );
}

#[test]
fn nullability_lint_does_not_flag_inner_joins_or_self_joins() {
    let (_d, conn) = migrated_db();
    conn.execute_batch(
        "CREATE TABLE OjParent2 (id INTEGER PRIMARY KEY);
         CREATE TABLE OjChild2 (id INTEGER PRIMARY KEY, parent_id INTEGER, label TEXT NOT NULL);
         CREATE TABLE OjTree (id INTEGER PRIMARY KEY, parent_id INTEGER, label TEXT NOT NULL);
         INSERT INTO OjParent2 (id) VALUES (1);
         INSERT INTO OjChild2 (id, parent_id, label) VALUES (1, 1, 'x');
         INSERT INTO OjTree (id, parent_id, label) VALUES (1, NULL, 'root');",
    )
    .unwrap();

    use carbon_repos::from_row::{ColumnSpec, TypeClass};
    const NON_OPTION: &[ColumnSpec] = &[ColumnSpec {
        name: "label",
        ty: TypeClass::Text,
        nullable: false,
        explicit_nullable: false,
    }];

    // A plain INNER JOIN never widens nullability.
    let inner_sql =
        "SELECT OjChild2.label FROM OjParent2 JOIN OjChild2 ON OjParent2.id = OjChild2.parent_id";
    let inner = [QueryCheck {
        name: "inner_join_not_widened",
        sql: inner_sql,
        params: &[],
        columns: Some(NON_OPTION),
        class: carbon_repos::registry::class_of(inner_sql),
        routes_write: false,
    }];
    assert!(
        check_nullability(&conn, &inner).is_empty(),
        "an INNER JOIN must never be treated as widening"
    );

    // A self-join is ambiguous — the same schema table is introduced twice
    // (once on the preserved side, once on the optional side), and
    // column_metadata reports only the bare schema name, with no way to tell
    // which occurrence a given result column came from. Selecting the
    // PRESERVED side's column proves the ambiguity guard actually suppresses
    // the rule here: without it, this LEFT JOIN's mere presence would flag
    // every column resolving to "OjTree", including this one, which is wrong.
    let self_join_sql = "SELECT parent.label FROM OjTree AS parent \
                          LEFT JOIN OjTree AS child ON child.parent_id = parent.id";
    let self_join = [QueryCheck {
        name: "self_join_preserved_side_not_flagged",
        sql: self_join_sql,
        params: &[],
        columns: Some(NON_OPTION),
        class: carbon_repos::registry::class_of(self_join_sql),
        routes_write: false,
    }];
    assert!(
        check_nullability(&conn, &self_join).is_empty(),
        "a self-join must be skipped: column_metadata can't disambiguate which side a column is from, got: {:?}",
        check_nullability(&conn, &self_join)
    );
}

#[test]
fn query_plan_lint_catches_planted_full_scan() {
    // CENSUS-SELFTEST: checker.query-plan-full-scan
    // ModMetadata is a guarded table with no index on `name`, so a WHERE on it
    // full-scans and must be flagged.
    let (_d, conn) = migrated_db();
    let planted = [QueryCheck {
        name: "scan_mod_metadata",
        sql: "SELECT id FROM ModMetadata WHERE name = :name",
        params: &[":name"],
        columns: None,
        class: carbon_repos::registry::class_of("SELECT id FROM ModMetadata WHERE name = :name"),
        routes_write: false,
    }];
    let v = check_query_plans(&conn, &planted);
    assert!(
        v.iter()
            .any(|m| m.contains("scan_mod_metadata") && m.contains("ModMetadata")),
        "must flag a full scan of a guarded table, got: {v:?}"
    );
}

#[test]
fn query_plan_lint_catches_a_full_scan_behind_a_table_alias() {
    // CENSUS-SELFTEST: checker.query-plan-full-scan
    // The plan names the alias, not the table, so matching the plan's object
    // against the guarded table name alone lets an aliased scan through — and
    // aliasing is exactly what a join-shaped query does.
    let (_d, conn) = migrated_db();
    for sql in [
        "SELECT m.id FROM ModMetadata m WHERE m.name = :name",
        "SELECT m.id FROM ModMetadata AS m WHERE m.name = :name",
    ] {
        let planted = [QueryCheck {
            name: "scan_mod_metadata_aliased",
            sql,
            params: &[":name"],
            columns: None,
            class: carbon_repos::registry::class_of(sql),
            routes_write: false,
        }];
        let v = check_query_plans(&conn, &planted);
        assert!(
            v.iter().any(|m| m.contains("scan_mod_metadata_aliased")),
            "must flag an aliased full scan of a guarded table for {sql:?}, got: {v:?}"
        );
    }
}

#[test]
fn insert_datetime_lint_catches_planted_failures() {
    // CENSUS-SELFTEST: checker.insert-datetime-explicit
    let (_d, conn) = migrated_db();
    let planted = [
        // Omits the DATETIME `lastUpdatedAt` column from its list.
        QueryCheck {
            name: "bad_insert_missing_datetime",
            sql: "INSERT INTO VersionInfoCache (id, versionInfo) VALUES (:id, :v)",
            params: &[":id", ":v"],
            columns: None,
            class: carbon_repos::registry::class_of(
                "INSERT INTO VersionInfoCache (id, versionInfo) VALUES (:id, :v)",
            ),
            routes_write: true,
        },
        // No column list at all — flagged outright regardless of the table.
        QueryCheck {
            name: "bad_insert_no_column_list",
            sql: "INSERT INTO VersionInfoCache VALUES (:id, :v, :t)",
            params: &[":id", ":v", ":t"],
            columns: None,
            class: carbon_repos::registry::class_of(
                "INSERT INTO VersionInfoCache VALUES (:id, :v, :t)",
            ),
            routes_write: true,
        },
    ];
    let v = check_insert_datetime_columns(&conn, &planted);
    assert!(
        v.iter()
            .any(|m| m.contains("bad_insert_missing_datetime") && m.contains("lastUpdatedAt")),
        "must flag an INSERT that omits a DATETIME column, got: {v:?}"
    );
    assert!(
        v.iter().any(|m| m.contains("bad_insert_no_column_list")),
        "must flag a column-list-less INSERT outright, got: {v:?}"
    );

    // A conforming INSERT (every DATETIME column listed) must pass.
    let ok = [QueryCheck {
        name: "good_insert",
        sql: "INSERT INTO VersionInfoCache (id, versionInfo, lastUpdatedAt) VALUES (:id, :v, :t)",
        params: &[":id", ":v", ":t"],
        columns: None,
        class: carbon_repos::registry::class_of(
            "INSERT INTO VersionInfoCache (id, versionInfo, lastUpdatedAt) VALUES (:id, :v, :t)",
        ),
        routes_write: true,
    }];
    assert!(
        check_insert_datetime_columns(&conn, &ok).is_empty(),
        "a conforming INSERT must not be flagged"
    );

    // A non-INSERT statement is ignored entirely, even one touching the same
    // table and omitting the same column in its SET list.
    let non_insert = [QueryCheck {
        name: "an_update",
        sql: "UPDATE VersionInfoCache SET versionInfo = :v WHERE id = :id",
        params: &[":v", ":id"],
        columns: None,
        class: carbon_repos::registry::class_of(
            "UPDATE VersionInfoCache SET versionInfo = :v WHERE id = :id",
        ),
        routes_write: true,
    }];
    assert!(
        check_insert_datetime_columns(&conn, &non_insert).is_empty(),
        "a non-INSERT statement must never be flagged by this lint"
    );
}

#[test]
fn classification_lint_catches_planted_read_that_actually_writes() {
    // CENSUS-SELFTEST: checker.read-class-no-writes
    let (_d, conn) = migrated_db();

    // A statement whose engine-reported actions include a DELETE, but forced
    // to `class: Read` — as if a `WITH` CTE wrapper had fooled the leading-verb
    // heuristic that ordinarily derives the class. This is exactly what the
    // manifest lock exists to catch: the async wrapper would trust `class` and
    // route this to the read-only pool.
    let planted = [QueryCheck {
        name: "fake_read_that_deletes",
        sql: "DELETE FROM Java WHERE id = :id",
        params: &[":id"],
        columns: None,
        class: carbon_repos::registry::QueryClass::Read,
        // Not itself a test of L5's routing rule: this literal simulates a
        // WITH-CTE-fooled `class`, so `routes_write` agrees with it (as if it
        // had gone through a read-routing arm) rather than planting a second,
        // unrelated failure here.
        routes_write: false,
    }];
    let v = check_classification(&conn, &planted);
    assert_eq!(
        v.len(),
        1,
        "must flag a Read-classified query whose engine manifest reports writes, got: {v:?}"
    );
    assert!(v[0].contains("fake_read_that_deletes"));

    // A genuine read, correctly classified, must pass.
    let real_read = [QueryCheck {
        name: "real_read",
        sql: "SELECT id FROM Java WHERE id = :id",
        params: &[":id"],
        columns: None,
        class: carbon_repos::registry::class_of("SELECT id FROM Java WHERE id = :id"),
        routes_write: false,
    }];
    assert!(
        check_classification(&conn, &real_read).is_empty(),
        "a real read must not be flagged"
    );

    // A Write-classified statement is never checked by this rule, even one
    // whose own write-set happens to be empty (e.g. `PRAGMA`-shaped statement
    // classified conservatively as Write) — the conservative default can never
    // be a violation.
    let write_class_no_writes = [QueryCheck {
        name: "write_class_but_no_writes",
        sql: "PRAGMA table_info(Java)",
        params: &[],
        columns: None,
        class: carbon_repos::registry::QueryClass::Write,
        routes_write: true,
    }];
    assert!(
        check_classification(&conn, &write_class_no_writes).is_empty(),
        "write-class queries must never be checked by the read-class-no-writes rule"
    );
}

#[test]
fn pool_routing_lint_catches_a_write_declared_through_a_read_routing_arm() {
    // CENSUS-SELFTEST: checker.routing-matches-class
    // A future `UPDATE … RETURNING x -> i64`-shaped query: SQLite genuinely
    // returns rows for it (RETURNING makes any DML row-returning), so it fits
    // the `i64` arm's `query_row` shape — but that arm hard-codes
    // `routes_write: false`, routing it to the read-only pool where it would
    // fail on every call (SQLITE_READONLY). `class` correctly derives `Write`
    // from the leading verb; this is the one place both facts are compared.
    let planted = [QueryCheck {
        name: "bump_major_returning",
        sql: "UPDATE Java SET major = major + 1 WHERE id = :id RETURNING major",
        params: &[":id"],
        columns: None,
        class: carbon_repos::registry::class_of(
            "UPDATE Java SET major = major + 1 WHERE id = :id RETURNING major",
        ),
        routes_write: false,
    }];
    assert_eq!(
        planted[0].class,
        carbon_repos::registry::QueryClass::Write,
        "sanity: the leading verb is UPDATE, so class_of must derive Write"
    );
    let v = check_pool_routing(&planted);
    assert_eq!(
        v.len(),
        1,
        "must flag a Write-classified query whose routes_write is false, got: {v:?}"
    );
    assert!(v[0].contains("bump_major_returning"));

    // The inverse mismatch — a genuine read declared through the write arm —
    // must also be caught: nothing about the rule is one-directional.
    let over_routed = [QueryCheck {
        name: "over_routed_read",
        sql: "SELECT id FROM Java WHERE id = :id",
        params: &[":id"],
        columns: None,
        class: carbon_repos::registry::class_of("SELECT id FROM Java WHERE id = :id"),
        routes_write: true,
    }];
    let v = check_pool_routing(&over_routed);
    assert_eq!(
        v.len(),
        1,
        "must flag a Read-classified query whose routes_write is true, got: {v:?}"
    );

    // A query whose class and routes_write agree — true for every real
    // registered query today — must not be flagged either way.
    let agreeing = [
        QueryCheck {
            name: "agreeing_read",
            sql: "SELECT id FROM Java WHERE id = :id",
            params: &[":id"],
            columns: None,
            class: carbon_repos::registry::class_of("SELECT id FROM Java WHERE id = :id"),
            routes_write: false,
        },
        QueryCheck {
            name: "agreeing_write",
            sql: "DELETE FROM Java WHERE id = :id",
            params: &[":id"],
            columns: None,
            class: carbon_repos::registry::class_of("DELETE FROM Java WHERE id = :id"),
            routes_write: true,
        },
    ];
    assert!(
        check_pool_routing(&agreeing).is_empty(),
        "agreeing class/routes_write must never be flagged"
    );
}

/// Every hand-written SQL statement in the repos modules is registered: the
/// SQL travels through a shared const that a QueryCheck also references.
#[test]
fn all_handwritten_repo_sql_is_registered() {
    let dir = format!("{}/src/repos", env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            files.push((
                path.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&path).unwrap(),
            ));
        }
    }
    assert!(!files.is_empty(), "repos dir scan found no sources");
    let v = carbon_repos::checker::check_handwritten_sql(&files);
    assert!(
        v.is_empty(),
        "hand-written SQL census violations:\n{}",
        v.join("\n")
    );
}

/// CENSUS-SELFTEST: checker.handwritten-sql-registered
#[test]
fn handwritten_sql_census_catches_planted_failures() {
    use carbon_repos::checker::check_handwritten_sql;
    let inline = (
        "a.rs".to_string(),
        "fn f() { conn.prepare_cached(\"SELECT 1\") }".to_string(),
    );
    let v = check_handwritten_sql(&[inline]);
    assert_eq!(v.len(), 1, "inline literal must be flagged: {v:?}");

    let dynamic = (
        "b.rs".to_string(),
        "fn f() { conn.execute_batch(format!(\"DELETE FROM {t}\")) }".to_string(),
    );
    let v = check_handwritten_sql(&[dynamic]);
    assert_eq!(v.len(), 1, "format!-built SQL must be flagged: {v:?}");

    let unreferenced = (
        "c.rs".to_string(),
        "const X_SQL: &str = \"SELECT 1\";\nfn f() { conn.prepare_cached(X_SQL)?; }".to_string(),
    );
    let v = check_handwritten_sql(&[unreferenced]);
    assert_eq!(
        v.len(),
        1,
        "const without a QueryCheck reference must be flagged: {v:?}"
    );

    let conforming = ("d.rs".to_string(),
        "const X_SQL: &str = \"SELECT 1\";\nfn f() { conn.prepare_cached(X_SQL)?; }\nconst CHECK: QueryCheck = QueryCheck { sql: X_SQL };".to_string());
    assert!(
        check_handwritten_sql(&[conforming]).is_empty(),
        "conforming const must pass"
    );

    let statement_receiver = ("e.rs".to_string(),
        "fn f() { let mut st = conn.prepare_cached(X_SQL)?; st.execute(rusqlite::named_params! {})?; }\nconst X_SQL: &str = \"\";\n// sql: X_SQL".to_string());
    assert!(
        check_handwritten_sql(&[statement_receiver]).is_empty(),
        "st.execute is a params call, not SQL"
    );
}

/// CENSUS-SELFTEST: checker.sql-ascii-leading
#[test]
fn ascii_leading_rule_catches_unicode_whitespace_prefix() {
    let (_d, conn) = migrated_db();
    let planted = [QueryCheck {
        name: "unicode_prefixed",
        sql: "\u{00A0}SELECT id FROM Java",
        params: &[],
        columns: None,
        class: carbon_repos::registry::QueryClass::Read,
        routes_write: false,
    }];
    let v = check_module(&conn, &planted);
    assert!(
        v.iter().any(|m| m.contains("non-ASCII whitespace")),
        "unicode-prefixed SQL must be flagged: {v:?}"
    );
    let clean = [QueryCheck {
        name: "ascii_prefixed",
        sql: "  \n\tSELECT id FROM Java",
        params: &[],
        columns: None,
        class: carbon_repos::registry::QueryClass::Read,
        routes_write: false,
    }];
    assert!(
        !check_module(&conn, &clean)
            .iter()
            .any(|m| m.contains("non-ASCII")),
        "ASCII-prefixed SQL must pass the leading-whitespace rule"
    );
}
