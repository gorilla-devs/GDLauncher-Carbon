//! Instance-level datapacks are staging. Copies into explicitly selected saves
//! are independent files and are never synchronized by staged addon mutations.
use super::{
    InstanceId, InstanceManager,
    installer::publication,
    mods::{InvalidInstanceModIdError, addon_metadata, validate_addon_basename},
    world_refusal::{PackFault, SaveFault, WorldRefusal},
};
use crate::{
    domain::instance::AddonType,
    managers::{
        ManagerRef,
        runtime_lifecycle::{ActivityPolicy, spawn_retained},
    },
};
use anyhow::{Context, bail};
use carbon_repos::repos::{
    mod_file_cache as mfcdb,
    world_datapack_cache::{self as wdb, WorldDatapackFullRow},
};
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
    /// Put the copy in place of what the world holds of this data pack
    /// (see [`AlreadyInWorld`]).
    #[serde(default)]
    pub replace: bool,
}

/// The refusal of a copy into a world that already has the data pack: a
/// file of its name, or a copy of the same project under any name. Nothing
/// changed; asked again with `replace`, the copy takes their place.
#[derive(Debug, thiserror::Error)]
#[error("This world already has this data pack: {}", .filenames.join(", "))]
pub struct AlreadyInWorld {
    pub filenames: Vec<String>,
}

impl crate::error::FeErrorCode for AlreadyInWorld {
    fn error_code(&self) -> &'static str {
        "ALREADY_IN_WORLD"
    }

    fn error_data(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "filenames": self.filenames }))
    }
}

