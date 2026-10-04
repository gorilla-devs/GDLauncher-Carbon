use super::*;
use std::time::Duration;
use tokio::sync::oneshot;

// Inert owned-loopback GET fixture. No inherited endpoint, credential, or external URL.
async fn held_response(
    response: &'static str,
) -> (
    String,
    oneshot::Receiver<()>,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/fixture", listener.local_addr().unwrap());
    let (arrive, arrived) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let byte = socket.read_u8().await.unwrap();
            request.push(byte);
        }
        assert!(request.starts_with(b"GET /fixture HTTP/1.1\r\n"));
        arrive.send(()).unwrap();
        let _ = released.await;
        let _ = socket.write_all(response.as_bytes()).await;
    });
    (url, arrived, release, task)
}
const OK: &str = "HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntest";
fn options(token: CancellationToken) -> DownloadOptions {
    DownloadOptions::builder()
        .deep_check(true)
        .max_retries(0)
        .cancel_token(token)
        .build()
}
fn verified(url: String, path: &Path) -> Downloadable {
    Downloadable::new(url, path)
        .with_size(4)
        .with_checksum(Some(Checksum::Sha1(
            "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3".into(),
        )))
}
#[tokio::test]
async fn task06_cancel_headers_before_release() {
    let dir = tempfile::tempdir().unwrap();
    let (url, accepted, release, fixture) = held_response(OK).await;
    let token = CancellationToken::new();
    let file = verified(url, &dir.path().join("file"));
    let mut owner = tokio::spawn(download_owned(vec![file.clone()], options(token.clone())));
    accepted.await.unwrap();
    token.cancel();
    let before_release = tokio::time::timeout(Duration::from_millis(400), &mut owner).await;
    release.send(()).unwrap();
    fixture.await.unwrap();
    let timely = before_release.is_ok();
    let result = match before_release {
        Ok(r) => r.unwrap(),
        Err(_) => owner.await.unwrap(),
    };
    assert!(
        timely,
        "cancellation must finish while response headers are withheld"
    );
    assert!(matches!(result, Err(DownloadError::Cancelled)));
    assert!(!file.path.exists());
}
async fn download_owned(
    files: Vec<Downloadable>,
    options: DownloadOptions,
) -> Result<bool, DownloadError> {
    download_multiple(&files, options).await
}
#[tokio::test]
async fn task06_batch_failure_stops_held_worker() {
    let dir = tempfile::tempdir().unwrap();
    let (bad_url, bad_arrived, bad_release, bad_fixture) =
        held_response("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
    let (url, arrived, release, fixture) = held_response(OK).await;
    let file = verified(url, &dir.path().join("held"));
    let (tx, mut rx) = tokio::sync::watch::channel(Progress::new());
    let mut opts = options(CancellationToken::new());
    opts.progress_sender = Some(tx);
    let owner = tokio::spawn(download_owned(
        vec![
            Downloadable::new(bad_url, dir.path().join("bad")),
            file.clone(),
        ],
        opts,
    ));
    bad_arrived.await.unwrap();
    arrived.await.unwrap();
    bad_release.send(()).unwrap();
    assert!(owner.await.unwrap().is_err());
    bad_fixture.await.unwrap();
    // All worker progress senders must be gone before the owner reports failure.
    let settled = rx.has_changed().is_err();
    release.send(()).unwrap();
    fixture.await.unwrap();
    while rx.changed().await.is_ok() {}
    assert!(settled, "batch returned with live worker ownership");
    assert!(!file.path.exists());
}
#[tokio::test]
async fn task06_raw_owner_drop_closes_publication() {
    let dir = tempfile::tempdir().unwrap();
    let (url, arrived, release, fixture) = held_response(OK).await;
    let file = verified(url, &dir.path().join("file"));
    let (tx, mut rx) = tokio::sync::watch::channel(Progress::new());
    let mut opts = options(CancellationToken::new());
    opts.progress_sender = Some(tx);
    let owner = tokio::spawn(download_owned(vec![file.clone()], opts));
    arrived.await.unwrap();
    owner.abort();
    assert!(owner.await.unwrap_err().is_cancelled());
    release.send(()).unwrap();
    fixture.await.unwrap();
    while rx.changed().await.is_ok() {}
    assert!(
        !file.path.exists(),
        "dropped owner must have no final-publication authority"
    );
}
#[tokio::test]
async fn task06_independent_owner_revalidates() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file");
    let (url, arrived, release, fixture) = held_response(OK).await;
    let file = verified(url, &path);
    let owner = tokio::spawn(download_owned(
        vec![file],
        options(CancellationToken::new()),
    ));
    arrived.await.unwrap();
    // B validates only: this benign ownership test never gives B a writable partial.
    let b_file = verified("http://127.0.0.1:1/never-requested".into(), &path);
    let mut b_opts = options(CancellationToken::new());
    b_opts.only_validate = true;
    let mut b = tokio::spawn(download_owned(vec![b_file], b_opts));
    let queued = tokio::time::timeout(Duration::from_millis(100), &mut b).await;
    release.send(()).unwrap();
    fixture.await.unwrap();
    assert!(!owner.await.unwrap().unwrap());
    let was_queued = queued.is_err();
    let result = match queued {
        Ok(r) => r.unwrap(),
        Err(_) => b.await.unwrap(),
    };
    assert!(
        was_queued,
        "validation must queue behind existing destination owner"
    );
    assert_eq!(result.unwrap(), false);
    assert_eq!(std::fs::read(path).unwrap(), b"test");
}
#[tokio::test]
async fn task06_resume_protocol() {
    for (range, body, accepted) in [
        (Some("bytes 2-3/4"), "st", true),
        (Some("bytes 1-3/4"), "est", false),
        (None, "st", false),
        (Some("bytes 3-4/5"), "st", false),
        (Some("bytes 2-3/5"), "st", false),
        (Some("bytes 2-4/4"), "st", false),
        (Some("bytes 2-3/*"), "st", false),
        (Some("bytes 2-18446744073709551616/4"), "st", false),
        (Some("bytes 2-3/4"), "s", false),
        (Some("bytes 2-3/4"), "stu", false),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        let legacy = path.with_extension("txt.__gdl_part~");
        std::fs::write(&legacy, b"te").unwrap();
        let mut server = mockito::Server::new_async().await;
        let mut mock = server
            .mock("GET", "/range")
            .match_header("range", "bytes=2-")
            .with_status(206)
            .with_body(body);
        if let Some(range) = range {
            mock = mock.with_header("content-range", range);
        }
        let mock = mock.expect(1).create_async().await;
        // A refused range earns one request from zero, which fails here too.
        let fresh = server
            .mock("GET", "/range")
            .match_header("range", mockito::Matcher::Missing)
            .with_status(404)
            .expect(usize::from(!accepted))
            .create_async()
            .await;
        let result = download_multiple(
            &[Downloadable::new(format!("{}/range", server.url()), &path)],
            options(CancellationToken::new()),
        )
        .await;
        mock.assert_async().await;
        fresh.assert_async().await;
        assert_eq!(
            result.is_ok(),
            accepted,
            "range {range:?}, body {body:?}: {result:?}"
        );
        if accepted {
            assert_eq!(std::fs::read(path).unwrap(), b"test");
        } else {
            assert!(!path.exists());
        }
        // Spent on success; proven stale by a refused range otherwise.
        assert!(!legacy.exists(), "range {range:?}, body {body:?}");
    }
}
#[tokio::test]
async fn task06_complete_partial_416_restarts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file.txt");
    std::fs::write(path.with_extension("txt.__gdl_part~"), b"test").unwrap();
    let mut server = mockito::Server::new_async().await;
    let eof = server
        .mock("GET", "/file")
        .match_header("range", "bytes=4-")
        .with_status(416)
        .with_header("content-range", "bytes */4")
        .expect(1)
        .create_async()
        .await;
    let full = server
        .mock("GET", "/file")
        .match_header("range", mockito::Matcher::Missing)
        .with_status(200)
        .with_body("test")
        .expect(1)
        .create_async()
        .await;
    let result = download_multiple(
        &[Downloadable::new(format!("{}/file", server.url()), &path).with_size(4)],
        options(CancellationToken::new()),
    )
    .await;
    assert!(result.is_ok(), "{result:?}");
    eof.assert_async().await;
    full.assert_async().await;
    assert_eq!(std::fs::read(path).unwrap(), b"test");
}
#[tokio::test]
async fn task06_complete_verified_partial_needs_no_request() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file.txt");
    std::fs::write(path.with_extension("txt.__gdl_part~"), b"test").unwrap();
    let file = verified("invalid URL intentionally never requested".into(), &path);
    assert!(
        download_multiple(&[file.clone()], options(CancellationToken::new()))
            .await
            .is_ok()
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"test");
    assert!(
        download_multiple(&[file], options(CancellationToken::new()))
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn task06_header_stall_uses_production_read_budget() {
    let dir = tempfile::tempdir().unwrap();
    let (url, arrived, release, fixture) = held_response(OK).await;
    let file = verified(url, &dir.path().join("file"));
    let mut opts = options(CancellationToken::new());
    opts.read_stall_timeout = Duration::from_millis(80);
    let mut owner = tokio::spawn(download_owned(vec![file.clone()], opts));
    arrived.await.unwrap();
    let bounded = tokio::time::timeout(Duration::from_secs(2), &mut owner).await;
    release.send(()).unwrap();
    fixture.await.unwrap();
    assert!(
        bounded
            .expect("headers exceeded configured stall budget")
            .unwrap()
            .is_err()
    );
    assert!(!file.path.exists());
}
#[tokio::test]
async fn task06_body_stall_and_cancellation() {
    for cancel in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let file = verified(
            format!("http://{}/fixture", listener.local_addr().unwrap()),
            &dir.path().join("file"),
        );
        let token = CancellationToken::new();
        let mut opts = options(token.clone());
        opts.read_stall_timeout = if cancel {
            Duration::from_secs(30)
        } else {
            Duration::from_millis(80)
        };
        let (started, arrived) = oneshot::channel();
        let (release, released) = oneshot::channel();
        let fixture = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                request.push(socket.read_u8().await.unwrap());
            }
            assert!(request.starts_with(b"GET /fixture "));
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nt")
                .await
                .unwrap();
            started.send(()).unwrap();
            let _ = released.await;
            let _ = socket.write_all(b"est").await;
        });
        let owner = tokio::spawn(download_owned(vec![file.clone()], opts));
        arrived.await.unwrap();
        if cancel {
            token.cancel();
        }
        let result = tokio::time::timeout(Duration::from_secs(2), owner)
            .await
            .unwrap()
            .unwrap();
        if cancel {
            assert!(matches!(result, Err(DownloadError::Cancelled)));
        } else {
            assert!(result.is_err());
        }
        release.send(()).unwrap();
        fixture.await.unwrap();
        assert!(!file.path.exists());
    }
}
#[tokio::test]
async fn task06_progressing_body_has_no_small_total_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let file = Downloadable::new(
        format!("http://{}/fixture", listener.local_addr().unwrap()),
        dir.path().join("file"),
    );
    let mut opts = options(CancellationToken::new());
    opts.read_stall_timeout = Duration::from_millis(300);
    let fixture = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(socket.read_u8().await.unwrap());
        }
        assert!(request.starts_with(b"GET /fixture "));
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\n")
            .await
            .unwrap();
        for byte in b"progress" {
            tokio::time::sleep(Duration::from_millis(75)).await;
            socket.write_all(&[*byte]).await.unwrap();
        }
    });
    assert_eq!(
        download_multiple(&[file.clone()], opts).await.unwrap(),
        false
    );
    fixture.await.unwrap();
    assert_eq!(std::fs::read(file.path).unwrap(), b"progress");
}
#[tokio::test]
async fn task06_cancel_queued_destination_and_zero_admission() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file");
    let (url, arrived, release, fixture) = held_response(OK).await;
    let file = verified(url, &path);
    let owner = tokio::spawn(download_owned(
        vec![file.clone()],
        options(CancellationToken::new()),
    ));
    arrived.await.unwrap();
    for concurrency in [0, 1] {
        let token = CancellationToken::new();
        let mut opts = options(token.clone());
        opts.concurrency = concurrency;
        let queued = tokio::spawn(download_owned(vec![file.clone()], opts));
        tokio::task::yield_now().await;
        token.cancel();
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(2), queued)
                .await
                .unwrap()
                .unwrap(),
            Err(DownloadError::Cancelled)
        ));
    }
    release.send(()).unwrap();
    fixture.await.unwrap();
    owner.await.unwrap().unwrap();
}
#[tokio::test]
async fn task06_cancel_retry_backoff() {
    let dir = tempfile::tempdir().unwrap();
    let (url, arrived, release, fixture) =
        held_response("HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
    let file = verified(url, &dir.path().join("file"));
    let token = CancellationToken::new();
    let mut opts = options(token.clone());
    opts.max_retries = 3;
    let owner = tokio::spawn(download_owned(vec![file.clone()], opts));
    arrived.await.unwrap();
    release.send(()).unwrap();
    fixture.await.unwrap();
    tokio::task::yield_now().await;
    token.cancel();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(2), owner)
            .await
            .unwrap()
            .unwrap(),
        Err(DownloadError::Cancelled)
    ));
    assert!(!file.path.exists());
}
#[tokio::test]
async fn task06_closed_publication_cannot_commit() {
    let dir = tempfile::tempdir().unwrap();
    let part = dir.path().join("part");
    let target = dir.path().join("final");
    std::fs::write(&part, b"test").unwrap();
    let publication = Arc::new(Publication::default());
    let token = CancellationToken::new();
    let owner = DownloadOwner {
        publication: publication.clone(),
        cancel: token.clone(),
    };
    let mut opts = options(token);
    opts.publication = Some(publication);
    drop(owner);
    assert!(matches!(
        publish(&part, &target, &opts),
        Err(DownloadError::Cancelled)
    ));
    assert!(!target.exists());
    assert_eq!(std::fs::read(&part).unwrap(), b"test");
}
#[tokio::test]
async fn task06_inconsistent_416_cannot_publish() {
    // Either answer to the ranged request earns one fresh request, and a server
    // that refuses that one too still publishes nothing. The leftover that drew
    // the refused range is gone, so the next attempt cannot repeat it.
    for total in ["5", "4"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        let legacy = path.with_extension("txt.__gdl_part~");
        std::fs::write(&legacy, b"test").unwrap();
        let mut server = mockito::Server::new_async().await;
        let eof = server
            .mock("GET", "/file")
            .with_status(416)
            .with_header("content-range", &format!("bytes */{total}"))
            .expect(2)
            .create_async()
            .await;
        assert!(
            download_multiple(
                &[Downloadable::new(format!("{}/file", server.url()), &path).with_size(4)],
                options(CancellationToken::new())
            )
            .await
            .is_err()
        );
        eof.assert_async().await;
        assert!(!path.exists());
        assert!(!legacy.exists(), "total {total}: stale leftover kept");
    }
}

