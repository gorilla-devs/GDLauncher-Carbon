use super::*;
use std::{cell::Cell, io::Write};
fn pack(path: &Path, optional: bool, hashes: bool, override_bytes: &[u8]) {
    let entries = if optional {
        serde_json::json!([{"type":"optional", "description":"Shader support", "platforms":[{"sha512":"AB12", "sha1":"CD34", "murmur2":123}], "overridePaths":["config/iris"]}])
    } else if hashes {
        serde_json::json!([{"type":"platform","hashes":{"sha512":"AB12","sha1":"CD34","murmur2":123}}])
    } else {
        serde_json::json!([])
    };
    let mut z = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let opts = zip::write::FileOptions::<()>::default();
    z.start_file("gdlpack.json", opts).unwrap();
    z.write_all(serde_json::to_string(&serde_json::json!({"formatVersion":1,"name":"Fixture","createdAt":"2024-01-15T10:30:00Z","dependencies":{"minecraft":"1.20.1"},"entries":entries,"clientOverrides":"client-overrides"})).unwrap().as_bytes()).unwrap();
    for (name, value) in [
        ("overrides/config/iris/example.json", override_bytes),
        ("client-overrides/config/client.json", b"client"),
        ("overrides/../escape.txt", b"escape"),
        ("server-overrides/config/server.json", b"server"),
    ] {
        z.start_file(name, opts).unwrap();
        z.write_all(value).unwrap();
    }
    z.add_symlink("overrides/config/link", "example.json", opts)
        .unwrap();
    z.finish().unwrap();
}
fn empty_resolution() -> Resolution {
    Resolution {
        modrinth_results: HashMap::new(),
        curseforge_results: HashMap::new(),
        platform_error: None,
    }
}
#[tokio::test]
async fn task40_prepare_fresh_and_resume_refuse_before_lookup_or_extraction() {
    for skip in [false, true] {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("pack.gdlpack");
        pack(&p, true, false, b"{}");
        let instance = InstancePath::new(t.path().join("instance-root"));
        let calls = Cell::new(0);
        let result = prepare_gdlpack(
            &p,
            t.path().join("private"),
            &instance,
            skip,
            None,
            watch::channel(ProgressState::Idle).0,
            |_| {
                calls.set(calls.get() + 1);
                std::future::ready(empty_resolution())
            },
        )
        .await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("optional entries are not supported")
        );
        assert_eq!(calls.get(), 0);
        assert!(!instance.get_data_path().exists());
        assert!(p.is_file(), "existing setup archive remains recoverable");
    }
}
#[tokio::test]
async fn task40_required_hashes_reach_resolution_and_safe_overrides_remain_supported() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("pack.gdlpack");
    pack(&p, false, true, b"{}");
    let original = std::fs::read(&p).unwrap();
    let instance = InstancePath::new(t.path().join("instance-root"));
    let calls = Cell::new(0);
    let result = prepare_gdlpack(
        &p,
        t.path().join("private"),
        &instance,
        false,
        None,
        watch::channel(ProgressState::Idle).0,
        |hashes| {
            calls.set(calls.get() + 1);
            assert_eq!(hashes.len(), 1);
            assert_eq!(hashes[0].sha512, "ab12");
            assert_eq!(hashes[0].sha1, "cd34");
            assert_eq!(hashes[0].murmur2, 123);
            // Source replacement after validation must not change extracted content.
            pack(&p, true, false, b"replaced external override");
            let mut resolution = empty_resolution();
            resolution.modrinth_results.insert(
                "ab12".into(),
                ResolvedFile {
                    download_url: "https://fixture.invalid/required.jar".into(),
                    relative_path: "mods/required.jar".into(),
                    sha1: "verified-platform-sha1".into(),
                    size: 12,
                },
            );
            std::future::ready(resolution)
        },
    )
    .await
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(result.downloadables.len(), 1);
    assert_eq!(
        std::fs::read(instance.get_data_path().join("config/iris/example.json")).unwrap(),
        b"{}"
    );
    assert_eq!(
        std::fs::read(instance.get_data_path().join("config/client.json")).unwrap(),
        b"client"
    );
    assert!(!instance.get_data_path().join("config/link").exists());
    assert!(!t.path().join("instance-root/escape.txt").exists());
    assert!(!instance.get_data_path().join("config/server.json").exists());
    assert_ne!(std::fs::read(p).unwrap(), original);
}
#[tokio::test]
async fn task40_resume_skips_all_regular_overrides() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("pack.gdlpack");
    pack(&p, false, false, b"{}");
    let instance = InstancePath::new(t.path().join("instance-root"));
    std::fs::create_dir_all(instance.get_data_path().join("config/iris")).unwrap();
    let target = instance.get_data_path().join("config/iris/example.json");
    std::fs::write(&target, b"user edit").unwrap();
    prepare_gdlpack(
        &p,
        t.path().join("private"),
        &instance,
        true,
        None,
        watch::channel(ProgressState::Idle).0,
        |_| async { panic!("empty required entries must not call platforms") },
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(target).unwrap(), b"user edit");
    assert!(!instance.get_data_path().join("config/client.json").exists());
}
