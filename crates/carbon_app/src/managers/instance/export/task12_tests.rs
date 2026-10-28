use super::*;
use crate::{
    domain::instance::AddonType,
    managers::{
        App,
        minecraft::gdlpack::{ProgressState, prepare_modpack_from_gdlpack},
    },
};
use carbon_platforms::gdlauncher::manifest::schema::v1::Manifest;
use carbon_repos::dbtypes::DbDateTime;
use carbon_rt_path::InstancePath;

async fn fixture(name: &str) -> (crate::TestEnv, InstanceId, PathBuf) {
    let app = crate::setup_managers_for_test().await;
    let (id, _, path) = super::super::mods::tests::addon_fixture(
        &app,
        name,
        AddonType::Mods,
        "shared.zip",
        b"owned mod bytes",
    )
    .await;
    (
        app,
        id,
        path.parent().unwrap().parent().unwrap().to_path_buf(),
    )
}

fn filter(names: &[&str]) -> ExportEntry {
    ExportEntry(names.iter().map(|name| (name.to_string(), None)).collect())
}

async fn settled(app: &App, task: VisualTaskId) -> anyhow::Result<()> {
    use crate::managers::vtask::TaskOutcome;
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            match app.task_manager().get_task_outcome(task).await {
                TaskOutcome::Succeeded => return Ok(()),
                TaskOutcome::Failed(task) => anyhow::bail!("export failed: {:?}", task.progress),
                TaskOutcome::Running(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                TaskOutcome::Dismissed | TaskOutcome::Unavailable => {
                    anyhow::bail!("export terminal outcome is unavailable");
                }
            }
        }
    })
    .await
    .expect("export task must settle")
}

async fn wait(app: &App, task: VisualTaskId) {
    settled(app, task).await.unwrap();
}

async fn export(
    app: &App,
    id: InstanceId,
    output: &Path,
    target: ExportTarget,
    self_contained: bool,
    filter: ExportEntry,
    version: &str,
) {
    let task = app
        .instance_manager()
        .export_manager()
        .export_instance(
            id,
            target,
            output.to_path_buf(),
            self_contained,
            filter,
            version.into(),
        )
        .await
        .unwrap();
    wait(app, task).await;
    assert!(
        output.exists(),
        "completed exporter must publish an archive"
    );
}

fn manifest(output: &Path) -> Manifest {
    let mut archive = zip::ZipArchive::new(File::open(output).unwrap()).unwrap();
    serde_json::from_reader(archive.by_name("gdlpack.json").unwrap()).unwrap()
}

