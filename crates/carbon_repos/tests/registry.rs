use carbon_repos::db_exec::test_support::wg;
use rusqlite::Connection;

mod q {
    use carbon_repos::queries;

    #[derive(carbon_macro::FromRow, Debug, PartialEq)]
    pub struct Row {
        pub id: String,
        pub major: i32,
    }

    queries! {
        fn get_by_id(id: &str) -> Option<Row> = "SELECT id, major FROM Java WHERE id = :id";
        fn get_all() -> Vec<Row> = "SELECT id, major FROM Java";
        fn set_major(id: &str, major: i32) -> usize = "UPDATE Java SET major = :major WHERE id = :id";
    }
}

#[test]
fn typed_wrappers_and_registry() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE Java (id TEXT PRIMARY KEY, major INTEGER);
         INSERT INTO Java VALUES ('a', 17), ('b', 21);",
    )
    .unwrap();

    assert_eq!(
        q::get_by_id_conn(&wg(&mut conn), "a")
            .unwrap()
            .unwrap()
            .major,
        17
    );
    assert_eq!(q::get_all_conn(&wg(&mut conn)).unwrap().len(), 2);
    assert_eq!(q::set_major_conn(&wg(&mut conn), "a", 22).unwrap(), 1);
    assert_eq!(
        q::get_by_id_conn(&wg(&mut conn), "a")
            .unwrap()
            .unwrap()
            .major,
        22
    );
    assert_eq!(q::get_by_id_conn(&wg(&mut conn), "zz").unwrap(), None);

    // registry captures every query with its param names and row metadata
    assert_eq!(q::QUERIES.len(), 3);
    let set = q::QUERIES.iter().find(|c| c.name == "set_major").unwrap();
    assert_eq!(set.params, &[":id", ":major"]);
}

/// The macro's async wrappers must route a SELECT to the read pool and an
/// UPDATE to the write pool. Since the read pool is opened read-only, a write
/// misrouted there would fail loudly — so the write succeeding proves routing.
#[tokio::test]
async fn async_wrappers_route_reads_to_read_pool_and_writes_to_write_pool() {
    use carbon_repos::db_exec::Db;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    {
        let mut conn = Connection::open(&path).unwrap();
        let (m, _n) = carbon_repos::get_migrations();
        m.to_latest(&mut conn).unwrap();
        conn.execute_batch(
            "INSERT INTO Java (id, path, major, fullVersion, type, os, arch, vendor, isValid)
             VALUES ('a', '/j', 17, '17', 'local', 'linux', 'x64', 'az', 1)",
        )
        .unwrap();
    }
    let db = Db::open(&path, 2, false).unwrap();

    // read wrapper → read pool
    assert_eq!(q::get_by_id(&db, "a").await.unwrap().unwrap().major, 17);
    assert_eq!(q::get_all(&db).await.unwrap().len(), 1);
    // write wrapper → write pool (a misroute to the read-only pool would error)
    assert_eq!(q::set_major(&db, "a", 22).await.unwrap(), 1);
    assert_eq!(q::get_by_id(&db, "a").await.unwrap().unwrap().major, 22);
}

/// L5 (spec: pool routing keyed off return-shape, not SQL verb): a real
/// `queries!`-declared write whose return shape is `i64` rather than `usize`.
/// `RETURNING` makes any DML row-returning, so this fits the `i64` arm's
/// `query_row` shape and is legal Rust — but that arm hard-codes
/// `routes_write: false` (routes to the read pool) regardless of what the SQL
/// actually does, while `class_of` still correctly derives `Write` from the
/// leading verb. `check_pool_routing` (`src/checker.rs`) is the static fence
/// for this; the test below empirically reproduces the runtime failure it
/// exists to catch before it ships.
mod shape_verb_mismatch {
    use carbon_repos::queries;

    queries! {
        fn bump_major_returning(id: &str, major: i32) -> i64 =
            "UPDATE Java SET major = :major WHERE id = :id RETURNING major";
    }
}

#[tokio::test]
async fn a_write_declared_through_a_read_routing_arm_fails_against_the_read_only_pool() {
    use carbon_repos::db_exec::Db;
    use carbon_repos::registry::QueryClass;

    let check = shape_verb_mismatch::QUERIES[0];
    assert_eq!(
        check.class,
        QueryClass::Write,
        "the leading verb is UPDATE, so class_of must derive Write"
    );
    assert!(
        !check.routes_write,
        "the i64 arm always hard-codes routes_write: false"
    );

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    {
        let mut conn = Connection::open(&path).unwrap();
        let (m, _n) = carbon_repos::get_migrations();
        m.to_latest(&mut conn).unwrap();
        conn.execute_batch(
            "INSERT INTO Java (id, path, major, fullVersion, type, os, arch, vendor, isValid)
             VALUES ('a', '/j', 17, '17', 'local', 'linux', 'x64', 'az', 1)",
        )
        .unwrap();
    }

    // Sanity: the exact same statement succeeds when run directly against a
    // regular (non-read-only) connection — the SQL and the RETURNING shape
    // are both valid; only the pool it gets routed to is wrong.
    let mut direct_conn = Connection::open(&path).unwrap();
    let direct = shape_verb_mismatch::bump_major_returning_conn(&wg(&mut direct_conn), "a", 22);
    assert_eq!(direct.unwrap(), 22);

    // The async wrapper, routed by return-shape rather than verb, hits the
    // read-only pool and fails loudly (SQLITE_READONLY) rather than silently
    // no-op-ing or corrupting data.
    let db = Db::open(&path, 2, false).unwrap();
    let err = shape_verb_mismatch::bump_major_returning(&db, "a", 99).await;
    assert!(
        err.is_err(),
        "a write routed to the read-only pool must fail loudly, not silently succeed: {err:?}"
    );
}
