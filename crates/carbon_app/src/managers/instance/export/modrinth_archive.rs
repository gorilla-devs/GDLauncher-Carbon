use crate::{
    api::translation::Translation,
    domain::{
        instance::{
            ExportEntry, InstanceId,
            info::{GameVersion, ModLoaderType},
        },
        vtask::VisualTaskId,
    },
    managers::{
        AppInner,
        instance::{InstanceType, InvalidInstanceIdError},
        modplatforms::modrinth::convert_standard_version_to_mr_version,
        vtask::{TaskState, VisualTask},
    },
};
use anyhow::anyhow;
use carbon_platforms::modrinth::version::{
    Hashes, ModpackIndex, ModrinthFile, ModrinthGame, ModrinthPackDependencies,
};
use std::{collections::HashMap, fs::File, io::Write, path::PathBuf, sync::Arc};
use tokio::sync::mpsc;

use carbon_repos::repos::mod_file_cache as mfcdb;

use super::ZipMode;

pub async fn export_modrinth(
    app: Arc<AppInner>,
    instance_id: InstanceId,
    save_path: PathBuf,
    self_contained_addons_bundling: bool,
    mut filter: ExportEntry,
    version: Option<String>,
) -> anyhow::Result<(VisualTaskId, tokio::task::JoinHandle<()>)> {
    let instance_manager = app.instance_manager();
    let instances = instance_manager.instances.read().await;
    let instance = instances
        .get(&instance_id)
        .ok_or(InvalidInstanceIdError(instance_id))?;

    let basepath = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .get_instance_path(&instance.shortpath)
        .get_data_path();

    let InstanceType::Valid(data) = &instance.type_ else {
        return Err(anyhow!("Instance {instance_id} is not in a valid state"));
    };

    let config = data.config.clone();

    drop(instances);

    let Some(game_version) = config.game_configuration.version else {
        return Err(anyhow!(
            "Instance {instance_id}'s game version is not known so it cannot be exported"
        ));
    };

    let GameVersion::Standard(game_version) = game_version else {
        return Err(anyhow!(
            "Instance {instance_id} has a custom game version file so it cannot be exported"
        ));
    };

    let vtask = VisualTask::new(Translation::InstanceExport);
    let vtask_id = app.task_manager().spawn_task(&vtask).await;

    let handle = tokio::spawn(async move {
        let try_result: anyhow::Result<_> = async {
            let mut mods = Vec::new();
            let mut linked_files = Vec::new();

            let t_create_bundle = vtask.subtask(Translation::InstanceExportCreatingBundle);
            t_create_bundle.start_opaque();

            vtask
                .edit(|data| data.state = TaskState::Indeterminate)
                .await;

            if !self_contained_addons_bundling {
                let mods_filter = filter.0.get_mut("mods");
                if let Some(mods_filter) = mods_filter {

                    if mods_filter.is_none() {
                        let mut modsdir_entries = HashMap::new();

                        if let Ok(mut dir) = tokio::fs::read_dir(basepath.join("mods")).await {
                            while let Some(next) = dir.next_entry().await? {
                                let name = next.file_name();
                                let Some(name) = name.to_str() else { continue };
                                modsdir_entries.insert(name.to_string(), None);
                            }
                        }

                        *mods_filter = Some(ExportEntry(modsdir_entries));
                    }

                    let mods_filter = mods_filter.as_mut().map(|v| &mut v.0).unwrap();

                    // Re-read every mod before linking it: a same-size
                    // replacement must not ship as the old Modrinth file.
                    app.meta_cache_manager()
                        .override_caching_verified_and_wait(
                            crate::managers::metadata::cache::CacheEntityId::Instance(instance_id),
                            crate::managers::metadata::cache::VerifyScope::MODS,
                            false,
                            true,
                        )
                        .await?;

                    let instance_id_val = *instance_id;
                    let cached =
                        mfcdb::get_instance_mr_export_files(&app.db, instance_id_val).await?;

                    for row in cached {
                        if let Some((path, row)) = select_linked_mod(&basepath, mods_filter, row).await? {
                            linked_files.push((path, row.sha512.clone()));
                            mods.push((row.filename, row.filesize, row.sha512, row.sha1, row.mr_file_url.unwrap()));
                        }
                    }
                }
            }

            let mut file_count = 0;
            let mut size_count = 0;

            super::zip_excluding(
                ZipMode::<File, ()>::Count(&mut file_count, &mut size_count),
                &basepath,
                "overrides",
                &filter,
            )?;

            t_create_bundle.complete_opaque();

            vtask
            .edit(|data| data.state = TaskState::Indeterminate)
            .await;

            t_create_bundle.update_items(0, 100);

            let manifest = ModpackIndex {
                format_version: 1,
                game: ModrinthGame::Minecraft,
                version_id: version.unwrap_or_else(|| String::from("1.0.0")),
                name: config.name,
                summary: None,
                files: mods
                    .into_iter()
                    .map(
                        |(file_name, file_size, sha512, sha1, file_url)| ModrinthFile {
                            path: format!("mods/{file_name}"),
                            hashes: Hashes {
                                sha512: hex::encode(sha512),
                                sha1: hex::encode(sha1),
                                others: HashMap::new(),
                            },
                            env: None,
                            downloads: vec![file_url],
                            file_size: file_size as u32,
                        },
                    )
                    .collect(),
                dependencies: convert_standard_version_to_mr_version(game_version),
            };

            let tmpfile = app
                .settings_manager()
                .runtime_path
                .get_temp()
                .maketmpfile()
                .await?;

            let send_path = tmpfile.to_path_buf();
            let (notify_tx, mut notify_rx) = tokio::sync::watch::channel::<(u64, u64)>(
                (0, 0)
            );

            let ziptask = tokio::task::spawn_blocking(move || {
                let mut zip = zip::ZipWriter::new(File::create(send_path)?);
                let options = zip::write::FileOptions::<()>::default();
                zip.start_file("modrinth.index.json", options)?;
                zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;

                super::zip_excluding(
                    ZipMode::Create(&mut zip, options, notify_tx),
                    &basepath,
                    "overrides",
                    &filter,
                )?;

                zip.finish()?;
                Ok::<_, anyhow::Error>(())
            });

            tokio::select! {
                r = ziptask => r??,
                _ = async {
                    let mut counter = 0;
                    let mut size = 0;

                    loop {
                        if notify_rx.changed().await.is_ok() {
                            let (bytes_read, count) = notify_rx.borrow().clone();

                            counter += count;
                            size += bytes_read;

                            const SIZE_WEIGHT: f64 = 0.6;
                            const COUNT_WEIGHT: f64 = 0.4;

                            let estimated_total_files = file_count;
                            let estimated_total_size = size_count;

                            let count_progress = (counter as f64 / estimated_total_files as f64).min(1.0);
                            let size_progress = (size as f64 / estimated_total_size as f64).min(1.0);

                            let weighted_progress = (count_progress * COUNT_WEIGHT + size_progress * SIZE_WEIGHT) * 100.0;

                            let computed_curr = weighted_progress as u32;

                            t_create_bundle.update_items(computed_curr, 100);
                        } else {
                            futures::future::pending().await
                        }
                    }
                } => {},
            }

            #[cfg(test)]
            app.instance_manager().test_hooks.pause("task11_export_before_publish", instance_id).await;
            super::verify_linked_export_files(&app, instance_id, &linked_files).await?;
            tmpfile.try_rename_or_move(save_path).await?;

            println!("Completed items");
            t_create_bundle.complete_items();

            Ok(())
        }
        .await;

        if let Err(e) = try_result {
            vtask.fail(e).await;
        }
    });

    Ok((vtask_id, handle))
}

