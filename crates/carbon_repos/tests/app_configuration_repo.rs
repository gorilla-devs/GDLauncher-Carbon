use carbon_repos::repos::app_configuration as ac;
use rusqlite::Connection;

fn migrated_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = Connection::open(dir.path().join("t.db")).unwrap();
    let (m, _n) = carbon_repos::get_migrations();
    m.to_latest(&mut conn).unwrap();
    (dir, conn)
}

#[test]
fn singleton_insert_produces_id_0_with_ddl_defaults() {
    let (_d, conn) = migrated_db();
    assert_eq!(
        ac::insert_app_configuration(&conn, "beta", 2048, Some("inst-123")).unwrap(),
        1
    );
    let row = ac::get_app_configuration(&conn).unwrap().unwrap();
    assert_eq!(row.id, 0);
    assert_eq!(row.release_channel, "beta");
    assert_eq!(row.xmx, 2048);
    assert_eq!(row.installation_id.as_deref(), Some("inst-123"));
    // DDL defaults from the migrated schema
    assert_eq!(row.theme, "main");
    assert_eq!(row.language, "english");
    assert_eq!(row.xms, 1024);
    assert!(row.discord_integration);
    assert!(row.download_dependencies);
    assert_eq!(row.instances_tile_size, 2);
    assert_eq!(row.servers_tile_size, 2);
    assert!(!row.terms_and_privacy_accepted);
    assert_eq!(row.mod_channels, "stable:true,beta:true,alpha:true");
    assert!(row.auto_manage_java_system_profiles);
    // nullable columns default to NULL
    assert_eq!(row.default_instance_group, None);
    assert_eq!(row.default_server_group, None);
    assert_eq!(row.instances_sort_by, None);
    assert_eq!(ac::count_app_configuration(&conn).unwrap(), 1);
}

#[test]
fn singleton_insert_null_installation_id() {
    let (_d, conn) = migrated_db();
    ac::insert_app_configuration(&conn, "stable", 4096, None).unwrap();
    let row = ac::get_app_configuration(&conn).unwrap().unwrap();
    assert_eq!(row.installation_id, None);
}

#[test]
fn patch_empty_builds_none() {
    let patch = ac::AppConfigurationPatch::default();
    assert!(patch.build().is_none());
}

#[test]
fn patch_single_field_sql() {
    let patch = ac::AppConfigurationPatch {
        theme: Some("dark".into()),
        ..Default::default()
    };
    let q = patch.build().unwrap();
    assert_eq!(q.sql, "UPDATE AppConfiguration SET theme = :theme WHERE id = 0");
    assert_eq!(q.params.len(), 1);
}

#[test]
fn patch_multi_field_sql() {
    let patch = ac::AppConfigurationPatch {
        theme: Some("dark".into()),
        xmx: Some(8192),
        ..Default::default()
    };
    let q = patch.build().unwrap();
    assert_eq!(
        q.sql,
        "UPDATE AppConfiguration SET theme = :theme, xmx = :xmx WHERE id = 0"
    );
    assert_eq!(q.params.len(), 2);
}

#[test]
fn patch_executes_and_persists_single_field() {
    let (_d, conn) = migrated_db();
    ac::insert_app_configuration(&conn, "stable", 4096, None).unwrap();
    let patch = ac::AppConfigurationPatch {
        theme: Some("dark".into()),
        xmx: Some(8192),
        ..Default::default()
    };
    let affected = patch.build().unwrap().execute(&conn).unwrap();
    assert_eq!(affected, 1);
    let row = ac::get_app_configuration(&conn).unwrap().unwrap();
    assert_eq!(row.theme, "dark");
    assert_eq!(row.xmx, 8192);
    // untouched columns preserved
    assert_eq!(row.language, "english");
}

#[test]
fn patch_nullable_set_to_null_persists() {
    let (_d, conn) = migrated_db();
    ac::insert_app_configuration(&conn, "stable", 4096, None).unwrap();
    // First set a non-null value.
    ac::AppConfigurationPatch {
        instances_sort_by: Some(Some("name".into())),
        ..Default::default()
    }
    .build()
    .unwrap()
    .execute(&conn)
    .unwrap();
    assert_eq!(
        ac::get_app_configuration(&conn)
            .unwrap()
            .unwrap()
            .instances_sort_by
            .as_deref(),
        Some("name")
    );
    // Now clear it back to NULL.
    ac::AppConfigurationPatch {
        instances_sort_by: Some(None),
        ..Default::default()
    }
    .build()
    .unwrap()
    .execute(&conn)
    .unwrap();
    assert_eq!(
        ac::get_app_configuration(&conn)
            .unwrap()
            .unwrap()
            .instances_sort_by,
        None
    );
}

#[test]
fn patch_persists_blob_column() {
    let (_d, conn) = migrated_db();
    ac::insert_app_configuration(&conn, "stable", 4096, None).unwrap();
    let blob = vec![1u8, 2, 3, 4];
    ac::AppConfigurationPatch {
        gdl_account_status: Some(Some(blob.clone())),
        ..Default::default()
    }
    .build()
    .unwrap()
    .execute(&conn)
    .unwrap();
    assert_eq!(
        ac::get_app_configuration(&conn)
            .unwrap()
            .unwrap()
            .gdl_account_status,
        Some(blob)
    );
}