#[tokio::test]
async fn task06_chunked_resume_checks_actual_extent() {
    for body in ["s", "st", "stu"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        std::fs::write(path.with_extension("txt.__gdl_part~"), b"te").unwrap();
        let mut server = mockito::Server::new_async().await;
        let response = server
            .mock("GET", "/range")
            .match_header("range", "bytes=2-")
            .with_status(206)
            .with_header("content-range", "bytes 2-3/4")
            .with_chunked_body(move |w| w.write_all(body.as_bytes()))
            .expect(1)
            .create_async()
            .await;
        let result = download_multiple(
            &[Downloadable::new(format!("{}/range", server.url()), &path)],
            options(CancellationToken::new()),
        )
        .await;
        response.assert_async().await;
        assert_eq!(result.is_ok(), body == "st", "{body}: {result:?}");
        if body == "st" {
            assert_eq!(std::fs::read(path).unwrap(), b"test");
        } else {
            assert!(!path.exists());
        }
    }
}
#[tokio::test]
async fn task06_repairs_corrupt_destination_and_stops_admission() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file");
    std::fs::write(&path, b"fail").unwrap();
    let (url, arrived, release, fixture) = held_response(OK).await;
    let file = verified(url, &path);
    let owner = tokio::spawn(download_owned(
        vec![file],
        options(CancellationToken::new()),
    ));
    arrived.await.unwrap();
    release.send(()).unwrap();
    fixture.await.unwrap();
    owner.await.unwrap().unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"test");
    let (url, arrived, release, fixture) =
        held_response("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
    let queued_path = dir.path().join("must-not-start/file");
    let files = vec![
        Downloadable::new(url.clone(), dir.path().join("bad")),
        Downloadable::new(url, &queued_path),
    ];
    let parent_token = CancellationToken::new();
    let mut opts = options(parent_token.clone());
    opts.concurrency = 1;
    let owner = tokio::spawn(download_owned(files, opts));
    arrived.await.unwrap();
    release.send(()).unwrap();
    fixture.await.unwrap();
    assert!(owner.await.unwrap().is_err());
    assert!(!queued_path.parent().unwrap().exists());
    assert!(!parent_token.is_cancelled());
}
#[tokio::test]
async fn task06_failure_is_observed_out_of_input_order() {
    let dir = tempfile::tempdir().unwrap();
    let (slow_url, arrived, release, fixture) = held_response(OK).await;
    let (bad_url, bad_arrived, bad_release, bad_fixture) =
        held_response("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
    let files = vec![
        verified(slow_url, &dir.path().join("slow")),
        Downloadable::new(bad_url, dir.path().join("bad")),
    ];
    let owner = tokio::spawn(download_owned(files, options(CancellationToken::new())));
    arrived.await.unwrap();
    bad_arrived.await.unwrap();
    bad_release.send(()).unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), owner)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    release.send(()).unwrap();
    fixture.await.unwrap();
    bad_fixture.await.unwrap();
    assert!(!dir.path().join("slow").exists());
}

