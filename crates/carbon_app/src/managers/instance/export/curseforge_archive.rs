use std::{collections::HashMap, fs::File, io::Write, path::PathBuf, sync::Arc};

use anyhow::anyhow;
use tracing::trace;

use crate::{
    api::translation::Translation,
    domain::{
        instance::{info::GameVersion, ExportEntry, InstanceId},
        modplatforms::curseforge::manifest::{
            Manifest, ManifestFileReference, Minecraft, ModLoaders,
        },
        vtask::VisualTaskId,
    },
    managers::{
        instance::{InstanceType, InvalidInstanceIdError},
        modplatforms::curseforge::convert_standard_version_to_cf_version,
        vtask::VisualTask,
        AppInner,
    },
};

use crate::db::{mod_file_cache as fcdb, mod_metadata as metadb};

pub async fn export_curseforge(
    app: Arc<AppInner>,
    instance_id: InstanceId,
    save_path: PathBuf,
    link_mods: bool,
    mut filter: ExportEntry,
) -> anyhow::Result<VisualTaskId> {
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
            let mut mods = Vec::new();

            if link_mods {
                let mods_filter = filter.0.get_mut("mods");
                if let Some(mods_filter) = mods_filter {
                    let t_scan = vtask.subtask(Translation::InstanceExportScanningMods);
                    let t_cache_mods = vtask.subtask(Translation::InstanceExportCacheMods);
                    t_scan.start_opaque();

                    if mods_filter.is_none() {
                        let mut modsdir_entries = HashMap::new();

                        let mut dir = tokio::fs::read_dir(basepath.join("mods")).await?;
                        while let Some(next) = dir.next_entry().await? {
                            let name = next.file_name();
                            let Some(name) = name.to_str() else { continue };
                            modsdir_entries.insert(name.to_string(), None);
                        }

                        *mods_filter = Some(ExportEntry(modsdir_entries));
                    }

                    let mods_filter = mods_filter.as_mut().map(|v| &mut v.0).unwrap();

                    t_cache_mods.start_opaque();
                    app.meta_cache_manager()
                        .override_caching_and_wait(instance_id, true, false)
                        .await?;
                    t_cache_mods.complete_opaque();

                    let mods2 = app
                        .prisma_client
                        .mod_file_cache()
                        .find_many(vec![fcdb::instance_id::equals(*instance_id)])
                        .with(fcdb::metadata::fetch().with(metadb::curseforge::fetch()))
                        .exec()
                        .await?
                        .into_iter()
                        .filter_map(|m| {
                            let Some(metadata) = m.metadata else {
                                return None;
                            };

                            let Some(Some(curseforge)) = metadata.curseforge else {
                                return None;
                            };

                            match mods_filter.remove(&m.filename) {
                                Some(_) => Some((curseforge.project_id, curseforge.file_id)),
                                None => None,
                            }
                        });

                    mods.extend(mods2);
                    t_scan.complete_opaque();
                }
            }

            let t_create_bundle = vtask.subtask(Translation::InstanceExportCreatingBundle);
            t_create_bundle.start_opaque(); // TODO: track the total number of overrides and use update_items

            let manifest = Manifest {
                minecraft: convert_standard_version_to_cf_version(version.clone())?,
                manifest_type: String::from("minecraftModpack"),
                manifest_version: 1,
                name: config.name,
                version: None,
                author: String::new(),
                overrides: String::from("overrides"),
                files: mods
                    .iter()
                    .map(|(project_id, file_id)| ManifestFileReference {
                        project_id: *project_id,
                        file_id: *file_id,
                        required: true,
                    })
                    .collect(),
            };

            let tmpfile = app
                .settings_manager()
                .runtime_path
                .get_temp()
                .maketmpfile()
                .await?;

            let send_path = tmpfile.to_path_buf();
            tokio::task::spawn_blocking(move || {
                let mut zip = zip::ZipWriter::new(File::create(&send_path)?);
                let options = zip::write::FileOptions::default();
                zip.start_file("manifest.json", options)?;
                zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;

                super::zip_excluding(&mut zip, options, &basepath, "overrides", &filter)?;

                zip.finish()?;
                trace!("finished writing `{}`", send_path.to_string_lossy());
                Ok::<_, anyhow::Error>(())
            })
            .await??;

            trace!(
                "renaming export from `{}` to `{}`",
                tmpfile.to_string_lossy(),
                save_path.to_string_lossy()
            );
            tmpfile.try_rename_or_move(save_path).await?;

            t_create_bundle.complete_opaque();

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

    use flowtest::flowtest;
    use tracing_test::traced_test;
    use zip::ZipArchive;

    use crate::{
        domain::instance::{info, ExportEntry, InstanceId},
        managers::instance::{export::ExportTarget, InstanceVersionSource},
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
                .install_curseforge_mod(instance_id, 247560, 4024011, false, None)
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
        link_mods: bool,
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
                ExportTarget::Curseforge,
                target_file.clone(),
                link_mods,
                export_entry,
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

            let mut file = zip.by_name("manifest.json")?;
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
                true,
            )
            .await?;

            check_export(&app, "folder_linked.zip", |manifest, mut zip| {
                crate::assert_eq_display!(
                    manifest,
                    r#"{
  "minecraft": {
    "version": "1.16.5",
    "modLoaders": [
      {
        "id": "forge-36.2.34",
        "primary": true
      }
    ]
  },
  "manifestType": "minecraftModpack",
  "manifestVersion": 1,
  "name": "test",
  "version": null,
  "author": "",
  "overrides": "overrides",
  "files": [
    {
      "projectID": 247560,
      "fileID": 4024011,
      "required": true
    }
  ]
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
                false,
            )
            .await?;

            check_export(&app, "folder_unlinked.zip", |manifest, mut zip| {
                crate::assert_eq_display!(
                    manifest,
                    r#"{
  "minecraft": {
    "version": "1.16.5",
    "modLoaders": [
      {
        "id": "forge-36.2.34",
        "primary": true
      }
    ]
  },
  "manifestType": "minecraftModpack",
  "manifestVersion": 1,
  "name": "test",
  "version": null,
  "author": "",
  "overrides": "overrides",
  "files": []
}"#
                );

                assert!(zip.by_name("overrides/mods/byg-1.3.6.jar").is_ok());
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
                true,
            )
            .await?;

            check_export(&app, "nofolder_linked.zip", |manifest, mut zip| {
                crate::assert_eq_display!(
                    manifest,
                    r#"{
  "minecraft": {
    "version": "1.16.5",
    "modLoaders": [
      {
        "id": "forge-36.2.34",
        "primary": true
      }
    ]
  },
  "manifestType": "minecraftModpack",
  "manifestVersion": 1,
  "name": "test",
  "version": null,
  "author": "",
  "overrides": "overrides",
  "files": []
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
                false,
            )
            .await?;

            check_export(&app, "nofolder_unlinked.zip", |manifest, mut zip| {
                crate::assert_eq_display!(
                    manifest,
                    r#"{
  "minecraft": {
    "version": "1.16.5",
    "modLoaders": [
      {
        "id": "forge-36.2.34",
        "primary": true
      }
    ]
  },
  "manifestType": "minecraftModpack",
  "manifestVersion": 1,
  "name": "test",
  "version": null,
  "author": "",
  "overrides": "overrides",
  "files": []
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
                false,
            )
            .await?;

            check_export(&app, "fakefolder_linked.zip", |manifest, mut zip| {
                crate::assert_eq_display!(
                    manifest,
                    r#"{
  "minecraft": {
    "version": "1.16.5",
    "modLoaders": [
      {
        "id": "forge-36.2.34",
        "primary": true
      }
    ]
  },
  "manifestType": "minecraftModpack",
  "manifestVersion": 1,
  "name": "test",
  "version": null,
  "author": "",
  "overrides": "overrides",
  "files": []
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
