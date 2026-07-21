use carbon_repos::dbtypes::{from_millis, DbDateTime};
use carbon_repos::repos::mod_file_cache as mfc;
use rusqlite::Connection;
use carbon_repos::db_exec::test_support::wg;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    // Mirror production (db_exec opens connections without enabling FK — that is
    // Plan 3). The migration files leave `foreign_keys=ON`, so turn it back off
    // to match the runtime and to seed cache rows without full parent chains.
    conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
    (dir, conn)
}

fn ts(ms: i64) -> DbDateTime {
    DbDateTime(from_millis(ms).unwrap())
}

/// Insert a bare `ModMetadata` row. `name`/`modid` optional; hashes are the
/// two blobs the platform lookups key on.
fn insert_metadata(
    conn: &Connection,
    id: &str,
    murmur2: i32,
    sha512: &[u8],
    sha1: &[u8],
    modid: Option<&str>,
    name: Option<&str>,
) {
    conn.execute(
        "INSERT INTO ModMetadata (id, murmur2, sha512, sha1, modid, name, modloaders)
         VALUES (?, ?, ?, ?, ?, ?, 'forge')",
        rusqlite::params![id, murmur2, sha512, sha1, modid, name],
    )
    .unwrap();
}

#[allow(clippy::too_many_arguments)]
fn insert_cf(
    conn: &Connection,
    metadata_id: &str,
    project_id: i32,
    file_id: i32,
    name: &str,
    urlslug: &str,
    release_type: i32,
    update_paths: &str,
    cached_at: i64,
) {
    conn.execute(
        "INSERT INTO CurseForgeModCache
           (metadataId, murmur2, projectId, fileId, name, version, urlslug, summary, authors, releaseType, updatePaths, cachedAt)
         VALUES (?, 0, ?, ?, ?, 'v', ?, 'sum', 'auth', ?, ?, ?)",
        rusqlite::params![metadata_id, project_id, file_id, name, urlslug, release_type, update_paths, cached_at],
    )
    .unwrap();
}

#[allow(clippy::too_many_arguments)]
fn insert_mr(
    conn: &Connection,
    metadata_id: &str,
    project_id: &str,
    version_id: &str,
    title: &str,
    urlslug: &str,
    file_url: &str,
    cached_at: i64,
) {
    conn.execute(
        "INSERT INTO ModrinthModCache
           (metadataId, sha512, projectId, versionId, title, version, urlslug, description, authors, releaseType, updatePaths, filename, fileUrl, cachedAt)
         VALUES (?, 'sha', ?, ?, ?, 'v', ?, 'desc', 'auth', 2, '', 'f.jar', ?, ?)",
        rusqlite::params![metadata_id, project_id, version_id, title, urlslug, file_url, cached_at],
    )
    .unwrap();
}

// --- upsert semantics -------------------------------------------------------

#[test]
fn upsert_inserts_then_conflict_preserves_id_and_refreshes_freshness() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "meta-1", 111, b"s512", b"s1", None, None);

    let n = mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 7, "a.jar", 100, true, "mods", "meta-1", ts(1000))
        .unwrap();
    assert_eq!(n, 1);

    let row = mfc::get_mod_file_cache_by_instance_filename_conn(&wg(&mut conn), 7, "a.jar")
        .unwrap()
        .unwrap();
    let original_id = row.id.clone();
    assert!(!original_id.is_empty());
    assert_eq!(row.filesize, 100);
    assert_eq!(row.enabled, true);
    assert_eq!(row.last_updated_at, ts(1000).0);

    // Conflict on (instanceId, filename): row keeps its id, other cols + freshness update.
    insert_metadata(&conn, "meta-2", 222, b"s512b", b"s1b", None, None);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 7, "a.jar", 200, false, "shaders", "meta-2", ts(5000))
        .unwrap();

    let row = mfc::get_mod_file_cache_by_instance_filename_conn(&wg(&mut conn), 7, "a.jar")
        .unwrap()
        .unwrap();
    assert_eq!(row.id, original_id, "id must survive the conflict");
    assert_eq!(row.filesize, 200);
    assert_eq!(row.enabled, false);
    assert_eq!(row.addon_type, "shaders");
    assert_eq!(row.metadata_id, "meta-2");
    assert_eq!(row.last_updated_at, ts(5000).0, "freshness column refreshed");

    // Exactly one row for this instance.
    assert_eq!(mfc::get_mod_files_by_instance_conn(&wg(&mut conn), 7).unwrap().len(), 1);
}

