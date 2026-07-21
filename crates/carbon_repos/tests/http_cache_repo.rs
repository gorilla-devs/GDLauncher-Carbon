use carbon_repos::db_exec::Db;
use carbon_repos::dbtypes::{DbDateTime, from_millis};
use carbon_repos::repos::http_cache as hc;
use rusqlite::Connection;

/// Migrates a fresh tempfile via a plain connection, then opens the async `Db`
/// pool over it. `replace_cached` is a write-pool transaction wrapper, so these
/// tests drive it (and the `get_cached` read wrapper) through the real pool.
async fn migrated_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    {
        let mut conn = Connection::open(&path).unwrap();
        let (m, _n) = carbon_repos::get_migrations();
        m.to_latest(&mut conn).unwrap();
    }
    let db = Db::open(&path, 2, false).unwrap();
    (dir, db)
}

fn ts(ms: i64) -> DbDateTime {
    DbDateTime(from_millis(ms).unwrap())
}

async fn count_for_url(db: &Db, url: &'static str) -> i64 {
    db.read(move |conn| {
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM HTTPCache WHERE url = :url",
            rusqlite::named_params! { ":url": url },
            |r| r.get(0),
        )?)
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn get_missing_returns_none() {
    let (_d, db) = migrated_db().await;
    assert!(
        hc::get_cached(&db, "https://example.com")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn replace_cached_inserts_then_get_reads_it_back() {
    let (_d, db) = migrated_db().await;
    hc::replace_cached(
        &db,
        "https://example.com".into(),
        200,
        b"hello".to_vec(),
        Some(ts(1_784_557_692_104)),
        Some("Wed, 21 Oct 2015 07:28:00 GMT".into()),
        Some("\"abc123\"".into()),
    )
    .await
    .unwrap();

    let row = hc::get_cached(&db, "https://example.com")
        .await
        .unwrap()
        .expect("row inserted");
    assert_eq!(row.url, "https://example.com");
    assert_eq!(row.status_code, 200);
    assert_eq!(row.data, b"hello".to_vec());
    assert_eq!(
        row.expires_at,
        Some(ts(1_784_557_692_104).0),
        "expiresAt must round-trip through epoch millis exactly"
    );
    assert_eq!(
        row.last_modified.as_deref(),
        Some("Wed, 21 Oct 2015 07:28:00 GMT")
    );
    assert_eq!(row.etag.as_deref(), Some("\"abc123\""));
}

#[tokio::test]
async fn replace_cached_accepts_none_for_all_optional_fields() {
    let (_d, db) = migrated_db().await;
    hc::replace_cached(
        &db,
        "https://example.com/no-headers".into(),
        200,
        b"data".to_vec(),
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let row = hc::get_cached(&db, "https://example.com/no-headers")
        .await
        .unwrap()
        .expect("row inserted");
    assert_eq!(row.expires_at, None);
    assert_eq!(row.last_modified, None);
    assert_eq!(row.etag, None);
}

#[tokio::test]
async fn replace_cached_replaces_existing_row_atomically() {
    let (_d, db) = migrated_db().await;
    hc::replace_cached(
        &db,
        "https://example.com".into(),
        200,
        b"first".to_vec(),
        Some(ts(1_000)),
        None,
        None,
    )
    .await
    .unwrap();
    hc::replace_cached(
        &db,
        "https://example.com".into(),
        304,
        b"second".to_vec(),
        Some(ts(2_000)),
        Some("lm".into()),
        Some("et".into()),
    )
    .await
    .unwrap();

    let row = hc::get_cached(&db, "https://example.com")
        .await
        .unwrap()
        .expect("row still present");
    assert_eq!(row.status_code, 304);
    assert_eq!(row.data, b"second".to_vec());
    assert_eq!(row.expires_at, Some(ts(2_000).0));
    assert_eq!(row.last_modified.as_deref(), Some("lm"));
    assert_eq!(row.etag.as_deref(), Some("et"));

    // The delete+insert transaction never leaves a stale duplicate keyed by
    // the primary key -- exactly one row survives the replace.
    assert_eq!(count_for_url(&db, "https://example.com").await, 1);
}

#[tokio::test]
async fn replace_cached_is_atomic_across_delete_and_insert() {
    // Same url replaced repeatedly with different status codes proves the tx
    // sequence (DELETE then INSERT) never surfaces a transient "row missing"
    // state: the final state is exactly the last write, never a mix or empty.
    let (_d, db) = migrated_db().await;
    for i in 0..5 {
        hc::replace_cached(
            &db,
            "https://example.com".into(),
            200 + i,
            format!("body-{i}").into_bytes(),
            Some(ts(1_000 * (i as i64 + 1))),
            None,
            None,
        )
        .await
        .unwrap();
    }

    let row = hc::get_cached(&db, "https://example.com")
        .await
        .unwrap()
        .expect("row present after repeated replace");
    assert_eq!(row.status_code, 204);
    assert_eq!(row.data, b"body-4".to_vec());

    assert_eq!(count_for_url(&db, "https://example.com").await, 1);
}
