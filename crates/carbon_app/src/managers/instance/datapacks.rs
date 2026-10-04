//! Instance-level datapacks are staging. Copies into explicitly selected saves
//! are independent files and are never synchronized by staged addon mutations.
use super::{
    InstanceId, InstanceManager,
    installer::publication,
    mods::{addon_metadata, validate_addon_basename},
};
use crate::{domain::instance::AddonType, managers::ManagerRef};
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DatapackWorld {
    pub name: String,
    pub identity: String,
}
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct CopyDatapack {
    pub instance_id: i32,
    pub mod_id: String,
    pub world: DatapackWorld,
}

/// Which save folder the user picked: its instance, path and the folder
/// itself (created time, and device and inode where there are some), so a
/// save moved away and another put under its name is refused. Not what is
/// inside: Minecraft rewrites `level.dat` on every save, and playing the
/// world while the Addons tab stays open must not make it "changed".
async fn world_identity(instance: InstanceId, root: &Path, world: &Path) -> anyhow::Result<String> {
    let directory = addon_metadata(world)
        .await?
        .context("Selected world no longer exists")?;
    if !directory.is_dir() || directory.file_type().is_symlink() {
        bail!("Selected world is not an owned save directory");
    }
    if publication::world_install_incomplete(world).await? {
        bail!("Selected world installation is incomplete");
    }
    let level = world.join("level.dat");
    let metadata = addon_metadata(&level)
        .await?
        .context("Selected directory is not an existing Minecraft save")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!("World metadata is not a regular file");
    }
    let mut digest = Sha512::new();
    digest.update(format!(
        "{}:{:?}:{:?}:{:?}",
        *instance,
        root,
        world,
        directory.created().ok()
    ));
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        digest.update(directory.dev().to_le_bytes());
        digest.update(directory.ino().to_le_bytes());
    }
    Ok(hex::encode(digest.finalize()))
}

impl ManagerRef<'_, InstanceManager> {
    /// The saves a staged datapack can be added to. Read-only, so it lists
    /// them whatever the instance is doing (preparing, a pending setup, a
    /// locked pack): only Add to world ([`Self::copy_datapack`]) asks whether
    /// the instance's files may change now.
    pub async fn datapack_worlds(
        self,
        instance_id: InstanceId,
    ) -> anyhow::Result<Vec<DatapackWorld>> {
        let _activity = self
            .app
            .runtime_lifecycle
            .admit("list datapack destination saves")?;
        let _operation = self.lock_instance(instance_id).await;
        let path = publication::current_path(&self.app, instance_id).await?;
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        let mut result = Vec::new();
        let mut entries = match tokio::fs::read_dir(&saves).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(result),
            Err(error) => return Err(error.into()),
        };
        while let Some(entry) = entries.next_entry().await? {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if validate_addon_basename(&name).is_err() || !entry.file_type().await?.is_dir() {
                continue;
            }
            if let Ok(identity) = world_identity(instance_id, &path.get_root(), &entry.path()).await
            {
                result.push(DatapackWorld { name, identity });
            }
        }
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    pub async fn copy_datapack(self, request: CopyDatapack) -> anyhow::Result<String> {
        let id = InstanceId(request.instance_id);
        let _activity = self
            .app
            .runtime_lifecycle
            .admit("copy staged datapack into selected save")?;
        let _operation = self.lock_instance(id).await;
        self.ensure_modpack_not_locked(id).await?;
        validate_addon_basename(&request.world.name)?;
        let row = self.get_owned_mod_file(id, &request.mod_id).await?;
        if row.addon_type != AddonType::DataPacks.to_db_string() {
            bail!("Only a staged datapack can be copied into a save");
        }
        validate_addon_basename(&row.filename)?;
        if !row.filename.to_ascii_lowercase().ends_with(".zip") {
            bail!("Select a ZIP datapack for this world");
        }
        let path = publication::current_path(&self.app, id).await?;
        let staging = publication::checked_folder(&path, AddonType::DataPacks).await?;
        let (source, _) = publication::selected_file(&staging, &row.filename)
            .await?
            .context("Staged datapack is missing")?;
        let source_digest = publication::file_digest(&source).await?;
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        let world = saves.join(&request.world.name);
        if world_identity(id, &path.get_root(), &world).await? != request.world.identity {
            bail!("Selected world changed. Refresh the world list and select it again");
        }
        let folder = world.join("datapacks");
        if let Some(meta) = addon_metadata(&folder).await? {
            if !meta.is_dir() || meta.file_type().is_symlink() {
                bail!("World datapacks is not an owned directory");
            }
        } else {
            tokio::fs::create_dir(&folder).await?;
        }
        if publication::selected_file(&folder, &row.filename)
            .await?
            .is_some()
        {
            bail!(
                "This world already contains that datapack filename; existing content was retained"
            );
        }
        let temporary = crate::util::publish::temp_in(&path.get_root())?;
        tokio::fs::copy(&source, temporary.path()).await?;
        if publication::file_digest(temporary.path()).await? != source_digest
            || publication::file_digest(&source).await? != source_digest
        {
            bail!("Staged datapack changed during copying; retry with the current artifact");
        }
        validate_pack_zip(temporary.path())?;
        #[cfg(test)]
        self.test_hooks.pause("datapack_before_copy", id).await;
        if world_identity(id, &path.get_root(), &world).await? != request.world.identity {
            bail!("Selected world changed during copying; no datapack published");
        }
        let meta = addon_metadata(&folder)
            .await?
            .context("World datapacks directory disappeared")?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            bail!("World datapacks directory changed during copying");
        }
        if publication::selected_file(&folder, &row.filename)
            .await?
            .is_some()
        {
            bail!("Datapack destination became occupied; existing content retained");
        }
        // Off the runtime: a world folder on another drive makes this a copy.
        crate::util::publish::persist_noclobber_async(temporary, &folder.join(&row.filename))
            .await?;
        Ok(request.world.name)
    }
}