#[test]
fn upsert_generates_distinct_uuids_per_row() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m", 1, b"a", b"b", None, None);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "a.jar", 1, true, "mods", "m", ts(1)).unwrap();
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "b.jar", 1, true, "mods", "m", ts(1)).unwrap();
    let rows = mfc::get_mod_files_by_instance_conn(&wg(&mut conn), 1).unwrap();
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0].id, rows[1].id);
    // uuid v4 shape (36 chars with dashes)
    assert_eq!(rows[0].id.len(), 36);
}

#[test]
fn server_upsert_conflict_preserves_id_and_freshness() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m", 1, b"a", b"b", None, None);
    mfc::upsert_server_mod_file_cache_conn(&wg(&mut conn), 3, "a.jar", 10, true, "mods", "m", ts(1000)).unwrap();
    let row = mfc::get_server_mod_files_by_server_conn(&wg(&mut conn), 3).unwrap().remove(0);
    let id = row.id.clone();
    assert_eq!(row.last_updated_at, ts(1000).0);

    mfc::upsert_server_mod_file_cache_conn(&wg(&mut conn), 3, "a.jar", 20, false, "datapacks", "m", ts(2000))
        .unwrap();
    let row = mfc::get_server_mod_file_cache_by_id_conn(&wg(&mut conn), &id).unwrap().unwrap();
    assert_eq!(row.id, id);
    assert_eq!(row.filesize, 20);
    assert_eq!(row.enabled, false);
    assert_eq!(row.addon_type, "datapacks");
    assert_eq!(row.last_updated_at, ts(2000).0);
}

// --- update / delete --------------------------------------------------------

#[test]
fn update_enabled_sets_flag_and_freshness() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m", 1, b"a", b"b", None, None);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "a.jar", 1, true, "mods", "m", ts(1)).unwrap();
    let id = mfc::get_mod_files_by_instance_conn(&wg(&mut conn), 1).unwrap().remove(0).id;

    assert_eq!(mfc::update_mod_file_enabled_conn(&wg(&mut conn), &id, false, ts(9999)).unwrap(), 1);
    let row = mfc::get_mod_file_cache_by_id_conn(&wg(&mut conn), &id).unwrap().unwrap();
    assert_eq!(row.enabled, false);
    assert_eq!(row.last_updated_at, ts(9999).0);

    assert_eq!(mfc::update_server_mod_file_enabled_conn(&wg(&mut conn), "nope", true, ts(1)).unwrap(), 0);
}

#[test]
fn deletes_scope_correctly() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m", 1, b"a", b"b", None, None);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "a.jar", 1, true, "mods", "m", ts(1)).unwrap();
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "b.jar", 1, true, "mods", "m", ts(1)).unwrap();
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 2, "c.jar", 1, true, "mods", "m", ts(1)).unwrap();

    assert_eq!(mfc::delete_mod_file_cache_by_instance_filename_conn(&wg(&mut conn), 1, "a.jar").unwrap(), 1);
    assert_eq!(mfc::get_mod_files_by_instance_conn(&wg(&mut conn), 1).unwrap().len(), 1);
    assert_eq!(mfc::delete_mod_file_cache_by_instance_conn(&wg(&mut conn), 1).unwrap(), 1);
    assert_eq!(mfc::get_mod_files_by_instance_conn(&wg(&mut conn), 1).unwrap().len(), 0);
    // instance 2 untouched
    assert_eq!(mfc::get_mod_files_by_instance_conn(&wg(&mut conn), 2).unwrap().len(), 1);
}

// --- full flat JOIN row -----------------------------------------------------