fn bytes(output: &Path, name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(File::open(output).unwrap()).unwrap();
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

async fn platforms(app: &App, id: InstanceId) {
    app.meta_cache_manager()
        .override_caching_and_wait(
            crate::managers::metadata::cache::CacheEntityId::Instance(id),
            true,
            false,
        )
        .await
        .unwrap();
    let mut seen = std::collections::HashSet::new();
    for row in mfcdb::get_mod_files_by_instance(&app.db, *id)
        .await
        .unwrap()
    {
        if row.filename.starts_with("unknown") || !seen.insert(row.metadata_id.clone()) {
            continue;
        }
        let meta = metarepo::get_metadata_by_id(&app.db, &row.metadata_id)
            .await
            .unwrap()
            .unwrap();
        let now = DbDateTime(chrono::Utc::now().fixed_offset());
        metarepo::upsert_mr_mod_cache(
            &app.db,
            hex::encode(&meta.sha512),
            format!("project-{}", row.metadata_id),
            format!("version-{}", row.metadata_id),
            "title".into(),
            "1".into(),
            "slug".into(),
            "desc".into(),
            "".into(),
            2,
            "".into(),
            row.filename.clone(),
            "https://fixture.invalid/never-requested.zip".into(),
            now,
            row.metadata_id.clone(),
        )
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn task12_non_mod_platform_addons_keep_destinations_and_mod_controls() {
    let (app, id, base) = fixture("task12-destinations").await;
    for (folder, content) in [
        ("resourcepacks", b"resource bytes".as_slice()),
        ("shaderpacks", b"shader bytes".as_slice()),
    ] {
        fs::create_dir_all(base.join(folder)).unwrap();
        fs::write(base.join(folder).join("shared.zip"), content).unwrap();
        fs::write(
            base.join(folder).join("disabled.zip.disabled"),
            b"disabled bytes",
        )
        .unwrap();
    }
    fs::write(base.join("mods/disabled.zip.disabled"), b"disabled mod").unwrap();
    fs::write(base.join("mods/unknown.zip"), b"unknown platform").unwrap();
    platforms(&app, id).await;
    let output = base.parent().unwrap().join("destinations.gdlpack");
    export(
        &app,
        id,
        &output,
        ExportTarget::Gdlauncher,
        false,
        filter(&["mods", "resourcepacks", "shaderpacks"]),
        "1.0.0",
    )
    .await;
    assert_eq!(
        bytes(&output, "overrides/resourcepacks/shared.zip"),
        b"resource bytes"
    );
    assert_eq!(
        bytes(&output, "overrides/shaderpacks/shared.zip"),
        b"shader bytes"
    );
    for folder in ["mods", "resourcepacks", "shaderpacks"] {
        let expected = if folder == "mods" {
            b"disabled mod".as_slice()
        } else {
            b"disabled bytes".as_slice()
        };
        assert_eq!(
            bytes(
                &output,
                &format!("overrides/{folder}/disabled.zip.disabled")
            ),
            expected
        );
    }
    assert_eq!(
        bytes(&output, "overrides/mods/unknown.zip"),
        b"unknown platform"
    );
    let linked = manifest(&output);
    assert_eq!(
        linked.entries.len(),
        1,
        "only the verified enabled mod may become a v1 link"
    );
    let carbon_platforms::gdlauncher::manifest::schema::v1::PackFile::Platform(link) =
        &linked.entries[0]
    else {
        panic!("expected platform mod");
    };
    use sha2::Digest;
    assert_eq!(
        link.hashes.sha512,
        hex::encode(sha2::Sha512::digest(b"owned mod bytes"))
    );
    let mut archive = zip::ZipArchive::new(File::open(&output).unwrap()).unwrap();
    assert!(archive.by_name("overrides/mods/shared.zip").is_err());

    let bundled = base.parent().unwrap().join("self-contained.gdlpack");
    export(
        &app,
        id,
        &bundled,
        ExportTarget::Gdlauncher,
        true,
        filter(&["mods", "resourcepacks", "shaderpacks"]),
        "1.0.0",
    )
    .await;
    assert!(manifest(&bundled).entries.is_empty());
    assert_eq!(
        bytes(&bundled, "overrides/mods/shared.zip"),
        b"owned mod bytes"
    );
    assert_eq!(
        bytes(&bundled, "overrides/resourcepacks/shared.zip"),
        b"resource bytes"
    );
    assert_eq!(
        bytes(&bundled, "overrides/shaderpacks/shared.zip"),
        b"shader bytes"
    );
}

#[tokio::test]
async fn task12_non_mod_export_roundtrips_through_real_override_preparation() {
    let (app, id, base) = fixture("task12-roundtrip").await;
    for folder in ["resourcepacks", "shaderpacks"] {
        fs::create_dir_all(base.join(folder)).unwrap();
        fs::write(base.join(folder).join("shared.zip"), folder.as_bytes()).unwrap();
        fs::write(base.join(folder).join("off.zip.disabled"), b"disabled").unwrap();
        fs::write(base.join(folder).join("excluded.zip"), b"excluded").unwrap();
    }
    platforms(&app, id).await;
    let selected = ExportEntry(
        ["resourcepacks", "shaderpacks"]
            .into_iter()
            .map(|folder| {
                (
                    folder.into(),
                    Some(filter(&["shared.zip", "off.zip.disabled"])),
                )
            })
            .collect(),
    );
    let output = base.parent().unwrap().join("roundtrip.gdlpack");
    export(
        &app,
        id,
        &output,
        ExportTarget::Gdlauncher,
        false,
        selected,
        "1.0.0",
    )
    .await;
    assert!(
        manifest(&output).entries.is_empty(),
        "non-mod import must require no platform lookups"
    );
    let root = tempfile::tempdir().unwrap();
    let destination = InstancePath::new(root.path().join("import"));
    fs::create_dir_all(destination.get_data_path()).unwrap();
    let (progress, _) = tokio::sync::watch::channel(ProgressState::Idle);
    let prepared = prepare_modpack_from_gdlpack(&app, &output, &destination, false, None, progress)
        .await
        .unwrap();
    assert!(prepared.downloadables.is_empty());
    for folder in ["resourcepacks", "shaderpacks"] {
        assert_eq!(
            fs::read(destination.get_data_path().join(folder).join("shared.zip")).unwrap(),
            folder.as_bytes()
        );
        assert_eq!(
            fs::read(
                destination
                    .get_data_path()
                    .join(folder)
                    .join("off.zip.disabled")
            )
            .unwrap(),
            b"disabled"
        );
        assert!(
            !destination
                .get_data_path()
                .join(folder)
                .join("excluded.zip")
                .exists()
        );
    }
    assert!(!destination.get_data_path().join("mods").exists());
}

#[tokio::test]
async fn task12_direct_export_preserves_entered_version_and_other_targets() {
    let (app, id, base) = fixture("task12-version").await;
    for (index, version) in ["2.3.4", "1.0.0", ""].into_iter().enumerate() {
        for (target, name) in [
            (ExportTarget::Gdlauncher, "gdlpack"),
            (ExportTarget::Curseforge, "cf"),
            (ExportTarget::Modrinth, "mrpack"),
        ] {
            let output = base
                .parent()
                .unwrap()
                .join(format!("version-{index}.{name}"));
            export(&app, id, &output, target, true, filter(&["mods"]), version).await;
            let mut archive = zip::ZipArchive::new(File::open(&output).unwrap()).unwrap();
            let filename = match target {
                ExportTarget::Gdlauncher => "gdlpack.json",
                ExportTarget::Curseforge => "manifest.json",
                ExportTarget::Modrinth => "modrinth.index.json",
            };
            let value: serde_json::Value =
                serde_json::from_reader(archive.by_name(filename).unwrap()).unwrap();
            let key = if target == ExportTarget::Modrinth {
                "versionId"
            } else {
                "version"
            };
            assert_eq!(
                value[key], version,
                "{target:?} direct export must retain entered metadata"
            );
            if target == ExportTarget::Gdlauncher {
                assert_eq!(value["dependencies"]["minecraft"], "1.20.1");
            }
        }
    }
}

/// Runs the GDLauncher export the way a share does: links what it can and
/// stops the archive once it would pass `limit`.
async fn share_export(
    app: &App,
    id: InstanceId,
    output: &Path,
    self_contained: bool,
    filter: ExportEntry,
    limit: u64,
) -> VisualTaskId {
    gdlauncher_archive::export_gdlauncher(
        app.clone(),
        id,
        output.to_path_buf(),
        self_contained,
        filter,
        None,
        Some(CancellationToken::new()),
        Some(limit),
    )
    .await
    .unwrap()
}

/// The error a failed export task recorded.
async fn failure(app: &App, task: VisualTaskId) -> Arc<anyhow::Error> {
    use crate::managers::vtask::TaskOutcome;
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            match app.task_manager().get_task_outcome(task).await {
                TaskOutcome::Failed(task) => {
                    let Progress::Failed(error) = task.progress else {
                        panic!("a failed task records its error");
                    };
                    return error;
                }
                TaskOutcome::Running(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                TaskOutcome::Succeeded | TaskOutcome::Dismissed | TaskOutcome::Unavailable => {
                    panic!("the export must fail")
                }
            }
        }
    })
    .await
    .expect("export task must settle")
}

/// Bytes DEFLATE cannot shrink.
fn incompressible(len: usize) -> Vec<u8> {
    let mut state = 0x2545_f491_4f6c_dd1du64;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

#[tokio::test]
async fn task12_share_compressible_input_passes_real_export_and_archive_cap() {
    let (app, id, base) = fixture("task12-compression").await;
    fs::create_dir_all(base.join("logs")).unwrap();
    fs::write(base.join("logs/repeated.log"), vec![b'x'; 8193]).unwrap();
    for folder in ["saves", "Distant_Horizons_server_data"] {
        fs::create_dir_all(base.join(folder)).unwrap();
        File::create(base.join(folder).join("large.dat"))
            .unwrap()
            .set_len(MAX_SHARE_SIZE_BYTES * 2 + 1)
            .unwrap();
    }
    let selected = build_share_filter(&base, false).unwrap();
    let measured = InstanceTooLargeError::measure(&base, &selected, 4096);
    assert!(
        measured.total_bytes > 8192,
        "uncompressed bytes above the limit are only a trimming diagnostic"
    );
    assert!(
        !measured
            .largest_folders
            .iter()
            .any(|f| f.name == "saves" || f.name == "Distant_Horizons_server_data")
    );
    let output = base.parent().unwrap().join("compression.gdlpack");
    let task = share_export(&app, id, &output, false, selected, 4096).await;
    wait(&app, task).await;
    assert!(fs::metadata(&output).unwrap().len() < 4096);
    assert_eq!(
        bytes(&output, "overrides/logs/repeated.log"),
        vec![b'x'; 8193]
    );
    let mut archive = zip::ZipArchive::new(File::open(&output).unwrap()).unwrap();
    assert!(
        !archive
            .file_names()
            .any(|name| name.starts_with("overrides/saves/")
                || name.starts_with("overrides/Distant_Horizons_server_data/"))
    );
    fs::write(base.join("saves/large.dat"), b"included save").unwrap();
    let selected = build_share_filter(&base, true).unwrap();
    let included = base.parent().unwrap().join("included-saves.gdlpack");
    let task = share_export(&app, id, &included, true, selected, 4096).await;
    wait(&app, task).await;
    assert!(fs::metadata(&included).unwrap().len() <= 4096);
    assert_eq!(
        bytes(&included, "overrides/saves/large.dat"),
        b"included save"
    );
}

#[test]
fn archive_sink_takes_an_archive_exactly_at_its_limit_and_stops_one_byte_under() {
    fn write_archive(limit: Option<u64>) -> (anyhow::Result<u64>, Option<ArchiveStop>) {
        let (sink, stopped) = ArchiveSink::new(io::Cursor::new(Vec::new()), limit, None);
        let written = (|| {
            let mut zip = ZipWriter::new(sink);
            for name in ["first.txt", "second.txt"] {
                // Headers are filled in by seeking back, which must not count twice.
                zip.start_file(name, FileOptions::<()>::default())?;
                zip.write_all(&incompressible(4096))?;
            }
            Ok(zip.finish()?.inner.into_inner().len() as u64)
        })();
        (written, stopped.get().copied())
    }

    let (exact, stop) = write_archive(None);
    let exact = exact.unwrap();
    assert_eq!(stop, None);
    let (at_limit, stop) = write_archive(Some(exact));
    assert_eq!(at_limit.unwrap(), exact, "an archive at the limit fits");
    assert_eq!(stop, None);
    let (over, stop) = write_archive(Some(exact - 1));
    assert!(
        over.is_err(),
        "an archive one byte over the limit is refused"
    );
    assert_eq!(stop, Some(ArchiveStop::Limit(exact - 1)));
}

#[test]
fn archive_sink_refuses_every_write_after_it_stopped() {
    let (mut sink, stopped) = ArchiveSink::new(io::Cursor::new(Vec::new()), Some(8), None);
    sink.write_all(b"12345678").unwrap();
    assert!(sink.write_all(b"9").is_err());
    io::Seek::seek(&mut sink, io::SeekFrom::Start(0)).unwrap();
    assert!(
        sink.write_all(b"0").is_err(),
        "a stopped archive stays stopped, even for bytes inside the limit"
    );
    assert_eq!(sink.inner.get_ref(), b"12345678");
    assert_eq!(stopped.get(), Some(&ArchiveStop::Limit(8)));
}

#[test]
fn archive_sink_stops_at_the_first_write_after_a_cancel() {
    let cancel = CancellationToken::new();
    let (mut sink, stopped) =
        ArchiveSink::new(io::Cursor::new(Vec::new()), None, Some(cancel.clone()));
    sink.write_all(b"before").unwrap();
    cancel.cancel();
    let error = sink.write_all(b"after").unwrap_err();
    assert_eq!(error.to_string(), "Export cancelled");
    assert_eq!(sink.inner.get_ref(), b"before");
    let stop = *stopped.get().unwrap();
    assert_eq!(stop, ArchiveStop::Cancelled);
    assert_eq!(
        stop.into_error(Path::new("unused"), &filter(&[]))
            .to_string(),
        "Export cancelled"
    );
}

/// Linux shows a file a process still has open, even after it was deleted,
/// under `/proc/self/fd`.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn a_cancelled_share_export_has_stopped_writing_when_it_reports_the_cancel() {
    use crate::managers::vtask::TaskOutcome;
    let (app, id, base) = fixture("share-cancel").await;
    fs::create_dir_all(base.join("resourcepacks")).unwrap();
    fs::write(
        base.join("resourcepacks/textures.zip"),
        incompressible(48 * 1024 * 1024),
    )
    .unwrap();
    let cancel = CancellationToken::new();
    let output = base.parent().unwrap().join("cancelled.gdlpack");
    let task = gdlauncher_archive::export_gdlauncher(
        app.clone(),
        id,
        output.clone(),
        true,
        filter(&["resourcepacks"]),
        None,
        Some(cancel.clone()),
        Some(MAX_SHARE_SIZE_BYTES),
    )
    .await
    .unwrap();
    // Wait until the archive is being written.
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        while !matches!(
            app.task_manager().get_task_outcome(task).await,
            TaskOutcome::Running(task) if matches!(task.progress, Progress::Known(_))
        ) {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    cancel.cancel();
    let cancelled_at = std::time::Instant::now();

    let error = failure(&app, task).await;
    assert_eq!(error.to_string(), "Export cancelled");
    assert!(
        cancelled_at.elapsed() < std::time::Duration::from_secs(5),
        "the archive stops at its next write instead of compressing the rest"
    );
    let temp = app.settings_manager().runtime_path.get_temp().to_path();
    let still_open = fs::read_dir("/proc/self/fd")
        .unwrap()
        .flatten()
        .filter_map(|fd| fs::read_link(fd.path()).ok())
        .filter(|target| target.starts_with(&temp))
        .collect::<Vec<_>>();
    assert!(
        still_open.is_empty(),
        "a cancelled export stops compressing before it reports the cancel: {still_open:?}"
    );
    assert!(!output.exists());
}

#[tokio::test]
async fn a_share_over_the_limit_stops_and_names_only_the_folders_it_bundles() {
    let (app, id, base) = fixture("share-over-limit").await;
    // The instance's only mod is linked, so it is not part of the archive.
    platforms(&app, id).await;
    fs::create_dir_all(base.join("resourcepacks")).unwrap();
    fs::write(
        base.join("resourcepacks/textures.zip"),
        incompressible(256 * 1024),
    )
    .unwrap();
    fs::create_dir_all(base.join("config")).unwrap();
    fs::write(base.join("config/settings.toml"), b"setting = true").unwrap();
    let selected = build_share_filter(&base, false).unwrap();
    assert!(selected.0.contains_key("mods"));
    let output = base.parent().unwrap().join("over-limit.gdlpack");

    let task = share_export(&app, id, &output, false, selected, 64 * 1024).await;
    let error = failure(&app, task).await;
    let too_large = error
        .downcast_ref::<InstanceTooLargeError>()
        .unwrap_or_else(|| panic!("a share over the limit is refused as too large: {error:?}"));
    assert_eq!(too_large.limit_bytes, 64 * 1024);
    let folders = too_large
        .largest_folders
        .iter()
        .map(|folder| (folder.name.as_str(), folder.bytes))
        .collect::<Vec<_>>();
    assert_eq!(
        folders,
        [("resourcepacks", 256 * 1024), ("config", 14)],
        "linked mods are downloaded by the recipient and do not count"
    );
    assert_eq!(too_large.total_bytes, 256 * 1024 + 14);
    assert!(!output.exists(), "a refused share publishes no archive");
}

#[tokio::test]
async fn task12_optional_version_and_cancellation_slot_are_distinct() {
    let (app, id, base) = fixture("task12-optional-version").await;
    let output = base.parent().unwrap().join("optional.gdlpack");
    let task = gdlauncher_archive::export_gdlauncher(
        app.app.clone(),
        id,
        output.clone(),
        true,
        filter(&["mods"]),
        None,
        Some(CancellationToken::new()),
        None,
    )
    .await
    .unwrap();
    wait(&app, task).await;
    assert_eq!(manifest(&output).version, None);
    let mut archive = zip::ZipArchive::new(File::open(&output).unwrap()).unwrap();
    let value: serde_json::Value =
        serde_json::from_reader(archive.by_name("gdlpack.json").unwrap()).unwrap();
    assert!(
        value.get("version").is_none(),
        "absent share/default version is omitted"
    );
}

#[tokio::test]
async fn task12_gdl_forced_scan_does_not_link_equal_length_replaced_bytes() {
    let (app, id, base) = fixture("task12-current-bytes").await;
    platforms(&app, id).await;
    let path = base.join("mods/shared.zip");
    let stamp = fs::metadata(&path).unwrap().modified().unwrap();
    fs::write(&path, b"other mod bytes").unwrap();
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(stamp))
        .unwrap();
    let output = base.parent().unwrap().join("current.gdlpack");
    export(
        &app,
        id,
        &output,
        ExportTarget::Gdlauncher,
        false,
        filter(&["mods"]),
        "1",
    )
    .await;
    assert!(
        manifest(&output).entries.is_empty(),
        "old platform ownership does not identify new bytes"
    );
    assert_eq!(
        bytes(&output, "overrides/mods/shared.zip"),
        b"other mod bytes"
    );
}

#[tokio::test]
async fn task12_gdl_final_hash_recheck_rejects_mutated_link_before_publication() {
    let (app, id, base) = fixture("task12-final-hash").await;
    platforms(&app, id).await;
    let barrier = app
        .instance_manager()
        .test_hooks
        .arm("task12_gdl_export_before_publish", id);
    let output = base.parent().unwrap().join("mutated.gdlpack");
    let task = app
        .instance_manager()
        .export_manager()
        .export_instance(
            id,
            ExportTarget::Gdlauncher,
            output.clone(),
            false,
            filter(&["mods"]),
            "1".into(),
        )
        .await
        .unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(20),
        barrier.entered.notified(),
    )
    .await
    .unwrap();
    fs::write(base.join("mods/shared.zip"), b"other mod bytes").unwrap();
    barrier.release.notify_one();
    assert!(
        settled(&app, task).await.is_err(),
        "changed linked source must fail before publication"
    );
    assert!(
        !output.exists(),
        "an archive with stale linked hashes must not be published"
    );
}

