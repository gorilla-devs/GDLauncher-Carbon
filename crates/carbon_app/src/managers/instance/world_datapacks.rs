//! Data packs inside an instance's worlds. The game loads them only from
//! `saves/<world>/datapacks`; the scan keeps `WorldDatapackCache` in line
//! with those folders. A zip points at the shared `ModMetadata` of its bytes,
//! so platform data and images are found by hash as for any addon.
use super::{
    InstanceId, InstanceManager, InvalidInstanceIdError,
    datapacks::world_identity,
    installer::publication,
    mods::{addon_metadata, validate_addon_basename},
    run::LaunchState,
};
use crate::{
    domain::instance::AddonType,
    managers::{ManagerRef, metadata::cache::current_file_stamp},
};
use carbon_repos::{
    dbtypes::DbDateTime,
    repos::world_datapack_cache::{self as wdb, WorldDatapackRow},
};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::warn;

pub(super) fn world_datapacks_folder(saves: &Path, world: &str) -> PathBuf {
    saves.join(world).join("datapacks")
}

/// An entry of a world's `datapacks` folder that the game loads.
enum Pack {
    /// A zip file, with the stamp its size and times give now.
    Zip(String),
    /// A folder holding `pack.mcmeta`.
    Folder,
}

/// The packs in a world's `datapacks` folder by name; none when there is no
/// such folder. A linked folder is followed. An error (a link whose target is
/// gone, an unreadable folder) means the folder could not be listed.
async fn listed_packs(folder: &Path) -> anyhow::Result<HashMap<String, Pack>> {
    let mut packs = HashMap::new();
    if !publication::followed_folder(folder).await? {
        return Ok(packs);
    }
    let mut entries = tokio::fs::read_dir(folder).await?;
    while let Some(entry) = entries.next_entry().await? {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if validate_addon_basename(&name).is_err() {
            continue;
        }
        let path = entry.path();
        // Followed like the game follows it; a dangling link is no pack.
        let Ok(metadata) = tokio::fs::metadata(&path).await else {
            continue;
        };
        if metadata.is_file() && name.to_ascii_lowercase().ends_with(".zip") {
            if let Some(stamp) = current_file_stamp(&path).await {
                packs.insert(name, Pack::Zip(stamp));
            }
        } else if metadata.is_dir()
            && addon_metadata(&path.join("pack.mcmeta"))
                .await
                .ok()
                .flatten()
                .is_some_and(|metadata| metadata.is_file())
        {
            packs.insert(name, Pack::Folder);
        }
    }
    Ok(packs)
}

