//! Leftovers of interrupted downloads (2.0.40 `.__gdl_part~` files and owned
//! partials of a killed run): when they resume a download, when they go, and why
//! a stale one can no longer make a file fail to download for good.
//!
//! Every request goes to a mockito server owned by the test.

use super::*;
use mockito::Matcher;
use std::io::Write;
use std::time::Duration;

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
fn pause(stage: PartialStage) -> Arc<PartialPause> {
    Arc::new(PartialPause {
        stage,
        entered: tokio::sync::Notify::new(),
        release: tokio::sync::Semaphore::new(0),
    })
}
fn owned_prefix(path: &Path) -> String {
    format!(
        ".__gdl_owned_part_{}-",
        hex::encode(Sha256::digest(path.file_name().unwrap().as_encoded_bytes()))
    )
}

#[tokio::test]
async fn leftover_longer_than_the_known_size_is_not_resumed_and_goes_after_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("server.jar");
    let legacy = legacy_partial(&path);
    std::fs::write(&legacy, b"stale!!").unwrap();
    let mut server = mockito::Server::new_async().await;
    // What a CDN answers when asked to resume past the end of the file.
    let ranged = server
        .mock("GET", "/file")
        .match_header("range", Matcher::Any)
        .with_status(416)
        .with_header("content-range", "bytes */4")
        .expect(0)
        .create_async()
        .await;
    let full = server
        .mock("GET", "/file")
        .match_header("range", Matcher::Missing)
        .with_body("test")
        .expect(1)
        .create_async()
        .await;
    let file = verified(format!("{}/file", server.url()), &path);
    download_multiple(&[file], options()).await.unwrap();
    ranged.assert_async().await;
    full.assert_async().await;
    assert_eq!(std::fs::read(&path).unwrap(), b"test");
    assert!(!legacy.exists(), "stale leftover kept");
}

#[tokio::test]
async fn refused_range_of_an_unsized_file_retries_from_zero_and_drops_the_leftover() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("server.jar");
    let legacy = legacy_partial(&path);
    std::fs::write(&legacy, b"stale!!").unwrap();
    let mut server = mockito::Server::new_async().await;
    let ranged = server
        .mock("GET", "/file")
        .match_header("range", "bytes=7-")
        .with_status(416)
        .with_header("content-range", "bytes */4")
        .expect(1)
        .create_async()
        .await;
    let full = server
        .mock("GET", "/file")
        .match_header("range", Matcher::Missing)
        .with_body("test")
        .expect(1)
        .create_async()
        .await;
    let file = Downloadable::new(format!("{}/file", server.url()), &path);
    download_multiple(&[file], options()).await.unwrap();
    ranged.assert_async().await;
    full.assert_async().await;
    assert_eq!(std::fs::read(&path).unwrap(), b"test");
    assert!(!legacy.exists(), "stale leftover kept");
}

#[tokio::test]
async fn client_error_to_a_ranged_request_retries_from_zero() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mod.jar");
    let legacy = legacy_partial(&path);
    std::fs::write(&legacy, b"te").unwrap();
    let mut server = mockito::Server::new_async().await;
    let ranged = server
        .mock("GET", "/file")
        .match_header("range", "bytes=2-")
        .with_status(403)
        .expect(1)
        .create_async()
        .await;
    let full = server
        .mock("GET", "/file")
        .match_header("range", Matcher::Missing)
        .with_body("test")
        .expect(1)
        .create_async()
        .await;
    let file = verified(format!("{}/file", server.url()), &path);
    download_multiple(&[file], options()).await.unwrap();
    ranged.assert_async().await;
    full.assert_async().await;
    assert_eq!(std::fs::read(&path).unwrap(), b"test");
    assert!(!legacy.exists(), "spent leftover kept");
}

#[tokio::test]
async fn server_error_to_a_ranged_request_keeps_the_leftover_for_the_next_try() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mod.jar");
    let legacy = legacy_partial(&path);
    std::fs::write(&legacy, b"te").unwrap();
    let mut server = mockito::Server::new_async().await;
    let ranged = server
        .mock("GET", "/file")
        .match_header("range", "bytes=2-")
        .with_status(503)
        .expect(1)
        .create_async()
        .await;
    let full = server
        .mock("GET", "/file")
        .match_header("range", Matcher::Missing)
        .expect(0)
        .create_async()
        .await;
    let file = verified(format!("{}/file", server.url()), &path);
    let result = download_multiple(&[file], options()).await;
    assert!(
        matches!(result, Err(DownloadError::Non200StatusCode(_, 503))),
        "{result:?}"
    );
    ranged.assert_async().await;
    full.assert_async().await;
    assert!(!path.exists());
    assert_eq!(std::fs::read(&legacy).unwrap(), b"te");
}