#[tokio::test]
async fn task12r1_share_wait_keeps_running_progress_one_pending() {
    use crate::managers::vtask::{TaskOutcome, TaskState};
    let app = crate::setup_managers_for_test().await;
    let producer = VisualTask::new(Translation::InstanceExport);
    producer.require_explicit_completion();
    let task = app.task_manager().spawn_task(&producer).await;
    let progress = producer.subtask(Translation::InstanceExportCreatingBundle);
    progress.update_items(1, 1);
    producer
        .edit(|data| data.state = TaskState::KnownProgress)
        .await;
    assert!(matches!(app.task_manager().get_task_outcome(task).await,
        TaskOutcome::Running(task) if task.progress == Progress::Known(1.0)));
    let (tx, mut rx) = mpsc::channel(1);
    let cancel = CancellationToken::new();
    let mut waiter = std::pin::pin!(wait_for_share_export(&app, task, &tx, 25.0, &cancel));
    assert!(
        futures::poll!(&mut waiter).is_pending(),
        "share must await terminal success while progress1 still belongs to a running producer"
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        ShareInstanceProgress::Progress(25)
    ));
    drop(rx); // A closed progress receiver does not manufacture a task failure.
    producer.complete();
    drop(producer);
    tokio::time::timeout(std::time::Duration::from_secs(5), &mut waiter)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        app.task_manager().get_task_outcome(task).await,
        TaskOutcome::Succeeded
    ));
}