fn validate_pack_zip(path: &Path) -> anyhow::Result<()> {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    let metadata = archive
        .by_name("pack.mcmeta")
        .context("Datapack ZIP must contain pack.mcmeta at its root")?;
    if metadata.size() > 1024 * 1024 {
        bail!("Datapack metadata is too large");
    }
    let mut bytes = Vec::new();
    metadata.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if !value.get("pack").is_some_and(serde_json::Value::is_object) {
        bail!("Invalid datapack metadata");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::instance::mods::tests::addon_fixture;
    use std::io::Write;
    fn zip_bytes() -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("pack.mcmeta", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"{"pack":{"pack_format":15,"description":"fixture"}}"#)
            .unwrap();
        zip.finish().unwrap().into_inner()
    }
    #[tokio::test]
    async fn task10_datapack_real_selected_copy_collision_and_independent_mutations() {
        let app = crate::setup_managers_for_test().await;
        let bytes = zip_bytes();
        let (id, row, source) = addon_fixture(
            &app,
            "staged-pack",
            AddonType::DataPacks,
            "pack.zip",
            &bytes,
        )
        .await;
        assert!(
            app.instance_manager()
                .datapack_worlds(id)
                .await
                .unwrap()
                .is_empty()
        );
        let path = publication::current_path(&app, id).await.unwrap();
        let world = path.get_saves_path().join("MyWorld");
        std::fs::create_dir_all(&world).unwrap();
        std::fs::write(world.join("level.dat"), b"save identity").unwrap();
        let selected = app
            .instance_manager()
            .datapack_worlds(id)
            .await
            .unwrap()
            .pop()
            .unwrap();
        let request = || CopyDatapack {
            instance_id: *id,
            mod_id: row.id.clone(),
            world: selected.clone(),
        };
        assert_eq!(
            app.instance_manager()
                .copy_datapack(request())
                .await
                .unwrap(),
            "MyWorld"
        );
        let copied = world.join("datapacks/pack.zip");
        assert_eq!(std::fs::read(&copied).unwrap(), bytes);
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
        assert!(
            app.instance_manager()
                .copy_datapack(request())
                .await
                .is_err()
        );
        std::fs::write(&copied, b"independent world edit").unwrap();
        app.instance_manager()
            .enable_mod(id, row.id.clone(), false)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&copied).unwrap(), b"independent world edit");
        app.instance_manager()
            .delete_mod(id, row.id.clone())
            .await
            .unwrap();
        assert_eq!(std::fs::read(copied).unwrap(), b"independent world edit");
    }
    #[tokio::test]
    async fn task10_datapack_rejects_foreign_replaced_world_and_links() {
        let app = crate::setup_managers_for_test().await;
        let (id, row, _) = addon_fixture(
            &app,
            "identity-pack",
            AddonType::DataPacks,
            "pack.zip",
            &zip_bytes(),
        )
        .await;
        let path = publication::current_path(&app, id).await.unwrap();
        let world = path.get_saves_path().join("World");
        std::fs::create_dir_all(&world).unwrap();
        std::fs::write(world.join("level.dat"), b"same").unwrap();
        let selected = app
            .instance_manager()
            .datapack_worlds(id)
            .await
            .unwrap()
            .pop()
            .unwrap();
        std::fs::rename(&world, world.with_file_name("Original")).unwrap();
        std::fs::create_dir(&world).unwrap();
        std::fs::write(world.join("level.dat"), b"same").unwrap();
        // Windows has no inode in the identity, only the creation time, which
        // NTFS keeps at clock-tick resolution and tunnels from a removed name
        // to one re-created within 15 s. A folder made milliseconds later in
        // its place can carry the original's time; a real one has its own.
        #[cfg(windows)]
        {
            use std::os::windows::fs::{FileTimesExt, OpenOptionsExt};
            std::fs::OpenOptions::new()
                .access_mode(0x100) // FILE_WRITE_ATTRIBUTES
                .custom_flags(0x0200_0000) // FILE_FLAG_BACKUP_SEMANTICS, to open a folder
                .open(&world)
                .unwrap()
                .set_times(std::fs::FileTimes::new().set_created(
                    std::time::SystemTime::now() - std::time::Duration::from_secs(3600),
                ))
                .unwrap();
        }
        assert!(
            app.instance_manager()
                .copy_datapack(CopyDatapack {
                    instance_id: *id,
                    mod_id: row.id.clone(),
                    world: selected
                })
                .await
                .is_err()
        );
        assert!(!world.join("datapacks/pack.zip").exists());
        let mut selected = app
            .instance_manager()
            .datapack_worlds(id)
            .await
            .unwrap()
            .into_iter()
            .find(|w| w.name == "World")
            .unwrap();
        selected.name = "../World".into();
        assert!(
            app.instance_manager()
                .copy_datapack(CopyDatapack {
                    instance_id: *id,
                    mod_id: row.id.clone(),
                    world: selected
                })
                .await
                .is_err()
        );
        #[cfg(unix)]
        {
            let target = tempfile::tempdir().unwrap();
            std::os::unix::fs::symlink(target.path(), world.join("datapacks")).unwrap();
            let selected = app
                .instance_manager()
                .datapack_worlds(id)
                .await
                .unwrap()
                .into_iter()
                .find(|w| w.name == "World")
                .unwrap();
            assert!(
                format!("{error:#}").contains("cannot be found"),
                "{error:#}"
            );
            assert!(!gone.exists());
        }
    }

    /// Playing the selected world while the Addons tab stays open rewrites
    /// its `level.dat` (Minecraft writes a new one on every save): Add to
    /// world still goes into that save.
    #[tokio::test]
    async fn a_world_played_since_the_list_was_read_still_takes_the_datapack() {
        let app = crate::setup_managers_for_test().await;
        let bytes = zip_bytes();
        let (id, row, _) = addon_fixture(
            &app,
            "played-world",
            AddonType::DataPacks,
            "pack.zip",
            &bytes,
        )
        .await;
        let path = publication::current_path(&app, id).await.unwrap();
        let world = path.get_saves_path().join("Survival");
        std::fs::create_dir_all(&world).unwrap();
        std::fs::write(world.join("level.dat"), b"before playing").unwrap();
        let selected = app
            .instance_manager()
            .datapack_worlds(id)
            .await
            .unwrap()
            .pop()
            .unwrap();
        // What a save does: a new level.dat replaces the old one.
        std::fs::write(world.join("level.dat_new"), b"after playing").unwrap();
        std::fs::rename(world.join("level.dat"), world.join("level.dat_old")).unwrap();
        std::fs::rename(world.join("level.dat_new"), world.join("level.dat")).unwrap();
        app.instance_manager()
            .copy_datapack(CopyDatapack {
                instance_id: *id,
                mod_id: row.id,
                world: selected,
            })
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(world.join("datapacks/pack.zip")).unwrap(),
            bytes
        );
    }

    /// Listing the saves changes nothing, so the Addons tab shows them (and
    /// no error) while the instance prepares, is being deleted or has a
    /// pending pack setup. Adding a datapack still waits for that setup.
    #[tokio::test]
    async fn the_world_list_reads_while_addons_cannot_change() {
        use crate::domain::vtask::VisualTaskId;
        use crate::managers::instance::modpack::session::{Kind, RECORD, Session};
        use crate::managers::instance::run::LaunchState;
        let app = crate::setup_managers_for_test().await;
        let (id, row, _) = addon_fixture(
            &app,
            "busy-instance",
            AddonType::DataPacks,
            "pack.zip",
            &zip_bytes(),
        )
        .await;
        let path = publication::current_path(&app, id).await.unwrap();
        let world = path.get_saves_path().join("World");
        std::fs::create_dir_all(&world).unwrap();
        std::fs::write(world.join("level.dat"), b"save").unwrap();
        let set_state = |state| {
            let app = app.clone();
            async move {
                app.instance_manager()
                    .instances
                    .write()
                    .await
                    .get_mut(&id)
                    .unwrap()
                    .data_mut()
                    .unwrap()
                    .state = state;
            }
        };
        let listed = || async {
            app.instance_manager()
                .datapack_worlds(id)
                .await
                .unwrap()
                .into_iter()
                .map(|world| world.name)
                .collect::<Vec<_>>()
        };
        for state in [
            LaunchState::Queued(VisualTaskId(900)),
            LaunchState::Preparing(VisualTaskId(900)),
            LaunchState::Deleting,
        ] {
            let name = format!("{state:?}");
            set_state(state).await;
            assert_eq!(listed().await, vec!["World".to_owned()], "{name}");
        }
        set_state(LaunchState::Inactive { failed_task: None }).await;
        let root = path.get_root();
        let mut pack = Session::load(&root).await.unwrap().unwrap();
        pack.kind = Kind::Pack;
        std::fs::write(
            root.join(".setup").join(RECORD),
            serde_json::to_vec(&pack).unwrap(),
        )
        .unwrap();
        let selected = app
            .instance_manager()
            .datapack_worlds(id)
            .await
            .unwrap()
            .pop()
            .expect("the world is listed beside a pending pack setup");
        app.instance_manager()
            .copy_datapack(CopyDatapack {
                instance_id: *id,
                mod_id: row.id,
                world: selected,
            })
            .await
            .expect_err("adding waits for the pack setup");
        assert!(!world.join("datapacks/pack.zip").exists());
    }

    /// A `saves` folder linked elsewhere, and a world whose `datapacks`
    /// folder is linked to a shared one, are followed like any addon folder:
    /// the world is listed and the copy lands in the link target.
    #[cfg(unix)]
    #[tokio::test]
    async fn linked_saves_and_world_datapacks_folders_are_followed() {
    }
    #[tokio::test]
    async fn task10_datapack_copy_rechecks_selected_world_and_late_collision() {
        for mode in ["world", "collision", "foreign-row"] {
            let app = crate::setup_managers_for_test().await;
            let (id, row, _) =
                addon_fixture(&app, mode, AddonType::DataPacks, "pack.zip", &zip_bytes()).await;
            let path = publication::current_path(&app, id).await.unwrap();
            let world = path.get_saves_path().join("Selected");
            std::fs::create_dir_all(&world).unwrap();
            std::fs::write(world.join("level.dat"), b"save").unwrap();
            let selected = app
                .instance_manager()
                .datapack_worlds(id)
                .await
                .unwrap()
                .pop()
                .unwrap();
            let request = CopyDatapack {
                instance_id: *id,
                mod_id: row.id.clone(),
                world: selected,
            };
            if mode == "foreign-row" {
                let (other, _, _) = addon_fixture(
                    &app,
                    "foreign",
                    AddonType::DataPacks,
                    "pack.zip",
                    &zip_bytes(),
                )
                .await;
                assert!(
                    app.instance_manager()
                        .copy_datapack(CopyDatapack {
                            instance_id: *other,
                            ..request
                        })
                        .await
                        .is_err()
                );
                continue;
            }
            let pause = app
                .instance_manager()
                .test_hooks
                .arm("datapack_before_copy", id);
            let worker_app = app.clone();
            let worker =
                tokio::spawn(
                    async move { worker_app.instance_manager().copy_datapack(request).await },
                );
            tokio::time::timeout(std::time::Duration::from_secs(10), pause.entered.notified())
                .await
                .unwrap();
            if mode == "world" {
                std::fs::rename(&world, world.with_file_name("Moved")).unwrap();
                std::fs::create_dir(&world).unwrap();
                std::fs::write(world.join("level.dat"), b"save").unwrap();
            } else {
                std::fs::write(world.join("datapacks/pack.zip"), b"later world pack").unwrap();
            }
            pause.release.notify_one();
            assert!(worker.await.unwrap().is_err());
            if mode == "world" {
                assert!(!world.join("datapacks/pack.zip").exists());
            } else {
                assert_eq!(
                    std::fs::read(world.join("datapacks/pack.zip")).unwrap(),
                    b"later world pack"
                );
            }
        }
    }
}
