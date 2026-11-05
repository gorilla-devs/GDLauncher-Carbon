use super::*;
use mockito::Matcher;

fn verified(url: String, path: &Path) -> Downloadable {
    Downloadable::new(url, path)
        .with_size(4)
        .with_checksum(Some(Checksum::Sha1(
            "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".into(),
        )))
}
fn options() -> DownloadOptions {
    DownloadOptions::builder()
        .deep_check(true)
        .max_retries(0)
        .build()
}

// All actual requests are inert GETs to listeners owned by this test under the
// report-local cooperative guard. No real endpoints or credentials are used.
#[tokio::test]
async fn task09_ordered_mirrors_verify_reset_and_progress() {
    for (status, body, seed, unknown_size) in [
        (404, "", false, false),
        (500, "", false, false),
        (200, "bad!", false, false),
        (200, "bad", false, false),
        (206, "st", true, false),
        (200, "bad!", false, true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("text.txt");
        if seed {
            std::fs::write(legacy_partial(&path), b"zz").unwrap();
        }
        let mut server = mockito::Server::new_async().await;
        // A resumed attempt the range answer cannot serve first retries the same
        // URL once from zero, as a refused range says nothing about the URL.
        let first = server
            .mock("GET", "/first")
            .with_status(status)
            .with_body(body)
            .expect(if seed { 2 } else { 1 })
            .create_async()
            .await;
        let second = server
            .mock("GET", "/second")
            .match_header("range", Matcher::Missing)
            .match_header("x-api-key", Matcher::Missing)
            .with_status(200)
            .with_body("test")
            .expect(1)
            .create_async()
            .await;
        let mut file = verified(format!("{}/first", server.url()), &path);
        file.mirrors = vec![format!("{}/second", server.url())];
        if unknown_size {
            file.size = None;
        }
        let (tx, rx) = tokio::sync::watch::channel(Progress::new());
        let mut opts = options();
        opts.progress_sender = Some(tx);
        let result = download_multiple(&[file], opts).await;
        assert!(
            result.is_ok(),
            "candidate {status}/{body} must fall back: {result:?}"
        );
        first.assert_async().await;
        second.assert_async().await;
        assert_eq!(std::fs::read(path).unwrap(), b"test");
        let p = rx.borrow();
        assert_eq!(
            (p.total_count, p.current_count, p.total_size, p.current_size),
            (1, 1, 4, 4)
        );
        assert!(std::fs::read_dir(dir.path()).unwrap().all(|e| {
            let n = e.unwrap().file_name();
            !n.to_string_lossy().contains("owned")
        }));
    }
}
#[tokio::test]
async fn task09_mirrors_exhaust_once_and_preserve_single_url() {
    for mirrors in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("text");
        let mut server = mockito::Server::new_async().await;
        let first = server
            .mock("GET", "/first")
            .with_status(if mirrors { 404 } else { 200 })
            .with_body("test")
            .expect(1)
            .create_async()
            .await;
        let second = server
            .mock("GET", "/second")
            .with_status(404)
            .expect(usize::from(mirrors))
            .create_async()
            .await;
        let mut file = verified(format!("{}/first", server.url()), &path);
        if mirrors {
            file.mirrors = vec![format!("{}/second", server.url())];
        }
        let result = download_multiple(&[file], options()).await;
        assert_eq!(result.is_err(), mirrors);
        first.assert_async().await;
        second.assert_async().await;
        if mirrors {
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        }
    }
}
#[tokio::test]
async fn task09_complete_resume_uses_no_mirror() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("text");
    std::fs::write(legacy_partial(&path), b"test").unwrap();
    let mut server = mockito::Server::new_async().await;
    let any = server
        .mock("GET", Matcher::Any)
        .expect(0)
        .create_async()
        .await;
    let mut file = verified(format!("{}/first", server.url()), &path);
    file.mirrors = vec![format!("{}/second", server.url())];
    download_multiple(&[file], options()).await.unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"test");
    any.assert_async().await;
}
#[tokio::test]
async fn task09_cancel_preparation_and_publication_failure_never_try_mirror() {
    for cancel in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("text");
        let mut server = mockito::Server::new_async().await;
        let first = server
            .mock("GET", "/first")
            .with_status(200)
            .with_body("test")
            .expect(usize::from(!cancel))
            .create_async()
            .await;
        let second = server.mock("GET", "/second").expect(0).create_async().await;
        let mut file = verified(format!("{}/first", server.url()), &path);
        file.mirrors = vec![format!("{}/second", server.url())];
        let mut opts = options();
        let token = opts.cancel_token.clone();
        let pause = Arc::new(PartialPause {
            stage: if cancel {
                PartialStage::Preparing
            } else {
                PartialStage::Publishing
            },
            entered: tokio::sync::Notify::new(),
            release: tokio::sync::Semaphore::new(0),
        });
        opts.partial_pause = Some(pause.clone());
        let job = tokio::spawn(async move { download_multiple(&[file], opts).await });
        pause.entered.notified().await;
        if cancel {
            token.cancel();
        } else {
            std::fs::create_dir(&path).unwrap();
        }
        pause.release.add_permits(1);
        let result = job.await.unwrap();
        assert!(result.is_err());
        if cancel {
            assert!(matches!(result, Err(DownloadError::Cancelled)));
        }
        first.assert_async().await;
        second.assert_async().await;
    }
}