#[test]
fn full_row_decodes_populated_and_bare_metadata() {
    let (_d, mut conn) = migrated_db();

    // Fully-populated: metadata + local image + cf(+image w/ data) + mr(+image w/ data).
    insert_metadata(&conn, "full", 111, b"S512", b"S1", Some("jei"), Some("JEI"));
    conn.execute(
        "INSERT INTO LocalModImageCache (metadataId, data) VALUES ('full', X'0102')",
        [],
    )
    .unwrap();
    insert_cf(&conn, "full", 500, 900, "JEI CF", "jei-cf", 2, "1.20,forge,stable", 123);
    conn.execute(
        "INSERT INTO CurseForgeModImageCache (metadataId, url, data, upToDate) VALUES ('full', 'cfurl', X'AA', 1)",
        [],
    )
    .unwrap();
    insert_mr(&conn, "full", "PROJ", "VER", "JEI MR", "jei-mr", "http://f", 123);
    conn.execute(
        "INSERT INTO ModrinthModImageCache (metadataId, url, data, upToDate) VALUES ('full', 'mrurl', X'BB', 1)",
        [],
    )
    .unwrap();
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "full.jar", 42, true, "mods", "full", ts(1)).unwrap();

    // Bare: metadata only, no platform rows, no local image.
    insert_metadata(&conn, "bare", 222, b"b512", b"b1", None, None);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "bare.jar", 7, false, "resourcepacks", "bare", ts(1))
        .unwrap();

    let mut rows = mfc::get_instance_mods_full_conn(&wg(&mut conn), 1).unwrap();
    rows.sort_by(|a, b| a.filename.cmp(&b.filename));
    assert_eq!(rows.len(), 2);

    let bare = &rows[0];
    assert_eq!(bare.filename, "bare.jar");
    assert_eq!(bare.enabled, false);
    assert_eq!(bare.addon_type, "resourcepacks");
    assert_eq!(bare.meta_id, "bare");
    assert_eq!(bare.sha512, b"b512");
    assert_eq!(bare.murmur2, 222);
    assert_eq!(bare.has_local_image, false);
    assert_eq!(bare.cf_project_id, None);
    assert_eq!(bare.has_cf_image, false);
    assert_eq!(bare.mr_project_id, None);
    assert_eq!(bare.has_mr_image, false);

    let full = &rows[1];
    assert_eq!(full.filename, "full.jar");
    assert_eq!(full.meta_name.as_deref(), Some("JEI"));
    assert_eq!(full.modid.as_deref(), Some("jei"));
    assert_eq!(full.sha512, b"S512");
    assert_eq!(full.has_local_image, true);
    assert_eq!(full.cf_project_id, Some(500));
    assert_eq!(full.cf_file_id, Some(900));
    assert_eq!(full.cf_name.as_deref(), Some("JEI CF"));
    assert_eq!(full.cf_release_type, Some(2));
    assert_eq!(full.cf_update_paths.as_deref(), Some("1.20,forge,stable"));
    assert_eq!(full.has_cf_image, true);
    assert_eq!(full.mr_project_id.as_deref(), Some("PROJ"));
    assert_eq!(full.mr_version_id.as_deref(), Some("VER"));
    assert_eq!(full.mr_title.as_deref(), Some("JEI MR"));
    assert_eq!(full.has_mr_image, true);
}

#[test]
fn cf_image_presence_checks_data_for_instance_but_relation_for_server() {
    // Instance full row: has_cf_image = (ci.data IS NOT NULL).
    // Server full row:   has_cf_image = (ci.metadataId IS NOT NULL).
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m", 1, b"a", b"b", None, None);
    insert_cf(&conn, "m", 1, 1, "n", "s", 2, "", 1);
    // image row exists but data is NULL (marked stale, not yet downloaded)
    conn.execute(
        "INSERT INTO CurseForgeModImageCache (metadataId, url, data, upToDate) VALUES ('m', 'u', NULL, 0)",
        [],
    )
    .unwrap();
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "a.jar", 1, true, "mods", "m", ts(1)).unwrap();
    mfc::upsert_server_mod_file_cache_conn(&wg(&mut conn), 1, "a.jar", 1, true, "mods", "m", ts(1)).unwrap();

    let inst = mfc::get_instance_mods_full_conn(&wg(&mut conn), 1).unwrap().remove(0);
    assert_eq!(inst.has_cf_image, false, "instance checks data presence");

    let srv = mfc::get_server_mods_full_conn(&wg(&mut conn), 1).unwrap().remove(0);
    assert_eq!(srv.has_cf_image, true, "server checks relation presence");
    assert_eq!(srv.cf_project_id, Some(1));
}

// --- icon data --------------------------------------------------------------