#[cfg(test)]
mod test {
    use std::{
        collections::{HashMap, HashSet},
        fs::File,
        io::Read,
        sync::Arc,
    };

    use flowtest::flowtest;
    use tracing_test::traced_test;
    use zip::ZipArchive;

    use crate::{
        domain::instance::{ExportEntry, InstanceId, info},
        managers::instance::{InstanceVersionSource, export::ExportTarget},
    };

    #[traced_test]
    #[test]
    #[flowtest]
    fn _setup() -> anyhow::Result<(
        Arc<tokio::runtime::Runtime>,
        Arc<crate::TestEnv>,
        InstanceId,
    )> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let rt = Arc::new(rt);

        rt.block_on(async {
            let app = Arc::new(crate::setup_managers_for_test().await);

            let default_group_id = app.instance_manager().get_default_group().await?;
            let instance_id = app
                .instance_manager()
                .create_instance(
                    default_group_id,
                    String::from("test"),
                    false,
                    InstanceVersionSource::Version(info::GameVersion::Standard(
                        info::StandardVersion {
                            release: String::from("1.16.5"),
                            modloaders: HashSet::from([info::ModLoader {
                                type_: info::ModLoaderType::Forge,
                                version: String::from("36.2.34"),
                            }]),
                        },
                    )),
                    String::new(),
                )
                .await?;

            let task = app
                .instance_manager()
                .install_modrinth_mod(
                    instance_id,
                    String::from("fPetb5Kh"),
                    String::from("o0SCfsMe"),
                    false,
                    None,
                )
                .await?;

            app.task_manager().wait_with_log(task).await?;

            Ok((rt.clone(), app, instance_id))
        })
    }

    async fn run_export(
        app: &Arc<crate::TestEnv>,
        instance_id: InstanceId,
        filename: &str,
        export_entry: ExportEntry,
        self_contained_addons_bundling: bool,
    ) -> anyhow::Result<()> {
        let target_file = app
            .settings_manager()
            .runtime_path
            .get_root()
            .to_path()
            .join(filename);

        let task = app
            .instance_manager()
            .export_manager()
            .export_instance(
                instance_id,
                ExportTarget::Modrinth,
                target_file.clone(),
                self_contained_addons_bundling,
                export_entry,
                String::from("1.0.0"),
            )
            .await?;

        app.task_manager().wait_with_log(task).await?;

        Ok(())
    }

    async fn check_export(
        app: &Arc<crate::TestEnv>,
        filename: &str,
        check: impl FnOnce(String, ZipArchive<File>) -> anyhow::Result<()> + Send + 'static,
    ) -> anyhow::Result<()> {
        let target_file = app
            .settings_manager()
            .runtime_path
            .get_root()
            .to_path()
            .join(filename);

        tokio::task::spawn_blocking(|| {
            let mut zip = ZipArchive::new(File::open(target_file)?)?;

            let mut file = zip.by_name("modrinth.index.json")?;
            let mut manifest = String::new();
            file.read_to_string(&mut manifest)?;
            drop(file);

            check(manifest, zip)
        })
        .await?
    }

    #[traced_test]
    #[test]
    #[flowtest(_setup: (rt, app, instance_id))]
    fn export_with_folder_linked() -> anyhow::Result<()> {
        rt.block_on(async {
            run_export(
                &app,
                instance_id,
                "folder_linked.zip",
                ExportEntry(HashMap::from([(String::from("mods"), None)])),
                false,
            )
            .await?;

            check_export(&app, "folder_linked.zip", |manifest, mut zip| {
                crate::assert_eq_display!(
                    manifest,
                    r#"{
  "formatVersion": 1,
  "game": "minecraft",
  "versionId": "1.0.0",
  "name": "test",
  "summary": null,
  "files": [
    {
      "path": "mods/NaturesCompass-1.16.5-1.9.1-forge.jar",
      "hashes": {
        "sha512": "bc99c1abb320f84ad7670f35649386855e877d8cce3aaeb12654107e4cdd52acb8475a2a66e6cb5f419dc8cc4d1ecf4c3f6d521e51ee9f1525d1403007e2c0b2",
        "sha1": "38c37c257dcdcf47d5b363eb3e39eebc645b7be4"
      },
      "env": null,
      "downloads": [
        "https://cdn.modrinth.com/data/fPetb5Kh/versions/o0SCfsMe/NaturesCompass-1.16.5-1.9.1-forge.jar"
      ],
      "fileSize": 203573
    }
  ],
  "dependencies": {
    "minecraft": "1.16.5",
    "forge": "36.2.34"
  }
}"#
                );

                assert!(zip.by_name("overrides/mods").is_err());
                Ok(())
            })
            .await?;

            Ok(())
        })
    }

    #[traced_test]
    #[test]
    #[flowtest(_setup: (rt, app, instance_id))]
    fn export_with_folder_unlinked() -> anyhow::Result<()> {
        rt.block_on(async {
            run_export(
                &app,
                instance_id,
                "folder_unlinked.zip",
                ExportEntry(HashMap::from([(String::from("mods"), None)])),
                true,
            )
            .await?;

            check_export(&app, "folder_unlinked.zip", |manifest, mut zip| {
                crate::assert_eq_display!(
                    manifest,
                    r#"{
  "formatVersion": 1,
  "game": "minecraft",
  "versionId": "1.0.0",
  "name": "test",
  "summary": null,
  "files": [],
  "dependencies": {
    "minecraft": "1.16.5",
    "forge": "36.2.34"
  }
}"#
                );

                assert!(
                    zip.by_name("overrides/mods/NaturesCompass-1.16.5-1.9.1-forge.jar")
                        .is_ok()
                );
                Ok(())
            })
            .await?;

            Ok(())
        })
    }

    #[traced_test]
    #[test]
    #[flowtest(_setup: (rt, app, instance_id))]
    fn export_without_folder_linked() -> anyhow::Result<()> {
        rt.block_on(async {
            run_export(
                &app,
                instance_id,
                "nofolder_linked.zip",
                ExportEntry(HashMap::from([])),
                false,
            )
            .await?;

            check_export(&app, "nofolder_linked.zip", |manifest, mut zip| {
                crate::assert_eq_display!(
                    manifest,
                    r#"{
  "formatVersion": 1,
  "game": "minecraft",
  "versionId": "1.0.0",
  "name": "test",
  "summary": null,
  "files": [],
  "dependencies": {
    "minecraft": "1.16.5",
    "forge": "36.2.34"
  }
}"#
                );

                assert!(zip.by_name("overrides/mods").is_err());
                Ok(())
            })
            .await?;

            Ok(())
        })
    }

    #[traced_test]
    #[test]
    #[flowtest(_setup: (rt, app, instance_id))]
    fn export_without_folder_unlinked() -> anyhow::Result<()> {
        rt.block_on(async {
            run_export(
                &app,
                instance_id,
                "nofolder_unlinked.zip",
                ExportEntry(HashMap::from([])),
                true,
            )
            .await?;

            check_export(&app, "nofolder_unlinked.zip", |manifest, mut zip| {
                crate::assert_eq_display!(
                    manifest,
                    r#"{
  "formatVersion": 1,
  "game": "minecraft",
  "versionId": "1.0.0",
  "name": "test",
  "summary": null,
  "files": [],
  "dependencies": {
    "minecraft": "1.16.5",
    "forge": "36.2.34"
  }
}"#
                );

                assert!(zip.by_name("overrides/mods").is_err());
                Ok(())
            })
            .await?;

            Ok(())
        })
    }

    #[traced_test]
    #[test]
    #[flowtest(_setup: (rt, app, instance_id))]
    fn export_with_fake_folder_linked() -> anyhow::Result<()> {
        rt.block_on(async {
            run_export(
                &app,
                instance_id,
                "fakefolder_linked.zip",
                ExportEntry(HashMap::from([(
                    String::from("mods"),
                    Some(ExportEntry(HashMap::from([(
                        String::from("fake-mod.jar"),
                        None,
                    )]))),
                )])),
                true,
            )
            .await?;

            check_export(&app, "fakefolder_linked.zip", |manifest, mut zip| {
                crate::assert_eq_display!(
                    manifest,
                    r#"{
  "formatVersion": 1,
  "game": "minecraft",
  "versionId": "1.0.0",
  "name": "test",
  "summary": null,
  "files": [],
  "dependencies": {
    "minecraft": "1.16.5",
    "forge": "36.2.34"
  }
}"#
                );

                assert!(zip.by_name("overrides/mods").is_err());
                Ok(())
            })
            .await?;

            Ok(())
        })
    }
}