#[tokio::test]
async fn task09_cancel_held_candidate_never_admits_mirror() {
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let first_url = format!("http://{}/held", listener.local_addr().unwrap());
    let (entered, arrival) = tokio::sync::oneshot::channel();
    let (release, wait) = tokio::sync::oneshot::channel();
    let fixture = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(socket.read_u8().await.unwrap());
        }
        assert!(request.starts_with(b"GET /held HTTP/1.1\r\n"));
        entered.send(()).unwrap();
        let _ = wait.await;
    });
    let mut server = mockito::Server::new_async().await;
    let second = server.mock("GET", "/second").expect(0).create_async().await;
    let file = verified(first_url, &dir.path().join("text"))
        .with_mirrors(vec![format!("{}/second", server.url())]);
    let opts = options();
    let token = opts.cancel_token.clone();
    let job = tokio::spawn(async move { download_multiple(&[file], opts).await });
    arrival.await.unwrap();
    token.cancel();
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), job)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(result, Err(DownloadError::Cancelled)));
    release.send(()).unwrap();
    fixture.await.unwrap();
    second.assert_async().await;
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn task09_body_transport_failure_uses_mirror_without_old_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/short", listener.local_addr().unwrap());
    let fixture = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(socket.read_u8().await.unwrap());
        }
        assert!(request.starts_with(b"GET /short HTTP/1.1\r\n"));
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nba")
            .await
            .unwrap();
    });
    let mut server = mockito::Server::new_async().await;
    let second = server
        .mock("GET", "/second")
        .match_header("range", Matcher::Missing)
        .with_body("test")
        .expect(1)
        .create_async()
        .await;
    let path = dir.path().join("text");
    let file = verified(url, &path).with_mirrors(vec![format!("{}/second", server.url())]);
    download_multiple(&[file], options()).await.unwrap();
    fixture.await.unwrap();
    second.assert_async().await;
    assert_eq!(std::fs::read(path).unwrap(), b"test");
}

#[cfg(unix)]
#[tokio::test]
async fn task09_cleanup_uncertainty_retains_original_error_without_mirror() {
    use std::os::unix::fs::PermissionsExt;
    struct Restore(PathBuf, std::fs::Permissions);
    impl Drop for Restore {
        fn drop(&mut self) {
            std::fs::set_permissions(&self.0, self.1.clone()).unwrap();
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("text");
    std::fs::write(legacy_partial(&path), b"test").unwrap();
    std::fs::create_dir(&path).unwrap();
    let mut server = mockito::Server::new_async().await;
    let any = server
        .mock("GET", Matcher::Any)
        .expect(0)
        .create_async()
        .await;
    let file = verified(format!("{}/first", server.url()), &path)
        .with_mirrors(vec![format!("{}/second", server.url())]);
    let pause = Arc::new(PartialPause {
        stage: PartialStage::Publishing,
        entered: tokio::sync::Notify::new(),
        release: tokio::sync::Semaphore::new(0),
    });
    let mut opts = options();
    opts.partial_pause = Some(pause.clone());
    let job = tokio::spawn(async move { download_multiple(&[file], opts).await });
    pause.entered.notified().await;
    let restore = Restore(
        dir.path().to_owned(),
        std::fs::metadata(dir.path()).unwrap().permissions(),
    );
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
    assert!(std::fs::write(dir.path().join("denied"), b"x").is_err());
    pause.release.add_permits(1);
    let error = job.await.unwrap().unwrap_err();
    drop(restore);
    match error {
        DownloadError::OwnedPartialCleanup { original, path, .. } => {
            assert!(matches!(*original, DownloadError::CannotMoveFile(..)));
            assert_eq!(std::fs::read(path).unwrap(), b"test");
        }
        other => panic!("{other:?}"),
    }
    any.assert_async().await;
}