#[tokio::test]
async fn task06_round1_platform_policy_keeps_independent_transfers_parallel() {
    let dir = tempfile::tempdir().unwrap();
    let (a_url, a_arrived, a_release, a_fixture) = held_response(OK).await;
    let (b_url, mut b_arrived, b_release, b_fixture) = held_response(OK).await;
    let token = CancellationToken::new();
    let mut opts = options(token.clone());
    opts.destination_policy = DestinationPolicy::CaseInsensitive;
    let a = verified(a_url, &dir.path().join("a"));
    let b = verified(b_url, &dir.path().join("b"));
    let owner = tokio::spawn(download_owned(vec![a.clone(), b.clone()], opts));
    a_arrived.await.unwrap();
    let parallel = tokio::time::timeout(Duration::from_millis(300), &mut b_arrived)
        .await
        .is_ok();
    if !parallel {
        token.cancel();
    }
    let _ = a_release.send(());
    let _ = b_release.send(());
    a_fixture.await.unwrap();
    if parallel {
        b_fixture.await.unwrap();
    } else {
        b_fixture.abort();
        let _ = b_fixture.await;
    }
    let result = owner.await.unwrap();
    assert!(
        parallel,
        "supported-platform identity policy serialized unrelated HTTP transfers"
    );
    result.unwrap();
    assert_eq!(std::fs::read(a.path).unwrap(), b"test");
    assert_eq!(std::fs::read(b.path).unwrap(), b"test");
}