/// Which save folder the user picked: its instance, path and the folder
/// itself (created time, and device and inode where there are some), so a
/// save moved away and another put under its name is refused. Not what is
/// inside: Minecraft rewrites `level.dat` on every save, and playing the
/// world while the Addons tab stays open must not make it "changed".
pub(super) async fn world_identity(
    instance: InstanceId,
    root: &Path,
    world: &Path,
) -> anyhow::Result<String> {
    let directory = addon_metadata(world)
        .await?
        .ok_or(WorldRefusal::WorldGone)?;
    if !directory.is_dir() || directory.file_type().is_symlink() {
        bail!(WorldRefusal::NotASave(SaveFault::NotAFolder));
    }
    if publication::world_install_incomplete(world).await? {
        bail!(WorldRefusal::NotASave(SaveFault::InstallIncomplete));
    }
    let level = world.join("level.dat");
    let metadata = addon_metadata(&level)
        .await?
        .ok_or(WorldRefusal::NotASave(SaveFault::NoLevelDat))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!(WorldRefusal::NotASave(SaveFault::LevelDatNotAFile));
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

    /// Copies an instance-level data pack into a world. Refused with
    /// [`AlreadyInWorld`] when the world already has it, unless `replace`
    /// asks to put the copy in their place: they are set aside, and removed
    /// as the "Deletion through Recycle Bin" setting asks once the copy is
    /// in place, or put back when it cannot be. Returns the world's name.
    pub async fn copy_datapack(self, request: CopyDatapack) -> anyhow::Result<String> {
        let activity = self.app.runtime_lifecycle.admit_owned(
            "copy staged datapack into selected save",
            ActivityPolicy::CoreWork,
        )?;
        // Its own task, so a request dropped midway (the window closed)
        // never stops a replace that set old packs aside; a quit waits.
        let app = self.app.clone();
        spawn_retained(activity, move |_| {
            Box::pin(async move { app.instance_manager().copy_datapack_owned(request).await })
        })
        .await
        .map_err(|error| anyhow::anyhow!("The data pack copy stopped: {error}"))?
    }

    /// [`Self::copy_datapack`]'s work, in the task that owns its activity.
    async fn copy_datapack_owned(self, request: CopyDatapack) -> anyhow::Result<String> {
        let id = InstanceId(request.instance_id);
        // The rows tell which files in the world are copies of this pack.
        self.scan_world_datapacks(id).await?;
        #[cfg(test)]
        self.test_hooks.pause("datapack_copy_scanned", id).await;
        {
            let _operation = self.lock_instance(id).await;
            self.copy_datapack_gated(id, &request).await?;
        }
        self.refresh_worlds(id, "a copy").await;
        Ok(request.world.name)
    }

    /// [`Self::copy_datapack`] under the instance gate.
    async fn copy_datapack_gated(
        self,
        id: InstanceId,
        request: &CopyDatapack,
    ) -> anyhow::Result<()> {
        self.ensure_world_datapacks_may_change(id).await?;
        validate_addon_basename(&request.world.name)?;
        let row = self.get_owned_mod_file(id, &request.mod_id).await?;
        if row.addon_type != AddonType::DataPacks.to_db_string() {
            bail!("Only a staged datapack can be copied into a save");
        }
        validate_addon_basename(&row.filename)?;
        if !row.filename.to_ascii_lowercase().ends_with(".zip") {
            bail!(WorldRefusal::NotAZip);
        }
        let path = publication::current_path(&self.app, id).await?;
        let staging = publication::checked_folder(&path, AddonType::DataPacks).await?;
        let (source, _) = publication::selected_file(&staging, &row.filename)
            .await?
            .context(WorldRefusal::InstancePackGone)?;
        let source_digest = publication::file_digest(&source).await?;
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        let world = saves.join(&request.world.name);
        if world_identity(id, &path.get_root(), &world).await? != request.world.identity {
            bail!(WorldRefusal::WorldChanged);
        }
        // The world's datapacks folder may be linked to a shared one, as an
        // addon folder may (`publication::followed_folder`).
        let folder = world.join("datapacks");
        if !publication::followed_folder(&folder).await? {
            tokio::fs::create_dir(&folder).await?;
        }
        let copies = self
            .copies_in_world(id, &request.mod_id, &request.world.name, &row.filename)
            .await?;
        // An entry under the name with no row of a file (a folder, a file
        // the scan does not list) is not a copy, so not replaced either.
        let named_copy = copies.iter().any(|copy| copy.filename == row.filename);
        if addon_metadata(&folder.join(format!("{}.disabled", row.filename)))
            .await?
            .is_some()
            || (!named_copy && addon_metadata(&folder.join(&row.filename)).await?.is_some())
        {
            bail!(WorldRefusal::NameTaken {
                filename: row.filename.clone(),
            });
        }
        if !copies.is_empty() && !request.replace {
            bail!(AlreadyInWorld {
                filenames: copies.into_iter().map(|copy| copy.filename).collect(),
            });
        }
        let mut replaced = Vec::new();
        for copy in &copies {
            if let Some(entry) = self
                .replaced_entry(id, &request.world.name, &copy.id)
                .await?
            {
                replaced.push(entry);
            }
        }
        let destination = folder.join(&row.filename);
        // Not std's copy: it always copies the mode and fails where the drive
        // refuses that, as one whose files all belong to another account
        // does. There the copy keeps the mode the drive gives it.
        let temporary = {
            let (source, root, destination) =
                (source.clone(), path.get_root(), destination.clone());
            tokio::task::spawn_blocking(move || -> std::io::Result<_> {
                let mut input = std::fs::File::open(&source)?;
                let mut temporary = crate::util::publish::temp_in(&root)?;
                std::io::copy(&mut input, temporary.as_file_mut())?;
                #[cfg(unix)]
                crate::util::publish::copy_mode(
                    temporary.as_file(),
                    input.metadata()?.permissions(),
                    &destination,
                )?;
                #[cfg(not(unix))]
                let _ = destination;
                Ok(temporary)
            })
            .await??
        };
        if publication::file_digest(temporary.path()).await? != source_digest
            || publication::file_digest(&source).await? != source_digest
        {
            bail!(WorldRefusal::InstancePackChanged);
        }
        self.publish_into_world_replacing(
            id,
            &path.get_root(),
            temporary,
            &world,
            &request.world.identity,
            &row.filename,
            replaced,
        )
        .await?;
        Ok(())
    }

    /// The world data packs of `world` that are copies of the instance-level
    /// data pack `mod_id`, named `filename`: the file of that name, and
    /// every file of the same Modrinth or CurseForge project. Sorted by name.
    async fn copies_in_world(
        self,
        id: InstanceId,
        mod_id: &str,
        world: &str,
        filename: &str,
    ) -> anyhow::Result<Vec<WorldDatapackFullRow>> {
        let pack = mfcdb::get_instance_mods_full(&self.app.db, *id)
            .await?
            .into_iter()
            .find(|row| row.id == mod_id)
            .ok_or_else(|| InvalidInstanceModIdError(id, mod_id.to_owned()))?;
        // A project no platform knows names no other copy.
        let same_project = |row: &WorldDatapackFullRow| {
            (pack.mr_project_id.is_some() && row.mr_project_id == pack.mr_project_id)
                || (pack.cf_project_id.is_some() && row.cf_project_id == pack.cf_project_id)
        };
        let mut copies: Vec<_> = wdb::get_world_datapacks_full(&self.app.db, *id)
            .await?
            .into_iter()
            .filter(|row| {
                row.world == world
                    && !row.is_folder
                    && (row.filename == filename || same_project(row))
            })
            .collect();
        copies.sort_by(|a, b| a.filename.cmp(&b.filename));
        Ok(copies)
    }

    /// Publishes `temporary`, a private file in the instance root, as
    /// `filename` in the `datapacks` folder of `world`, a save of `instance`
    /// whose identity the user selected. Refused, leaving the world as it is,
    /// when the file is no data pack, the save is no longer the selected one,
    /// its `datapacks` folder is gone or `filename` is taken there. Call with
    /// the instance gate held. Returns the published file.
    pub(super) async fn publish_into_world(
        self,
        instance: InstanceId,
        root: &Path,
        temporary: tempfile::NamedTempFile,
        world: &Path,
        identity: &str,
        filename: &str,
    ) -> anyhow::Result<PathBuf> {
        let destination = self
            .world_destination(instance, root, &temporary, world, identity, filename)
            .await?;
        // Off the runtime: a world folder on another drive makes this a copy.
        crate::util::publish::persist_noclobber_async(temporary, &destination).await?;
        Ok(destination)
    }

    /// The checks of [`Self::publish_into_world`], up to the path the file
    /// would be published at.
    pub(super) async fn world_destination(
        self,
        instance: InstanceId,
        root: &Path,
        temporary: &tempfile::NamedTempFile,
        world: &Path,
        identity: &str,
        filename: &str,
    ) -> anyhow::Result<PathBuf> {
        validate_pack_zip(temporary.path())?;
        #[cfg(test)]
        self.test_hooks
            .pause("datapack_before_copy", instance)
            .await;
        if world_identity(instance, root, world).await? != identity {
            bail!(WorldRefusal::WorldChangedDuringCopy);
        }
        let folder = world.join("datapacks");
        if !publication::followed_folder(&folder).await? {
            bail!(WorldRefusal::WorldChangedDuringCopy);
        }
        if publication::selected_file(&folder, filename)
            .await?
            .is_some()
        {
            bail!(WorldRefusal::NameTaken {
                filename: filename.to_owned(),
            });
        }
        Ok(folder.join(filename))
    }
}

