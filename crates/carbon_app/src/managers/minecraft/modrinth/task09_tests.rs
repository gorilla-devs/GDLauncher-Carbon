use super::*;
use sha2::{Digest, Sha512};
use std::io::Write;

fn archive(path: &Path, files: serde_json::Value, reverse: bool) {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let opts = zip::write::FileOptions::<()>::default();
    zip.start_file("modrinth.index.json", opts).unwrap();
    serde_json::to_writer(&mut zip, &serde_json::json!({"formatVersion":1,"game":"minecraft","versionId":"v1","name":"Test","files":files,"dependencies":{}})).unwrap();
    let mut entries = vec![
        ("overrides/config/common", "common"),
        ("overrides/config/shared", "common shared"),
        ("client-overrides/config/shared", "client shared"),
        ("client-overrides/config/client", "client"),
        ("server-overrides/config/server", "server"),
        ("overrides-extra/config/extra", "extra"),
    ];
    if reverse {
        entries.reverse();
    }
    for (name, bytes) in entries {
        zip.start_file(name, opts).unwrap();
        zip.write_all(bytes.as_bytes()).unwrap();
    }
    zip.add_symlink("client-overrides/config/link", "target", opts)
        .unwrap();
    zip.finish().unwrap();
}
fn entry(sha512: &str, sha1: &str) -> serde_json::Value {
    serde_json::json!({"path":"ordinary-text.txt","hashes":{"sha512":sha512,"sha1":sha1},"downloads":["https://first.invalid/text", "https://second.invalid/text"],"fileSize":4})
}
async fn prepare(path: &Path, target: &InstancePath, skip: bool) -> anyhow::Result<ModpackInfo> {
    let (tx, _rx) = tokio::sync::watch::channel(ProgressState::Idle);
    prepare_mrpack(
        path,
        target,
        skip,
        None,
        tx,
        MAX_EXTRACTED_OVERRIDE_BYTES,
        None,
    )
    .await
}
#[tokio::test]
async fn task09_common_client_precedence_and_effective_packinfo() {
    use crate::managers::instance::modpack::packinfo::{make_packinfo, parse_packinfo, scan_dir};
    for reverse in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pack.mrpack");
        let target = InstancePath::new(dir.path().join("staging"));
        archive(&path, serde_json::json!([]), reverse);
        prepare(&path, &target, false).await.unwrap();
        let data = target.get_data_path();
        assert_eq!(
            std::fs::read(data.join("config/common")).unwrap(),
            b"common"
        );
        assert_eq!(
            std::fs::read(data.join("config/shared")).unwrap(),
            b"client shared",
            "client must win independent of archive order"
        );
        assert_eq!(
            std::fs::read(data.join("config/client")).unwrap(),
            b"client"
        );
        for name in ["server", "extra", "link"] {
            assert!(!data.join("config").join(name).exists());
        }
        let info =
            parse_packinfo(&make_packinfo(scan_dir(&data, None).await.unwrap()).unwrap()).unwrap();
        assert_eq!(info.files.len(), 3);
        assert_eq!(
            info.files["/config/shared"].sha512.as_slice(),
            Sha512::digest(b"client shared").as_slice()
        );
        std::fs::write(data.join("config/shared"), b"retained").unwrap();
        prepare(&path, &target, true).await.unwrap();
        assert_eq!(
            std::fs::read(data.join("config/shared")).unwrap(),
            b"retained"
        );
    }
}
#[tokio::test]
async fn task09_required_digest_admission_precedes_override_writes() {
    for (sha512, sha1) in [
        ("", ""),
        ("abc", ""),
        ("", "xyz"),
        (&"z".repeat(128), &"a".repeat(40)),
        (&"a".repeat(128), "malformed"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pack.mrpack");
        let target = InstancePath::new(dir.path().join("staging"));
        archive(&path, serde_json::json!([entry(sha512, sha1)]), false);
        let result = prepare(&path, &target, false).await;
        assert!(
            result.is_err(),
            "malformed required digest was admitted: {result:?}"
        );
        assert!(format!("{:#}", result.unwrap_err()).contains("ordinary-text.txt"));
        assert!(!target.get_data_path().join("config/common").exists());
    }
}
#[tokio::test]
async fn task09_digest_selection_and_ignored_optional_controls() {
    for (sha512, sha1, want_sha512) in [
        ("A".repeat(128), "b".repeat(40), true),
        (String::new(), "A".repeat(40), false),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pack.mrpack");
        let target = InstancePath::new(dir.path().join("staging"));
        let mut optional = entry("", "");
        optional["env"] = serde_json::json!({"client":"optional","server":"required"});
        let mut server = entry("", "");
        server["env"] = serde_json::json!({"client":"unsupported","server":"required"});
        archive(
            &path,
            serde_json::json!([entry(&sha512, &sha1), optional, server]),
            false,
        );
        let result = prepare(&path, &target, true).await.unwrap();
        assert_eq!(result.downloadables.len(), 1);
        let checksum = result.downloadables[0].0.checksum.as_ref().unwrap();
        match checksum {
            carbon_net::Checksum::Sha512(s) => {
                assert!(want_sha512);
                assert_eq!(s, &sha512.to_lowercase());
            }
            carbon_net::Checksum::Sha1(s) => {
                assert!(!want_sha512);
                assert_eq!(s, &sha1.to_lowercase());
            }
            _ => panic!("unexpected digest"),
        }
    }
}
#[tokio::test]
async fn task09_adapter_preserves_ordered_mirrors_and_rejects_empty_urls() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pack.mrpack");
    let target = InstancePath::new(dir.path().join("staging"));
    archive(
        &path,
        serde_json::json!([entry(&"a".repeat(128), "")]),
        false,
    );
    let info = prepare(&path, &target, true).await.unwrap();
    assert_eq!(info.downloadables[0].0.url, "https://first.invalid/text");
    assert_eq!(
        info.downloadables[0].0.mirrors,
        vec!["https://second.invalid/text"]
    );
    let mut no_urls = entry(&"a".repeat(128), "");
    no_urls["downloads"] = serde_json::json!([]);
    std::fs::remove_file(&path).unwrap();
    archive(&path, serde_json::json!([no_urls]), false);
    assert!(prepare(&path, &target, false).await.is_err());
    assert!(!target.get_data_path().join("config/common").exists());
}

