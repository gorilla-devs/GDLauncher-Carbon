use super::*;
use carbon_repos::{
    db_exec::{Db, WriteAccess},
    dbtypes::{DbDateTime, from_millis},
    repos::{account, skin},
};
use reqwest_middleware::{ClientBuilder, Middleware, Next};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
struct Transport {
    responses: Arc<Mutex<VecDeque<anyhow::Result<reqwest::Response>>>>,
    calls: Arc<Mutex<Vec<(String, bool)>>>,
}
#[async_trait::async_trait]
impl Middleware for Transport {
    async fn handle(
        &self,
        request: reqwest::Request,
        _: &mut axum::http::Extensions,
        _: Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        assert_eq!(request.method(), reqwest::Method::GET);
        self.calls.lock().unwrap().push((
            request.url().to_string(),
            request.headers().contains_key("avoid-caching"),
        ));
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unplanned transport call")
            .map_err(reqwest_middleware::Error::Middleware)
    }
}
impl Transport {
    fn reply(&self, status: u16, body: Vec<u8>) {
        self.responses
            .lock()
            .unwrap()
            .push_back(Ok(hyper::Response::builder()
                .status(status)
                .body(body)
                .unwrap()
                .into()));
    }
    fn profile(&self) {
        self.reply(200, br#"{"id":"synthetic","name":"Example","skins":[{"id":"custom","state":"ACTIVE","url":"https://synthetic.invalid/skin"}]}"#.to_vec());
    }
    fn client(&self) -> reqwest_middleware::ClientWithMiddleware {
        ClientBuilder::new(reqwest::Client::builder().no_proxy().build().unwrap())
            .with(self.clone())
            .build()
    }
}
fn image(height: u32) -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    image::RgbaImage::new(64, height)
        .write_to(&mut output, ImageFormat::Png)
        .unwrap();
    output.into_inner()
}
async fn fixture(token: bool, id: Option<&str>) -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("skin.db");
    let mut conn = rusqlite::Connection::open(&path).unwrap();
    carbon_repos::get_migrations()
        .0
        .to_latest(&mut conn)
        .unwrap();
    drop(conn);
    let db = Db::open(&path, 2, false).unwrap();
    account::insert_account_microsoft(
        &db,
        "synthetic",
        "Example",
        DbDateTime(from_millis(1000).unwrap()),
        "dummy-token",
        None,
        Some("dummy-refresh"),
        Some("dummy-id"),
        Some("dummy-gdl"),
        id,
    )
    .await
    .unwrap();
    if !token {
        db.write(|conn| {
            conn.execute("UPDATE Account SET accessToken = NULL", [])?;
            Ok(())
        })
        .await
        .unwrap();
    }
    (dir, db)
}
async fn head(db: &Db, transport: &Transport) -> anyhow::Result<Vec<u8>> {
    let skin = get_skin(db, &transport.client(), "synthetic".into()).await?;
    stitch_head(&skin.data)
}

