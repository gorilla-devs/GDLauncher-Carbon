use carbon_repos::repos::java as j;
use rusqlite::Connection;

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
    let (_d, conn) = migrated_db();
    assert_eq!(j::insert_java(&conn, &sample()).unwrap(), 1);
    assert_eq!(j::get_all_java(&conn).unwrap(), vec![sample()]);
    assert_eq!(j::get_java_by_id(&conn, "id-1").unwrap(), Some(sample()));
    assert_eq!(j::get_java_by_path(&conn, "/usr/bin/java").unwrap(), Some(sample()));
    assert_eq!(j::count_java(&conn).unwrap(), 1);
    assert_eq!(j::set_java_validity(&conn, "id-1", false).unwrap(), 1);
    assert!(!j::get_java_by_id(&conn, "id-1").unwrap().unwrap().is_valid);
    assert_eq!(j::delete_java(&conn, "id-1").unwrap(), 1);
    assert_eq!(j::count_java(&conn).unwrap(), 0);
}

#[test]
fn java_profile_crud_roundtrip() {
    let (_d, conn) = migrated_db();
    j::insert_java(&conn, &sample()).unwrap();
    assert_eq!(j::upsert_profile(&conn, "gaming", false).unwrap(), 1);
    assert_eq!(j::set_profile_java(&conn, "gaming", Some("id-1")).unwrap(), 1);
    let p = j::get_profile(&conn, "gaming").unwrap().unwrap();
    assert_eq!(p.java_id.as_deref(), Some("id-1"));
    assert_eq!(j::get_all_profiles(&conn).unwrap().len(), 1);
    assert_eq!(j::set_profile_java(&conn, "gaming", None).unwrap(), 1);
    assert_eq!(j::get_profile(&conn, "gaming").unwrap().unwrap().java_id, None);
}