#[tokio::test]
async fn success_removes_every_spent_leftover_but_no_live_or_unrelated_partial() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mod.jar");
    let file = verified("unused".into(), &path);
    let opts = options();
    let legacy = legacy_partial(&path);
    std::fs::write(&legacy, b"te").unwrap();
    // Left by an earlier worker of this process whose cleanup failed.
    let earlier = owned_partial(&file, &opts).await.unwrap();
    let own_orphan = earlier.path.clone();
    drop(earlier);
    std::fs::write(&own_orphan, b"t").unwrap();
    // Left by a run that was killed.
    let killed_orphan = dir
        .path()
        .join(format!("{}4000000001-0", owned_prefix(&path)));
    std::fs::write(&killed_orphan, b"tes").unwrap();
    // Held by a live worker of this process: never a leftover.
    let live = owned_partial(&file, &opts).await.unwrap();
    std::fs::write(&live.path, b"te").unwrap();
    // Another file's partial in the same folder.
    let other = owned_partial(
        &verified("unused".into(), &dir.path().join("other.jar")),
        &opts,
    )
    .await
    .unwrap();
    let other_orphan = other.path.clone();
    drop(other);
    std::fs::write(&other_orphan, b"other").unwrap();

    let mut server = mockito::Server::new_async().await;
    let resumed = server
        .mock("GET", "/file")
        .match_header("range", "bytes=2-")
        .with_status(206)
        .with_header("content-range", "bytes 2-3/4")
        .with_body("st")
        .expect(1)
        .create_async()
        .await;
    let file = verified(format!("{}/file", server.url()), &path);
    download_multiple(&[file], options()).await.unwrap();
    resumed.assert_async().await;
    assert_eq!(std::fs::read(&path).unwrap(), b"test");
    assert!(!legacy.exists(), "2.0.40 leftover kept");
    assert!(!own_orphan.exists(), "orphan of this process kept");
    assert!(!killed_orphan.exists(), "orphan of a killed run kept");
    assert_eq!(std::fs::read(&live.path).unwrap(), b"te");
    assert_eq!(std::fs::read(&other_orphan).unwrap(), b"other");
    drop(live);
}

#[tokio::test]
async fn leftover_that_changed_since_it_was_picked_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mod.jar");
    let legacy = legacy_partial(&path);
    std::fs::write(&legacy, b"te").unwrap();
    let mut server = mockito::Server::new_async().await;
    let resumed = server
        .mock("GET", "/file")
        .match_header("range", "bytes=2-")
        .with_status(206)
        .with_header("content-range", "bytes 2-3/4")
        .with_body("st")
        .expect(1)
        .create_async()
        .await;
    let publishing = pause(PartialStage::Publishing);
    let mut opts = options();
    opts.partial_pause = Some(publishing.clone());
    let file = verified(format!("{}/file", server.url()), &path);
    let job = tokio::spawn(async move { download_multiple(&[file], opts).await });
    tokio::time::timeout(Duration::from_secs(5), publishing.entered.notified())
        .await
        .unwrap();
    // Something this process cannot see is still writing to it.
    std::fs::OpenOptions::new()
        .append(true)
        .open(&legacy)
        .unwrap()
        .write_all(b"s")
        .unwrap();
    publishing.release.add_permits(1);
    job.await.unwrap().unwrap();
    resumed.assert_async().await;
    assert_eq!(std::fs::read(&path).unwrap(), b"test");
    assert_eq!(std::fs::read(&legacy).unwrap(), b"tes");
}

#[tokio::test]
async fn leftover_removed_after_it_was_picked_starts_the_download_from_zero() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mod.jar");
    let legacy = legacy_partial(&path);
    std::fs::write(&legacy, b"te").unwrap();
    let mut server = mockito::Server::new_async().await;
    let full = server
        .mock("GET", "/file")
        .match_header("range", Matcher::Missing)
        .with_body("test")
        .expect(1)
        .create_async()
        .await;
    let created = pause(PartialStage::Created);
    let mut opts = options();
    opts.partial_pause = Some(created.clone());
    let file = verified(format!("{}/file", server.url()), &path);
    let job = tokio::spawn(async move { download_multiple(&[file], opts).await });
    tokio::time::timeout(Duration::from_secs(5), created.entered.notified())
        .await
        .unwrap();
    // A worker publishing the same file under an alias the gate cannot
    // recognize spent it in the meantime.
    std::fs::remove_file(&legacy).unwrap();
    created.release.add_permits(1);
    job.await.unwrap().unwrap();
    full.assert_async().await;
    assert_eq!(std::fs::read(&path).unwrap(), b"test");
}

#[tokio::test]
async fn metadata_only_check_of_a_present_file_needs_no_destination_gate() {
    let dir = tempfile::tempdir().unwrap();
    let present = dir.path().join("present");
    std::fs::write(&present, b"test").unwrap();
    let missing = dir.path().join("missing");
    let opts = DownloadOptions::builder()
        .only_validate(true)
        .deep_check(false)
        .max_retries(0)
        .build();
    // Stand in for a writer of each destination that is still busy.
    let present_gate = destination_gate(&present, opts.destination_policy)
        .await
        .unwrap();
    let _present_owner = present_gate.lock().await;
    let missing_gate = destination_gate(&missing, opts.destination_policy)
        .await
        .unwrap();
    let missing_owner = missing_gate.lock().await;

    let file = verified("http://127.0.0.1:1/never-requested".into(), &present);
    let checked = tokio::time::timeout(
        Duration::from_secs(2),
        download_multiple(&[file], opts.clone()),
    )
    .await
    .expect("a metadata-only check of a present file waited for its gate");
    assert!(
        !checked.unwrap(),
        "present file reported as needing download"
    );

    // A file that is not there yet still waits for the writer that may be
    // about to publish it, and then sees what it published.
    let file = verified("http://127.0.0.1:1/never-requested".into(), &missing);
    let mut queued = tokio::spawn(download_owned(file, opts));
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut queued)
            .await
            .is_err(),
        "check of a missing file skipped its destination gate"
    );
    std::fs::write(&missing, b"test").unwrap();
    drop(missing_owner);
    assert!(!queued.await.unwrap().unwrap());
}
async fn download_owned(
    file: Downloadable,
    options: DownloadOptions,
) -> Result<bool, DownloadError> {
    download_multiple(&[file], options).await
}