#[tokio::test]
async fn task12r1_share_wait_cancellation_interrupts_progress_backpressure() {
    use crate::managers::vtask::TaskState;
    let app = crate::setup_managers_for_test().await;
    let producer = VisualTask::new(Translation::InstanceExport);
    let task = app.task_manager().spawn_task(&producer).await;
    let progress = producer.subtask(Translation::InstanceExportCreatingBundle);
    progress.update_items(1, 2);
    producer
        .edit(|data| data.state = TaskState::KnownProgress)
        .await;
    let (tx, mut rx) = mpsc::channel(1);
    tx.send(ShareInstanceProgress::Progress(7)).await.unwrap();
    let cancel = CancellationToken::new();
    let mut waiter = std::pin::pin!(wait_for_share_export(&app, task, &tx, 25.0, &cancel));
    assert!(futures::poll!(&mut waiter).is_pending());
    cancel.cancel();
    let error = tokio::time::timeout(std::time::Duration::from_secs(1), &mut waiter)
        .await
        .expect("cancellation must interrupt the actual share wait under progress backpressure")
        .unwrap_err();
    assert_eq!(error.to_string(), "Share cancelled");
    assert!(matches!(
        rx.try_recv().unwrap(),
        ShareInstanceProgress::Progress(7)
    ));
    let error = wait_for_share_export(&app, task, &tx, 25.0, &cancel)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Share cancelled",
        "pre-cancelled waits remain cancelled"
    );
    drop(producer);
}