#[tokio::test]
async fn task06_round1_uncoalesced_aliases_publish_private_inodes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("FILE");
    let (a_url, a_arrived, a_release, a_fixture) = held_response(OK).await;
    let (b_url, b_arrived, b_release, b_fixture) = held_response(OK).await;
    let mut a_options = options(CancellationToken::new());
    a_options.destination_policy = DestinationPolicy::Canonical;
    let mut b_options = options(CancellationToken::new());
    b_options.destination_policy = DestinationPolicy::CaseInsensitive;
    assert!(!Arc::ptr_eq(
        &destination_gate(&path, a_options.destination_policy)
            .await
            .unwrap(),
        &destination_gate(&path, b_options.destination_policy)
            .await
            .unwrap()
    ));
    let a = tokio::spawn(download_owned(vec![verified(a_url, &path)], a_options));
    let b = tokio::spawn(download_owned(vec![verified(b_url, &path)], b_options));
    tokio::time::timeout(Duration::from_secs(2), async {
        a_arrived.await.unwrap();
        b_arrived.await.unwrap();
    })
    .await
    .unwrap();
    a_release.send(()).unwrap();
    b_release.send(()).unwrap();
    a_fixture.await.unwrap();
    b_fixture.await.unwrap();
    a.await.unwrap().unwrap();
    b.await.unwrap().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"test");
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 4);
}
#[tokio::test]
async fn task06_round1_resume_copies_legacy_and_private_inodes() {
    for legacy in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file");
        let file = verified("http://127.0.0.1:1/never-requested".into(), &path);
        let opts = options(CancellationToken::new());
        let original = if legacy {
            legacy_partial(&path)
        } else {
            let private = owned_partial(&file, &opts).await.unwrap();
            let path = private.path.clone();
            drop(private);
            path
        };
        std::fs::write(&original, b"test").unwrap();
        let earlier_handle = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&original)
            .unwrap();
        download_owned(vec![file], opts).await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"test");
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_ne!(
                earlier_handle.metadata().unwrap().ino(),
                std::fs::metadata(&path).unwrap().ino()
            );
        }
        drop(earlier_handle);
        // Published from the copy, the original is spent and goes, as 2.0.40
        // consumed its partial. A writer it might still have would only ever
        // reach the unlinked inode, never the published file.
        assert!(!original.exists(), "spent leftover kept");
    }
}
#[tokio::test]
async fn task06_round1_platform_policy_revalidates_same_destination() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file");
    let (url, arrived, release, fixture) = held_response(OK).await;
    let mut opts = options(CancellationToken::new());
    opts.destination_policy = DestinationPolicy::CaseInsensitive;
    let a = tokio::spawn(download_owned(vec![verified(url, &path)], opts.clone()));
    arrived.await.unwrap();
    opts.only_validate = true;
    let mut b = tokio::spawn(download_owned(
        vec![verified("http://127.0.0.1:1/never-requested".into(), &path)],
        opts,
    ));
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut b)
            .await
            .is_err()
    );
    release.send(()).unwrap();
    fixture.await.unwrap();
    a.await.unwrap().unwrap();
    b.await.unwrap().unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"test");
}