pub(super) fn validate_pack_zip(path: &Path) -> anyhow::Result<()> {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    let metadata = archive
        .by_name("pack.mcmeta")
        .map_err(|_| WorldRefusal::InvalidPack(PackFault::NoPackMcmeta))?;
    if metadata.size() > 1024 * 1024 {
        bail!(WorldRefusal::InvalidPack(PackFault::MetadataTooLarge));
    }
    let mut bytes = Vec::new();
    metadata.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    // Read the file as Minecraft does: UTF-8 with malformed bytes replaced
    // (a description saved in a Windows code page still loads), after one
    // leading byte order mark, which Notepad and PowerShell 5.1 write.
    let text = String::from_utf8_lossy(&bytes);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|_| WorldRefusal::InvalidPack(PackFault::MetadataNotJson))?;
    if !value.get("pack").is_some_and(serde_json::Value::is_object) {
        bail!(WorldRefusal::InvalidPack(PackFault::NoPackSection));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::instance::mods::tests::addon_fixture;
    use std::io::Write;
    fn zip_with_metadata(metadata: &[u8]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("pack.mcmeta", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(metadata).unwrap();
        zip.finish().unwrap().into_inner()
    }
    fn zip_bytes() -> Vec<u8> {
        zip_with_metadata(br#"{"pack":{"pack_format":15,"description":"fixture"}}"#)
    }

    /// Notepad and PowerShell 5.1 start a "UTF-8" file with a byte order
    /// mark, and older Notepad saves in the Windows code page (`\xA7` is §,
    /// `\xFC` is ü there). Minecraft loads both, so Add to world takes them.
    #[tokio::test]
    async fn a_datapack_whose_pack_mcmeta_was_saved_by_windows_tools_is_added_to_the_world() {
        let app = crate::setup_managers_for_test().await;
        let with_bom = [
            b"\xEF\xBB\xBF".as_slice(),
            br#"{"pack":{"pack_format":15,"description":"fixture"}}"#,
        ]
        .concat();
        let code_page: &[u8] =
            b"{\"pack\":{\"pack_format\":15,\"description\":\"\xA7aPack f\xFCr\"}}";
        for (name, metadata) in [("bom-pack", with_bom.as_slice()), ("ansi-pack", code_page)] {
            let bytes = zip_with_metadata(metadata);
            let (id, row, _) =
                addon_fixture(&app, name, AddonType::DataPacks, "pack.zip", &bytes).await;
            let path = publication::current_path(&app, id).await.unwrap();
            let world = path.get_saves_path().join("World");
            std::fs::create_dir_all(&world).unwrap();
            std::fs::write(world.join("level.dat"), b"save").unwrap();
            let selected = app
                .instance_manager()
                .datapack_worlds(id)
                .await
                .unwrap()
                .pop()
                .unwrap();
            assert_eq!(
                app.instance_manager()
                    .copy_datapack(CopyDatapack {
                        instance_id: *id,
                        mod_id: row.id,
                        world: selected,
                        replace: false,
                    })
                    .await
                    .unwrap(),
                "World",
                "{name}"
            );
            assert_eq!(
                std::fs::read(world.join("datapacks/pack.zip")).unwrap(),
                bytes,
                "{name}"
            );
        }
    }

    #[test]
    fn a_pack_mcmeta_that_is_not_json_is_refused_with_a_plain_reason() {
        let validate = |metadata: &[u8]| {
            let mut file = tempfile::NamedTempFile::new().unwrap();
            file.write_all(&zip_with_metadata(metadata)).unwrap();
            validate_pack_zip(file.path())
        };
        let error = validate(b"pack_format=15").unwrap_err();
        assert_eq!(error.to_string(), "Datapack pack.mcmeta is not valid JSON");
        // Minecraft skips one byte order mark, not two.
        let two_marks = [
            b"\xEF\xBB\xBF\xEF\xBB\xBF".as_slice(),
            br#"{"pack":{"pack_format":15,"description":"x"}}"#,
        ]
        .concat();
        assert!(validate(&two_marks).is_err());
        assert_eq!(
            validate(br#"{"description":"x"}"#).unwrap_err().to_string(),
            "Invalid datapack metadata"
        );
        validate(br#"{"pack":{"pack_format":15,"description":"x"}}"#).unwrap();
    }
    #[tokio::test]
    async fn datapack_real_selected_copy_collision_and_independent_mutations() {
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
            replace: false,
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
    async fn datapack_rejects_foreign_replaced_world_and_links() {
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
                    world: selected,
                    replace: false,
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
                    world: selected,
                    replace: false,
                })
                .await
                .is_err()
        );
        #[cfg(unix)]
        {
            // A datapacks folder the world links to an absent target refuses
            // with a reason; nothing is created in its place.
            let gone = world.with_file_name("gone-datapacks");
            std::os::unix::fs::symlink(&gone, world.join("datapacks")).unwrap();
            let selected = app
                .instance_manager()
                .datapack_worlds(id)
                .await
                .unwrap()
                .into_iter()
                .find(|w| w.name == "World")
                .unwrap();
            let error = app
                .instance_manager()
                .copy_datapack(CopyDatapack {
                    instance_id: *id,
                    mod_id: row.id,
                    world: selected,
                    replace: false,
                })
                .await
                .unwrap_err();
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
                replace: false,
            })
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(world.join("datapacks/pack.zip")).unwrap(),
            bytes
        );
    }

    /// The copy keeps the staged datapack's mode. An instance on a drive
    /// whose files all belong to another account (exFAT, FAT or an SMB share
    /// mounted without `uid=`) refuses that with EPERM even on a file just
    /// created there; the datapack still lands, with the mode the drive
    /// gives it, and no private copy is left in the instance folder.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_world_on_a_drive_that_keeps_its_own_modes_takes_the_datapack() {
        use crate::util::publish::fault::{Step, inject, pending};
        use std::os::unix::fs::PermissionsExt;
        let app = crate::setup_managers_for_test().await;
        let bytes = zip_bytes();
        let (id, row, source) = addon_fixture(
            &app,
            "fixed-owner-drive",
            AddonType::DataPacks,
            "pack.zip",
            &bytes,
        )
        .await;
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o640)).unwrap();
        let path = publication::current_path(&app, id).await.unwrap();
        let root_entries = || {
            let mut names: Vec<_> = std::fs::read_dir(path.get_root())
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect();
            names.sort();
            names
        };
        let before = root_entries();
        for (world_name, refused) in [("Ordinary", false), ("FixedOwner", true)] {
            let world = path.get_saves_path().join(world_name);
            std::fs::create_dir_all(&world).unwrap();
            std::fs::write(world.join("level.dat"), b"save").unwrap();
            let selected = app
                .instance_manager()
                .datapack_worlds(id)
                .await
                .unwrap()
                .into_iter()
                .find(|world| world.name == world_name)
                .unwrap();
            let copied = world.join("datapacks").join("pack.zip");
            if refused {
                inject(Step::Mode, &copied, libc::EPERM);
            }
            app.instance_manager()
                .copy_datapack(CopyDatapack {
                    instance_id: *id,
                    mod_id: row.id.clone(),
                    world: selected,
                    replace: false,
                })
                .await
                .unwrap();
            assert!(!pending(Step::Mode, &copied), "the mode copy ran");
            assert_eq!(std::fs::read(&copied).unwrap(), bytes);
            if !refused {
                assert_eq!(
                    std::fs::metadata(&copied).unwrap().permissions().mode() & 0o777,
                    0o640
                );
            }
            assert_eq!(root_entries(), before);
        }
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
                replace: false,
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
        let app = crate::setup_managers_for_test().await;
        let bytes = zip_bytes();
        let (id, row, _) = addon_fixture(
            &app,
            "linked-saves",
            AddonType::DataPacks,
            "pack.zip",
            &bytes,
        )
        .await;
        let path = publication::current_path(&app, id).await.unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let saves = elsewhere.path().join("saves");
        let shared = elsewhere.path().join("shared-datapacks");
        std::fs::create_dir_all(saves.join("World")).unwrap();
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::write(saves.join("World/level.dat"), b"save").unwrap();
        std::os::unix::fs::symlink(&shared, saves.join("World/datapacks")).unwrap();
        std::os::unix::fs::symlink(&saves, path.get_saves_path()).unwrap();
        let selected = app
            .instance_manager()
            .datapack_worlds(id)
            .await
            .unwrap()
            .pop()
            .expect("the world in the linked saves folder is listed");
        assert_eq!(selected.name, "World");
        app.instance_manager()
            .copy_datapack(CopyDatapack {
                instance_id: *id,
                mod_id: row.id,
                world: selected,
                replace: false,
            })
            .await
            .unwrap();
        assert_eq!(std::fs::read(shared.join("pack.zip")).unwrap(), bytes);
    }
    #[tokio::test]
    async fn datapack_copy_rechecks_selected_world_and_late_collision() {
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
                replace: false,
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

    /// What the world already holds of a data pack, for the replace tests.
    mod replacing {
        use super::*;
        use crate::managers::App;
        use crate::managers::instance::world_datapack_changes::tests::{
            entries, pack_zip, put_pack, row_of, scanned_rows, snapshot, staging_folders, tree,
            without_recycle_bin,
        };
        use crate::managers::instance::world_datapack_updates::tests::on_modrinth;

        /// An instance whose instance-level data pack `pack.zip` holds
        /// `bytes` and is version `newer` of Modrinth project `project`,
        /// with the save `World`. Returns the instance, the pack's row id,
        /// the world and its selection.
        async fn staged(
            app: &App,
            name: &str,
            bytes: &[u8],
        ) -> (InstanceId, String, PathBuf, DatapackWorld) {
            let (id, row, _) =
                addon_fixture(app, name, AddonType::DataPacks, "pack.zip", bytes).await;
            on_modrinth(app, &row.metadata_id, "project", "newer").await;
            let path = publication::current_path(app, id).await.unwrap();
            let world = path.get_saves_path().join("World");
            std::fs::create_dir_all(world.join("datapacks")).unwrap();
            std::fs::write(world.join("level.dat"), b"save").unwrap();
            let selected = app
                .instance_manager()
                .datapack_worlds(id)
                .await
                .unwrap()
                .pop()
                .unwrap();
            (id, row.id, world, selected)
        }

        /// `pack-1.zip` in the world, an older version of the same project.
        async fn older_build(app: &App, id: InstanceId, world: &Path) -> PathBuf {
            let path = put_pack(world, "pack-1.zip", &pack_zip("older"));
            let rows = scanned_rows(app, id).await;
            let row = row_of(&rows, "World", "pack-1.zip");
            on_modrinth(app, row.metadata_id.as_deref().unwrap(), "project", "older").await;
            path
        }

        async fn copy(
            app: &App,
            id: InstanceId,
            mod_id: &str,
            world: &DatapackWorld,
            replace: bool,
        ) -> anyhow::Result<String> {
            app.instance_manager()
                .copy_datapack(CopyDatapack {
                    instance_id: *id,
                    mod_id: mod_id.to_owned(),
                    world: world.clone(),
                    replace,
                })
                .await
        }

        /// The refusal as the frontend reads it: its code and file names.
        fn already_in_world(error: &anyhow::Error) -> Vec<String> {
            let sent = serde_json::to_value(crate::error::FeError::from_anyhow(error)).unwrap();
            let cause = &sent["cause"][0];
            assert_eq!(cause["code"], "ALREADY_IN_WORLD", "{sent}");
            serde_json::from_value(cause["data"]["filenames"].clone()).unwrap()
        }

        async fn root_of(app: &App, id: InstanceId) -> PathBuf {
            publication::current_path(app, id).await.unwrap().get_root()
        }

        #[tokio::test]
        async fn a_name_the_world_holds_is_refused_with_its_name_and_kept() {
            let app = crate::setup_managers_for_test().await;
            let new = pack_zip("new");
            let (id, mod_id, world, selected) = staged(&app, "copy-same-name", &new).await;
            put_pack(&world, "pack.zip", b"the user's own");
            let root = root_of(&app, id).await;
            let before = snapshot(&root, &world);

            let error = copy(&app, id, &mod_id, &selected, false).await.unwrap_err();

            assert_eq!(already_in_world(&error), vec!["pack.zip".to_owned()]);
            assert_eq!(
                error.to_string(),
                "This world already has this data pack: pack.zip"
            );
            assert_eq!(snapshot(&root, &world), before);
        }

        #[tokio::test]
        async fn with_replace_the_world_holds_the_new_bytes() {
            let app = crate::setup_managers_for_test().await;
            let new = pack_zip("new");
            let (id, mod_id, world, selected) = staged(&app, "copy-replace-same", &new).await;
            put_pack(&world, "pack.zip", &pack_zip("old"));
            scanned_rows(&app, id).await;
            let root = root_of(&app, id).await;

            copy(&app, id, &mod_id, &selected, true).await.unwrap();

            assert_eq!(
                entries(&world.join("datapacks")),
                vec![("pack.zip".to_owned(), new)]
            );
            assert!(staging_folders(&root).is_empty());
            let rows = scanned_rows(&app, id).await;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].filename, "pack.zip");
        }

        /// The update case the copy fixes: a newer build of the project has
        /// another file name; the older one in the world gives way to it.
        #[tokio::test]
        async fn another_version_of_the_project_is_replaced_not_joined() {
            let app = crate::setup_managers_for_test().await;
            without_recycle_bin(&app).await;
            let new = pack_zip("new");
            let (id, mod_id, world, selected) = staged(&app, "copy-other-name", &new).await;
            older_build(&app, id, &world).await;
            let root = root_of(&app, id).await;
            let before = snapshot(&root, &world);

            let error = copy(&app, id, &mod_id, &selected, false).await.unwrap_err();
            assert_eq!(already_in_world(&error), vec!["pack-1.zip".to_owned()]);
            assert_eq!(snapshot(&root, &world), before);

            copy(&app, id, &mod_id, &selected, true).await.unwrap();

            assert_eq!(
                entries(&world.join("datapacks")),
                vec![("pack.zip".to_owned(), new)]
            );
            assert!(staging_folders(&root).is_empty());
        }

        /// A file of the name and another version under another name: both
        /// are the copies, listed together and replaced by the one new file.
        #[tokio::test]
        async fn the_same_name_and_another_version_both_give_way() {
            let app = crate::setup_managers_for_test().await;
            without_recycle_bin(&app).await;
            let new = pack_zip("new");
            let (id, mod_id, world, selected) = staged(&app, "copy-two-copies", &new).await;
            put_pack(&world, "pack.zip", b"the user's own");
            older_build(&app, id, &world).await;
            let root = root_of(&app, id).await;

            let error = copy(&app, id, &mod_id, &selected, false).await.unwrap_err();
            assert_eq!(
                already_in_world(&error),
                vec!["pack-1.zip".to_owned(), "pack.zip".to_owned()]
            );

            assert_eq!(
                copy(&app, id, &mod_id, &selected, true).await.unwrap(),
                "World"
            );

            assert_eq!(
                entries(&world.join("datapacks")),
                vec![("pack.zip".to_owned(), new)]
            );
            assert!(staging_folders(&root).is_empty());
        }

        /// The new file cannot be published: both old ones are back under
        /// their names with their bytes, and nothing is left aside.
        #[tokio::test]
        async fn a_failed_publish_puts_every_old_copy_back() {
            use crate::util::publish::fault::{IN_USE, Step, inject, pending};
            let app = crate::setup_managers_for_test().await;
            let (id, mod_id, world, selected) =
                staged(&app, "copy-put-back", &pack_zip("new")).await;
            let same_name = put_pack(&world, "pack.zip", b"the user's own");
            older_build(&app, id, &world).await;
            let root = root_of(&app, id).await;
            let before = snapshot(&root, &world);
            inject(Step::NoReplace, &same_name, IN_USE);

            copy(&app, id, &mod_id, &selected, true)
                .await
                .expect_err("the publication fails");

            assert!(!pending(Step::NoReplace, &same_name), "publication tried");
            assert_eq!(snapshot(&root, &world), before);
        }

        /// An unpacked pack folder named like the zip is no copy: the copy
        /// is refused with the occupied-name text, with or without replace.
        #[tokio::test]
        async fn an_unpacked_folder_named_like_the_zip_is_not_replaced() {
            let app = crate::setup_managers_for_test().await;
            let (id, mod_id, world, selected) =
                staged(&app, "copy-folder-name", &pack_zip("new")).await;
            let folder = world.join("datapacks/pack.zip");
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(
                folder.join("pack.mcmeta"),
                br#"{"pack":{"pack_format":15}}"#,
            )
            .unwrap();
            let rows = scanned_rows(&app, id).await;
            assert!(row_of(&rows, "World", "pack.zip").is_folder);
            let before = tree(&world);

            for replace in [true, false] {
                let error = copy(&app, id, &mod_id, &selected, replace)
                    .await
                    .unwrap_err();
                assert_eq!(
                    error.downcast_ref(),
                    Some(&WorldRefusal::NameTaken {
                        filename: "pack.zip".into()
                    }),
                    "replace: {replace}"
                );
                assert_eq!(tree(&world), before, "replace: {replace}");
            }
        }

        /// As for an install into a world: refused while the game is queued,
        /// preparing or running, and nothing changes.
        #[tokio::test]
        async fn a_copy_while_the_game_runs_is_refused_and_changes_nothing() {
            use crate::domain::vtask::VisualTaskId;
            use crate::managers::instance::run::{LaunchState, RunningInstance};
            let app = crate::setup_managers_for_test().await;
            let (id, mod_id, world, selected) =
                staged(&app, "copy-running", &pack_zip("new")).await;
            older_build(&app, id, &world).await;
            let root = root_of(&app, id).await;
            let before = snapshot(&root, &world);
            for state in [
                LaunchState::Queued(VisualTaskId(900)),
                LaunchState::Preparing(VisualTaskId(900)),
                LaunchState::Running(RunningInstance::adopted(
                    u32::MAX,
                    1,
                    chrono::Utc::now(),
                    chrono::Utc::now(),
                )),
            ] {
                let name = format!("{state:?}");
                app.instance_manager()
                    .instances
                    .write()
                    .await
                    .get_mut(&id)
                    .unwrap()
                    .data_mut()
                    .unwrap()
                    .state = state;
                for replace in [false, true] {
                    let error = copy(&app, id, &mod_id, &selected, replace)
                        .await
                        .unwrap_err();
                    assert!(
                        error.to_string().contains("running or being installed"),
                        "{name}: {error:#}"
                    );
                    assert_eq!(snapshot(&root, &world), before, "{name}");
                }
            }
        }

        /// An older frontend sends no `replace`: the copy does not replace.
        #[tokio::test]
        async fn a_request_without_replace_does_not_replace() {
            let app = crate::setup_managers_for_test().await;
            let (id, mod_id, world, selected) =
                staged(&app, "copy-older-frontend", &pack_zip("new")).await;
            older_build(&app, id, &world).await;
            let root = root_of(&app, id).await;
            let before = snapshot(&root, &world);
            let request: CopyDatapack = serde_json::from_value(serde_json::json!({
                "instance_id": *id,
                "mod_id": mod_id,
                "world": { "name": selected.name, "identity": selected.identity },
            }))
            .unwrap();
            assert!(!request.replace);

            let error = app
                .instance_manager()
                .copy_datapack(request)
                .await
                .unwrap_err();

            assert_eq!(already_in_world(&error), vec!["pack-1.zip".to_owned()]);
            assert_eq!(snapshot(&root, &world), before);
        }

        /// The user puts their own build under the old copy's name after the
        /// copy read the world's rows: that file is not taken away.
        #[tokio::test]
        async fn a_file_put_under_the_old_name_after_the_rows_were_read_is_kept() {
            let app = crate::setup_managers_for_test().await;
            let (id, mod_id, world, selected) =
                staged(&app, "copy-swapped", &pack_zip("new")).await;
            let older = older_build(&app, id, &world).await;
            let pause = app
                .instance_manager()
                .test_hooks
                .arm("datapack_copy_scanned", id);
            let worker = {
                let (app, mod_id, selected) = (app.clone(), mod_id.clone(), selected.clone());
                tokio::spawn(async move { copy(&app, id, &mod_id, &selected, true).await })
            };
            tokio::time::timeout(std::time::Duration::from_secs(10), pause.entered.notified())
                .await
                .unwrap();
            std::fs::write(&older, b"the user's own build").unwrap();
            pause.release.notify_one();

            let error = worker.await.unwrap().unwrap_err();

            assert_eq!(error.downcast_ref(), Some(&WorldRefusal::PackChanged));
            assert_eq!(
                entries(&world.join("datapacks")),
                vec![("pack-1.zip".to_owned(), b"the user's own build".to_vec())]
            );
            assert!(staging_folders(&root_of(&app, id).await).is_empty());
        }

        /// The request is dropped (its window closed) once the old copy is
        /// set aside: the replace still runs to its end, leaving the new
        /// file alone and the old one removed by the removal setting.
        #[tokio::test]
        async fn a_replace_whose_request_is_dropped_midway_still_settles() {
            let app = crate::setup_managers_for_test().await;
            without_recycle_bin(&app).await;
            let new = pack_zip("new");
            let (id, mod_id, world, selected) = staged(&app, "copy-dropped", &new).await;
            older_build(&app, id, &world).await;
            let root = root_of(&app, id).await;
            let pause = app
                .instance_manager()
                .test_hooks
                .arm("world_datapack_taken_away", id);
            let request = {
                let app = app.clone();
                tokio::spawn(async move { copy(&app, id, &mod_id, &selected, true).await })
            };
            tokio::time::timeout(std::time::Duration::from_secs(10), pause.entered.notified())
                .await
                .unwrap();

            request.abort();
            assert!(request.await.unwrap_err().is_cancelled());
            pause.release.notify_one();

            let settled = || {
                entries(&world.join("datapacks")) == vec![("pack.zip".to_owned(), new.clone())]
                    && staging_folders(&root).is_empty()
            };
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                while !settled() {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
            })
            .await
            .expect("the replace ends with the new file alone");
        }
    }
}