#[tokio::test]
async fn task12r1_share_wait_observes_real_publication_and_late_hash_failure() {
    use crate::managers::vtask::TaskOutcome;
    for mutate in [false, true] {
        let (app, id, base) = fixture("task12r1-publication").await;
        platforms(&app, id).await;
        let barrier = app
            .instance_manager()
            .test_hooks
            .arm("task12_gdl_export_before_publish", id);
        let output = base.parent().unwrap().join("share-boundary.gdlpack");
        let cancel = CancellationToken::new();
        let task = gdlauncher_archive::export_gdlauncher(
            app.app.clone(),
            id,
            output.clone(),
            false,
            filter(&["mods"]),
            None,
            Some(cancel.clone()),
            Some(4096),
        )
        .await
        .unwrap();
        tokio::time::timeout(
            std::time::Duration::from_secs(20),
            barrier.entered.notified(),
        )
        .await
        .unwrap();
        assert!(matches!(
            app.task_manager().get_task_outcome(task).await,
            TaskOutcome::Running(_)
        ));
        assert!(
            !output.exists(),
            "producer remains before archive publication"
        );
        let (tx, mut rx) = mpsc::channel(1);
        let mut waiter = std::pin::pin!(wait_for_share_export(&app, task, &tx, 25.0, &cancel));
        assert!(
            futures::poll!(&mut waiter).is_pending(),
            "actual share wait cannot complete before producer publication"
        );
        if mutate {
            fs::write(base.join("mods/shared.zip"), b"other mod bytes").unwrap();
        }
        barrier.release.notify_one();
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                tokio::select! {
                    result = &mut waiter => break result,
                    progress = rx.recv() => assert!(progress.is_some()),
                }
            }
        })
        .await
        .unwrap();
        if mutate {
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains("Export task failed:"),
                "retain exporter failure context: {error}"
            );
            assert!(
                error.contains(
                    "mods/shared.zip changed while the instance was being exported. Try the export again."
                ),
                "surface late hash verification failure, naming the file: {error}"
            );
            assert!(!output.exists());
        } else {
            result.unwrap();
            assert!(matches!(
                app.task_manager().get_task_outcome(task).await,
                TaskOutcome::Succeeded
            ));
            assert!(
                output.exists(),
                "terminal success follows actual publication"
            );
            assert_eq!(manifest(&output).entries.len(), 1);
            assert!(fs::metadata(&output).unwrap().len() <= 4096);
        }
    }
}