#[tokio::test]
async fn task06_round1_resume_identity_preserves_exact_filename() {
    let mut pairs = vec![(OsString::from("A"), OsString::from("a"))];
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        pairs.push((
            OsString::from_vec(vec![0xff]),
            OsString::from_vec(vec![0xfe]),
        ));
    }
    for (first, second) in pairs {
        let dir = tempfile::tempdir().unwrap();
        let mut opts = options(CancellationToken::new());
        opts.destination_policy = DestinationPolicy::CaseInsensitive;
        let first = Downloadable::new("unused", dir.path().join(first));
        let second = Downloadable::new("unused", dir.path().join(second));
        let partial = owned_partial(&first, &opts).await.unwrap();
        let retained = partial.path.clone();
        std::fs::write(&retained, b"other artifact").unwrap();
        drop(partial);
        let private = owned_partial(&second, &opts).await.unwrap();
        assert_eq!(
            std::fs::metadata(&private.path).unwrap().len(),
            0,
            "contention normalization must not import another filename's resume bytes"
        );
        assert_eq!(std::fs::read(retained).unwrap(), b"other artifact");
    }
}

fn round2_private(root: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".__gdl_owned_part_")
        })
        .map(|entry| entry.path())
        .collect()
}
fn round2_pause(stage: PartialStage) -> Arc<PartialPause> {
    Arc::new(PartialPause {
        stage,
        entered: tokio::sync::Notify::new(),
        release: tokio::sync::Semaphore::new(0),
    })
}
#[tokio::test]
async fn task06_round2_failed_publication_does_not_accumulate_copies() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("artifact.jar");
    let original = legacy_partial(&path);
    std::fs::write(&original, b"test").unwrap();
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("occupant"), b"keep").unwrap();
    let file = verified("http://127.0.0.1:1/never-requested".into(), &path);
    for attempt in 1..=3 {
        let result = download_multiple(&[file.clone()], options(CancellationToken::new())).await;
        assert!(
            matches!(result, Err(DownloadError::CannotMoveFile(..))),
            "{result:?}"
        );
        assert_eq!(std::fs::read(&original).unwrap(), b"test");
        assert_eq!(std::fs::read(path.join("occupant")).unwrap(), b"keep");
        let retained = round2_private(dir.path());
        assert!(
            retained.is_empty(),
            "attempt {attempt} leaked new owned copies: {retained:?}"
        );
    }
    std::fs::remove_file(path.join("occupant")).unwrap();
    std::fs::remove_dir(&path).unwrap();
    download_multiple(&[file], options(CancellationToken::new()))
        .await
        .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"test");
    // Failed publications kept it for the next try; the successful one spends it.
    assert!(!original.exists(), "spent leftover kept");
    assert!(round2_private(dir.path()).is_empty());
}
async fn round2_cancel(stage: PartialStage, seed: bool, raw_drop: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("artifact.jar");
    let original = legacy_partial(&path);
    if seed {
        std::fs::write(&original, b"test").unwrap();
    }
    for _attempt in 0..3 {
        let pause = round2_pause(stage);
        let cancel = CancellationToken::new();
        let mut opts = options(cancel.clone());
        opts.partial_pause = Some(pause.clone());
        let (progress, mut settled) = tokio::sync::watch::channel(Progress::new());
        opts.progress_sender = Some(progress);
        let file = verified("http://127.0.0.1:1/never-requested".into(), &path);
        let owner = tokio::spawn(async move { download_multiple(&[file], opts).await });
        tokio::time::timeout(Duration::from_secs(2), pause.entered.notified())
            .await
            .unwrap();
        if raw_drop {
            owner.abort();
            assert!(owner.await.unwrap_err().is_cancelled());
            assert_eq!(
                round2_private(dir.path()).len(),
                1,
                "raw join alone cannot clean an unsettled writer"
            );
            pause.release.add_permits(1);
            tokio::time::timeout(Duration::from_secs(2), async {
                while settled.changed().await.is_ok() {}
            })
            .await
            .unwrap();
        } else {
            cancel.cancel();
            pause.release.add_permits(1);
            let result = tokio::time::timeout(Duration::from_secs(2), owner)
                .await
                .unwrap()
                .unwrap();
            assert!(
                matches!(result, Err(DownloadError::Cancelled)),
                "{stage:?}: {result:?}"
            );
        }
        assert!(!path.exists());
        if seed {
            assert_eq!(std::fs::read(&original).unwrap(), b"test");
        }
        assert!(
            round2_private(dir.path()).is_empty(),
            "{stage:?}: cancelled owned copy remained"
        );
    }
}
#[tokio::test]
async fn task06_round2_cancel_after_create_cleans_empty_partial() {
    round2_cancel(PartialStage::Created, false, false).await;
}
#[tokio::test]
async fn task06_round2_cancel_snapshot_cleans_partial() {
    round2_cancel(PartialStage::SnapshotWritten, true, false).await;
}
#[tokio::test]
async fn task06_round2_cancel_preparation_cleans_partial() {
    round2_cancel(PartialStage::Preparing, true, false).await;
}
#[tokio::test]
async fn task06_round2_cancel_publication_cleans_partial() {
    round2_cancel(PartialStage::Publishing, true, false).await;
}
#[tokio::test]
async fn task06_round2_raw_drop_supervisor_cleans_after_drain() {
    round2_cancel(PartialStage::SnapshotWritten, true, true).await;
}

