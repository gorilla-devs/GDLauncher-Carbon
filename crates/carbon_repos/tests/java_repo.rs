use carbon_repos::repos::java as j;
use rusqlite::Connection;
use carbon_repos::db_exec::test_support::{rg, wg};

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    (dir, conn)
}

fn sample() -> j::JavaRow {
    j::JavaRow {
        id: "id-1".into(), path: "/usr/bin/java".into(), major: 17,
        full_version: "17.0.2".into(), r#type: "Local".into(),
        os: "linux".into(), arch: "x64".into(), vendor: "Azul".into(), is_valid: true,
    }
}

#[test]
fn java_crud_roundtrip() {
    let (_d, mut conn) = migrated_db();
    assert_eq!(j::insert_java_conn(&wg(&mut conn), &sample()).unwrap(), 1);
    assert_eq!(j::get_all_java_conn(&wg(&mut conn)).unwrap(), vec![sample()]);
    assert_eq!(j::get_java_by_id_conn(&wg(&mut conn), "id-1").unwrap(), Some(sample()));
    assert_eq!(j::get_java_by_path_conn(&wg(&mut conn), "/usr/bin/java").unwrap(), Some(sample()));
    assert_eq!(j::count_java_conn(&wg(&mut conn)).unwrap(), 1);
    assert_eq!(j::set_java_validity_conn(&wg(&mut conn), "id-1", false).unwrap(), 1);
    assert!(!j::get_java_by_id_conn(&wg(&mut conn), "id-1").unwrap().unwrap().is_valid);
    assert_eq!(j::delete_java_conn(&wg(&mut conn), "id-1").unwrap(), 1);
    assert_eq!(j::count_java_conn(&wg(&mut conn)).unwrap(), 0);
}

#[test]
fn java_profile_crud_roundtrip() {
    let (_d, mut conn) = migrated_db();
    j::insert_java_conn(&wg(&mut conn), &sample()).unwrap();
    assert_eq!(j::upsert_profile_conn(&wg(&mut conn), "gaming", false).unwrap(), 1);
    assert_eq!(j::set_profile_java_conn(&wg(&mut conn), "gaming", Some("id-1")).unwrap(), 1);
    let p = j::get_profile_conn(&wg(&mut conn), "gaming").unwrap().unwrap();
    assert_eq!(p.java_id.as_deref(), Some("id-1"));
    assert_eq!(j::get_all_profiles_conn(&wg(&mut conn)).unwrap().len(), 1);
    assert_eq!(j::set_profile_java_conn(&wg(&mut conn), "gaming", None).unwrap(), 1);
    assert_eq!(j::get_profile_conn(&wg(&mut conn), "gaming").unwrap().unwrap().java_id, None);
}


fn sample2() -> j::JavaRow {
    j::JavaRow {
        id: "id-2".into(), path: "/opt/java/bin/java".into(), major: 8,
        full_version: "8.0.392".into(), r#type: "Managed".into(),
        os: "linux".into(), arch: "x64".into(), vendor: "Temurin".into(), is_valid: true,
    }
}