impl ManagerRef<'_, InstanceManager> {
    /// Bring the instance's world data pack rows in line with its worlds'
    /// `datapacks` folders. Returns whether any row was added, changed or
    /// removed. Does nothing while the game is starting or running, as the
    /// addon scan does not.
    pub async fn scan_world_datapacks(self, instance_id: InstanceId) -> anyhow::Result<bool> {
        let _activity = self
            .app
            .runtime_lifecycle
            .admit("scan the data packs in an instance's worlds")?;
        let _operation = self.lock_instance(instance_id).await;
        {
            let instances = self.instances.read().await;
            let state = &instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?
                .data()?
                .state;
            if !matches!(state, LaunchState::Inactive { .. }) {
                return Ok(false);
            }
        }
        let path = publication::current_path(&self.app, instance_id).await?;
        let root = path.get_root();
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        let mut worlds = Vec::new();
        match tokio::fs::read_dir(&saves).await {
            Ok(mut entries) => {
                while let Some(entry) = entries.next_entry().await? {
                    if let Some(name) = entry.file_name().to_str()
                        && validate_addon_basename(name).is_ok()
                    {
                        worlds.push(name.to_owned());
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        let mut stored: HashMap<String, HashMap<String, WorldDatapackRow>> = HashMap::new();
        for row in wdb::get_world_datapacks_by_instance(&self.app.db, *instance_id).await? {
            stored
                .entry(row.world.clone())
                .or_default()
                .insert(row.filename.clone(), row);
        }
        let mut changed = false;
        let mut kept = HashSet::new();
        for world in worlds {
            if world_identity(instance_id, &root, &saves.join(&world))
                .await
                .is_err()
            {
                continue;
            }
            kept.insert(world.clone());
            let folder = world_datapacks_folder(&saves, &world);
            let packs = match listed_packs(&folder).await {
                Ok(packs) => packs,
                Err(error) => {
                    // Kept as they are: a drive that is away comes back.
                    warn!(
                        ?error,
                        "Instance {instance_id}: could not list the data packs of world {world}"
                    );
                    continue;
                }
            };
            let mut rows = stored.remove(&world).unwrap_or_default();
            for (name, pack) in packs {
                let row = rows.remove(&name);
                changed |= self
                    .record_world_datapack(instance_id, &world, &folder, &name, pack, row)
                    .await?;
            }
            for row in rows.into_values() {
                wdb::delete_world_datapack_by_id(&self.app.db, &row.id).await?;
                changed = true;
            }
        }
        for world in stored.into_keys().filter(|world| !kept.contains(world)) {
            wdb::delete_world_datapacks_of_world(&self.app.db, *instance_id, &world).await?;
            changed = true;
        }
        Ok(changed)
    }

    /// Write the row of one pack unless `row` already describes it. A zip is
    /// read only when its stamp changed; one that cannot be read keeps its
    /// row and is read again by the next scan.
    async fn record_world_datapack(
        self,
        instance_id: InstanceId,
        world: &str,
        folder: &Path,
        name: &str,
        pack: Pack,
        row: Option<WorldDatapackRow>,
    ) -> anyhow::Result<bool> {
        let (filesize, is_folder, metadata_id, stamp) = match pack {
            Pack::Folder => {
                if row.is_some_and(|row| {
                    row.is_folder && row.filesize == 0 && row.metadata_id.is_none()
                }) {
                    return Ok(false);
                }
                (0, true, None, None)
            }
            Pack::Zip(stamp) => {
                if row.is_some_and(|row| {
                    !row.is_folder
                        && row.metadata_id.is_some()
                        && row.file_stamp.as_deref() == Some(stamp.as_str())
                }) {
                    return Ok(false);
                }
                let cache = self.app.meta_cache_manager();
                let result = match cache
                    .hash_and_parse_mod_file(&folder.to_path_buf(), name, true)
                    .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        warn!(
                            ?error,
                            "Instance {instance_id}: could not read data pack {name} of world {world}"
                        );
                        return Ok(false);
                    }
                };
                let metadata_id = cache.ensure_mod_metadata(&result, name).await?;
                (
                    i32::try_from(result.content_len).unwrap_or(i32::MAX),
                    false,
                    Some(metadata_id),
                    result.file_stamp,
                )
            }
        };
        wdb::upsert_world_datapack(
            &self.app.db,
            *instance_id,
            world,
            name,
            filesize,
            is_folder,
            metadata_id.as_deref(),
            stamp.as_deref(),
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::instance::AddonType;
    use crate::managers::App;
    use crate::managers::instance::{installer::publication, mods::tests::addon_fixture};
    use crate::managers::metadata::cache::{CacheEntityId, whole_file_reads};
    use carbon_repos::repos::world_datapack_cache::{self as wdb, WorldDatapackRow};
    use std::io::Write;

    fn pack_zip(description: &str) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("pack.mcmeta", zip::write::SimpleFileOptions::default())
            .unwrap();
        write!(
            zip,
            r#"{{"pack":{{"pack_format":15,"description":"{description}"}}}}"#
        )
        .unwrap();
        zip.finish().unwrap().into_inner()
    }

    /// An instance with a save `world` (holding `level.dat`) and its empty
    /// `datapacks` folder. Returns the instance and the `saves` folder.
    async fn instance_with_world(app: &App, name: &str, world: &str) -> (InstanceId, PathBuf) {
        let (id, _, _) = addon_fixture(
            app,
            name,
            AddonType::DataPacks,
            "staged.zip",
            &pack_zip("staged"),
        )
        .await;
        let saves = publication::current_path(app, id)
            .await
            .unwrap()
            .get_saves_path();
        add_world(&saves, world);
        (id, saves)
    }

    fn add_world(saves: &Path, world: &str) {
        std::fs::create_dir_all(world_datapacks_folder(saves, world)).unwrap();
        std::fs::write(saves.join(world).join("level.dat"), b"save").unwrap();
    }

    async fn rows(app: &App, id: InstanceId) -> Vec<WorldDatapackRow> {
        let mut rows = wdb::get_world_datapacks_by_instance(&app.db, *id)
            .await
            .unwrap();
        rows.sort_by(|a, b| (&a.world, &a.filename).cmp(&(&b.world, &b.filename)));
        rows
    }

    async fn scan(app: &App, id: InstanceId) -> bool {
        app.instance_manager()
            .scan_world_datapacks(id)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn a_zip_in_a_world_is_listed_with_its_metadata() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "listed-zip", "World").await;
        let bytes = pack_zip("listed");
        std::fs::write(
            world_datapacks_folder(&saves, "World").join("Pack.ZIP"),
            &bytes,
        )
        .unwrap();
        assert!(scan(&app, id).await);
        let listed = rows(&app, id).await;
        assert_eq!(listed.len(), 1, "{listed:?}");
        let row = &listed[0];
        assert_eq!(
            (row.world.as_str(), row.filename.as_str()),
            ("World", "Pack.ZIP")
        );
        assert_eq!(row.filesize as usize, bytes.len());
        assert!(!row.is_folder);
        assert!(row.metadata_id.is_some());
        assert!(row.file_stamp.is_some());
        // Other files and folders without pack.mcmeta are not packs.
        let folder = world_datapacks_folder(&saves, "World");
        std::fs::write(folder.join("notes.txt"), b"not a pack").unwrap();
        std::fs::create_dir(folder.join("empty-folder")).unwrap();
        assert!(!scan(&app, id).await);
        assert_eq!(rows(&app, id).await, listed);
    }

    #[tokio::test]
    async fn an_unchanged_zip_is_not_read_again() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "unchanged-zip", "World").await;
        let path = world_datapacks_folder(&saves, "World").join("pack.zip");
        std::fs::write(&path, pack_zip("unchanged")).unwrap();
        assert!(scan(&app, id).await);
        let reads = whole_file_reads(&path);
        assert!(reads > 0);
        let before = rows(&app, id).await;
        assert!(!scan(&app, id).await);
        assert_eq!(whole_file_reads(&path), reads);
        assert_eq!(rows(&app, id).await, before);
        // A new file under the same name is read and its row follows it.
        std::fs::write(&path, pack_zip("a different pack, longer")).unwrap();
        assert!(scan(&app, id).await);
        assert!(whole_file_reads(&path) > reads);
        let after = rows(&app, id).await;
        assert_eq!(after[0].id, before[0].id);
        assert_ne!(after[0].metadata_id, before[0].metadata_id);
    }

    #[tokio::test]
    async fn a_removed_pack_loses_its_row() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "removed-zip", "World").await;
        let folder = world_datapacks_folder(&saves, "World");
        std::fs::write(folder.join("kept.zip"), pack_zip("kept")).unwrap();
        std::fs::write(folder.join("gone.zip"), pack_zip("gone")).unwrap();
        assert!(scan(&app, id).await);
        assert_eq!(rows(&app, id).await.len(), 2);
        std::fs::remove_file(folder.join("gone.zip")).unwrap();
        assert!(scan(&app, id).await);
        let rows = rows(&app, id).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].filename, "kept.zip");
    }

    #[tokio::test]
    async fn an_unpacked_folder_pack_is_listed_without_metadata() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "folder-pack", "World").await;
        let pack = world_datapacks_folder(&saves, "World").join("my pack");
        std::fs::create_dir_all(pack.join("data")).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), br#"{"pack":{"pack_format":15}}"#).unwrap();
        assert!(scan(&app, id).await);
        let rows = rows(&app, id).await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].filename, "my pack");
        assert!(rows[0].is_folder);
        assert_eq!(rows[0].filesize, 0);
        assert_eq!(rows[0].metadata_id, None);
        assert!(!scan(&app, id).await);
    }

    #[tokio::test]
    async fn a_zip_without_pack_mcmeta_is_listed() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "zip-without-mcmeta", "World").await;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("readme.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"no metadata").unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let path = world_datapacks_folder(&saves, "World").join("odd.zip");
        std::fs::write(&path, bytes).unwrap();
        assert!(crate::managers::instance::datapacks::validate_pack_zip(&path).is_err());
        assert!(scan(&app, id).await);
        let rows = rows(&app, id).await;
        assert_eq!(rows.len(), 1);
        assert!(rows[0].metadata_id.is_some());
    }

    #[tokio::test]
    async fn a_world_without_level_dat_has_no_packs() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "no-level-dat", "World").await;
        let folder = saves.join("NotASave").join("datapacks");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("pack.zip"), pack_zip("orphan")).unwrap();
        assert!(!scan(&app, id).await);
        assert!(rows(&app, id).await.is_empty());
        // A world that stops being a save loses its rows, and so does one
        // that is gone.
        std::fs::write(
            world_datapacks_folder(&saves, "World").join("pack.zip"),
            pack_zip("listed"),
        )
        .unwrap();
        add_world(&saves, "Other");
        std::fs::write(
            world_datapacks_folder(&saves, "Other").join("pack.zip"),
            pack_zip("other"),
        )
        .unwrap();
        assert!(scan(&app, id).await);
        assert_eq!(rows(&app, id).await.len(), 2);
        std::fs::remove_file(saves.join("World").join("level.dat")).unwrap();
        std::fs::remove_dir_all(saves.join("Other")).unwrap();
        assert!(scan(&app, id).await);
        assert!(rows(&app, id).await.is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_linked_datapacks_folder_is_followed() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "linked-datapacks", "World").await;
        let shared = tempfile::tempdir().unwrap();
        std::fs::write(shared.path().join("shared.zip"), pack_zip("shared")).unwrap();
        let folder = world_datapacks_folder(&saves, "World");
        std::fs::remove_dir(&folder).unwrap();
        std::os::unix::fs::symlink(shared.path(), &folder).unwrap();
        assert!(scan(&app, id).await);
        let rows = rows(&app, id).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].filename, "shared.zip");
        assert!(rows[0].metadata_id.is_some());
    }

    #[tokio::test]
    async fn the_same_pack_in_two_worlds_shares_one_metadata() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "two-worlds", "First").await;
        add_world(&saves, "Second");
        let bytes = pack_zip("shared");
        for world in ["First", "Second"] {
            std::fs::write(
                world_datapacks_folder(&saves, world).join("pack.zip"),
                &bytes,
            )
            .unwrap();
        }
        assert!(scan(&app, id).await);
        let rows = rows(&app, id).await;
        assert_eq!(rows.len(), 2);
        assert_ne!(rows[0].id, rows[1].id);
        assert!(rows[0].metadata_id.is_some());
        assert_eq!(rows[0].metadata_id, rows[1].metadata_id);
    }

    #[tokio::test]
    async fn the_scan_waits_while_the_game_is_starting() {
        use crate::domain::vtask::VisualTaskId;
        use crate::managers::instance::run::LaunchState;
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "game-starting", "World").await;
        std::fs::write(
            world_datapacks_folder(&saves, "World").join("pack.zip"),
            pack_zip("waiting"),
        )
        .unwrap();
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
        for state in [
            LaunchState::Queued(VisualTaskId(900)),
            LaunchState::Preparing(VisualTaskId(900)),
        ] {
            set_state(state).await;
            assert!(!scan(&app, id).await);
            assert!(rows(&app, id).await.is_empty());
        }
        set_state(LaunchState::Inactive { failed_task: None }).await;
        assert!(scan(&app, id).await);
        assert_eq!(rows(&app, id).await.len(), 1);
    }

    #[tokio::test]
    async fn the_instance_scan_also_scans_its_worlds() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "instance-pass", "World").await;
        std::fs::write(
            world_datapacks_folder(&saves, "World").join("pack.zip"),
            pack_zip("found by the instance pass"),
        )
        .unwrap();
        app.meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
            .unwrap();
        let rows = rows(&app, id).await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].filename, "pack.zip");
    }
}