#[cfg(unix)]
struct Round2Permissions(PathBuf, std::fs::Permissions);
#[cfg(unix)]
impl Drop for Round2Permissions {
    fn drop(&mut self) {
        std::fs::set_permissions(&self.0, self.1.clone()).unwrap();
    }
}
#[cfg(unix)]
#[tokio::test]
async fn task06_round2_snapshot_error_cleans_new_partial() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("artifact.jar");
    let original = legacy_partial(&path);
    std::fs::write(&original, b"test").unwrap();
    let restore = Round2Permissions(
        original.clone(),
        std::fs::metadata(&original).unwrap().permissions(),
    );
    let pause = round2_pause(PartialStage::Created);
    let mut opts = options(CancellationToken::new());
    opts.partial_pause = Some(pause.clone());
    let file = verified("http://127.0.0.1:1/never-requested".into(), &path);
    let owner = tokio::spawn(async move { download_multiple(&[file], opts).await });
    pause.entered.notified().await;
    std::fs::set_permissions(&original, std::fs::Permissions::from_mode(0)).unwrap();
    assert!(
        std::fs::File::open(&original).is_err(),
        "fixture must deny the actual read"
    );
    pause.release.add_permits(1);
    let result = owner.await.unwrap();
    drop(restore);
    assert!(
        matches!(result, Err(DownloadError::IOError(_))),
        "{result:?}"
    );
    assert_eq!(std::fs::read(original).unwrap(), b"test");
    assert!(
        round2_private(dir.path()).is_empty(),
        "snapshot error leaked newly created copy"
    );
}
#[cfg(unix)]
#[tokio::test]
async fn task06_round2_cleanup_failure_reports_original_and_remaining_path() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("artifact.jar");
    let original = legacy_partial(&path);
    std::fs::write(&original, b"test").unwrap();
    std::fs::create_dir(&path).unwrap();
    let restore = Round2Permissions(
        dir.path().to_owned(),
        std::fs::metadata(dir.path()).unwrap().permissions(),
    );
    let pause = round2_pause(PartialStage::Publishing);
    let mut opts = options(CancellationToken::new());
    opts.partial_pause = Some(pause.clone());
    let file = verified("http://127.0.0.1:1/never-requested".into(), &path);
    let owner = tokio::spawn(async move { download_multiple(&[file], opts).await });
    pause.entered.notified().await;
    let partial = round2_private(dir.path()).pop().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
    assert!(
        std::fs::write(dir.path().join("denied"), b"x").is_err(),
        "fixture must deny parent mutation"
    );
    pause.release.add_permits(1);
    let error = owner.await.unwrap().unwrap_err();
    drop(restore);
    let diagnostic = error.to_string();
    assert!(
        diagnostic.contains("owned partial") && diagnostic.contains(partial.to_str().unwrap()),
        "remaining artifact must be reported: {diagnostic}"
    );
    let DownloadError::OwnedPartialCleanup {
        original: original_error,
        path: remaining,
        cleanup,
    } = &error
    else {
        panic!("{error:?}");
    };
    assert!(matches!(
        original_error.as_ref(),
        DownloadError::CannotMoveFile(..)
    ));
    assert_eq!(
        std::error::Error::source(&error).unwrap().to_string(),
        original_error.to_string()
    );
    assert_eq!(remaining, &partial);
    assert_eq!(cleanup.kind(), std::io::ErrorKind::PermissionDenied);
    assert_eq!(std::fs::read(&partial).unwrap(), b"test");
    assert_eq!(std::fs::read(original).unwrap(), b"test");
    assert!(path.is_dir());
}

