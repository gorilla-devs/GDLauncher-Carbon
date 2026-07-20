use carbon_repos::dbtypes::from_millis;
use rusqlite::Connection;

const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test-assets/golden_pcr.db");
const KNOWN_MS: i64 = 1_784_557_692_104;

/// Regenerates the golden DB THROUGH PCR. Run manually while PCR still exists:
/// `cargo test -p carbon_repos --test golden -- --ignored regenerate`
#[tokio::test]
#[ignore]
async fn regenerate() {
    let _ = std::fs::remove_file(GOLDEN);
    // migrate first with the production runner
    let mut conn = Connection::open(GOLDEN).unwrap();
    let (migrations, _n) = carbon_repos::get_migrations();
    migrations.to_latest(&mut conn).unwrap();
    drop(conn);

    // write rows through PCR so the on-disk encoding is quaint's, not ours
    let client = carbon_repos::db::new_client_with_url(&format!("file:{GOLDEN}"))
        .await
        .unwrap();
    // Java's `id` is `@default(uuid())`, so it is not a positional create argument;
    // pin it to a known value through the optional-params vec.
    client
        .java()
        .create(
            "/golden/java".into(),
            17,
            "17.0.2".into(),
            "Local".into(),
            "linux".into(),
            "x64".into(),
            "Azul".into(),
            vec![carbon_repos::db::java::id::set("golden-java-id".into())],
        )
        .exec()
        .await
        .unwrap();
    client
        .account()
        .create(
            "golden-uuid".into(),
            "GoldenUser".into(),
            from_millis(KNOWN_MS).unwrap(),
            vec![carbon_repos::db::account::token_expires::set(Some(
                from_millis(KNOWN_MS).unwrap(),
            ))],
        )
        .exec()
        .await
        .unwrap();
    drop(client);

    // Collapse the WAL back into the main file so only golden_pcr.db needs committing.
    let conn = Connection::open(GOLDEN).unwrap();
    conn.pragma_update(None, "wal_checkpoint", &"TRUNCATE")
        .unwrap();
    drop(conn);
}

#[test]
fn golden_db_reads_back_exact_values_through_new_layer() {
    let conn = Connection::open(GOLDEN).unwrap();

    // storage class is INTEGER and the raw value is exact millis
    let (typeof_, raw): (String, i64) = conn
        .query_row(
            "SELECT typeof(tokenExpires), tokenExpires FROM Account WHERE uuid = 'golden-uuid'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(typeof_, "integer", "PCR must have written INTEGER millis");
    assert_eq!(raw, KNOWN_MS);

    // our codec decodes it to the same instant
    let via_codec: carbon_repos::dbtypes::DbDateTime = conn
        .query_row(
            "SELECT tokenExpires FROM Account WHERE uuid = 'golden-uuid'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(via_codec.0, from_millis(KNOWN_MS).unwrap());

    let major: i32 = conn
        .query_row(
            "SELECT major FROM Java WHERE id = 'golden-java-id'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(major, 17);
}
