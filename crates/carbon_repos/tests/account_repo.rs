use carbon_repos::dbtypes::{DbDateTime, from_millis};
use carbon_repos::repos::{account as a, active_downloads as ad, skin as s};
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    (dir, conn)
}

// Real millis observed in the golden PCR fixture (see tests/golden.rs).
const KNOWN_MS: i64 = 1_784_557_692_104;

#[test]
fn insert_and_get_offline_account() {
    let (_d, conn) = migrated_db();
    let last_used = DbDateTime(from_millis(KNOWN_MS).unwrap());
    assert_eq!(
        a::insert_account_offline(&conn, "uuid-1", "Steve", last_used, None).unwrap(),
        1
    );
    let row = a::get_account(&conn, "uuid-1").unwrap().unwrap();
    assert_eq!(row.uuid, "uuid-1");
    assert_eq!(row.username, "Steve");
    assert_eq!(row.access_token, None);
    assert_eq!(row.token_expires, None);
    assert_eq!(row.last_used, from_millis(KNOWN_MS).unwrap());
    assert_eq!(row.skin_id, None);
}

#[test]
fn insert_and_get_microsoft_account_roundtrips_millis_exactly() {
    let (_d, conn) = migrated_db();
    let last_used = DbDateTime(from_millis(KNOWN_MS).unwrap());
    let token_expires = DbDateTime(from_millis(KNOWN_MS + 3_600_000).unwrap());
    a::insert_account_microsoft(
        &conn,
        "uuid-2",
        "Alex",
        last_used,
        "access-tok",
        Some(token_expires),
        Some("refresh-tok"),
        Some("id-tok"),
        Some("gdl-tok"),
        Some("skin-1"),
    )
    .unwrap();

    let row = a::get_account(&conn, "uuid-2").unwrap().unwrap();
    assert_eq!(row.access_token.as_deref(), Some("access-tok"));
    assert_eq!(row.ms_refresh_token.as_deref(), Some("refresh-tok"));
    assert_eq!(row.id_token.as_deref(), Some("id-tok"));
    assert_eq!(row.gdl_token.as_deref(), Some("gdl-tok"));
    assert_eq!(row.skin_id.as_deref(), Some("skin-1"));
    // exact millis round-trip, not just "close enough"
    assert_eq!(row.last_used, from_millis(KNOWN_MS).unwrap());
    assert_eq!(
        row.token_expires,
        Some(from_millis(KNOWN_MS + 3_600_000).unwrap())
    );

    // storage class is INTEGER with the exact millis value (matches PCR's encoding)
    let (typeof_, raw): (String, i64) = conn
        .query_row(
            "SELECT typeof(tokenExpires), tokenExpires FROM Account WHERE uuid = 'uuid-2'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(typeof_, "integer");
    assert_eq!(raw, KNOWN_MS + 3_600_000);
}

#[test]
fn update_offline_nulls_the_three_token_columns() {
    let (_d, conn) = migrated_db();
    let last_used = DbDateTime(from_millis(KNOWN_MS).unwrap());
    let token_expires = DbDateTime(from_millis(KNOWN_MS).unwrap());
    a::insert_account_microsoft(
        &conn,
        "uuid-3",
        "WasMicrosoft",
        last_used,
        "access-tok",
        Some(token_expires),
        Some("refresh-tok"),
        Some("id-tok"),
        Some("gdl-tok"),
        Some("skin-1"),
    )
    .unwrap();

    // Account switched to offline: username updates, the three token
    // columns are explicitly NULLed, everything else (idToken, gdlToken,
    // skinId, lastUsed) is untouched — mirrors PCR's SetParam list exactly.
    assert_eq!(
        a::update_account_offline(&conn, "uuid-3", "NowOffline").unwrap(),
        1
    );

    let row = a::get_account(&conn, "uuid-3").unwrap().unwrap();
    assert_eq!(row.username, "NowOffline");
    assert_eq!(row.access_token, None);
    assert_eq!(row.ms_refresh_token, None);
    assert_eq!(row.token_expires, None);
    // untouched by the offline update
    assert_eq!(row.id_token.as_deref(), Some("id-tok"));
    assert_eq!(row.gdl_token.as_deref(), Some("gdl-tok"));
    assert_eq!(row.skin_id.as_deref(), Some("skin-1"));
    assert_eq!(row.last_used, from_millis(KNOWN_MS).unwrap());
}

#[test]
fn update_microsoft_sets_all_seven_columns() {
    let (_d, conn) = migrated_db();
    let last_used = DbDateTime(from_millis(KNOWN_MS).unwrap());
    a::insert_account_offline(&conn, "uuid-4", "Old", last_used, None).unwrap();

    let new_expires = DbDateTime(from_millis(KNOWN_MS + 1000).unwrap());
    a::update_account_microsoft(
        &conn,
        "uuid-4",
        "New",
        "new-access",
        Some(new_expires),
        Some("new-refresh"),
        Some("new-id"),
        Some("new-gdl"),
        Some("new-skin"),
    )
    .unwrap();

    let row = a::get_account(&conn, "uuid-4").unwrap().unwrap();
    assert_eq!(row.username, "New");
    assert_eq!(row.access_token.as_deref(), Some("new-access"));
    assert_eq!(row.ms_refresh_token.as_deref(), Some("new-refresh"));
    assert_eq!(row.id_token.as_deref(), Some("new-id"));
    assert_eq!(row.gdl_token.as_deref(), Some("new-gdl"));
    assert_eq!(row.skin_id.as_deref(), Some("new-skin"));
    assert_eq!(row.token_expires, Some(from_millis(KNOWN_MS + 1000).unwrap()));
    // lastUsed is never touched by an update
    assert_eq!(row.last_used, from_millis(KNOWN_MS).unwrap());
}

#[test]
fn next_active_account_orders_by_last_used_desc_excluding_self() {
    let (_d, conn) = migrated_db();
    a::insert_account_offline(
        &conn,
        "older",
        "Older",
        DbDateTime(from_millis(1000).unwrap()),
        None,
    )
    .unwrap();
    a::insert_account_offline(
        &conn,
        "newest",
        "Newest",
        DbDateTime(from_millis(3000).unwrap()),
        None,
    )
    .unwrap();
    a::insert_account_offline(
        &conn,
        "middle",
        "Middle",
        DbDateTime(from_millis(2000).unwrap()),
        None,
    )
    .unwrap();

    // excluding the most-recently-used account should surface the next
    // most recent, not just any other row
    let next = a::get_next_active_account(&conn, "newest").unwrap().unwrap();
    assert_eq!(next.uuid, "middle");

    let all = a::get_accounts_by_last_used(&conn).unwrap();
    assert_eq!(
        all.iter().map(|r| r.uuid.as_str()).collect::<Vec<_>>(),
        vec!["newest", "middle", "older"]
    );
}

#[test]
fn set_account_gdl_token_and_expire_now() {
    let (_d, conn) = migrated_db();
    a::insert_account_offline(
        &conn,
        "uuid-5",
        "User",
        DbDateTime(from_millis(KNOWN_MS).unwrap()),
        None,
    )
    .unwrap();

    a::set_account_gdl_token(&conn, "uuid-5", Some("tok-a")).unwrap();
    assert_eq!(
        a::get_account(&conn, "uuid-5").unwrap().unwrap().gdl_token.as_deref(),
        Some("tok-a")
    );

    let now = DbDateTime(from_millis(KNOWN_MS + 42).unwrap());
    a::expire_account_token_now(&conn, "uuid-5", now).unwrap();
    assert_eq!(
        a::get_account(&conn, "uuid-5").unwrap().unwrap().token_expires,
        Some(from_millis(KNOWN_MS + 42).unwrap())
    );
}

#[test]
fn update_account_profile_sets_username_and_skin() {
    let (_d, conn) = migrated_db();
    a::insert_account_offline(
        &conn,
        "uuid-6",
        "OldName",
        DbDateTime(from_millis(KNOWN_MS).unwrap()),
        None,
    )
    .unwrap();

    a::update_account_profile(&conn, "uuid-6", "NewName", Some("skin-x")).unwrap();
    let row = a::get_account(&conn, "uuid-6").unwrap().unwrap();
    assert_eq!(row.username, "NewName");
    assert_eq!(row.skin_id.as_deref(), Some("skin-x"));
}

#[test]
fn delete_account_reports_rows_affected() {
    let (_d, conn) = migrated_db();
    a::insert_account_offline(
        &conn,
        "uuid-7",
        "Gone",
        DbDateTime(from_millis(KNOWN_MS).unwrap()),
        None,
    )
    .unwrap();

    assert_eq!(a::delete_account(&conn, "uuid-7").unwrap(), 1);
    assert_eq!(a::get_account(&conn, "uuid-7").unwrap(), None);
    // deleting again affects 0 rows — this is how the call site detects
    // the PCR `RecordNotFound` case without a dedicated error type.
    assert_eq!(a::delete_account(&conn, "uuid-7").unwrap(), 0);
}

#[test]
fn skin_get_missing_is_none() {
    let (_d, conn) = migrated_db();
    assert_eq!(s::get_skin(&conn, "nope").unwrap(), None);
}

#[test]
fn replace_skin_and_link_account_is_atomic_and_links() {
    let (_d, mut conn) = migrated_db();
    a::insert_account_offline(
        &conn,
        "uuid-8",
        "SkinOwner",
        DbDateTime(from_millis(KNOWN_MS).unwrap()),
        None,
    )
    .unwrap();

    s::replace_skin_and_link_account(&mut conn, "skin-abc", &[1, 2, 3, 4], "uuid-8").unwrap();

    let skin = s::get_skin(&conn, "skin-abc").unwrap().unwrap();
    assert_eq!(skin.skin, vec![1, 2, 3, 4]);
    let account = a::get_account(&conn, "uuid-8").unwrap().unwrap();
    assert_eq!(account.skin_id.as_deref(), Some("skin-abc"));
}

#[test]
fn replace_skin_and_link_account_replaces_existing_row_with_same_id() {
    let (_d, mut conn) = migrated_db();
    a::insert_account_offline(
        &conn,
        "uuid-9",
        "SkinOwner2",
        DbDateTime(from_millis(KNOWN_MS).unwrap()),
        None,
    )
    .unwrap();

    // first write, then overwrite the same skin id with new bytes — the
    // DELETE-then-INSERT shape must not error on a duplicate primary key
    s::replace_skin_and_link_account(&mut conn, "skin-shared", &[1], "uuid-9").unwrap();
    s::replace_skin_and_link_account(&mut conn, "skin-shared", &[9, 9, 9], "uuid-9").unwrap();

    let skin = s::get_skin(&conn, "skin-shared").unwrap().unwrap();
    assert_eq!(skin.skin, vec![9, 9, 9]);
}

#[test]
fn active_downloads_crud_roundtrip() {
    let (_d, conn) = migrated_db();
    assert_eq!(
        ad::find_active_download_by_url(&conn, "https://example.com/f.jar")
            .unwrap(),
        None
    );

    ad::insert_active_download(&conn, "https://example.com/f.jar", "file-id-1").unwrap();

    let row = ad::find_active_download_by_url(&conn, "https://example.com/f.jar")
        .unwrap()
        .unwrap();
    assert_eq!(row.file_id, "file-id-1");

    assert_eq!(
        ad::delete_active_download_by_file_id(&conn, "file-id-1").unwrap(),
        1
    );
    assert_eq!(
        ad::find_active_download_by_url(&conn, "https://example.com/f.jar").unwrap(),
        None
    );
    // deleting again affects 0 rows (mirrors the "won't error on 0 deleted" comment)
    assert_eq!(
        ad::delete_active_download_by_file_id(&conn, "file-id-1").unwrap(),
        0
    );
}
