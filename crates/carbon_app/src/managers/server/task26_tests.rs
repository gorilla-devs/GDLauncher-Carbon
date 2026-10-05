use super::{ServerId, ServerSettingsUpdate, persist_server_settings};
use carbon_repos::{
    db_exec::{Db, WriteAccess},
    repos::server as repo,
};

async fn fixture() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("owned.db");
    {
        let mut conn = rusqlite::Connection::open(&path).unwrap();
        carbon_repos::get_migrations()
            .0
            .to_latest(&mut conn)
            .unwrap();
    }
    let db = Db::open(&path, 2, false).unwrap();
    db.write(|conn| {
        conn.execute("INSERT INTO Server(id,name,shortpath,\"index\",groupId,gameVersion,serverType,dateCreated,xmx,xms,extraJavaArgs) VALUES(7,'synthetic','synthetic',0,1,'1.21','vanilla','2026-10-05T00:00:00Z',2048,1024,'kept')", [])?;
        Ok(())
    }).await.unwrap();
    (dir, db)
}
fn update() -> ServerSettingsUpdate {
    ServerSettingsUpdate {
        server_id: ServerId(7),
        name: None,
        xmx: None,
        xms: None,
        extra_java_args: None,
        auto_restart: None,
    }
}
#[tokio::test]
async fn task26_heap_partial_patch_rejects_invalid_final_pair_without_any_write() {
    let (_dir, db) = fixture().await;
    let mut patch = update();
    patch.xmx = Some(512);
    patch.name = Some("must not write".into());
    assert!(
        persist_server_settings(&db, patch).await.is_err(),
        "merged Xms 1024 exceeds new Xmx 512"
    );
    let row = repo::get_server(&db, 7).await.unwrap().unwrap();
    assert_eq!((row.xmx, row.xms), (2048, 1024));
    assert_eq!(row.name, "synthetic");
    let mut patch = update();
    patch.xms = Some(4096);
    assert!(persist_server_settings(&db, patch).await.is_err());
}
#[tokio::test]
async fn task26_heap_writer_merges_two_queued_partial_patches_inside_serialized_publication() {
    let (_dir, db) = fixture().await;
    let mut lower = update();
    lower.xmx = Some(1536);
    let mut raise = update();
    raise.xms = Some(1792);
    let (a, b) = tokio::join!(
        persist_server_settings(&db, lower),
        persist_server_settings(&db, raise)
    );
    assert_ne!(
        a.is_ok(),
        b.is_ok(),
        "exactly one incompatible partial patch can publish in either queue order"
    );
    let row = repo::get_server(&db, 7).await.unwrap().unwrap();
    assert!(row.xms <= row.xmx);
    assert_eq!(
        (row.xmx, row.xms),
        if a.is_ok() {
            (1536, 1024)
        } else {
            (2048, 1792)
        }
    );
}
#[tokio::test]
async fn task26_heap_positive_bounds_coherent_pair_and_argument_omission_null_empty_controls() {
    let (_dir, db) = fixture().await;
    for (xmx, xms) in [(0, 256), (512, 0), (-1, 256), (512, -1)] {
        let mut patch = update();
        patch.xmx = Some(xmx);
        patch.xms = Some(xms);
        assert!(persist_server_settings(&db, patch).await.is_err());
    }
    let mut patch = update();
    patch.xmx = Some(512);
    patch.xms = Some(512);
    persist_server_settings(&db, patch).await.unwrap();
    let row = repo::get_server(&db, 7).await.unwrap().unwrap();
    assert_eq!((row.xmx, row.xms), (512, 512));
    assert_eq!(row.extra_java_args, "kept");
    let mut patch = update();
    patch.extra_java_args = Some(None);
    persist_server_settings(&db, patch).await.unwrap();
    assert_eq!(
        repo::get_server(&db, 7)
            .await
            .unwrap()
            .unwrap()
            .extra_java_args,
        ""
    );
    let mut patch = update();
    patch.extra_java_args = Some(Some("new".into()));
    patch.auto_restart = Some(true);
    persist_server_settings(&db, patch).await.unwrap();
    let mut patch = update();
    patch.xmx = Some(3072);
    persist_server_settings(&db, patch).await.unwrap();
    let row = repo::get_server(&db, 7).await.unwrap().unwrap();
    assert_eq!((row.xmx, row.xms), (3072, 512));
    assert_eq!(row.extra_java_args, "new");
    assert!(row.auto_restart);
    let mut patch = update();
    patch.extra_java_args = Some(Some(String::new()));
    persist_server_settings(&db, patch).await.unwrap();
    assert_eq!(
        repo::get_server(&db, 7)
            .await
            .unwrap()
            .unwrap()
            .extra_java_args,
        ""
    );
}
