//! Data packs inside an instance's worlds. The game loads them only from
//! `saves/<world>/datapacks`; the scan keeps `WorldDatapackCache` in line
//! with those folders. A zip points at the shared `ModMetadata` of its bytes,
//! so platform data and images are found by hash as for any addon.
use super::{
    InstanceId, InstanceManager, InvalidInstanceIdError,
    datapacks::world_identity,
    installer::publication,
    mods::{
        CachedListing, InvalidInstanceModIdError, addon_metadata, fresh_mr_update_paths,
        has_cached_update, validate_addon_basename,
    },
    run::LaunchState,
    world_level::read_level_name,
};
use crate::{
    api::keys::instance::GET_WORLDS,
    domain::instance::{self as domain, AddonType},
    managers::{
        ManagerRef,
        metadata::cache::{
            CacheEntityId, current_file_stamp,
            eligibility::{WORLD_DATAPACKS, world_context},
            modrinth::target_compatibility,
        },
    },
};
use anyhow::bail;
use carbon_repos::{
    dbtypes::DbDateTime,
    repos::{
        mod_file_cache as mfcdb,
        world_datapack_cache::{self as wdb, WorldDatapackFullRow, WorldDatapackRow},
    },
};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

pub(super) fn world_datapacks_folder(saves: &Path, world: &str) -> PathBuf {
    saves.join(world).join("datapacks")
}

/// A world of an instance, as the Worlds tab lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct World {
    /// The save's folder name.
    pub name: String,
    /// The name the game shows, from `level.dat`; `None` when it has none or
    /// the file cannot be read.
    pub title: Option<String>,
    /// The id of the world's addon row.
    pub addon_id: String,
    /// `None` when the folder is not a save the launcher may change.
    pub identity: Option<String>,
    pub datapacks: Vec<WorldDatapack>,
}

/// A data pack in a world's `datapacks` folder.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldDatapack {
    pub id: String,
    pub filename: String,
    pub file_size: u64,
    pub is_folder: bool,
    pub name: Option<String>,
    pub version: Option<String>,
    pub curseforge: Option<domain::CurseForgeModMetadata>,
    pub modrinth: Option<domain::ModrinthModMetadata>,
    pub has_update: bool,
}