#[tokio::test]
async fn task06_round2_fresh_and_resumed_success_consume_new_partial() {
    for resumed in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artifact.jar");
        let original = legacy_partial(&path);
        if resumed {
            std::fs::write(&original, b"te").unwrap();
        }
        let response = if resumed {
            "HTTP/1.1 206 Partial Content\r\nContent-Length: 2\r\nContent-Range: bytes 2-3/4\r\nConnection: close\r\n\r\nst"
        } else {
            OK
        };
        let (url, arrived, release, fixture) = held_response(response).await;
        let file = verified(url, &path);
        let owner = tokio::spawn(async move {
            download_multiple(&[file], options(CancellationToken::new())).await
        });
        arrived.await.unwrap();
        release.send(()).unwrap();
        fixture.await.unwrap();
        owner.await.unwrap().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"test");
        assert!(round2_private(dir.path()).is_empty());
        assert!(!original.exists(), "spent leftover kept");
    }
}

/// Some Windows volumes (RAM disks, drives the Mount Manager does not map)
/// cannot report a final path. The gate key is only a contention
/// optimization, so a download there goes ahead under the lexical path, and
/// two downloads of that path still share one gate.
#[cfg(unix)]
#[tokio::test]
async fn destination_gate_keys_a_path_it_cannot_resolve_lexically() {
    let dir = tempfile::tempdir().unwrap();
    // A link that leads to itself fails to resolve, and not as NotFound.
    std::os::unix::fs::symlink("loop", dir.path().join("loop")).unwrap();
    let path = dir.path().join("loop/natives/file.jar");
    assert_ne!(
        std::fs::canonicalize(path.parent().unwrap())
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotFound
    );
    for policy in [
        DestinationPolicy::Canonical,
        DestinationPolicy::CaseInsensitive,
    ] {
        let first = destination_gate(&path, policy).await.unwrap();
        let second = destination_gate(&path, policy).await.unwrap();
        assert!(Arc::ptr_eq(&first, &second));
    }
}