// This is the same artifact qualification used by the archive producer. Keeping
// selection independent of App startup permits controlled archive tests.
async fn select_linked_mod(
    basepath: &std::path::Path,
    mods_filter: &mut HashMap<String, Option<ExportEntry>>,
    row: mfcdb::MrExportRow,
) -> anyhow::Result<Option<(PathBuf, mfcdb::MrExportRow)>> {
    // Match the downloader's parsed absolute-host and HTTP(S) acceptance, but
    // retain the original string: normalizing a signed URL can invalidate it.
    let usable_url = row.mr_file_url.as_deref().is_some_and(|url| {
        reqwest::Url::parse(url)
            .is_ok_and(|parsed| parsed.has_host() && matches!(parsed.scheme(), "http" | "https"))
    });
    if !usable_url
        || !row
            .mr_sha512
            .as_deref()
            .is_some_and(|hash| hash.eq_ignore_ascii_case(&hex::encode(&row.sha512)))
    {
        return Ok(None);
    }
    if !mods_filter.contains_key(&row.filename) {
        return Ok(None);
    }
    let path = basepath.join("mods").join(&row.filename);
    super::verify_linked_mod(&path, &row.sha512, row.file_stamp.as_deref()).await?;
    mods_filter.remove(&row.filename);
    Ok(Some((path, row)))
}