#[tokio::test]
async fn task09_extraction_budget_is_cumulative_across_layers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pack.mrpack");
    let target = InstancePath::new(dir.path().join("staging"));
    archive(&path, serde_json::json!([]), true);
    let (tx, _rx) = tokio::sync::watch::channel(ProgressState::Idle);
    // Common entries consume 19 bytes. Client shared is 13: each layer fits 20
    // independently, but together they must refuse before finishing extraction.
    let error = prepare_mrpack(&path, &target, false, None, tx, 20, None)
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("decompressed size limit"));
    assert_eq!(
        std::fs::read(target.get_data_path().join("config/common")).unwrap(),
        b"common"
    );
}

// The observer reads the actual watch channel after every production send, before
// extraction can coalesce another update. It belongs only to this invocation.
async fn round1_progress_case(order: &str, skip: bool, budget: u64) -> Vec<(u64, u64)> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("progress.mrpack");
    let target = InstancePath::new(dir.path().join("staging"));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let opts = zip::write::FileOptions::<()>::default();
    zip.start_file("modrinth.index.json", opts).unwrap();
    zip.write_all(br#"{"formatVersion":1,"game":"minecraft","versionId":"v1","name":"Test","files":[],"dependencies":{}}"#).unwrap();
    let entries = match order {
        "ordinary" => vec![
            ("overrides/shared", "common"),
            ("client-overrides/shared", "client"),
        ],
        "reversed" => vec![
            ("client-overrides/shared", "client"),
            ("overrides/shared", "common"),
        ],
        "interleaved" => vec![
            ("client-overrides/shared", "client"),
            ("overrides/shared", "common"),
            ("server-overrides/server", "server"),
            ("client-overrides/client-only", "only"),
            ("overrides/common-only", "common"),
        ],
        "empty" => vec![],
        "ignored" => vec![
            ("server-overrides/server", "server"),
            ("overrides-extra/extra", "extra"),
            ("../overrides/escape", "escape"),
        ],
        _ => panic!("unknown order"),
    };
    for (name, bytes) in entries {
        zip.start_file(name, opts).unwrap();
        zip.write_all(bytes.as_bytes()).unwrap();
    }
    if order == "ignored" {
        zip.add_directory("overrides/directory/", opts).unwrap();
        zip.add_symlink("client-overrides/link", "target", opts)
            .unwrap();
    }
    zip.finish().unwrap();
    let (tx, rx) = tokio::sync::watch::channel(ProgressState::Idle);
    let observations = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = observations.clone();
    let observe = Box::new(move || {
        if let ProgressState::ExtractingPackOverrides(current, total) = *rx.borrow() {
            recorded.lock().unwrap().push((current, total));
        }
    });
    let result = prepare_mrpack(&path, &target, skip, None, tx, budget, Some(observe)).await;
    let samples = observations.lock().unwrap().clone();
    if budget == 6 && !skip {
        assert!(format!("{:#}", result.unwrap_err()).contains("decompressed size limit"));
        assert!(!samples.is_empty());
        assert!(
            samples.iter().all(|(current, total)| current < total),
            "failed work cannot complete: {samples:?}"
        );
    } else {
        result.unwrap();
        if !skip && !matches!(order, "empty" | "ignored") {
            assert_eq!(
                std::fs::read(target.get_data_path().join("shared")).unwrap(),
                b"client"
            );
            if order == "interleaved" {
                assert_eq!(
                    std::fs::read(target.get_data_path().join("common-only")).unwrap(),
                    b"common"
                );
                assert_eq!(
                    std::fs::read(target.get_data_path().join("client-only")).unwrap(),
                    b"only"
                );
            }
        }
        for excluded in ["server", "extra", "escape", "directory", "link"] {
            assert!(!target.get_data_path().join(excluded).exists());
        }
    }
    samples
}
fn round1_assert_complete(samples: &[(u64, u64)]) {
    assert!(
        !samples.is_empty(),
        "successful extraction must report completion even with no eligible entries"
    );
    assert!(
        samples
            .windows(2)
            .all(|pair| pair[0].0 <= pair[1].0 && pair[0].1 == pair[1].1),
        "extraction work must never decrease or change denominator: {samples:?}"
    );
    assert!(samples.iter().all(|(current, total)| current <= total));
    let &(current, total) = samples.last().unwrap();
    assert_eq!(
        current, total,
        "successful extraction must finish its full work: {samples:?}"
    );
}
#[tokio::test]
async fn task09_round1_reversed_progress_is_cumulative() {
    round1_assert_complete(
        &round1_progress_case("reversed", false, MAX_EXTRACTED_OVERRIDE_BYTES).await,
    );
}
#[tokio::test]
async fn task09_round1_interleaved_progress_is_cumulative() {
    round1_assert_complete(
        &round1_progress_case("interleaved", false, MAX_EXTRACTED_OVERRIDE_BYTES).await,
    );
}
#[tokio::test]
async fn task09_round1_ordinary_progress_completes() {
    round1_assert_complete(
        &round1_progress_case("ordinary", false, MAX_EXTRACTED_OVERRIDE_BYTES).await,
    );
}
#[tokio::test]
async fn task09_round1_empty_and_ignored_archives_complete() {
    for order in ["empty", "ignored"] {
        round1_assert_complete(
            &round1_progress_case(order, false, MAX_EXTRACTED_OVERRIDE_BYTES).await,
        );
    }
}
#[tokio::test]
async fn task09_round1_skip_and_budget_progress_controls() {
    assert!(
        round1_progress_case("reversed", true, MAX_EXTRACTED_OVERRIDE_BYTES)
            .await
            .is_empty()
    );
    round1_progress_case("reversed", false, 6).await;
}