impl WorldDatapack {
    fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.filename)
    }
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
    // A file in place of the folder holds no packs; a warning would come
    // back on every pass.
    if tokio::fs::metadata(folder)
        .await
        .is_ok_and(|metadata| !metadata.is_dir())
    {
        debug!("{} is a file, not a data pack folder", folder.display());
        return Ok(packs);
    }
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
    /// addon scan does not. Stops when the launcher closes.
    pub async fn scan_world_datapacks(self, instance_id: InstanceId) -> anyhow::Result<bool> {
        self.scan_world_datapacks_started(instance_id, || {}).await
    }

    /// [`Self::scan_world_datapacks`], calling `started` once the scan holds
    /// the instance gate: what changes on disk after that may be missed.
    pub(crate) async fn scan_world_datapacks_started(
        self,
        instance_id: InstanceId,
        started: impl FnOnce() + Send,
    ) -> anyhow::Result<bool> {
        let changed = self
            .scan_world_datapacks_unannounced(instance_id, started)
            .await?;
        if changed {
            self.app.invalidate(GET_WORLDS, Some((*instance_id).into()));
        }
        Ok(changed)
    }

    /// The scan without telling the frontend that the worlds changed.
    async fn scan_world_datapacks_unannounced(
        self,
        instance_id: InstanceId,
        started: impl FnOnce() + Send,
    ) -> anyhow::Result<bool> {
        let activity = self
            .app
            .runtime_lifecycle
            .admit("scan the data packs in an instance's worlds")?;
        let cancel = activity.cancellation();
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(anyhow::anyhow!(
                "Data pack scan of the worlds of instance {instance_id} stopped because the launcher is closing"
            )),
            result = self.scan_worlds(instance_id, started) => result,
        }
    }

    async fn scan_worlds(
        self,
        instance_id: InstanceId,
        started: impl FnOnce() + Send,
    ) -> anyhow::Result<bool> {
        let _operation = self.lock_instance(instance_id).await;
        started();
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

    /// The instance's worlds, one per `worlds` addon row, each with the data
    /// packs found in it, after scanning them. Sorted by name, packs by the
    /// name they show. Only cached platform data is read, so the list does not
    /// wait for CurseForge or Modrinth. A failed scan is logged and the rows
    /// there are are listed.
    pub async fn list_worlds(self, instance_id: InstanceId) -> anyhow::Result<Vec<World>> {
        // Not announced: this answer already holds what it changed, and an
        // invalidation would only make the frontend ask again. Packs it finds
        // are asked about as those of a background scan are.
        match self
            .scan_world_datapacks_unannounced(instance_id, || {})
            .await
        {
            Ok(true) => {
                self.app
                    .meta_cache_manager()
                    .queue_platform_passes(CacheEntityId::Instance(instance_id))
                    .await;
            }
            Ok(false) => {}
            Err(error) => warn!(
                ?error,
                "Instance {instance_id}: could not scan the data packs in its worlds before listing them"
            ),
        }

        let config = {
            let instances = self.instances.read().await;
            instances
                .get(&instance_id)
                .ok_or(InvalidInstanceIdError(instance_id))?
                .data()?
                .config
                .clone()
        };
        let context = target_compatibility(self.app, CacheEntityId::Instance(instance_id))
            .await
            .map(|context| world_context(&context.game));
        let sources = self.instance_cfg_mod_sources(&config).await?;
        let path = publication::current_path(&self.app, instance_id).await?;
        let root = path.get_root();
        // Worlds behind a saves folder that fails its checks get no identity.
        let saves = publication::checked_folder(&path, AddonType::Worlds)
            .await
            .ok();

        let mut packs: HashMap<String, Vec<WorldDatapack>> = HashMap::new();
        for row in wdb::get_world_datapacks_full(&self.app.db, *instance_id).await? {
            let world = row.world.clone();
            let pack = self
                .listed_world_datapack(row, context.as_ref(), &sources)
                .await?;
            packs.entry(world).or_default().push(pack);
        }

        let mut worlds = Vec::new();
        let mut rows = Vec::new();
        for row in mfcdb::get_mod_files_by_instance(&self.app.db, *instance_id).await? {
            if row.addon_type == AddonType::Worlds.to_db_string() {
                rows.push(row);
            }
        }

        let mut titles = match &saves {
            Some(saves) => {
                let folders: Vec<_> = rows.iter().map(|row| saves.join(&row.filename)).collect();
                tokio::task::spawn_blocking(move || {
                    folders
                        .iter()
                        .map(|folder| match read_level_name(folder) {
                            Ok(title) => title,
                            Err(error) => {
                                debug!(?error, "could not read the name of {}", folder.display());
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .await?
            }
            None => vec![None; rows.len()],
        }
        .into_iter();

        for row in rows {
            let title = titles.next().flatten();
            let identity = match &saves {
                Some(saves) => world_identity(instance_id, &root, &saves.join(&row.filename))
                    .await
                    .ok(),
                None => None,
            };
            let mut datapacks = match identity {
                Some(_) => packs.remove(&row.filename).unwrap_or_default(),
                None => Vec::new(),
            };
            datapacks.sort_by_cached_key(|pack| pack.display_name().to_lowercase());
            worlds.push(World {
                name: row.filename,
                title,
                addon_id: row.id,
                identity,
                datapacks,
            });
        }
        worlds.sort_by_cached_key(|world| world.name.to_lowercase());
        Ok(worlds)
    }

    async fn listed_world_datapack(
        self,
        row: WorldDatapackFullRow,
        context: Option<&crate::managers::metadata::cache::eligibility::UpdateContext>,
        sources: &carbon_platforms::ModSources,
    ) -> anyhow::Result<WorldDatapack> {
        // A world's packs have their own key: paths cached for the instance
        // may name builds a world never loads, for its mod loaders or, as
        // its own data packs are asked for, resource packs.
        let mr_update_paths = match (&row.meta_id, context) {
            (Some(meta_id), Some(context)) => {
                fresh_mr_update_paths(&self.app.db, meta_id, &context.key(WORLD_DATAPACKS)).await?
            }
            _ => None,
        };
        let has_update = row.meta_id.as_deref().is_some_and(|meta_id| {
            has_cached_update(
                context,
                sources,
                "datapacks",
                meta_id,
                row.cf_project_id.map(|_| CachedListing {
                    release_type: row.cf_release_type,
                    update_paths: row.cf_update_paths.as_deref(),
                }),
                row.mr_project_id.as_ref().map(|_| CachedListing {
                    release_type: row.mr_release_type,
                    update_paths: mr_update_paths.as_deref(),
                }),
                &[],
            )
        });
        let curseforge = match (
            row.cf_project_id,
            row.cf_file_id,
            row.cf_name,
            row.cf_version,
            row.cf_urlslug,
            row.cf_summary,
            row.cf_authors,
        ) {
            (
                Some(project_id),
                Some(file_id),
                Some(name),
                Some(version),
                Some(urlslug),
                Some(summary),
                Some(authors),
            ) => Some(domain::CurseForgeModMetadata {
                project_id: project_id as u32,
                file_id: file_id as u32,
                name,
                version,
                urlslug,
                summary,
                authors,
                has_image: row.has_cf_image,
            }),
            _ => None,
        };
        let modrinth = match (
            row.mr_project_id,
            row.mr_version_id,
            row.mr_title,
            row.mr_version,
            row.mr_urlslug,
            row.mr_description,
            row.mr_authors,
        ) {
            (
                Some(project_id),
                Some(version_id),
                Some(title),
                Some(version),
                Some(urlslug),
                Some(description),
                Some(authors),
            ) => Some(domain::ModrinthModMetadata {
                project_id,
                version_id,
                title,
                version,
                urlslug,
                description,
                authors,
                has_image: row.has_mr_image,
            }),
            _ => None,
        };
        let name = curseforge
            .as_ref()
            .map(|cf| cf.name.clone())
            .or_else(|| modrinth.as_ref().map(|mr| mr.title.clone()));
        let version = curseforge
            .as_ref()
            .map(|cf| cf.version.clone())
            .or_else(|| modrinth.as_ref().map(|mr| mr.version.clone()));
        Ok(WorldDatapack {
            id: row.id,
            filename: row.filename,
            file_size: row.filesize.max(0) as u64,
            is_folder: row.is_folder,
            name,
            version,
            curseforge,
            modrinth,
            has_update,
        })
    }

    /// The icon of a world data pack of `instance_id` from `platformid`'s
    /// image cache (0 local, 1 CurseForge, 2 Modrinth), as
    /// `get_mod_icon` serves an addon's. `None` when there is none, as for a
    /// folder pack.
    pub async fn get_world_datapack_icon(
        self,
        instance_id: InstanceId,
        id: String,
        platformid: i32,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        if !(0..=2).contains(&platformid) {
            bail!("unsupported platform");
        }
        let owned = wdb::get_world_datapack_by_id(&self.app.db, &id)
            .await?
            .is_some_and(|row| row.instance_id == *instance_id);
        if !owned {
            bail!(InvalidInstanceModIdError(instance_id, id));
        }
        let Some(icons) = wdb::get_world_datapack_icon_data(&self.app.db, &id).await? else {
            return Ok(None);
        };
        Ok(match platformid {
            0 => icons.local_data,
            1 => icons.cf_data,
            _ => icons.mr_data,
        })
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
        app.meta_cache_manager().world_scans_settled(id).await;
        let rows = rows(&app, id).await;
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].filename, "pack.zip");
    }

    #[tokio::test]
    async fn a_datapacks_file_holds_no_packs() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "datapacks-file", "World").await;
        let folder = world_datapacks_folder(&saves, "World");
        std::fs::remove_dir(&folder).unwrap();
        std::fs::write(&folder, b"not a folder").unwrap();
        assert!(listed_packs(&folder).await.unwrap().is_empty());
        assert!(!scan(&app, id).await);
        assert!(rows(&app, id).await.is_empty());
    }

    #[tokio::test]
    async fn shutdown_stops_a_world_scan() {
        use crate::managers::runtime_lifecycle::ShutdownPurpose;
        use std::time::Duration;
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "world-scan-quit", "World").await;
        let pack = world_datapacks_folder(&saves, "World").join("pack.zip");
        std::fs::write(&pack, pack_zip("held")).unwrap();
        let pause = app.meta_cache_manager().arm_scan_pause(&pack, "hash");
        let scan_app = app.clone();
        let scan =
            tokio::spawn(async move { scan_app.instance_manager().scan_world_datapacks(id).await });
        tokio::time::timeout(Duration::from_secs(5), pause.entered.notified())
            .await
            .expect("the scan reaches the pack");
        app.runtime_lifecycle
            .prepare_shutdown("world-scan-quit", ShutdownPurpose::Quit, async { Ok(()) })
            .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(1), scan)
            .await
            .expect("a cancelled scan returns promptly")
            .unwrap();
        assert!(result.is_err());
        assert!(rows(&app, id).await.is_empty());
    }

    /// The world scan runs after the instance pass is released: a pause that
    /// comes while it runs neither discards the finished pass nor runs it
    /// again.
    #[tokio::test]
    async fn a_pause_during_the_world_scan_does_not_run_the_instance_pass_again() {
        use std::time::Duration;
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_with_world(&app, "paused-world-scan", "World").await;
        let mcm = app.meta_cache_manager();
        let entity = CacheEntityId::Instance(id);
        mcm.override_caching_and_wait(entity, false, false)
            .await
            .unwrap();
        mcm.world_scans_settled(id).await;
        let passes =
            || mcm.scan_observations(Path::new(&format!("instance-{}", *id)), "cache-read");
        let before = passes();

        let pack = world_datapacks_folder(&saves, "World").join("pack.zip");
        std::fs::write(&pack, pack_zip("paused")).unwrap();
        let pause = mcm.arm_scan_pause(&pack, "hash");
        mcm.queue_caching(entity, false).await;
        tokio::time::timeout(Duration::from_secs(5), pause.entered.notified())
            .await
            .expect("the world scan reaches the pack");
        assert_eq!(passes(), before + 1);

        let running = &app.instance_manager().any_instance_running;
        running.send_replace(true);
        mcm.wait_scans_paused().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        running.send_replace(false);
        pause.release.notify_one();
        tokio::time::timeout(Duration::from_secs(5), async {
            while rows(&app, id).await.is_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the world scan finishes");
        mcm.world_scans_settled(id).await;
        assert_eq!(passes(), before + 1, "the instance pass ran once");
    }

    /// An instance with a held world scan whose pack `pack.zip` is waiting
    /// at its hash boundary.
    async fn held_world_scan(
        app: &App,
        name: &str,
    ) -> (
        InstanceId,
        std::sync::Arc<crate::managers::metadata::cache::ScanPause>,
    ) {
        use std::time::Duration;
        let (id, saves) = instance_with_world(app, name, "World").await;
        let mcm = app.meta_cache_manager();
        mcm.override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
            .unwrap();
        mcm.world_scans_settled(id).await;
        let pack = world_datapacks_folder(&saves, "World").join("pack.zip");
        std::fs::write(&pack, pack_zip(name)).unwrap();
        let pause = mcm.arm_scan_pause(&pack, "hash");
        tokio::time::timeout(
            Duration::from_secs(5),
            mcm.override_caching_and_wait(CacheEntityId::Instance(id), false, false),
        )
        .await
        .expect("the instance pass does not wait for the world scan")
        .unwrap();
        tokio::time::timeout(Duration::from_secs(5), pause.entered.notified())
            .await
            .expect("the world scan reaches the pack");
        (id, pause)
    }

    /// A platform pass waits for its instance's world scan only so long, and
    /// one of another instance does not wait for it at all.
    #[tokio::test]
    async fn a_held_world_scan_holds_the_platform_passes_of_its_instance_only_briefly() {
        use crate::managers::metadata::cache::WORLD_SCAN_WAIT;
        use std::time::{Duration, Instant};
        let app = crate::setup_managers_for_test().await;
        let (id, pause) = held_world_scan(&app, "held-platform-passes").await;
        let (other, _) = instance_with_world(&app, "other-platform-passes", "World").await;
        let mcm = app.meta_cache_manager();

        tokio::time::timeout(
            Duration::from_millis(100),
            mcm.wait_for_world_scans(CacheEntityId::Instance(other)),
        )
        .await
        .expect("another instance does not wait");
        tokio::time::timeout(
            Duration::from_millis(100),
            mcm.wait_for_world_scans(CacheEntityId::Server(1)),
        )
        .await
        .expect("nor does a server");
        let started_own = Instant::now();
        tokio::time::timeout(
            Duration::from_secs(5),
            mcm.wait_for_world_scans(CacheEntityId::Instance(id)),
        )
        .await
        .expect("its own instance goes on within the bound");
        assert!(started_own.elapsed() >= WORLD_SCAN_WAIT - Duration::from_millis(100));
        assert!(rows(&app, id).await.is_empty(), "the scan is still held");

        pause.release.notify_one();
        tokio::time::timeout(Duration::from_secs(5), mcm.world_scans_settled(id))
            .await
            .expect("the scan finishes once released");
        assert_eq!(rows(&app, id).await.len(), 1);
    }

    /// Packs a world scan finds after the platform passes went on without
    /// them are asked about by the passes it queues when it finishes.
    #[tokio::test]
    async fn packs_found_after_the_platform_passes_went_on_are_asked_about_next() {
        use std::time::Duration;
        let app = crate::setup_managers_for_test().await;
        let (id, pause) = held_world_scan(&app, "late-world-rows").await;
        let mcm = app.meta_cache_manager();
        let entity = CacheEntityId::Instance(id);
        tokio::time::timeout(Duration::from_secs(5), mcm.wait_for_world_scans(entity))
            .await
            .expect("the platform passes go on without the held scan");
        let (cf, mr) = mcm.platform_requests(entity).await;

        pause.release.notify_one();
        tokio::time::timeout(Duration::from_secs(5), mcm.world_scans_settled(id))
            .await
            .expect("the scan finishes once released");
        assert_eq!(rows(&app, id).await.len(), 1);
        tokio::time::timeout(Duration::from_secs(5), async {
            while mcm.platform_requests(entity).await != (cf + 1, mr + 1) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("both platform passes are queued for the instance");

        // A scan that changes nothing queues nothing.
        mcm.override_caching_and_wait(entity, false, false)
            .await
            .unwrap();
        mcm.world_scans_settled(id).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(mcm.platform_requests(entity).await, (cf + 1, mr + 1));
    }

    /// An instance at 1.20.1 running `loaders`, with its `saves` folder.
    async fn listing_instance(
        app: &App,
        name: &str,
        loaders: &[crate::domain::instance::info::ModLoaderType],
    ) -> (InstanceId, PathBuf) {
        use crate::domain::instance::info::{GameVersion, ModLoader, StandardVersion};
        use crate::managers::instance::InstanceVersionSource;
        let group = app.instance_manager().get_default_group().await.unwrap();
        let id = app
            .instance_manager()
            .create_instance(
                group,
                name.to_owned(),
                false,
                InstanceVersionSource::Version(GameVersion::Standard(StandardVersion {
                    release: "1.20.1".into(),
                    modloaders: loaders
                        .iter()
                        .map(|type_| ModLoader {
                            type_: *type_,
                            version: "fixture".into(),
                        })
                        .collect(),
                })),
                String::new(),
            )
            .await
            .unwrap();
        let saves = publication::current_path(app, id)
            .await
            .unwrap()
            .get_saves_path();
        std::fs::create_dir_all(&saves).unwrap();
        (id, saves)
    }

    /// The `worlds` addon row of `world`, as the local pass keeps one for
    /// every folder in `saves`. Returns its id.
    async fn world_row(app: &App, id: InstanceId, world: &str) -> String {
        use carbon_repos::repos::{mod_file_cache as mfcdb, mod_metadata as metarepo};
        let metadata_id = format!("{}-{world}-metadata", *id);
        let now = DbDateTime(Utc::now().fixed_offset());
        metarepo::insert_metadata(
            &app.db,
            &metadata_id,
            0,
            metadata_id.as_bytes(),
            metadata_id.as_bytes(),
            "",
            None,
            None,
            None,
            None,
            None,
            now,
        )
        .await
        .unwrap();
        mfcdb::upsert_mod_file_cache(
            &app.db,
            *id,
            world.to_owned(),
            0,
            true,
            AddonType::Worlds.to_db_string().to_owned(),
            metadata_id,
            now,
        )
        .await
        .unwrap();
        mfcdb::get_mod_file_cache_by_instance_kind_filename(
            &app.db,
            *id,
            AddonType::Worlds.to_db_string(),
            world,
        )
        .await
        .unwrap()
        .unwrap()
        .id
    }

    /// A save `world` with its addon row and the zips `packs` (file name,
    /// pack description) in its `datapacks` folder.
    async fn listed_world(
        app: &App,
        id: InstanceId,
        saves: &Path,
        world: &str,
        packs: &[(&str, &str)],
    ) -> String {
        add_world(saves, world);
        for (filename, description) in packs {
            std::fs::write(
                world_datapacks_folder(saves, world).join(filename),
                pack_zip(description),
            )
            .unwrap();
        }
        world_row(app, id, world).await
    }

    async fn list(app: &App, id: InstanceId) -> Vec<World> {
        app.instance_manager().list_worlds(id).await.unwrap()
    }

    async fn pack_metadata(app: &App, id: InstanceId, world: &str, filename: &str) -> String {
        rows(app, id)
            .await
            .into_iter()
            .find(|row| row.world == world && row.filename == filename)
            .and_then(|row| row.metadata_id)
            .expect("the pack was scanned")
    }

    /// Modrinth knows the pack as a Release of project `project`.
    async fn known_on_modrinth(app: &App, metadata_id: &str, project: &str) {
        use carbon_repos::repos::mod_metadata as metarepo;
        let sha512 = metarepo::get_metadata_by_id(&app.db, metadata_id)
            .await
            .unwrap()
            .unwrap()
            .sha512;
        metarepo::upsert_mr_mod_cache(
            &app.db,
            hex::encode(sha512),
            project.into(),
            format!("{project}-installed"),
            format!("{project} title"),
            "1.0".into(),
            project.into(),
            String::new(),
            "someone".into(),
            carbon_platforms::ModChannel::Stable as i32,
            String::new(),
            "pack.zip".into(),
            "https://cdn.invalid/pack.zip".into(),
            DbDateTime(Utc::now().fixed_offset()),
            metadata_id.into(),
        )
        .await
        .unwrap();
    }

    /// CurseForge knows the pack as a Release with `update_paths`.
    async fn known_on_curseforge(app: &App, metadata_id: &str, name: &str, update_paths: &str) {
        carbon_repos::repos::mod_metadata::upsert_cf_mod_cache(
            &app.db,
            0,
            7,
            70,
            name.into(),
            "2.0".into(),
            "slug".into(),
            String::new(),
            "someone".into(),
            carbon_platforms::ModChannel::Stable as i32,
            update_paths.into(),
            DbDateTime(Utc::now().fixed_offset()),
            metadata_id.into(),
        )
        .await
        .unwrap();
    }

    /// The Modrinth update paths of the pack under the context key `key`.
    async fn modrinth_paths(app: &App, metadata_id: &str, key: &str, paths: &str) {
        carbon_repos::repos::mod_metadata::upsert_mr_update_context(
            &app.db,
            metadata_id,
            key,
            paths,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
    }

    const NEWER_DATA_PACK: &str = "1.20.1,datapack,stable";

    fn world_key() -> String {
        crate::managers::metadata::cache::eligibility::world_context("1.20.1").key(WORLD_DATAPACKS)
    }

    #[tokio::test]
    async fn a_world_without_packs_is_listed() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = listing_instance(&app, "list-empty-world", &[]).await;
        let addon_id = listed_world(&app, id, &saves, "World", &[]).await;
        let worlds = list(&app, id).await;
        assert_eq!(worlds.len(), 1, "{worlds:?}");
        assert_eq!(worlds[0].name, "World");
        assert_eq!(worlds[0].addon_id, addon_id);
        assert!(worlds[0].identity.is_some());
        assert!(worlds[0].datapacks.is_empty());
    }

    #[tokio::test]
    async fn the_list_gives_each_world_its_in_game_name_and_survives_one_it_cannot_read() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = listing_instance(&app, "list-titles", &[]).await;
        for world in ["Named", "Plain", "Unnamed"] {
            listed_world(&app, id, &saves, world, &[]).await;
        }
        let level_dat = |world: &str| saves.join(world).join("level.dat");
        std::fs::write(
            level_dat("Named"),
            super::super::world_level::level_dat_bytes(Some("My Survival")),
        )
        .unwrap();
        std::fs::write(
            level_dat("Unnamed"),
            super::super::world_level::level_dat_bytes(None),
        )
        .unwrap();

        let worlds = list(&app, id).await;
        let summary: Vec<_> = worlds
            .iter()
            .map(|w| (w.name.as_str(), w.title.as_deref(), w.identity.is_some()))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("Named", Some("My Survival"), true),
                ("Plain", None, true),
                ("Unnamed", None, true),
            ]
        );
    }

    #[tokio::test]
    async fn an_unrecognised_pack_has_no_name_and_no_update() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = listing_instance(&app, "list-unrecognised", &[]).await;
        listed_world(&app, id, &saves, "World", &[("pack.zip", "plain")]).await;
        let folder = world_datapacks_folder(&saves, "World").join("folder-pack");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("pack.mcmeta"), b"{}").unwrap();
        let worlds = list(&app, id).await;
        let packs = &worlds[0].datapacks;
        assert_eq!(packs.len(), 2, "{packs:?}");
        for pack in packs {
            assert_eq!(pack.name, None, "{pack:?}");
            assert_eq!(pack.version, None, "{pack:?}");
            assert!(pack.curseforge.is_none() && pack.modrinth.is_none());
            assert!(!pack.has_update, "{pack:?}");
        }
        let folder = packs.iter().find(|pack| pack.is_folder).unwrap();
        assert_eq!(folder.filename, "folder-pack");
        let zip = packs.iter().find(|pack| !pack.is_folder).unwrap();
        assert_eq!(zip.file_size as usize, pack_zip("plain").len());
    }

    #[tokio::test]
    async fn a_recognised_pack_with_a_newer_cached_build_has_an_update() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = listing_instance(&app, "list-recognised", &[]).await;
        listed_world(
            &app,
            id,
            &saves,
            "World",
            &[
                ("modrinth.zip", "on modrinth"),
                ("curseforge.zip", "on curseforge"),
            ],
        )
        .await;
        assert!(
            app.instance_manager()
                .scan_world_datapacks(id)
                .await
                .unwrap()
        );
        let on_modrinth = pack_metadata(&app, id, "World", "modrinth.zip").await;
        known_on_modrinth(&app, &on_modrinth, "listed").await;
        let on_curseforge = pack_metadata(&app, id, "World", "curseforge.zip").await;
        known_on_curseforge(&app, &on_curseforge, "Listed on CurseForge", "").await;
        let packs = list(&app, id).await.remove(0).datapacks;
        let modrinth = packs.iter().find(|p| p.filename == "modrinth.zip").unwrap();
        assert_eq!(modrinth.name.as_deref(), Some("listed title"));
        assert_eq!(modrinth.version.as_deref(), Some("1.0"));
        assert_eq!(modrinth.modrinth.as_ref().unwrap().project_id, "listed");
        assert!(!modrinth.has_update, "no newer build is cached yet");
        let curseforge = packs
            .iter()
            .find(|p| p.filename == "curseforge.zip")
            .unwrap();
        assert_eq!(curseforge.curseforge.as_ref().unwrap().project_id, 7);
        assert!(!curseforge.has_update);

        modrinth_paths(&app, &on_modrinth, &world_key(), NEWER_DATA_PACK).await;
        known_on_curseforge(
            &app,
            &on_curseforge,
            "Listed on CurseForge",
            "1.20.1,datapacks,stable",
        )
        .await;
        let packs = list(&app, id).await.remove(0).datapacks;
        assert!(packs.iter().all(|pack| pack.has_update), "{packs:?}");

        // An answer more than a day old is not trusted.
        carbon_repos::repos::mod_metadata::upsert_mr_update_context(
            &app.db,
            &on_modrinth,
            &world_key(),
            NEWER_DATA_PACK,
            DbDateTime((Utc::now() - chrono::Duration::days(2)).fixed_offset()),
        )
        .await
        .unwrap();
        let packs = list(&app, id).await.remove(0).datapacks;
        let modrinth = packs.iter().find(|p| p.filename == "modrinth.zip").unwrap();
        assert!(!modrinth.has_update);
    }

    #[tokio::test]
    async fn a_path_cached_for_the_instance_context_only_is_no_update() {
        use crate::domain::instance::info::ModLoaderType;
        use crate::managers::metadata::cache::modrinth::target_compatibility;
        let app = crate::setup_managers_for_test().await;
        let (id, saves) =
            listing_instance(&app, "list-instance-context", &[ModLoaderType::Fabric]).await;
        listed_world(&app, id, &saves, "World", &[("pack.zip", "fabric")]).await;
        assert!(
            app.instance_manager()
                .scan_world_datapacks(id)
                .await
                .unwrap()
        );
        let metadata = pack_metadata(&app, id, "World", "pack.zip").await;
        known_on_modrinth(&app, &metadata, "loader-build").await;
        let instance_key = target_compatibility(&app, CacheEntityId::Instance(id))
            .await
            .unwrap()
            .key("datapacks");
        assert_ne!(instance_key, world_key());
        // A build for the instance's loader, which a world never loads.
        modrinth_paths(&app, &metadata, &instance_key, "1.20.1,fabric,stable").await;
        modrinth_paths(&app, &metadata, &world_key(), "").await;
        let packs = list(&app, id).await.remove(0).datapacks;
        assert!(!packs[0].has_update, "{packs:?}");
        // A data pack build under the world key is one.
        modrinth_paths(&app, &metadata, &world_key(), NEWER_DATA_PACK).await;
        assert!(list(&app, id).await[0].datapacks[0].has_update);
    }

    #[tokio::test]
    async fn a_blacklisted_platform_gives_no_update() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = listing_instance(&app, "list-blacklist", &[]).await;
        listed_world(&app, id, &saves, "World", &[("pack.zip", "blacklisted")]).await;
        assert!(
            app.instance_manager()
                .scan_world_datapacks(id)
                .await
                .unwrap()
        );
        let metadata = pack_metadata(&app, id, "World", "pack.zip").await;
        known_on_modrinth(&app, &metadata, "blacklisted").await;
        modrinth_paths(&app, &metadata, &world_key(), NEWER_DATA_PACK).await;
        assert!(list(&app, id).await[0].datapacks[0].has_update);
        app.settings_manager()
            .set(
                carbon_repos::repos::app_configuration::AppConfigurationPatch {
                    mod_platform_blacklist: Some("modrinth".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let pack = list(&app, id).await.remove(0).datapacks.remove(0);
        assert!(pack.modrinth.is_some(), "still shown as known");
        assert!(!pack.has_update);
    }

    #[tokio::test]
    async fn a_folder_without_level_dat_is_listed_without_identity_or_packs() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = listing_instance(&app, "list-no-level-dat", &[]).await;
        let folder = world_datapacks_folder(&saves, "NotASave");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("pack.zip"), pack_zip("orphan")).unwrap();
        let addon_id = world_row(&app, id, "NotASave").await;
        let worlds = list(&app, id).await;
        assert_eq!(worlds.len(), 1, "{worlds:?}");
        assert_eq!(worlds[0].addon_id, addon_id);
        assert_eq!(worlds[0].identity, None);
        assert!(worlds[0].datapacks.is_empty());
    }

    /// A data pack zip that also carries a `fabric.mod.json` naming it.
    fn pack_zip_with_fabric_metadata(name: &str) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("pack.mcmeta", options).unwrap();
        write!(zip, r#"{{"pack":{{"pack_format":15,"description":"x"}}}}"#).unwrap();
        zip.start_file("fabric.mod.json", options).unwrap();
        write!(
            zip,
            r#"{{"schemaVersion":1,"id":"jarlike","version":"9.9","name":"{name}"}}"#
        )
        .unwrap();
        zip.finish().unwrap().into_inner()
    }

    /// Only a platform names a pack: one whose own metadata names it but no
    /// platform knows is shown and sorted by its file name.
    #[tokio::test]
    async fn a_pack_named_only_by_its_own_metadata_is_unrecognised() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = listing_instance(&app, "list-jar-metadata", &[]).await;
        listed_world(&app, id, &saves, "World", &[("x.zip", "plain")]).await;
        std::fs::write(
            world_datapacks_folder(&saves, "World").join("y-fabric.zip"),
            pack_zip_with_fabric_metadata("Aardvark"),
        )
        .unwrap();
        assert!(
            app.instance_manager()
                .scan_world_datapacks(id)
                .await
                .unwrap()
        );
        let metadata = pack_metadata(&app, id, "World", "y-fabric.zip").await;
        let own = carbon_repos::repos::mod_metadata::get_metadata_by_id(&app.db, &metadata)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            own.name.as_deref(),
            Some("Aardvark"),
            "the zip names itself"
        );
        let packs = list(&app, id).await.remove(0).datapacks;
        let filenames: Vec<_> = packs.iter().map(|pack| pack.filename.as_str()).collect();
        assert_eq!(filenames, ["x.zip", "y-fabric.zip"]);
        let jarlike = &packs[1];
        assert_eq!(jarlike.name, None);
        assert_eq!(jarlike.version, None);
        assert!(jarlike.curseforge.is_none() && jarlike.modrinth.is_none());
        assert!(!jarlike.has_update);
    }

    #[tokio::test]
    async fn worlds_and_packs_are_sorted_by_the_name_they_show() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = listing_instance(&app, "list-sorted", &[]).await;
        listed_world(&app, id, &saves, "beta", &[]).await;
        listed_world(
            &app,
            id,
            &saves,
            "Alpha",
            &[("b.zip", "b"), ("Zulu.zip", "z"), ("a-known.zip", "known")],
        )
        .await;
        assert!(
            app.instance_manager()
                .scan_world_datapacks(id)
                .await
                .unwrap()
        );
        // Shown as "Middle", between "b.zip" and "Zulu.zip".
        let known = pack_metadata(&app, id, "Alpha", "a-known.zip").await;
        known_on_curseforge(&app, &known, "Middle", "").await;
        let worlds = list(&app, id).await;
        let names: Vec<_> = worlds.iter().map(|world| world.name.as_str()).collect();
        assert_eq!(names, ["Alpha", "beta"]);
        let packs: Vec<_> = worlds[0]
            .datapacks
            .iter()
            .map(|pack| pack.filename.as_str())
            .collect();
        assert_eq!(packs, ["b.zip", "a-known.zip", "Zulu.zip"]);
    }

    /// The list reads the cache only: it asks no platform, so it is
    /// answered whether they can be reached or not.
    #[tokio::test]
    async fn the_list_asks_no_platform() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = listing_instance(&app, "list-offline", &[]).await;
        listed_world(&app, id, &saves, "World", &[("pack.zip", "offline")]).await;
        assert!(
            app.instance_manager()
                .scan_world_datapacks(id)
                .await
                .unwrap()
        );
        let entity = CacheEntityId::Instance(id);
        let requests = app.meta_cache_manager().platform_requests(entity).await;
        let worlds = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            app.instance_manager().list_worlds(id),
        )
        .await
        .expect("the list does not wait for a platform")
        .unwrap();
        assert_eq!(worlds[0].datapacks.len(), 1);
        assert_eq!(
            app.meta_cache_manager().platform_requests(entity).await,
            requests
        );
    }

    #[tokio::test]
    async fn packs_the_list_finds_are_asked_about_by_the_platform_passes() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = listing_instance(&app, "list-queues-passes", &[]).await;
        listed_world(&app, id, &saves, "World", &[]).await;
        let entity = CacheEntityId::Instance(id);
        let mcm = app.meta_cache_manager();
        let (cf, mr) = mcm.platform_requests(entity).await;
        list(&app, id).await;
        assert_eq!(mcm.platform_requests(entity).await, (cf, mr), "nothing new");
        std::fs::write(
            world_datapacks_folder(&saves, "World").join("new.zip"),
            pack_zip("new"),
        )
        .unwrap();
        assert_eq!(list(&app, id).await[0].datapacks.len(), 1);
        assert_eq!(mcm.platform_requests(entity).await, (cf + 1, mr + 1));
    }

    fn worlds_invalidated(
        events: &mut tokio::sync::broadcast::Receiver<crate::api::InvalidationEvent>,
        id: InstanceId,
    ) -> bool {
        let mut seen = false;
        while let Ok(event) = events.try_recv() {
            seen |= event.key == GET_WORLDS.full && event.args == Some((*id).into());
        }
        seen
    }

    #[tokio::test]
    async fn a_scan_that_changes_rows_invalidates_the_worlds_but_the_list_does_not() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = listing_instance(&app, "list-invalidation", &[]).await;
        listed_world(&app, id, &saves, "World", &[]).await;
        let mut events = app.invalidation_channel.subscribe();
        assert!(
            !app.instance_manager()
                .scan_world_datapacks(id)
                .await
                .unwrap()
        );
        assert!(!worlds_invalidated(&mut events, id));

        let folder = world_datapacks_folder(&saves, "World");
        std::fs::write(folder.join("one.zip"), pack_zip("one")).unwrap();
        assert!(
            app.instance_manager()
                .scan_world_datapacks(id)
                .await
                .unwrap()
        );
        assert!(worlds_invalidated(&mut events, id));

        // The list's own scan finds the new pack and says nothing: its
        // answer holds it, so the frontend need not ask again.
        std::fs::write(folder.join("two.zip"), pack_zip("two")).unwrap();
        assert_eq!(list(&app, id).await[0].datapacks.len(), 2);
        assert!(!worlds_invalidated(&mut events, id));
    }

    #[tokio::test]
    async fn a_world_pack_icon_is_served_for_its_own_instance_only() {
        use carbon_repos::repos::mod_metadata as metarepo;
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = listing_instance(&app, "icon-owner", &[]).await;
        let (other, _) = listing_instance(&app, "icon-other", &[]).await;
        listed_world(&app, id, &saves, "World", &[("pack.zip", "icon")]).await;
        let folder = world_datapacks_folder(&saves, "World").join("folder-pack");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("pack.mcmeta"), b"{}").unwrap();
        assert!(
            app.instance_manager()
                .scan_world_datapacks(id)
                .await
                .unwrap()
        );
        let metadata = pack_metadata(&app, id, "World", "pack.zip").await;
        known_on_modrinth(&app, &metadata, "icon").await;
        metarepo::upsert_mr_image(&app.db, &metadata, "https://cdn.invalid/icon.png")
            .await
            .unwrap();
        metarepo::mark_mr_image_downloaded(&app.db, &metadata, b"png")
            .await
            .unwrap();
        let packs = list(&app, id).await.remove(0).datapacks;
        let zip = packs.iter().find(|pack| !pack.is_folder).unwrap();
        let folder = packs.iter().find(|pack| pack.is_folder).unwrap();
        assert!(zip.modrinth.as_ref().unwrap().has_image);

        let manager = app.instance_manager();
        assert_eq!(
            manager
                .get_world_datapack_icon(id, zip.id.clone(), 2)
                .await
                .unwrap()
                .as_deref(),
            Some(&b"png"[..])
        );
        assert_eq!(
            manager
                .get_world_datapack_icon(id, zip.id.clone(), 1)
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            manager
                .get_world_datapack_icon(id, folder.id.clone(), 2)
                .await
                .unwrap(),
            None
        );
        assert!(
            manager
                .get_world_datapack_icon(other, zip.id.clone(), 2)
                .await
                .is_err(),
            "a pack of another instance is refused"
        );
        assert!(
            manager
                .get_world_datapack_icon(id, "missing".into(), 2)
                .await
                .is_err()
        );
        assert!(
            manager
                .get_world_datapack_icon(id, zip.id.clone(), 3)
                .await
                .is_err()
        );
    }
}