#[test]
fn java_lookups_and_updates_by_type_and_path() {
    let (_d, mut conn) = migrated_db();
    j::insert_java_conn(&wg(&mut conn), &sample()).unwrap();
    j::insert_java_conn(&wg(&mut conn), &sample2()).unwrap();

    // get_java_by_type filters on the (keyword-quoted) `type` column.
    let managed = j::get_java_by_type_conn(&wg(&mut conn), "Managed").unwrap();
    assert_eq!(managed, vec![sample2()]);
    assert!(j::get_java_by_type_conn(&wg(&mut conn), "Custom").unwrap().is_empty());

    // set_java_validity_by_path flips exactly the row at that path.
    assert_eq!(
        j::set_java_validity_by_path_conn(&wg(&mut conn), "/opt/java/bin/java", false).unwrap(),
        1
    );
    assert!(!j::get_java_by_id_conn(&wg(&mut conn), "id-2").unwrap().unwrap().is_valid);
    assert!(j::get_java_by_id_conn(&wg(&mut conn), "id-1").unwrap().unwrap().is_valid);
    assert_eq!(j::set_java_validity_by_path_conn(&wg(&mut conn), "/nope", true).unwrap(), 0);

    // update_java_component rewrites the component fields and forces isValid = 1.
    assert_eq!(
        j::update_java_component_conn(&wg(&mut conn), "id-2", 21, "21.0.2", "arm64", "macos", "Zulu")
            .unwrap(),
        1
    );
    let row = j::get_java_by_id_conn(&wg(&mut conn), "id-2").unwrap().unwrap();
    assert_eq!(row.major, 21);
    assert_eq!(row.full_version, "21.0.2");
    assert_eq!(row.arch, "arm64");
    assert_eq!(row.os, "macos");
    assert_eq!(row.vendor, "Zulu");
    assert!(row.is_valid, "update_java_component marks the component valid");

    // delete_java_by_path removes exactly the row at that path.
    assert_eq!(j::delete_java_by_path_conn(&wg(&mut conn), "/usr/bin/java").unwrap(), 1);
    assert_eq!(j::count_java_conn(&wg(&mut conn)).unwrap(), 1);
    assert_eq!(j::delete_java_by_path_conn(&wg(&mut conn), "/usr/bin/java").unwrap(), 0);
}

#[test]
fn profile_linked_paths_and_delete_profile() {
    let (_d, mut conn) = migrated_db();
    j::insert_java_conn(&wg(&mut conn), &sample()).unwrap();
    j::insert_java_conn(&wg(&mut conn), &sample2()).unwrap();
    j::upsert_profile_conn(&wg(&mut conn), "p1", false).unwrap();
    j::upsert_profile_conn(&wg(&mut conn), "p2", true).unwrap();
    j::upsert_profile_conn(&wg(&mut conn), "p3", false).unwrap();
    j::set_profile_java_conn(&wg(&mut conn), "p1", Some("id-1")).unwrap();
    j::set_profile_java_conn(&wg(&mut conn), "p2", Some("id-2")).unwrap();
    // p3 stays unlinked: the INNER JOIN must skip it.

    let mut paths: Vec<String> = j::get_profile_linked_java_paths_conn(&wg(&mut conn))
        .unwrap()
        .into_iter()
        .map(|r| r.path)
        .collect();
    paths.sort();
    assert_eq!(paths, vec!["/opt/java/bin/java".to_string(), "/usr/bin/java".to_string()]);

    assert_eq!(j::delete_profile_conn(&wg(&mut conn), "p2").unwrap(), 1);
    assert_eq!(j::get_all_profiles_conn(&wg(&mut conn)).unwrap().len(), 2);
    let paths: Vec<String> = j::get_profile_linked_java_paths_conn(&wg(&mut conn))
        .unwrap()
        .into_iter()
        .map(|r| r.path)
        .collect();
    assert_eq!(paths, vec!["/usr/bin/java".to_string()]);
    assert_eq!(j::delete_profile_conn(&wg(&mut conn), "nope").unwrap(), 0);
}

/// Read-class `_conn` fns accept `ReadAccess`-only guards — exercised at the
/// suite level, not just by the compile_fail doctests.
#[test]
fn read_class_fns_accept_read_guards() {
    let (_d, mut conn) = migrated_db();
    j::insert_java_conn(&wg(&mut conn), &sample()).unwrap();
    assert_eq!(j::get_all_java_conn(&rg(&conn)).unwrap().len(), 1);
    assert_eq!(j::get_java_by_id_conn(&rg(&conn), "id-1").unwrap(), Some(sample()));
    assert_eq!(j::get_java_by_path_conn(&rg(&conn), "/usr/bin/java").unwrap(), Some(sample()));
    assert_eq!(j::count_java_conn(&rg(&conn)).unwrap(), 1);
    assert_eq!(j::get_all_profiles_conn(&rg(&conn)).unwrap().len(), 0);
}
