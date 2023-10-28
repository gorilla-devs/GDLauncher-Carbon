use std::{collections::HashMap, fs::File, io::Write, path::PathBuf, sync::Arc};

use anyhow::anyhow;

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

                    app.meta_cache_manager()
                        .override_caching_and_wait(instance_id, true, false)
                        .await?;

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
                minecraft: Minecraft {
                    version: version.release,
                    mod_loaders: version
                        .modloaders
                        .into_iter()
                        .enumerate()
                        .map(|(i, loader)| ModLoaders {
                            id: format!("{}-{}", loader.type_.to_string(), loader.version),
                            primary: i == 0,
                        })
                        .collect(),
                },
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
                let mut zip = zip::ZipWriter::new(File::create(send_path)?);
                let options = zip::write::FileOptions::default();
                zip.start_file("manifest.json", options)?;
                zip.write(&serde_json::to_vec_pretty(&manifest)?)?;

                super::zip_excluding(&mut zip, options, &basepath, &filter)?;

                zip.finish()?;
                Ok::<_, anyhow::Error>(())
            })
            .await??;

            tmpfile.rename(save_path).await?;

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
    };

    use zip::ZipArchive;

    use crate::{
        domain::instance::{info, ExportEntry},
        managers::instance::{export::ExportTarget, InstanceVersionSource},
    };

    #[tokio::test]
    async fn manifest() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        let default_group_id = app.instance_manager().get_default_group().await?;
        let instance_id = app
            .instance_manager()
            .create_instance(
                default_group_id,
                String::from("test"),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::from([info::ModLoader {
                            type_: info::ModLoaderType::Forge,
                            version: String::from("10.13.4.1614"),
                        }]),
                    },
                )),
                String::new(),
            )
            .await?;

        let target_file = app
            .settings_manager()
            .runtime_path
            .get_root()
            .to_path()
            .join("export-test.zip");

        let task = app
            .instance_manager()
            .export_manager()
            .export_instance(
                instance_id,
                ExportTarget::Curseforge,
                target_file.clone(),
                false,
                ExportEntry(HashMap::new()),
            )
            .await?;

        app.task_manager().wait_with_log(task).await?;

        tokio::task::spawn_blocking(|| {
            let mut zip = ZipArchive::new(File::open(target_file)?)?;
            let mut file = zip.by_name("manifest.json")?;
            let mut s = String::new();
            file.read_to_string(&mut s)?;

            let expected = r#"{
  "minecraft": {
    "version": "1.7.10",
    "modLoaders": [
      {
        "id": "forge-10.13.4.1614",
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
}"#;

            assert_eq!(s, expected);
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        // todo check
        Ok(())
    }

    #[tokio::test]
    async fn link_mods() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

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
            .install_curseforge_mod(instance_id, 247560, 4024011)
            .await?;

        app.task_manager().wait_with_log(task).await?;

        let target_file = app
            .settings_manager()
            .runtime_path
            .get_root()
            .to_path()
            .join("export-test.zip");

        // mods folder exported, mods linked
        let export_entry = ExportEntry(HashMap::from([(String::from("mods"), None)]));

        let task = app
            .instance_manager()
            .export_manager()
            .export_instance(
                instance_id,
                ExportTarget::Curseforge,
                target_file.clone(),
                true,
                export_entry,
            )
            .await?;

        app.task_manager().wait_with_log(task).await?;

        let target_file2 = target_file.clone();
        tokio::task::spawn_blocking(|| {
            let mut zip = ZipArchive::new(File::open(target_file2)?)?;
            assert!(zip.by_name("mods").is_err());

            let mut file = zip.by_name("manifest.json")?;
            let mut s = String::new();
            file.read_to_string(&mut s)?;

            let expected = r#"{
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
}"#;

            assert_eq!(s, expected);
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        // mods exported, mods not linked
        let export_entry = ExportEntry(HashMap::from([(String::from("mods"), None)]));

        let task = app
            .instance_manager()
            .export_manager()
            .export_instance(
                instance_id,
                ExportTarget::Curseforge,
                target_file.clone(),
                false,
                export_entry,
            )
            .await?;

        app.task_manager().wait_with_log(task).await?;

        let target_file2 = target_file.clone();
        tokio::task::spawn_blocking(|| {
            let mut zip = ZipArchive::new(File::open(target_file2)?)?;
            assert!(zip.by_name("mods/byg-1.3.6.jar").is_ok());

            let mut file = zip.by_name("manifest.json")?;
            let mut s = String::new();
            file.read_to_string(&mut s)?;

            let expected = r#"{
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
}"#;

            assert_eq!(s, expected);
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        // mods not exported, mods linked
        let export_entry = ExportEntry(HashMap::new());

        let task = app
            .instance_manager()
            .export_manager()
            .export_instance(
                instance_id,
                ExportTarget::Curseforge,
                target_file.clone(),
                true,
                export_entry,
            )
            .await?;

        app.task_manager().wait_with_log(task).await?;

        let target_file2 = target_file.clone();
        tokio::task::spawn_blocking(|| {
            let mut zip = ZipArchive::new(File::open(target_file2)?)?;
            assert!(zip.by_name("mods/byg-1.3.6.jar").is_err());

            let mut file = zip.by_name("manifest.json")?;
            let mut s = String::new();
            file.read_to_string(&mut s)?;

            let expected = r#"{
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
}"#;

            assert_eq!(s, expected);
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        // one (fake) mod exported, mods linked
        let export_entry = ExportEntry(HashMap::from([(
            String::from("mods"),
            Some(ExportEntry(HashMap::from([(
                String::from("fake-mod.jar"),
                None,
            )]))),
        )]));

        let task = app
            .instance_manager()
            .export_manager()
            .export_instance(
                instance_id,
                ExportTarget::Curseforge,
                target_file.clone(),
                true,
                export_entry,
            )
            .await?;

        app.task_manager().wait_with_log(task).await?;

        let target_file2 = target_file.clone();
        tokio::task::spawn_blocking(|| {
            let mut zip = ZipArchive::new(File::open(target_file2)?)?;
            assert!(zip.by_name("mods/byg-1.3.6.jar").is_err());

            let mut file = zip.by_name("manifest.json")?;
            let mut s = String::new();
            file.read_to_string(&mut s)?;

            let expected = r#"{
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
}"#;

            assert_eq!(s, expected);
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        // mods not exported, mods not linked
        let export_entry = ExportEntry(HashMap::new());

        let task = app
            .instance_manager()
            .export_manager()
            .export_instance(
                instance_id,
                ExportTarget::Curseforge,
                target_file.clone(),
                false,
                export_entry,
            )
            .await?;

        app.task_manager().wait_with_log(task).await?;

        let target_file2 = target_file.clone();
        tokio::task::spawn_blocking(|| {
            let mut zip = ZipArchive::new(File::open(target_file2)?)?;
            assert!(zip.by_name("mods/byg-1.3.6.jar").is_err());

            let mut file = zip.by_name("manifest.json")?;
            let mut s = String::new();
            file.read_to_string(&mut s)?;

            let expected = r#"{
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
}"#;

            assert_eq!(s, expected);
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }
}