#[tokio::test]
async fn failed_response_never_poisoned_and_same_id_retries() {
    let (_dir, db) = fixture(true, None).await;
    let transport = Transport::default();
    transport.profile();
    transport.reply(404, b"not found".to_vec());
    assert!(head(&db, &transport).await.is_err());
    assert!(
        skin::get_skin(&db, "custom").await.unwrap().is_none(),
        "failed response persisted in Skin"
    );
    assert_eq!(
        account::get_account(&db, "synthetic")
            .await
            .unwrap()
            .unwrap()
            .skin_id,
        None
    );
    transport.profile();
    transport.reply(200, image(64));
    assert_eq!(
        image::load_from_memory(&head(&db, &transport).await.unwrap())
            .unwrap()
            .dimensions(),
        (144, 144)
    );
    assert_eq!(
        skin::get_skin(&db, "custom").await.unwrap().unwrap().skin,
        image(64)
    );
    let calls = transport.calls.lock().unwrap().len();
    head(&db, &transport).await.unwrap();
    assert_eq!(transport.calls.lock().unwrap().len(), calls);
    assert!(
        transport
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(url, _)| url.ends_with("/skin"))
            .all(|(_, bypass)| *bypass)
    );
}
#[tokio::test]
async fn invalid_success_bodies_never_publish() {
    for body in [b"error page".to_vec(), image(16), image(64)[..25].to_vec()] {
        let (_dir, db) = fixture(true, None).await;
        let transport = Transport::default();
        transport.profile();
        transport.reply(200, body);
        assert!(head(&db, &transport).await.is_err());
        assert!(
            skin::get_skin(&db, "custom").await.unwrap().is_none(),
            "invalid body persisted"
        );
        assert_eq!(
            account::get_account(&db, "synthetic")
                .await
                .unwrap()
                .unwrap()
                .skin_id,
            None
        );
        transport.profile();
        transport.reply(200, image(32));
        head(&db, &transport).await.unwrap();
    }
}
#[tokio::test]
async fn corrupt_cache_recovers_and_preserves_credentials() {
    let (_dir, db) = fixture(true, Some("custom")).await;
    let transport = Transport::default();
    skin::replace_skin_and_link_account(
        &db,
        "custom".into(),
        b"legacy error".to_vec(),
        "synthetic".into(),
    )
    .await
    .unwrap();
    skin::replace_skin_and_link_account(&db, "other".into(), image(32), "absent".into())
        .await
        .unwrap();
    let before = account::get_account(&db, "synthetic")
        .await
        .unwrap()
        .unwrap();
    transport.profile();
    transport.reply(200, image(64));
    head(&db, &transport)
        .await
        .expect("legacy corrupt cache must recover");
    assert_eq!(
        skin::get_skin(&db, "custom").await.unwrap().unwrap().skin,
        image(64)
    );
    assert_eq!(
        skin::get_skin(&db, "other").await.unwrap().unwrap().skin,
        image(32)
    );
    assert_eq!(
        account::get_account(&db, "synthetic")
            .await
            .unwrap()
            .unwrap(),
        before
    );
}
#[tokio::test]
async fn supported_cache_and_default_controls() {
    for height in [32, 64] {
        let (_dir, db) = fixture(true, Some("custom")).await;
        let transport = Transport::default();
        skin::replace_skin_and_link_account(
            &db,
            "custom".into(),
            image(height),
            "synthetic".into(),
        )
        .await
        .unwrap();
        assert_eq!(
            image::load_from_memory(&head(&db, &transport).await.unwrap())
                .unwrap()
                .dimensions(),
            (144, 144)
        );
        assert!(transport.calls.lock().unwrap().is_empty());
    }
    let (_dir, db) = fixture(false, None).await;
    let transport = Transport::default();
    transport.reply(200, image(32));
    head(&db, &transport).await.unwrap();
    assert_eq!(
        account::get_account(&db, "synthetic")
            .await
            .unwrap()
            .unwrap()
            .skin_id
            .as_deref(),
        Some("STEVE")
    );
    assert_eq!(transport.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn failed_custom_recovery_is_transient_and_keeps_later_retry() {
    let (_dir, db) = fixture(true, Some("custom")).await;
    let transport = Transport::default();
    skin::replace_skin_and_link_account(
        &db,
        "custom".into(),
        b"corrupt".to_vec(),
        "synthetic".into(),
    )
    .await
    .unwrap();
    skin::cache_skin(&db, "STEVE", &image(32)).await.unwrap();
    let before = account::get_account(&db, "synthetic")
        .await
        .unwrap()
        .unwrap();
    transport.profile();
    transport.reply(404, b"missing".to_vec());
    head(&db, &transport).await.unwrap();
    assert!(skin::get_skin(&db, "custom").await.unwrap().is_none());
    assert_eq!(
        account::get_account(&db, "synthetic")
            .await
            .unwrap()
            .unwrap(),
        before
    );
    transport.profile();
    transport.reply(200, image(64));
    head(&db, &transport).await.unwrap();
    assert_eq!(
        skin::get_skin(&db, "custom").await.unwrap().unwrap().skin,
        image(64)
    );
    assert_eq!(
        account::get_account(&db, "synthetic")
            .await
            .unwrap()
            .unwrap(),
        before
    );
}
#[tokio::test]
async fn corrupt_default_does_not_recurse_or_mask_custom_failure() {
    let (_dir, db) = fixture(true, Some("custom")).await;
    let transport = Transport::default();
    skin::cache_skin(&db, "custom", b"corrupt").await.unwrap();
    skin::cache_skin(&db, "STEVE", b"also corrupt")
        .await
        .unwrap();
    transport.profile();
    transport.reply(404, b"custom missing".to_vec());
    transport.reply(500, b"default failed".to_vec());
    assert!(
        head(&db, &transport)
            .await
            .unwrap_err()
            .to_string()
            .contains("404")
    );
    assert_eq!(transport.calls.lock().unwrap().len(), 3);
    assert!(skin::get_skin(&db, "custom").await.unwrap().is_none());
    assert!(skin::get_skin(&db, "STEVE").await.unwrap().is_none());
    assert_eq!(
        account::get_account(&db, "synthetic")
            .await
            .unwrap()
            .unwrap()
            .skin_id
            .as_deref(),
        Some("custom")
    );
}
#[tokio::test]
async fn conditional_eviction_preserves_concurrent_healthy_repair_and_queries_check() {
    let (dir, db) = fixture(true, Some("custom")).await;
    skin::cache_skin(&db, "custom", b"corrupt").await.unwrap();
    let observed = skin::get_skin(&db, "custom").await.unwrap().unwrap().skin;
    skin::cache_skin(&db, "custom", &image(64)).await.unwrap();
    assert_eq!(
        skin::evict_skin_if_unchanged(&db, "custom", &observed)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        skin::get_skin(&db, "custom").await.unwrap().unwrap().skin,
        image(64)
    );
    let conn = rusqlite::Connection::open(dir.path().join("skin.db")).unwrap();
    assert!(carbon_repos::checker::check_module(&conn, &skin::all_queries()).is_empty());
    let violations: i64 = conn
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(violations, 0);
}
#[tokio::test]
async fn transport_and_incomplete_body_failure_publish_nothing() {
    for partial_body in [false, true] {
        let (_dir, db) = fixture(true, None).await;
        let transport = Transport::default();
        transport.profile();
        if partial_body {
            let body = reqwest::Body::wrap_stream(futures::stream::iter([Err::<Vec<u8>, _>(
                std::io::Error::other("synthetic truncated body"),
            )]));
            transport
                .responses
                .lock()
                .unwrap()
                .push_back(Ok(hyper::Response::builder()
                    .status(200)
                    .body(body)
                    .unwrap()
                    .into()));
        } else {
            transport
                .responses
                .lock()
                .unwrap()
                .push_back(Err(anyhow::anyhow!("synthetic connection failure")));
        }
        assert!(head(&db, &transport).await.is_err());
        assert!(skin::get_skin(&db, "custom").await.unwrap().is_none());
        assert_eq!(
            account::get_account(&db, "synthetic")
                .await
                .unwrap()
                .unwrap()
                .skin_id,
            None
        );
    }
}
#[tokio::test]
async fn missing_profile_no_active_skin_and_format_compatibility_controls() {
    for profile in [None, Some(404), Some(500), Some(200)] {
        let (_dir, db) = fixture(profile.is_some(), None).await;
        let transport = Transport::default();
        if let Some(status) = profile {
            transport.reply(
                status,
                br#"{"id":"synthetic","name":"Example","skins":[]}"#.to_vec(),
            );
        }
        let mut bytes = Cursor::new(Vec::new());
        image::RgbaImage::new(64, 32)
            .write_to(&mut bytes, ImageFormat::Bmp)
            .unwrap();
        transport.reply(200, bytes.into_inner());
        head(&db, &transport).await.unwrap();
        assert_eq!(
            account::get_account(&db, "synthetic")
                .await
                .unwrap()
                .unwrap()
                .skin_id
                .as_deref(),
            Some("STEVE")
        );
    }
    assert_eq!(DefaultSkin::from_uuid("invalid".into()).skin_id(), "STEVE");
    assert!(matches!(
        get_skin(
            &fixture(false, None).await.1,
            &Transport::default().client(),
            "absent".into()
        )
        .await,
        Err(_)
    ));
}

#[tokio::test]
async fn non_success_status_with_supported_image_never_publishes() {
    for status in [302, 404] {
        let (_dir, db) = fixture(true, None).await;
        let transport = Transport::default();
        transport.profile();
        transport.reply(status, image(64));
        assert!(head(&db, &transport).await.is_err());
        assert!(skin::get_skin(&db, "custom").await.unwrap().is_none());
        assert_eq!(
            account::get_account(&db, "synthetic")
                .await
                .unwrap()
                .unwrap()
                .skin_id,
            None
        );
    }
}
