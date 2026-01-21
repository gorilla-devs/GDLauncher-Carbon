use crate::{
    api::translation::Translation,
    domain::{
        instance::{
            info::{GameVersion, InstanceIcon, ModLoaderType, Modpack},
            ExportEntry, InstanceId,
        },
        vtask::VisualTaskId,
    },
    managers::{
        instance::{export::ZipMode, InstanceType, InvalidInstanceIdError},
        vtask::{TaskState, VisualTask},
        AppInner,
    },
};
use anyhow::anyhow;
use carbon_platforms::gdlauncher::manifest::schema::v1::{
    FileHashes, GameDependencies, Manifest, ModloaderDependency, ModloaderType, ModpackSource,
    PackFile, PlatformFile,
};
use carbon_repos::db::{
    curse_forge_mod_cache as cfdb, mod_file_cache as fcdb, mod_metadata as metadb,
    modrinth_mod_cache as mrdb,
};
use chrono::Utc;
use std::{collections::HashMap, fs::File, io::Write, path::Path, path::PathBuf, sync::Arc};
use tokio::sync::watch;
use tracing::trace;

/// Export an instance to the GDLPack format (.gdlpack)
///
/// The GDLPack format is a minimal, hash-based modpack distribution format that:
/// - Uses hashes to resolve files from CurseForge and Modrinth at import time
/// - Derives file path, addon type, size, and metadata from platform APIs
/// - Embeds icon for offline display
/// - Supports optional files and overrides
pub async fn export_gdlauncher(
    app: Arc<AppInner>,
    instance_id: InstanceId,
    save_path: PathBuf,
    self_contained_addons_bundling: bool,
    mut filter: ExportEntry,
) -> anyhow::Result<VisualTaskId> {
    let instance_manager = app.instance_manager();
    let instances = instance_manager.instances.read().await;
    let instance = instances
        .get(&instance_id)
        .ok_or(InvalidInstanceIdError(instance_id))?;

    let instance_path = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .get_instance_path(&instance.shortpath);

    let basepath = instance_path.get_data_path();

    // Instance root is where icons are stored (not inside /instance/ subdirectory)
    let instance_root = app
        .settings_manager()
        .runtime_path
        .get_instances()
        .to_path()
        .join(&instance.shortpath);

    let InstanceType::Valid(data) = &instance.type_ else {
        return Err(anyhow!("Instance {instance_id} is not in a valid state"));
    };

    let config = data.config.clone();
    let instance_icon = config.icon.clone();
    let modpack_info = config.modpack.clone();

    drop(instances);

    // Read icon data if present (before spawn_blocking)
    // Icons are stored at instance root, not inside the /instance/ data directory
    let icon_data: Option<(Vec<u8>, String)> = match &instance_icon {
        InstanceIcon::RelativePath(icon_path) => {
            let icon_full_path = instance_root.join(icon_path);
            if icon_full_path.exists() {
                let ext = Path::new(icon_path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("png")
                    .to_string();
                match std::fs::read(&icon_full_path) {
                    Ok(data) => Some((data, ext)),
                    Err(e) => {
                        tracing::warn!("Failed to read instance icon: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        }
        InstanceIcon::Default => None,
    };

    let Some(version) = config.game_configuration.version else {
        return Err(anyhow!(
            "Instance {instance_id}'s game version is not known so it cannot be exported"
        ));
    };

    let GameVersion::Standard(version) = version else {
        return Err(anyhow!(
            "Instance {instance_id} has a custom game version file so it cannot be exported"
        ));
    };

    let vtask = VisualTask::new(Translation::InstanceExport);
    let vtask_id = app.task_manager().spawn_task(&vtask).await;

    tokio::spawn(async move {
        let try_result: anyhow::Result<_> = async {
            let mut pack_files: Vec<PackFile> = Vec::new();

            let t_calc_size = vtask.subtask(Translation::InstanceExportCalculateSize);
            t_calc_size.set_weight(0.0);
            let t_create_bundle = vtask.subtask(Translation::InstanceExportCreatingBundle);

            vtask
                .edit(|data| data.state = TaskState::KnownProgress)
                .await;

            // Collect tracked addon files from ModFileCache
            if !self_contained_addons_bundling {
                // Process tracked addon folders (mods, resourcepacks, shaders, etc.)
                for addon_folder in &["mods", "resourcepacks", "shaderpacks"] {
                    let folder_filter = filter.0.get_mut(*addon_folder);
                    if let Some(folder_filter) = folder_filter {
                        // Build filter if not specified
                        if folder_filter.is_none() {
                            let mut entries = HashMap::new();
                            let folder_path = basepath.join(addon_folder);
                            if let Ok(mut dir) = tokio::fs::read_dir(&folder_path).await {
                                while let Some(next) = dir.next_entry().await? {
                                    let name = next.file_name();
                                    let Some(name) = name.to_str() else {
                                        continue;
                                    };
                                    entries.insert(name.to_string(), None);
                                }
                            }
                            *folder_filter = Some(ExportEntry(entries));
                        }

                        let folder_filter = folder_filter.as_mut().map(|v| &mut v.0).unwrap();

                        // Ensure cache is up to date
                        app.meta_cache_manager()
                            .override_caching_and_wait(instance_id, true, false)
                            .await?;

                        // Query ModFileCache for tracked files
                        let cached_files = app
                            .prisma_client
                            .mod_file_cache()
                            .find_many(vec![fcdb::instance_id::equals(*instance_id)])
                            .with(
                                fcdb::metadata::fetch()
                                    .with(metadb::curseforge::fetch())
                                    .with(metadb::modrinth::fetch()),
                            )
                            .exec()
                            .await?;

                        for cached_file in cached_files {
                            // Check if in filter (don't remove yet - we may need to keep it for overrides)
                            if !folder_filter.contains_key(&cached_file.filename) {
                                continue;
                            }

                            let Some(metadata) = cached_file.metadata else {
                                continue;
                            };

                            // Check if file has platform data in cache - if yes, it's resolvable
                            // This avoids making network calls for every file during export
                            let has_curseforge = metadata.curseforge.as_ref().map(|o| o.is_some()).unwrap_or(false);
                            let has_modrinth = metadata.modrinth.as_ref().map(|o| o.is_some()).unwrap_or(false);
                            let can_resolve = has_curseforge || has_modrinth;

                            // Build hashes from metadata
                            let sha512 = hex::encode(&metadata.sha_512);
                            let sha1 = hex::encode(&metadata.sha_1);
                            let murmur2 = metadata.murmur_2 as u32;

                            let hashes = FileHashes {
                                sha512,
                                sha1,
                                murmur2,
                            };

                            if can_resolve {
                                // Add as Platform entry for efficient hash-based resolution
                                // BUT keep in filter so it's also bundled in overrides as fallback
                                // This ensures the pack works even if platform resolution fails
                                // (e.g., file removed from platform, API issues, etc.)
                                pack_files.push(PackFile::Platform(PlatformFile { hashes }));
                            }
                            // Always keep files in overrides - they serve as fallback if
                            // platform resolution fails at import time
                        }
                    }
                }
            }

            t_calc_size.start_opaque();

            let mut file_count = 0;
            let mut total_bytes = 0u64;
            super::zip_excluding(
                ZipMode::<File, ()>::Count(&mut file_count, &mut total_bytes),
                &basepath,
                "overrides",
                &filter,
            )?;

            t_calc_size.complete_opaque();
            t_create_bundle.update_items(0, file_count);

            let instance_name = config.name.clone();

            // Build modloader dependencies
            let modloaders: Vec<ModloaderDependency> = version
                .modloaders
                .iter()
                .map(|ml| ModloaderDependency {
                    type_: match ml.type_ {
                        ModLoaderType::Forge => ModloaderType::Forge,
                        ModLoaderType::Neoforge => ModloaderType::Neoforge,
                        ModLoaderType::Fabric => ModloaderType::Fabric,
                        ModLoaderType::Quilt => ModloaderType::Quilt,
                    },
                    version: ml.version.clone(),
                    primary: true, // First modloader is primary
                })
                .collect();

            // Build source reference from modpack info if available
            let source = modpack_info.map(|info| match info.modpack {
                Modpack::Curseforge(cf) => ModpackSource::Curseforge {
                    project_id: cf.project_id,
                    file_id: cf.file_id,
                    name: None,
                    url: None,
                },
                Modpack::Modrinth(mr) => ModpackSource::Modrinth {
                    project_id: mr.project_id,
                    version_id: mr.version_id,
                    name: None,
                    url: None,
                },
            });

            // Prepare icon path
            let icon_archive_path = icon_data.as_ref().map(|(_, ext)| format!(".gdl/icon.{}", ext));

            let manifest = Manifest {
                format_version: 1,
                name: instance_name,
                version: None,
                summary: None,
                author: None,
                created_at: Utc::now(),
                icon: icon_archive_path.clone(),
                dependencies: GameDependencies {
                    minecraft: version.release.clone(),
                    modloaders,
                },
                entries: pack_files,
                overrides: "overrides".to_string(),
                server_overrides: None,
                client_overrides: None,
                source,
            };

            let tmpfile = app
                .settings_manager()
                .runtime_path
                .get_temp()
                .maketmpfile()
                .await?;

            let send_path = tmpfile.to_path_buf();
            let (notify_tx, mut notify_rx) = watch::channel::<(u64, u64)>((0, 0));

            let ziptask = tokio::task::spawn_blocking(move || {
                let mut zip = zip::ZipWriter::new(File::create(&send_path)?);
                let options = zip::write::FileOptions::<()>::default();

                // Write GDLPack manifest
                zip.start_file("gdlpack.json", options)?;
                zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;

                // Write icon if present
                if let (Some((icon_bytes, _)), Some(icon_path)) = (&icon_data, &icon_archive_path) {
                    zip.start_file(icon_path, options)?;
                    zip.write_all(icon_bytes)?;
                }

                // Write overrides
                super::zip_excluding(
                    ZipMode::Create(&mut zip, options, notify_tx),
                    &basepath,
                    "overrides",
                    &filter,
                )?;

                zip.finish()?;
                trace!("finished writing `{}`", send_path.to_string_lossy());
                Ok::<_, anyhow::Error>(())
            });

            tokio::select! {
                r = ziptask => r??,
                _ = async {
                    let mut counter = 0u32;

                    loop {
                        if notify_rx.changed().await.is_ok() {
                            let (_, count) = *notify_rx.borrow();
                            counter += count as u32;
                            t_create_bundle.update_items(counter, file_count);
                        } else {
                            futures::future::pending().await
                        }
                    }
                } => {},
            }

            tmpfile.try_rename_or_move(save_path).await?;

            t_create_bundle.complete_items();

            Ok(())
        }
        .await;

        if let Err(e) = try_result {
            vtask.fail(e).await;
        }
    });

    Ok(vtask_id)
}

#[cfg(test)]
mod test {
    use std::{
        collections::{HashMap, HashSet},
        fs::File,
        io::Read,
        sync::Arc,
    };

    use carbon_platforms::gdlauncher::manifest::schema::v1::Manifest;
    use flowtest::flowtest;
    use tracing_test::traced_test;
    use zip::ZipArchive;

    use crate::{
        domain::instance::{info, ExportEntry, ExportTarget, InstanceId},
        managers::instance::InstanceVersionSource,
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
                    String::from("test-gdlpack"),
                    false,
                    InstanceVersionSource::Version(info::GameVersion::Standard(
                        info::StandardVersion {
                            release: String::from("1.20.1"),
                            modloaders: HashSet::from([info::ModLoader {
                                type_: info::ModLoaderType::Forge,
                                version: String::from("47.2.0"),
                            }]),
                        },
                    )),
                    String::new(),
                )
                .await?;

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
                ExportTarget::Gdlauncher,
                target_file.clone(),
                self_contained_addons_bundling,
                export_entry,
                "1.0.0".to_string(),
            )
            .await?;

        app.task_manager().wait_with_log(task).await?;

        Ok(())
    }

    async fn check_export(
        app: &Arc<crate::TestEnv>,
        filename: &str,
        check: impl FnOnce(Manifest, ZipArchive<File>) -> anyhow::Result<()> + Send + 'static,
    ) -> anyhow::Result<()> {
        let target_file = app
            .settings_manager()
            .runtime_path
            .get_root()
            .to_path()
            .join(filename);

        tokio::task::spawn_blocking(|| {
            let mut zip = ZipArchive::new(File::open(target_file)?)?;

            let mut file = zip.by_name("gdlpack.json")?;
            let mut manifest_str = String::new();
            file.read_to_string(&mut manifest_str)?;
            drop(file);

            let manifest: Manifest = serde_json::from_str(&manifest_str)?;
            check(manifest, zip)
        })
        .await?
    }

    #[traced_test]
    #[test]
    #[flowtest(_setup: (rt, app, instance_id))]
    fn export_gdlpack_basic() -> anyhow::Result<()> {
        rt.block_on(async {
            run_export(
                &app,
                instance_id,
                "test.gdlpack",
                ExportEntry(HashMap::new()),
                true,
            )
            .await?;

            check_export(&app, "test.gdlpack", |manifest, _zip| {
                assert_eq!(manifest.format_version, 1);
                assert_eq!(manifest.name, "test-gdlpack");
                assert_eq!(manifest.dependencies.minecraft, "1.20.1");
                assert_eq!(manifest.dependencies.modloaders.len(), 1);
                assert_eq!(manifest.dependencies.modloaders[0].version, "47.2.0");
                assert_eq!(manifest.overrides, "overrides");
                Ok(())
            })
            .await?;

            Ok(())
        })
    }
}