#[test]
fn icon_data_returns_all_three_blobs() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m", 1, b"a", b"b", None, None);
    conn.execute(
        "INSERT INTO LocalModImageCache (metadataId, data) VALUES ('m', X'01')",
        [],
    )
    .unwrap();
    insert_cf(&conn, "m", 1, 1, "n", "s", 2, "", 1);
    conn.execute(
        "INSERT INTO CurseForgeModImageCache (metadataId, url, data, upToDate) VALUES ('m', 'u', X'02', 1)",
        [],
    )
    .unwrap();
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "a.jar", 1, true, "mods", "m", ts(1)).unwrap();
    let id = mfc::get_mod_files_by_instance_conn(&wg(&mut conn), 1).unwrap().remove(0).id;

    let icon = mfc::get_instance_mod_icon_data_conn(&wg(&mut conn), &id).unwrap().unwrap();
    assert_eq!(icon.local_data, Some(vec![1u8]));
    assert_eq!(icon.cf_data, Some(vec![2u8]));
    assert_eq!(icon.mr_data, None);
}

// --- refresh / stale-logo filters ------------------------------------------

#[test]
fn cf_refresh_selects_missing_and_stale_excluding_worlds() {
    let (_d, mut conn) = migrated_db();
    // no cf row -> needs refresh
    insert_metadata(&conn, "missing", 1, b"a", b"b", None, None);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "missing.jar", 1, true, "mods", "missing", ts(1)).unwrap();
    // fresh cf row (cachedAt AFTER cutoff) -> excluded
    insert_metadata(&conn, "fresh", 2, b"a", b"b", None, None);
    insert_cf(&conn, "fresh", 1, 1, "n", "s", 2, "", 10_000);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "fresh.jar", 1, true, "mods", "fresh", ts(1)).unwrap();
    // stale cf row (cachedAt BEFORE cutoff) -> included
    insert_metadata(&conn, "stale", 3, b"a", b"b", None, None);
    insert_cf(&conn, "stale", 2, 2, "n", "s", 2, "", 100);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "stale.jar", 1, true, "mods", "stale", ts(1)).unwrap();
    // world addon with no cf -> excluded by addonType filter
    insert_metadata(&conn, "world", 4, b"a", b"b", None, None);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "world1", 1, true, "worlds", "world", ts(1)).unwrap();

    let mut ids: Vec<String> = mfc::instance_mods_needing_cf_refresh_conn(&wg(&mut conn), 1, ts(5000))
        .unwrap()
        .into_iter()
        .map(|r| r.metadata_id)
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["missing".to_string(), "stale".to_string()]);
}

#[test]
fn mr_refresh_and_server_variants_work() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m1", 1, b"a", b"b", None, None);
    mfc::upsert_server_mod_file_cache_conn(&wg(&mut conn), 9, "a.jar", 1, true, "mods", "m1", ts(1)).unwrap();
    // fresh mr -> excluded
    insert_metadata(&conn, "m2", 2, b"a", b"b", None, None);
    insert_mr(&conn, "m2", "p", "v", "t", "s", "f", 10_000);
    mfc::upsert_server_mod_file_cache_conn(&wg(&mut conn), 9, "b.jar", 1, true, "mods", "m2", ts(1)).unwrap();

    let ids: Vec<String> = mfc::server_mods_needing_mr_refresh_conn(&wg(&mut conn), 9, ts(5000))
        .unwrap()
        .into_iter()
        .map(|r| r.metadata_id)
        .collect();
    assert_eq!(ids, vec!["m1".to_string()]);
}

#[test]
fn stale_cf_logo_selects_only_uptodate_zero() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m", 1, b"a", b"b", None, None);
    insert_cf(&conn, "m", 77, 88, "n", "s", 2, "", 1);
    conn.execute(
        "INSERT INTO CurseForgeModImageCache (metadataId, url, data, upToDate) VALUES ('m', 'the-url', NULL, 0)",
        [],
    )
    .unwrap();
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "a.jar", 1, true, "mods", "m", ts(1)).unwrap();

    let rows = mfc::instance_mods_stale_cf_logo_conn(&wg(&mut conn), 1).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].project_id, 77);
    assert_eq!(rows[0].file_id, 88);
    assert_eq!(rows[0].metadata_id, "m");
    assert_eq!(rows[0].url, "the-url");

    // mark up to date -> no longer selected
    conn.execute("UPDATE CurseForgeModImageCache SET upToDate = 1 WHERE metadataId = 'm'", [])
        .unwrap();
    assert_eq!(mfc::instance_mods_stale_cf_logo_conn(&wg(&mut conn), 1).unwrap().len(), 0);
}

// --- existence / update-id / shader / export --------------------------------