#[tokio::test]
async fn task12r1_share_wait_rejects_dismissed_and_unavailable_outcomes() {
    let app = crate::setup_managers_for_test().await;
    let producer = VisualTask::new(Translation::InstanceExport);
    let task = app.task_manager().spawn_task(&producer).await;
    producer
        .clone()
        .fail(anyhow::anyhow!("controlled exporter failure"))
        .await;
    app.task_manager().dismiss_task(task).await.unwrap();
    let (tx, mut rx) = mpsc::channel(1);
    let cancel = CancellationToken::new();
    let error = wait_for_share_export(&app, task, &tx, 25.0, &cancel)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Export task was dismissed");
    let error = wait_for_share_export(&app, VisualTaskId(i32::MIN), &tx, 25.0, &cancel)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Export task outcome is unavailable");
    assert!(matches!(
        rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    drop(producer);
}

/// What a share reports when its export fails with `error`.
async fn share_error_for(app: &App, error: anyhow::Error) -> anyhow::Error {
    let producer = VisualTask::new(Translation::InstanceExport);
    let task = app.task_manager().spawn_task(&producer).await;
    producer.clone().fail(error).await;
    let (tx, _rx) = mpsc::channel(8);
    let result = wait_for_share_export(app, task, &tx, 25.0, &CancellationToken::new()).await;
    drop(producer);
    result.unwrap_err()
}

#[tokio::test]
async fn a_share_keeps_the_too_large_and_disk_full_errors_of_its_export() {
    let app = crate::setup_managers_for_test().await;

    let error = share_error_for(
        &app,
        InstanceTooLargeError {
            total_bytes: 3,
            limit_bytes: 2,
            largest_folders: vec![ShareFolderSize {
                name: "saves".into(),
                bytes: 3,
            }],
        }
        .into(),
    )
    .await;
    let too_large = error
        .downcast_ref::<InstanceTooLargeError>()
        .expect("the share window shows the folders to trim");
    assert_eq!((too_large.total_bytes, too_large.limit_bytes), (3, 2));
    assert_eq!(too_large.largest_folders[0].name, "saves");

    for kind in [io::ErrorKind::StorageFull, io::ErrorKind::QuotaExceeded] {
        let full = anyhow::Error::from(zip::result::ZipError::Io(io::Error::from(kind)))
            .context("writing the archive");
        let error = share_error_for(&app, full).await;
        let share_error = error
            .downcast_ref::<InstanceShareError>()
            .unwrap_or_else(|| panic!("a full disk gets its own message: {error:?}"));
        assert!(matches!(
            share_error,
            InstanceShareError::NotEnoughDiskSpace
        ));
        use crate::error::FeErrorCode;
        assert_eq!(share_error.error_code(), "NOT_ENOUGH_DISK_SPACE");
    }

    let error = share_error_for(&app, anyhow::anyhow!("controlled exporter failure")).await;
    assert_eq!(
        error.to_string(),
        "Export task failed: controlled exporter failure"
    );
}