#[cfg(test)]
mod task15_export_tests {
    use super::*;
    use sha1::Sha1;
    use sha2::{Digest, Sha512};
    fn row(name: &str, bytes: &[u8], trusted: bool) -> mfcdb::MrExportRow {
        let hash = Sha512::digest(bytes).to_vec();
        mfcdb::MrExportRow {
            filename: name.into(),
            filesize: bytes.len() as i32,
            sha512: hash.clone(),
            sha1: Sha1::digest(bytes).to_vec(),
            mr_sha512: Some(if trusted {
                hex::encode(hash)
            } else {
                "wrong".into()
            }),
            mr_file_url: Some(format!("https://fixture.invalid/{name}")),
            file_stamp: None,
        }
    }
    #[tokio::test]
    async fn task15_linked_selection_bundles_untrusted_bytes_and_accepts_renamed_artifact() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("mods")).unwrap();
        let mut selected = HashMap::new();
        for (name, bytes) in [
            ("renamed.jar", b"one"),
            ("untrusted.jar", b"two"),
            ("disabled.jar.disabled", b"off"),
        ] {
            std::fs::write(dir.path().join("mods").join(name), bytes).unwrap();
            selected.insert(name.into(), None);
        }
        assert!(
            select_linked_mod(dir.path(), &mut selected, row("renamed.jar", b"one", true))
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            select_linked_mod(
                dir.path(),
                &mut selected,
                row("untrusted.jar", b"two", false)
            )
            .await
            .unwrap()
            .is_none()
        );
        let filter = ExportEntry(HashMap::from([(
            "mods".into(),
            Some(ExportEntry(selected)),
        )]));
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let (tx, _) = tokio::sync::watch::channel((0, 0));
        super::super::zip_excluding(
            ZipMode::Create(&mut zip, zip::write::FileOptions::<()>::default(), tx),
            dir.path(),
            "overrides",
            &filter,
        )
        .unwrap();
        let archive = zip.finish().unwrap();
        let mut archive = zip::ZipArchive::new(archive).unwrap();
        assert!(archive.by_name("overrides/mods/renamed.jar").is_err());
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(
            &mut archive.by_name("overrides/mods/untrusted.jar").unwrap(),
            &mut bytes,
        )
        .unwrap();
        assert_eq!(bytes, b"two");
        assert!(
            archive
                .by_name("overrides/mods/disabled.jar.disabled")
                .is_ok()
        );
    }
    #[tokio::test]
    async fn task15_empty_remote_url_remains_local() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("mods")).unwrap();
        std::fs::write(dir.path().join("mods/a.jar"), b"bytes").unwrap();
        let mut filter = HashMap::from([("a.jar".into(), None)]);
        let mut artifact = row("a.jar", b"bytes", true);
        artifact.mr_file_url = Some(String::new());
        assert!(
            select_linked_mod(dir.path(), &mut filter, artifact)
                .await
                .unwrap()
                .is_none()
        );
        assert!(filter.contains_key("a.jar"));
    }
    #[tokio::test]
    async fn task15_linked_selection_rechecks_changed_content_before_omitting_override() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("mods")).unwrap();
        std::fs::write(dir.path().join("mods/a.jar"), b"changed").unwrap();
        let mut filter = HashMap::from([("a.jar".into(), None)]);
        assert!(
            select_linked_mod(dir.path(), &mut filter, row("a.jar", b"original", true))
                .await
                .is_err()
        );
        assert!(filter.contains_key("a.jar"));
    }
    #[tokio::test]
    async fn task15_round01_remote_urls_preserve_local_bytes_or_original_download() {
        let cases = [
            ("malformed.jar", Some("not-a-url"), false),
            ("hostless.jar", Some("https:///"), false),
            ("file.jar", Some("file:///artifact.jar"), false),
            ("ftp.jar", Some("ftp://fixture.invalid/artifact.jar"), false),
            ("missing.jar", None, false),
            (
                "ordinary.jar",
                Some("http://fixture.invalid/artifact.jar"),
                true,
            ),
            (
                "renamed.jar",
                Some("https://fixture.invalid/original.jar?token=A%2Fb+Z&signature=AbCd%3D"),
                true,
            ),
        ];
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("mods")).unwrap();
        let mut selected = HashMap::new();
        for (name, _, _) in cases {
            std::fs::write(dir.path().join("mods").join(name), name.as_bytes()).unwrap();
            selected.insert(name.into(), None);
        }
        let mut decisions = Vec::new();
        for (name, url, expected) in cases {
            let mut artifact = row(name, name.as_bytes(), true);
            artifact.mr_file_url = url.map(str::to_owned);
            let result = select_linked_mod(dir.path(), &mut selected, artifact)
                .await
                .unwrap();
            if let Some((_, linked)) = &result {
                assert_eq!(
                    linked.mr_file_url.as_deref(),
                    url,
                    "preserve original signed download string"
                );
            }
            decisions.push((name, result.is_some(), expected));
        }
        let filter = ExportEntry(HashMap::from([(
            "mods".into(),
            Some(ExportEntry(selected)),
        )]));
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let (tx, _) = tokio::sync::watch::channel((0, 0));
        super::super::zip_excluding(
            ZipMode::Create(&mut zip, zip::write::FileOptions::<()>::default(), tx),
            dir.path(),
            "overrides",
            &filter,
        )
        .unwrap();
        let mut archive = zip::ZipArchive::new(zip.finish().unwrap()).unwrap();
        for (name, _, expected) in cases {
            if expected {
                assert!(archive.by_name(&format!("overrides/mods/{name}")).is_err());
            } else {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(
                    &mut archive
                        .by_name(&format!("overrides/mods/{name}"))
                        .unwrap_or_else(|_| {
                            panic!(
                                "invalid URL removed local override {name}; decisions={decisions:?}"
                            )
                        }),
                    &mut bytes,
                )
                .unwrap();
                assert_eq!(bytes, name.as_bytes());
            }
        }
        assert!(
            decisions
                .iter()
                .all(|(_, actual, expected)| actual == expected),
            "{decisions:?}"
        );
    }
}