#[test]
fn exists_by_project_matches_only_installed() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m", 1, b"a", b"b", None, None);
    insert_cf(&conn, "m", 4242, 1, "n", "s", 2, "", 1);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 5, "a.jar", 1, true, "mods", "m", ts(1)).unwrap();

    assert!(mfc::instance_mod_exists_by_cf_project_conn(&wg(&mut conn), 5, 4242).unwrap().is_some());
    assert!(mfc::instance_mod_exists_by_cf_project_conn(&wg(&mut conn), 5, 9999).unwrap().is_none());
    // wrong instance
    assert!(mfc::instance_mod_exists_by_cf_project_conn(&wg(&mut conn), 6, 4242).unwrap().is_none());
}

#[test]
fn update_ids_and_platform_ids() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m", 1, b"a", b"b", None, None);
    insert_cf(&conn, "m", 10, 20, "n", "s", 2, "", 1);
    insert_mr(&conn, "m", "mp", "mv", "t", "s", "f", 1);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "a.jar", 1, true, "mods", "m", ts(1)).unwrap();
    let id = mfc::get_mod_files_by_instance_conn(&wg(&mut conn), 1).unwrap().remove(0).id;

    let cf = mfc::get_instance_mod_cf_ids_conn(&wg(&mut conn), &id).unwrap().unwrap();
    assert_eq!((cf.cf_project_id, cf.cf_file_id), (Some(10), Some(20)));
    let mr = mfc::get_instance_mod_mr_ids_conn(&wg(&mut conn), &id).unwrap().unwrap();
    assert_eq!(mr.mr_project_id.as_deref(), Some("mp"));
    let both = mfc::get_instance_mod_update_ids_conn(&wg(&mut conn), &id).unwrap().unwrap();
    assert_eq!(both.cf_project_id, Some(10));
    assert_eq!(both.mr_version_id.as_deref(), Some("mv"));
}

#[test]
fn enabled_modids_only() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "on", 1, b"a", b"b", Some("iris"), None);
    insert_metadata(&conn, "off", 2, b"a", b"b", Some("optifine"), None);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "on.jar", 1, true, "mods", "on", ts(1)).unwrap();
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "off.jar", 1, false, "mods", "off", ts(1)).unwrap();

    let modids: Vec<String> = mfc::get_enabled_instance_mod_modids_conn(&wg(&mut conn), 1)
        .unwrap()
        .into_iter()
        .filter_map(|r| r.modid)
        .collect();
    assert_eq!(modids, vec!["iris".to_string()]);
}

#[test]
fn export_queries_shape() {
    let (_d, mut conn) = migrated_db();
    insert_metadata(&conn, "m", 55, b"S512", b"S1", None, Some("meta name"));
    insert_cf(&conn, "m", 1, 2, "cfname", "cfslug", 2, "", 1);
    insert_mr(&conn, "m", "mrp", "mrv", "mrtitle", "mrslug", "http://url", 1);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "a.jar", 321, true, "mods", "m", ts(1)).unwrap();
    // datapacks entry should be excluded from get_instance_export_mods (addonType = 'mods')
    insert_metadata(&conn, "dp", 66, b"a", b"b", None, None);
    mfc::upsert_mod_file_cache_conn(&wg(&mut conn), 1, "d.zip", 1, true, "datapacks", "dp", ts(1)).unwrap();

    let shared = mfc::get_instance_export_mods_conn(&wg(&mut conn), 1).unwrap();
    assert_eq!(shared.len(), 1);
    assert_eq!(shared[0].cf_name.as_deref(), Some("cfname"));
    assert_eq!(shared[0].mr_title.as_deref(), Some("mrtitle"));

    let mr = mfc::get_instance_mr_export_files_conn(&wg(&mut conn), 1).unwrap();
    let a = mr.iter().find(|r| r.filename == "a.jar").unwrap();
    assert_eq!(a.filesize, 321);
    assert_eq!(a.sha512, b"S512");
    assert_eq!(a.mr_file_url.as_deref(), Some("http://url"));

    let gdl = mfc::get_instance_gdl_export_files_conn(&wg(&mut conn), 1).unwrap();
    let a = gdl.iter().find(|r| r.filename == "a.jar").unwrap();
    assert_eq!(a.murmur2, 55);
    assert_eq!(a.cf_project_id, Some(1));
    assert_eq!(a.mr_project_id.as_deref(), Some("mrp"));

    let cf = mfc::get_instance_cf_export_files_conn(&wg(&mut conn), 1).unwrap();
    let a = cf.iter().find(|r| r.filename == "a.jar").unwrap();
    assert_eq!((a.cf_project_id, a.cf_file_id), (Some(1), Some(2)));
}

