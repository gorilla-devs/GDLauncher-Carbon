//! Updating a data pack that sits in a world to its newer build, and updating
//! every copy of one data pack project in an instance at once. A world copy
//! is only ever updated when the user asks for that copy or for "everywhere":
//! the instance's own updates never reach the worlds.
use super::{
    InstanceId, InstanceManager, InvalidInstanceIdError,
    datapacks::world_identity,
    installer::publication,
    mods::{InvalidInstanceModIdError, resolve_mod_update_where},
    world_datapack_changes::{
        DeleteWorldDatapack, InstallWorldDatapack, WorldDatapackSource, is_curseforge_world_file,
        modrinth_world_file,
    },
    world_refusal::WorldRefusal,
};
use crate::{
    api::translation::Translation,
    domain::{
        instance::{
            AddonType,
            info::{GameVersion, StandardVersion},
        },
        vtask::VisualTaskId,
    },
    managers::{
        ManagerRef,
        metadata::cache::{current_file_stamp, eligibility::world_context},
        modplatforms::{curseforge::CurseForge, modrinth::Modrinth},
        vtask::{TaskOutcome, VisualTask},
    },
};
use anyhow::{Context, anyhow, bail};
use carbon_platforms::{RemoteVersion, curseforge, curseforge::filters::ModParameters};
use carbon_repos::repos::{
    mod_file_cache as mfcdb, mod_metadata as metarepo,
    world_datapack_cache::{self as wdb, WorldDatapackFullRow, WorldDatapackRow},
};
use serde::Deserialize;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

const NOT_A_DATAPACK: &str = "Only a data pack can be updated in every place it is in";

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct UpdateWorldDatapack {
    pub instance_id: i32,
    /// The world data pack's id.
    pub id: String,
}

/// The copy "update everywhere" was asked from.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub enum DatapackOrigin {
    /// A world data pack's id.
    World(String),
    /// An instance-level addon's id.
    Instance(String),
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct UpdateDatapackEverywhere {
    pub instance_id: i32,
    pub origin: DatapackOrigin,
}

/// How the update of one copy went.
#[derive(Debug, Clone, PartialEq)]
pub struct DatapackUpdateResult {
    /// The world's folder name; `None` for the instance's own copy.
    pub place: Option<String>,
    pub ok: bool,
    pub error: Option<String>,
    /// The code and data of the refusal behind `error`, when it has one.
    pub code: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl DatapackUpdateResult {
    fn updated(place: Option<String>) -> Self {
        Self {
            place,
            ok: true,
            error: None,
            code: None,
            data: None,
        }
    }

    fn failed(place: Option<String>, error: &anyhow::Error) -> Self {
        let (code, data) = crate::error::coded_cause(error).unzip();
        Self {
            place,
            ok: false,
            error: Some(format!("{error:#}")),
            code,
            data: data.flatten(),
        }
    }
}

/// An update of a world's pack once it started.
enum WorldUpdate {
    /// The newer build is being installed in place of the old one.
    Installing(
        VisualTaskId,
        tokio::task::JoinHandle<anyhow::Result<PathBuf>>,
    ),
    /// The newer build was in the world already; the old one was removed.
    AlreadyThere,
}

/// Starts the update of the instance's own copy of a data pack; production
/// passes `update_mod`.
type UpdateInstanceCopy<'a> = &'a (
        dyn Fn(
    InstanceId,
    String,
) -> Pin<Box<dyn Future<Output = anyhow::Result<VisualTaskId>> + Send + 'a>>
            + Send
            + Sync
    );

/// One copy "update everywhere" acts on.
enum DatapackCopy {
    World(WorldDatapackFullRow),
    Instance(String),
}

impl ManagerRef<'_, InstanceManager> {
    /// Updates one data pack in a world to its newest build for the world
    /// (a data pack build, whatever mod loader the instance runs), as a
    /// visual task whose id this returns.
    pub async fn update_world_datapack(
        self,
        request: UpdateWorldDatapack,
    ) -> anyhow::Result<VisualTaskId> {
        let platforms = self.app.modplatforms_manager();
        let id = InstanceId(request.instance_id);
        let name = self.world_datapack_name(id, &request.id).await?;
        match self
            .update_world_datapack_from(&platforms.curseforge, &platforms.modrinth, id, &request.id)
            .await?
        {
            WorldUpdate::Installing(task, _) => Ok(task),
            WorldUpdate::AlreadyThere => {
                // Done already; the frontend still follows a task.
                let task = VisualTask::new(Translation::InstanceTaskStageDatapack {
                    mod_name: name,
                    instance_name: self.instance_name(id).await?,
                });
                let task_id = self.app.task_manager().spawn_task(&task).await;
                task.complete();
                Ok(task_id)
            }
        }
    }

    /// Updates every copy of a data pack project in the instance that has an
    /// update: the copies in each world and the instance's own. Copies are
    /// of one project when their Modrinth or their CurseForge project is
    /// the same. Returns once each update ended, one result per copy that
    /// had one.
    pub async fn update_datapack_everywhere(
        self,
        request: UpdateDatapackEverywhere,
    ) -> anyhow::Result<Vec<DatapackUpdateResult>> {
        let platforms = self.app.modplatforms_manager();
        self.update_datapack_everywhere_from(
            &platforms.curseforge,
            &platforms.modrinth,
            request,
            &|id, mod_id| Box::pin(self.update_mod(id, mod_id)),
        )
        .await
    }

    async fn update_datapack_everywhere_from(
        self,
        curseforge: &CurseForge,
        modrinth: &Modrinth,
        request: UpdateDatapackEverywhere,
        update_instance_copy: UpdateInstanceCopy<'_>,
    ) -> anyhow::Result<Vec<DatapackUpdateResult>> {
        let id = InstanceId(request.instance_id);
        let mut results = Vec::new();
        for copy in self.datapack_copies(id, &request.origin).await? {
            let (place, outcome) = match copy {
                DatapackCopy::World(row) => (
                    Some(row.world),
                    self.update_world_copy(curseforge, modrinth, id, &row.id)
                        .await,
                ),
                DatapackCopy::Instance(mod_id) => (
                    None,
                    self.update_instance_copy(
                        curseforge,
                        modrinth,
                        id,
                        mod_id,
                        update_instance_copy,
                    )
                    .await,
                ),
            };
            match outcome {
                Ok(false) => {}
                Ok(true) => results.push(DatapackUpdateResult::updated(place)),
                Err(error) => results.push(DatapackUpdateResult::failed(place, &error)),
            }
        }
        Ok(results)
    }

    /// The copies of the origin's project in the instance: the origin, and
    /// every world pack and instance-level data pack that shares its
    /// Modrinth or its CurseForge project. Worlds first, by name.
    async fn datapack_copies(
        self,
        id: InstanceId,
        origin: &DatapackOrigin,
    ) -> anyhow::Result<Vec<DatapackCopy>> {
        let worlds = wdb::get_world_datapacks_full(&self.app.db, *id).await?;
        let instance = mfcdb::get_instance_mods_full(&self.app.db, *id).await?;
        let (mr, cf) = match origin {
            DatapackOrigin::World(row_id) => {
                let row = worlds
                    .iter()
                    .find(|row| &row.id == row_id)
                    .ok_or(WorldRefusal::PackGone)?;
                (row.mr_project_id.clone(), row.cf_project_id)
            }
            DatapackOrigin::Instance(mod_id) => {
                let row = instance
                    .iter()
                    .find(|row| &row.id == mod_id)
                    .ok_or_else(|| InvalidInstanceModIdError(id, mod_id.clone()))?;
                if row.addon_type != AddonType::DataPacks.to_db_string() {
                    bail!(NOT_A_DATAPACK);
                }
                (row.mr_project_id.clone(), row.cf_project_id)
            }
        };
        // A project no platform knows names no other copy.
        let same_project = |row_mr: &Option<String>, row_cf: Option<i32>| {
            (mr.is_some() && *row_mr == mr) || (cf.is_some() && row_cf == cf)
        };
        let mut copies: Vec<_> = worlds
            .into_iter()
            .filter(|row| {
                !row.is_folder
                    && (matches!(origin, DatapackOrigin::World(origin) if *origin == row.id)
                        || same_project(&row.mr_project_id, row.cf_project_id))
            })
            .collect();
        copies.sort_by(|a, b| (&a.world, &a.filename).cmp(&(&b.world, &b.filename)));
        let mut copies: Vec<_> = copies.into_iter().map(DatapackCopy::World).collect();
        copies.extend(
            instance
                .into_iter()
                .filter(|row| {
                    row.addon_type == AddonType::DataPacks.to_db_string()
                        && (matches!(origin, DatapackOrigin::Instance(origin) if *origin == row.id)
                            || same_project(&row.mr_project_id, row.cf_project_id))
                })
                .map(|row| DatapackCopy::Instance(row.id)),
        );
        Ok(copies)
    }

    /// Updates one world copy and waits for its install. `false` when it has
    /// no update.
    async fn update_world_copy(
        self,
        curseforge: &CurseForge,
        modrinth: &Modrinth,
        id: InstanceId,
        row_id: &str,
    ) -> anyhow::Result<bool> {
        let Some((row, update)) = self
            .resolve_world_datapack_update(curseforge, modrinth, id, row_id)
            .await?
        else {
            return Ok(false);
        };
        match self
            .apply_world_datapack_update(curseforge, modrinth, id, row, update)
            .await?
        {
            WorldUpdate::AlreadyThere => {}
            WorldUpdate::Installing(_, work) => {
                work.await
                    .map_err(|error| anyhow!("The data pack install stopped: {error}"))??;
            }
        }
        Ok(true)
    }

    /// Updates the instance's own copy as the Addons tab would, and waits
    /// for its task. `false` when it has no update.
    async fn update_instance_copy(
        self,
        curseforge: &CurseForge,
        modrinth: &Modrinth,
        id: InstanceId,
        mod_id: String,
        update: UpdateInstanceCopy<'_>,
    ) -> anyhow::Result<bool> {
        if self
            .find_mod_update_from(curseforge, modrinth, id, mod_id.clone())
            .await?
            .is_none()
        {
            return Ok(false);
        }
        let task = update(id, mod_id).await?;
        self.task_ended(task).await?;
        Ok(true)
    }

    /// Waits for a visual task to end; its failure is the error.
    async fn task_ended(self, task: VisualTaskId) -> anyhow::Result<()> {
        loop {
            match self.app.task_manager().get_task_outcome(task).await {
                TaskOutcome::Running(_) => {}
                TaskOutcome::Succeeded => return Ok(()),
                TaskOutcome::Failed(failed) => {
                    return Err(match failed.progress {
                        crate::domain::vtask::Progress::Failed(error) => {
                            crate::error::flattened(&error)
                        }
                        _ => anyhow!("The update failed"),
                    });
                }
                TaskOutcome::Dismissed => bail!("The update was stopped"),
                TaskOutcome::Unavailable => bail!("The update's outcome is not known"),
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// [`Self::update_world_datapack`] asking the given platform clients.
    async fn update_world_datapack_from(
        self,
        curseforge: &CurseForge,
        modrinth: &Modrinth,
        id: InstanceId,
        row_id: &str,
    ) -> anyhow::Result<WorldUpdate> {
        let (row, update) = self
            .resolve_world_datapack_update(curseforge, modrinth, id, row_id)
            .await?
            .ok_or(WorldRefusal::NoUpdate)?;
        self.apply_world_datapack_update(curseforge, modrinth, id, row, update)
            .await
    }

    /// The build that updates the world pack `row_id`, chosen as for an
    /// addon but for the world: the instance's game version, no mod loader,
    /// and only data pack builds. `None` when there is none.
    async fn resolve_world_datapack_update(
        self,
        curseforge: &CurseForge,
        modrinth: &Modrinth,
        id: InstanceId,
        row_id: &str,
    ) -> anyhow::Result<Option<(WorldDatapackRow, RemoteVersion)>> {
        self.ensure_world_datapacks_may_change(id).await?;
        let row = self
            .instance_world_datapack(id, row_id)
            .await?
            .ok_or(WorldRefusal::PackGone)?;
        let Some(ids) = wdb::get_world_datapack_update_ids(&self.app.db, row_id).await? else {
            return Ok(None);
        };
        let config = {
            let instances = self.instances.read().await;
            instances
                .get(&id)
                .ok_or(InvalidInstanceIdError(id))?
                .data()?
                .config
                .clone()
        };
        let Some(GameVersion::Standard(version)) = &config.game_configuration.version else {
            bail!(
                "Instance uses a custom game version file. Cannot resolve minecraft version for mod installation"
            );
        };
        let game = version.release.clone();
        // No mod loader: CurseForge is asked for every build of the game
        // version, and only the data pack ones are taken.
        let world = StandardVersion {
            release: game.clone(),
            modloaders: Default::default(),
        };
        let sources = self.instance_cfg_mod_sources(&config).await?;
        // Whether a CurseForge file is a data pack build depends on its
        // project's class.
        let cf_project: Option<curseforge::Mod> = match ids.cf_project_id {
            Some(project_id) => Some(
                curseforge
                    .get_mod(ModParameters { mod_id: project_id })
                    .await?
                    .data,
            ),
            None => None,
        };
        let update = resolve_mod_update_where(
            curseforge,
            modrinth,
            &ids,
            &world,
            "datapacks",
            &world_context(&game),
            &sources,
            |candidate| match candidate {
                RemoteVersion::Modrinth(version) => modrinth_world_file(version, &game).is_some(),
                RemoteVersion::Curseforge(file) => cf_project
                    .as_ref()
                    .is_some_and(|project| is_curseforge_world_file(project, file, &game)),
            },
        )
        .await?;
        Ok(update.map(|update| (row, update)))
    }

    /// Puts `update` in place of `row`. When the world already holds that
    /// very build beside it, the old pack is only removed.
    async fn apply_world_datapack_update(
        self,
        curseforge: &CurseForge,
        modrinth: &Modrinth,
        id: InstanceId,
        row: WorldDatapackRow,
        update: RemoteVersion,
    ) -> anyhow::Result<WorldUpdate> {
        // The list may be older than the disk: the pack is replaced or
        // removed only while it holds what the list showed.
        if !self.holds_recorded_file(id, &row).await? {
            bail!(WorldRefusal::PackChanged);
        }
        let mut present = false;
        for other in wdb::get_world_datapacks_full(&self.app.db, *id).await? {
            let same_build = other.world == row.world
                && other.id != row.id
                && match &update {
                    RemoteVersion::Modrinth(version) => {
                        other.mr_version_id.as_deref() == Some(version.id.as_str())
                    }
                    RemoteVersion::Curseforge(file) => other.cf_file_id == Some(file.id),
                };
            // A row whose file went or changed is no build in the world.
            if same_build
                && let Some(other) = self.instance_world_datapack(id, &other.id).await?
                && self.holds_recorded_file(id, &other).await?
            {
                present = true;
                break;
            }
        }
        if present {
            self.delete_world_datapack(DeleteWorldDatapack {
                instance_id: *id,
                id: row.id,
            })
            .await?;
            return Ok(WorldUpdate::AlreadyThere);
        }
        let path = publication::current_path(&self.app, id).await?;
        let saves = publication::checked_folder(&path, AddonType::Worlds).await?;
        let identity = world_identity(id, &path.get_root(), &saves.join(&row.world)).await?;
        let source = match update {
            RemoteVersion::Modrinth(version) => WorldDatapackSource::Modrinth {
                project_id: version.project_id,
                version_id: Some(version.id),
            },
            RemoteVersion::Curseforge(file) => WorldDatapackSource::Curseforge {
                project_id: file.mod_id.try_into()?,
                file_id: Some(file.id.try_into()?),
            },
        };
        let (task, work) = self
            .install_world_datapack_from(
                curseforge,
                modrinth,
                InstallWorldDatapack {
                    instance_id: *id,
                    world: row.world,
                    identity,
                    source,
                    replaces: Some(row.id),
                },
            )
            .await?;
        Ok(WorldUpdate::Installing(task, work))
    }

    /// Whether the entry `row` lists is still the file the scan recorded:
    /// the same stamp, as the scan judges "unchanged", or else the same
    /// bytes as the row's metadata. `false` when it is gone or no file.
    pub(super) async fn holds_recorded_file(
        self,
        id: InstanceId,
        row: &WorldDatapackRow,
    ) -> anyhow::Result<bool> {
        let Some(path) = self.world_datapack_entry(id, row).await? else {
            return Ok(false);
        };
        if !tokio::fs::metadata(&path)
            .await
            .is_ok_and(|meta| meta.is_file())
        {
            return Ok(false);
        }
        if row.file_stamp.is_some() && current_file_stamp(&path).await == row.file_stamp {
            return Ok(true);
        }
        // A new time alone (a copy, a touch) keeps the same bytes.
        let Some(metadata_id) = &row.metadata_id else {
            return Ok(false);
        };
        let Some(recorded) = metarepo::get_metadata_by_id(&self.app.db, metadata_id).await? else {
            return Ok(false);
        };
        Ok(
            crate::managers::instance::modpack::origin_check::hash_file_sha512(&path)
                .await
                .is_ok_and(|hash| hash.as_slice() == recorded.sha512.as_slice()),
        )
    }

    /// The name a world pack shows: its platform name, or its file name.
    async fn world_datapack_name(self, id: InstanceId, row_id: &str) -> anyhow::Result<String> {
        let row = wdb::get_world_datapacks_full(&self.app.db, *id)
            .await?
            .into_iter()
            .find(|row| row.id == row_id)
            .ok_or(WorldRefusal::PackGone)?;
        Ok(row.cf_name.or(row.mr_title).unwrap_or(row.filename))
    }

    async fn instance_name(self, id: InstanceId) -> anyhow::Result<String> {
        let instances = self.instances.read().await;
        Ok(instances
            .get(&id)
            .ok_or(InvalidInstanceIdError(id))?
            .data()?
            .config
            .name
            .clone())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::managers::App;
    use crate::managers::instance::world_datapack_changes::tests::{
        Api, CF_PROJECT, GAME, api, cf_file, cf_project, entries, modrinth_serving, mr_version,
        pack_zip, put_pack, row_of, scanned_rows, snapshot, staging_folders, tree,
        without_recycle_bin,
    };
    use carbon_platforms::ModChannel;
    use carbon_repos::{dbtypes::DbDateTime, repos::mod_metadata as metarepo};
    use serde_json::json;
    use std::path::Path;

    /// A Fabric instance at 1.20.1 with the saves `worlds`. Returns the
    /// instance and its `saves` folder.
    async fn fabric_instance(app: &App, name: &str, worlds: &[&str]) -> (InstanceId, PathBuf) {
        use crate::domain::instance::info::ModLoaderType;
        instance_running(app, name, &[ModLoaderType::Fabric], worlds).await
    }

    /// An instance at 1.20.1 running `loaders` (none: no mod loader) with
    /// the saves `worlds`. Returns the instance and its `saves` folder.
    async fn instance_running(
        app: &App,
        name: &str,
        loaders: &[crate::domain::instance::info::ModLoaderType],
        worlds: &[&str],
    ) -> (InstanceId, PathBuf) {
        use crate::domain::instance::info::ModLoader;
        use crate::managers::instance::InstanceVersionSource;
        let group = app.instance_manager().get_default_group().await.unwrap();
        let id = app
            .instance_manager()
            .create_instance(
                group,
                name.to_owned(),
                false,
                InstanceVersionSource::Version(GameVersion::Standard(StandardVersion {
                    release: GAME.into(),
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
        for world in worlds {
            std::fs::create_dir_all(saves.join(world).join("datapacks")).unwrap();
            std::fs::write(saves.join(world).join("level.dat"), b"save").unwrap();
        }
        (id, saves)
    }

    fn now() -> DbDateTime {
        DbDateTime(chrono::Utc::now().fixed_offset())
    }

    /// Modrinth knows the file of `metadata_id` as version `version_id` of
    /// `project`, a Release.
    pub(crate) async fn on_modrinth(app: &App, metadata_id: &str, project: &str, version_id: &str) {
        let sha512 = metarepo::get_metadata_by_id(&app.db, metadata_id)
            .await
            .unwrap()
            .unwrap()
            .sha512;
        metarepo::upsert_mr_mod_cache(
            &app.db,
            hex::encode(sha512),
            project.into(),
            version_id.into(),
            "World Pack".into(),
            version_id.into(),
            "world-pack".into(),
            String::new(),
            "someone".into(),
            ModChannel::Stable as i32,
            String::new(),
            "pack.zip".into(),
            "https://cdn.invalid/pack.zip".into(),
            now(),
            metadata_id.into(),
        )
        .await
        .unwrap();
    }

    /// CurseForge knows the file of `metadata_id` as file `file_id` of
    /// `CF_PROJECT`, a Release.
    async fn on_curseforge(app: &App, metadata_id: &str, file_id: i32) {
        metarepo::upsert_cf_mod_cache(
            &app.db,
            0,
            CF_PROJECT,
            file_id,
            "Curse Pack".into(),
            "1.0".into(),
            "curse-pack".into(),
            String::new(),
            "someone".into(),
            ModChannel::Stable as i32,
            String::new(),
            now(),
            metadata_id.into(),
        )
        .await
        .unwrap();
    }

    fn metadata(row: &WorldDatapackRow) -> &str {
        row.metadata_id.as_deref().expect("a zip has metadata")
    }

    /// `bytes` as the instance-level data pack `filename`, with its addon
    /// row, as the instance scan keeps one. Returns the row id, its
    /// metadata id and the file.
    async fn instance_pack(
        app: &App,
        id: InstanceId,
        filename: &str,
        bytes: &[u8],
    ) -> (String, String, PathBuf) {
        let path = publication::current_path(app, id).await.unwrap();
        let folder = AddonType::DataPacks.get_folder_path(&path);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(filename), bytes).unwrap();
        let cache = app.meta_cache_manager();
        let parsed = cache
            .hash_and_parse_mod_file(&folder, filename, true)
            .await
            .unwrap();
        let metadata_id = cache.ensure_mod_metadata(&parsed, filename).await.unwrap();
        let kind = AddonType::DataPacks.to_db_string();
        mfcdb::upsert_mod_file_cache(
            &app.db,
            *id,
            filename.to_owned(),
            bytes.len() as i32,
            true,
            kind.to_owned(),
            metadata_id.clone(),
            now(),
        )
        .await
        .unwrap();
        let row = mfcdb::get_mod_file_cache_by_instance_kind_filename(&app.db, *id, kind, filename)
            .await
            .unwrap()
            .unwrap();
        (row.id, metadata_id, folder.join(filename))
    }

    /// A Modrinth data pack version `id`, published on `day`, whose zip
    /// `filename` holds `bytes` at `url`.
    fn data_pack(
        id: &str,
        day: u32,
        loaders: &[&str],
        (filename, url, bytes): (&str, &str, &[u8]),
    ) -> serde_json::Value {
        mr_version(id, day, "release", loaders, (filename, url, bytes))
    }

    /// The installed build and a newer data pack build of the Modrinth
    /// project, the newer one served as `pack-2.zip`. Returns its bytes.
    async fn newer_on_modrinth(api: &mut Api, old: &[u8]) -> Vec<u8> {
        let new = pack_zip("new");
        let url = api.file("pack-2.zip", &new).await;
        modrinth_serving(
            api,
            vec![
                data_pack(
                    "installed",
                    5,
                    &["datapack"],
                    ("pack-1.zip", "https://cdn.invalid/pack-1.zip", old),
                ),
                data_pack("newer", 10, &["datapack"], ("pack-2.zip", &url, &new)),
            ],
        )
        .await;
        new
    }

    async fn update(
        app: &App,
        api: &Api,
        id: InstanceId,
        row: &str,
    ) -> anyhow::Result<Option<PathBuf>> {
        match app
            .instance_manager()
            .update_world_datapack_from(&api.curseforge, &api.modrinth, id, row)
            .await?
        {
            WorldUpdate::AlreadyThere => Ok(None),
            WorldUpdate::Installing(_, work) => Ok(Some(work.await??)),
        }
    }

    /// A stand-in for `update_mod` that records the addon it was asked to
    /// update and reports a task that succeeded.
    struct InstanceUpdates {
        app: App,
        asked: std::sync::Mutex<Vec<String>>,
    }

    impl InstanceUpdates {
        fn new(app: &App) -> Self {
            Self {
                app: app.clone(),
                asked: Default::default(),
            }
        }

        fn start(
            &self,
            mod_id: String,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<VisualTaskId>> + Send + 'static>> {
            self.asked.lock().unwrap().push(mod_id);
            let app = self.app.clone();
            Box::pin(async move {
                let task = VisualTask::new(Translation::InstanceTaskStageDatapack {
                    mod_name: "pack".into(),
                    instance_name: "instance".into(),
                });
                let id = app.task_manager().spawn_task(&task).await;
                task.complete();
                Ok(id)
            })
        }

        fn asked(&self) -> Vec<String> {
            self.asked.lock().unwrap().clone()
        }
    }

    async fn everywhere(
        app: &App,
        api: &Api,
        id: InstanceId,
        origin: DatapackOrigin,
        instance_updates: &InstanceUpdates,
    ) -> Vec<DatapackUpdateResult> {
        app.instance_manager()
            .update_datapack_everywhere_from(
                &api.curseforge,
                &api.modrinth,
                UpdateDatapackEverywhere {
                    instance_id: *id,
                    origin,
                },
                &|_, mod_id| instance_updates.start(mod_id),
            )
            .await
            .unwrap()
    }

    fn updated(place: Option<&str>) -> DatapackUpdateResult {
        DatapackUpdateResult::updated(place.map(str::to_owned))
    }

    fn only(folder: &Path, filename: &str, bytes: &[u8]) {
        assert_eq!(
            entries(folder),
            vec![(filename.to_owned(), bytes.to_vec())],
            "{folder:?}"
        );
    }

    /// On a Fabric instance, the newest builds are one tagged only for
    /// Fabric and one for Fabric and data packs whose file is a jar; the
    /// world gets the newest zip tagged as a data pack, which replaces the
    /// old one.
    #[tokio::test]
    async fn an_update_takes_the_newest_data_pack_build_never_one_for_a_mod_loader() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, saves) = fabric_instance(&app, "update-world-context", &["World"]).await;
        let world = saves.join("World");
        let old = pack_zip("old");
        put_pack(&world, "pack-1.zip", &old);
        let rows = scanned_rows(&app, id).await;
        on_modrinth(&app, metadata(&rows[0]), "project", "installed").await;
        let mut api = api().await;
        let new = pack_zip("new");
        let url = api.file("pack-2.zip", &new).await;
        let unwanted = pack_zip("for fabric");
        let fabric_url = api.file("fabric.zip", &unwanted).await;
        let jar_url = api.file("pack.jar", &unwanted).await;
        modrinth_serving(
            &mut api,
            vec![
                data_pack(
                    "installed",
                    5,
                    &["datapack"],
                    ("pack-1.zip", "https://cdn.invalid/pack-1.zip", &old),
                ),
                data_pack("newer", 10, &["datapack"], ("pack-2.zip", &url, &new)),
                data_pack(
                    "jar",
                    20,
                    &["datapack", "fabric"],
                    ("pack.jar", &jar_url, &unwanted),
                ),
                data_pack(
                    "fabric-only",
                    25,
                    &["fabric"],
                    ("fabric.zip", &fabric_url, &unwanted),
                ),
            ],
        )
        .await;

        let published = update(&app, &api, id, &rows[0].id).await.unwrap();

        assert_eq!(published, Some(world.join("datapacks/pack-2.zip")));
        only(&world.join("datapacks"), "pack-2.zip", &new);
        let rows = scanned_rows(&app, id).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].filename, "pack-2.zip");
    }

    /// With nothing newer than the installed build but one for a mod loader,
    /// the update answers as an addon's does and nothing changes.
    #[tokio::test]
    async fn an_update_without_a_newer_data_pack_build_changes_nothing() {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = fabric_instance(&app, "update-none", &["World"]).await;
        let world = saves.join("World");
        let old = pack_zip("old");
        put_pack(&world, "pack-1.zip", &old);
        let rows = scanned_rows(&app, id).await;
        on_modrinth(&app, metadata(&rows[0]), "project", "installed").await;
        let mut api = api().await;
        modrinth_serving(
            &mut api,
            vec![
                data_pack(
                    "installed",
                    5,
                    &["datapack"],
                    ("pack-1.zip", "https://cdn.invalid/pack-1.zip", &old),
                ),
                data_pack(
                    "fabric-only",
                    25,
                    &["fabric"],
                    ("fabric.zip", "https://cdn.invalid/fabric.zip", &old),
                ),
            ],
        )
        .await;
        let root = publication::current_path(&app, id)
            .await
            .unwrap()
            .get_root();
        let before = snapshot(&root, &world);

        let error = update(&app, &api, id, &rows[0].id).await.unwrap_err();

        assert_eq!(error.downcast_ref(), Some(&WorldRefusal::NoUpdate));
        assert_eq!(snapshot(&root, &world), before);
        assert_eq!(scanned_rows(&app, id).await, rows);
    }

    /// The newer build is already in the world beside the old one: the old
    /// one is removed and the newer one kept, with nothing downloaded.
    #[tokio::test]
    async fn the_newer_build_already_beside_the_old_one_is_kept() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, saves) = fabric_instance(&app, "update-already-there", &["World"]).await;
        let world = saves.join("World");
        let old = pack_zip("old");
        let new = pack_zip("new");
        put_pack(&world, "pack-1.zip", &old);
        put_pack(&world, "pack-2.zip", &new);
        let rows = scanned_rows(&app, id).await;
        let (old_row, new_row) = (
            row_of(&rows, "World", "pack-1.zip").clone(),
            row_of(&rows, "World", "pack-2.zip").clone(),
        );
        on_modrinth(&app, metadata(&old_row), "project", "installed").await;
        on_modrinth(&app, metadata(&new_row), "project", "newer").await;
        let mut api = api().await;
        // Not served for download: nothing may be fetched.
        modrinth_serving(
            &mut api,
            vec![
                data_pack(
                    "installed",
                    5,
                    &["datapack"],
                    ("pack-1.zip", "https://cdn.invalid/pack-1.zip", &old),
                ),
                data_pack(
                    "newer",
                    10,
                    &["datapack"],
                    ("pack-2.zip", "https://cdn.invalid/pack-2.zip", &new),
                ),
            ],
        )
        .await;
        let root = publication::current_path(&app, id)
            .await
            .unwrap()
            .get_root();

        assert_eq!(update(&app, &api, id, &old_row.id).await.unwrap(), None);

        only(&world.join("datapacks"), "pack-2.zip", &new);
        assert!(staging_folders(&root).is_empty());
        assert_eq!(scanned_rows(&app, id).await, vec![new_row]);
    }

    /// Updating one world's copy leaves the same pack in another world and
    /// at instance level as it is.
    #[tokio::test]
    async fn updating_one_copy_leaves_the_other_copies_alone() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, saves) = fabric_instance(&app, "update-one-copy", &["A", "B"]).await;
        let old = pack_zip("old");
        put_pack(&saves.join("A"), "pack-1.zip", &old);
        let other = put_pack(&saves.join("B"), "pack-1.zip", &old);
        let (_, instance_metadata, instance_file) =
            instance_pack(&app, id, "pack-1.zip", &old).await;
        let rows = scanned_rows(&app, id).await;
        for meta in [metadata(&rows[0]), &instance_metadata] {
            on_modrinth(&app, meta, "project", "installed").await;
        }
        let other_row = row_of(&rows, "B", "pack-1.zip").clone();
        let mut api = api().await;
        let new = newer_on_modrinth(&mut api, &old).await;

        update(&app, &api, id, &row_of(&rows, "A", "pack-1.zip").id)
            .await
            .unwrap();

        only(&saves.join("A/datapacks"), "pack-2.zip", &new);
        assert_eq!(std::fs::read(&other).unwrap(), old);
        assert_eq!(std::fs::read(&instance_file).unwrap(), old);
        let rows = scanned_rows(&app, id).await;
        assert_eq!(row_of(&rows, "B", "pack-1.zip"), &other_row);
    }

    /// From a world's copy: both worlds' copies and the instance-level copy
    /// are updated, one result each; a world whose copy is the newest build
    /// is left alone and not reported.
    #[tokio::test]
    async fn everywhere_updates_every_copy_that_has_an_update() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, saves) = fabric_instance(&app, "everywhere", &["A", "B", "C"]).await;
        let old = pack_zip("old");
        put_pack(&saves.join("A"), "pack-1.zip", &old);
        put_pack(&saves.join("B"), "pack-1.zip", &old);
        let mut api = api().await;
        let new = newer_on_modrinth(&mut api, &old).await;
        let current = put_pack(&saves.join("C"), "pack-2.zip", &new);
        let (instance_id, instance_metadata, _) = instance_pack(&app, id, "pack-1.zip", &old).await;
        let rows = scanned_rows(&app, id).await;
        on_modrinth(
            &app,
            metadata(row_of(&rows, "A", "pack-1.zip")),
            "project",
            "installed",
        )
        .await;
        on_modrinth(&app, &instance_metadata, "project", "installed").await;
        on_modrinth(
            &app,
            metadata(row_of(&rows, "C", "pack-2.zip")),
            "project",
            "newer",
        )
        .await;
        let instance_updates = InstanceUpdates::new(&app);

        let results = everywhere(
            &app,
            &api,
            id,
            DatapackOrigin::World(row_of(&rows, "A", "pack-1.zip").id.clone()),
            &instance_updates,
        )
        .await;

        assert_eq!(
            results,
            vec![updated(Some("A")), updated(Some("B")), updated(None)]
        );
        only(&saves.join("A/datapacks"), "pack-2.zip", &new);
        only(&saves.join("B/datapacks"), "pack-2.zip", &new);
        assert_eq!(std::fs::read(&current).unwrap(), new);
        assert_eq!(instance_updates.asked(), vec![instance_id]);
    }

    /// From the instance-level copy, the worlds' copies are updated too.
    #[tokio::test]
    async fn everywhere_from_the_instance_copy_updates_the_worlds() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, saves) = fabric_instance(&app, "everywhere-from-instance", &["A"]).await;
        let old = pack_zip("old");
        put_pack(&saves.join("A"), "pack-1.zip", &old);
        let (instance_id, instance_metadata, _) = instance_pack(&app, id, "pack-1.zip", &old).await;
        let rows = scanned_rows(&app, id).await;
        for meta in [metadata(&rows[0]), &instance_metadata] {
            on_modrinth(&app, meta, "project", "installed").await;
        }
        let mut api = api().await;
        let new = newer_on_modrinth(&mut api, &old).await;
        let instance_updates = InstanceUpdates::new(&app);

        let results = everywhere(
            &app,
            &api,
            id,
            DatapackOrigin::Instance(instance_id.clone()),
            &instance_updates,
        )
        .await;

        assert_eq!(results, vec![updated(Some("A")), updated(None)]);
        only(&saves.join("A/datapacks"), "pack-2.zip", &new);
        assert_eq!(instance_updates.asked(), vec![instance_id]);
    }

    /// A copy Modrinth does not know is still a copy when its CurseForge
    /// project is the origin's, and is updated from CurseForge.
    #[tokio::test]
    async fn everywhere_matches_a_copy_by_its_curseforge_project() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, saves) = fabric_instance(&app, "everywhere-curseforge", &["A", "B"]).await;
        let old = pack_zip("old");
        let other_old = pack_zip("old, from CurseForge");
        put_pack(&saves.join("A"), "pack-1.zip", &old);
        put_pack(&saves.join("B"), "curse-1.zip", &other_old);
        let rows = scanned_rows(&app, id).await;
        let (a, b) = (
            row_of(&rows, "A", "pack-1.zip").clone(),
            row_of(&rows, "B", "curse-1.zip").clone(),
        );
        on_modrinth(&app, metadata(&a), "project", "installed").await;
        on_curseforge(&app, metadata(&a), 100).await;
        // Another file of the project: one file has one cached owner.
        on_curseforge(&app, metadata(&b), 101).await;
        let mut api = api().await;
        let new = newer_on_modrinth(&mut api, &old).await;
        let curse_new = pack_zip("new, from CurseForge");
        let url = api.file("curse-2.zip", &curse_new).await;
        let installed = cf_file(
            100,
            4,
            1,
            ("pack-1.zip", "https://cdn.invalid/pack-1.zip", &old),
            0,
        );
        let other_installed = cf_file(
            101,
            5,
            1,
            ("curse-1.zip", "https://cdn.invalid/curse-1.zip", &other_old),
            0,
        );
        let newer = cf_file(200, 10, 1, ("curse-2.zip", &url, &curse_new), 0);
        api.get(
            &format!("/mods/{CF_PROJECT}"),
            json!({ "data": cf_project(6945) }),
        )
        .await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files"),
            json!({ "data": [installed.clone(), other_installed.clone(), newer.clone()] }),
        )
        .await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files/100"),
            json!({ "data": installed }),
        )
        .await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files/101"),
            json!({ "data": other_installed }),
        )
        .await;
        api.get(
            &format!("/mods/{CF_PROJECT}/files/200"),
            json!({ "data": newer }),
        )
        .await;
        let instance_updates = InstanceUpdates::new(&app);

        let results = everywhere(
            &app,
            &api,
            id,
            DatapackOrigin::World(a.id.clone()),
            &instance_updates,
        )
        .await;

        assert_eq!(results, vec![updated(Some("A")), updated(Some("B"))]);
        let a_now = entries(&saves.join("A/datapacks"));
        assert_eq!(a_now.len(), 1, "{a_now:?}");
        assert!(
            a_now[0] == ("pack-2.zip".to_owned(), new.clone())
                || a_now[0] == ("curse-2.zip".to_owned(), curse_new.clone()),
            "{a_now:?}"
        );
        only(&saves.join("B/datapacks"), "curse-2.zip", &curse_new);
        assert!(instance_updates.asked().is_empty());
    }

    /// A world that is no save any more refuses its update and reports why;
    /// the other worlds are updated all the same, and the refused one is as
    /// it was.
    #[tokio::test]
    async fn a_refused_place_reports_its_error_while_the_others_update() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, saves) = fabric_instance(&app, "everywhere-refused", &["A", "B", "C"]).await;
        let old = pack_zip("old");
        for world in ["A", "B", "C"] {
            put_pack(&saves.join(world), "pack-1.zip", &old);
        }
        let rows = scanned_rows(&app, id).await;
        on_modrinth(&app, metadata(&rows[0]), "project", "installed").await;
        let mut api = api().await;
        let new = newer_on_modrinth(&mut api, &old).await;
        // B is swapped for a folder that is no save.
        std::fs::rename(saves.join("B"), saves.join("B-old")).unwrap();
        put_pack(&saves.join("B"), "pack-1.zip", &old);
        let before = tree(&saves.join("B"));
        let instance_updates = InstanceUpdates::new(&app);

        let results = everywhere(
            &app,
            &api,
            id,
            DatapackOrigin::World(row_of(&rows, "A", "pack-1.zip").id.clone()),
            &instance_updates,
        )
        .await;

        assert_eq!(results.len(), 3, "{results:?}");
        assert_eq!(results[0], updated(Some("A")));
        assert_eq!(results[1].place.as_deref(), Some("B"));
        assert!(!results[1].ok);
        // In words, and by its code and data for the desktop to word.
        assert_eq!(
            results[1].error,
            Some(WorldRefusal::PackGone.to_string()),
            "{results:?}"
        );
        assert_eq!(results[1].code.as_deref(), Some("DATAPACK_GONE"));
        assert_eq!(results[1].data, None);
        assert_eq!(results[2], updated(Some("C")));
        only(&saves.join("A/datapacks"), "pack-2.zip", &new);
        only(&saves.join("C/datapacks"), "pack-2.zip", &new);
        assert_eq!(tree(&saves.join("B")), before);
    }

    /// The instance-level copy of a project a world also holds, updated
    /// as `update_mod` does it, against stand-in platforms: the update is
    /// found with `find_mod_update_from` and installed by the installer
    /// `update_mod` starts, replacing that addon. The newer build lands in
    /// the instance's own `datapacks` folder; the world's copy, its file
    /// and its row stay as they were. Asked with the world copy's id,
    /// `update_mod` is refused before any platform is asked.
    #[tokio::test]
    async fn an_instance_level_update_never_reaches_a_world_copy() {
        use crate::managers::instance::installer::{IntoInstaller, ModrinthModInstaller};
        use carbon_platforms::modrinth::search::ProjectID;
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = fabric_instance(&app, "update-mod-only-instance", &["A"]).await;
        let old = pack_zip("old");
        let world_file = put_pack(&saves.join("A"), "pack-1.zip", &old);
        let (instance_id, instance_metadata, instance_file) =
            instance_pack(&app, id, "pack-1.zip", &old).await;
        let rows = scanned_rows(&app, id).await;
        for meta in [metadata(&rows[0]), &instance_metadata] {
            on_modrinth(&app, meta, "project", "installed").await;
        }
        let mut api = api().await;
        let new = newer_on_modrinth(&mut api, &old).await;

        let Some(RemoteVersion::Modrinth(version)) = app
            .instance_manager()
            .find_mod_update_from(&api.curseforge, &api.modrinth, id, instance_id.clone())
            .await
            .unwrap()
        else {
            panic!("the newer Modrinth build is the update");
        };
        let project = api
            .modrinth
            .get_project(ProjectID(version.project_id.clone()))
            .await
            .unwrap();
        let task = ModrinthModInstaller::from_version(version, project)
            .unwrap()
            .into_installer()
            .install(&app, id, false, Some(instance_id))
            .await
            .unwrap();
        app.instance_manager().task_ended(task).await.unwrap();

        only(instance_file.parent().unwrap(), "pack-2.zip", &new);
        assert_eq!(std::fs::read(&world_file).unwrap(), old);
        assert_eq!(entries(&saves.join("A/datapacks")).len(), 1);
        assert_eq!(scanned_rows(&app, id).await, rows);
        let error = app
            .instance_manager()
            .update_mod(id, rows[0].id.clone())
            .await
            .unwrap_err();
        assert!(
            error.downcast_ref::<InvalidInstanceModIdError>().is_some(),
            "{error:#}"
        );
    }

    /// The instance's own copy of a project that also publishes mod jars:
    /// its update is the newest data pack build, never a newer jar or a
    /// build for the instance's mod loader, and it replaces the copy in the
    /// instance's `datapacks` folder.
    #[tokio::test]
    async fn an_instance_level_update_takes_the_newest_data_pack_build_never_a_jar() {
        use crate::managers::instance::installer::{IntoInstaller, ModrinthModInstaller};
        use carbon_platforms::modrinth::search::ProjectID;
        let app = crate::setup_managers_for_test().await;
        let (id, _) = fabric_instance(&app, "update-instance-pack", &[]).await;
        let old = pack_zip("old");
        let (instance_id, instance_metadata, instance_file) =
            instance_pack(&app, id, "pack-1.zip", &old).await;
        on_modrinth(&app, &instance_metadata, "project", "installed").await;
        let mut api = api().await;
        let new = pack_zip("new");
        let url = api.file("pack-2.zip", &new).await;
        let unwanted = pack_zip("for fabric");
        let fabric_url = api.file("fabric.zip", &unwanted).await;
        let jar_url = api.file("pack.jar", &unwanted).await;
        modrinth_serving(
            &mut api,
            vec![
                data_pack(
                    "installed",
                    5,
                    &["datapack"],
                    ("pack-1.zip", "https://cdn.invalid/pack-1.zip", &old),
                ),
                data_pack("newer", 10, &["datapack"], ("pack-2.zip", &url, &new)),
                data_pack(
                    "jar",
                    20,
                    &["datapack", "fabric"],
                    ("pack.jar", &jar_url, &unwanted),
                ),
                data_pack(
                    "fabric-only",
                    25,
                    &["fabric"],
                    ("fabric.zip", &fabric_url, &unwanted),
                ),
            ],
        )
        .await;

        let Some(RemoteVersion::Modrinth(version)) = app
            .instance_manager()
            .find_mod_update_from(&api.curseforge, &api.modrinth, id, instance_id.clone())
            .await
            .unwrap()
        else {
            panic!("the newer data pack build is the update");
        };
        assert_eq!(version.id, "newer");
        let project = api
            .modrinth
            .get_project(ProjectID(version.project_id.clone()))
            .await
            .unwrap();
        let task = ModrinthModInstaller::from_version(version, project)
            .unwrap()
            .into_installer()
            .install(&app, id, false, Some(instance_id))
            .await
            .unwrap();
        app.instance_manager().task_ended(task).await.unwrap();

        let folder = instance_file.parent().unwrap();
        only(folder, "pack-2.zip", &new);
        assert!(!folder.with_file_name("mods").join("pack-2.zip").exists());
    }

    /// The newer build's row is there but its file was deleted outside the
    /// launcher: the old pack is not just removed, the newer build is
    /// installed in its place.
    #[tokio::test]
    async fn a_newer_build_whose_file_is_gone_is_installed_again() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, saves) = fabric_instance(&app, "update-stale-row", &["World"]).await;
        let world = saves.join("World");
        let old = pack_zip("old");
        put_pack(&world, "pack-1.zip", &old);
        let mut api = api().await;
        let new = newer_on_modrinth(&mut api, &old).await;
        let gone = put_pack(&world, "pack-2.zip", &new);
        let rows = scanned_rows(&app, id).await;
        let old_row = row_of(&rows, "World", "pack-1.zip").clone();
        on_modrinth(&app, metadata(&old_row), "project", "installed").await;
        on_modrinth(
            &app,
            metadata(row_of(&rows, "World", "pack-2.zip")),
            "project",
            "newer",
        )
        .await;
        std::fs::remove_file(&gone).unwrap();

        let published = update(&app, &api, id, &old_row.id).await.unwrap();

        assert_eq!(published, Some(gone));
        only(&world.join("datapacks"), "pack-2.zip", &new);
    }

    /// One served newer build and a scanned, recognised `pack-1.zip` in
    /// World. Returns the instance, the world, the row and the API.
    async fn recognised_pack(
        app: &App,
        name: &str,
    ) -> (InstanceId, PathBuf, WorldDatapackRow, Api, Vec<u8>) {
        let (id, saves) = fabric_instance(app, name, &["World"]).await;
        let world = saves.join("World");
        let old = pack_zip("old");
        put_pack(&world, "pack-1.zip", &old);
        let rows = scanned_rows(app, id).await;
        on_modrinth(app, metadata(&rows[0]), "project", "installed").await;
        let mut api = api().await;
        let new = newer_on_modrinth(&mut api, &old).await;
        (id, world, rows[0].clone(), api, new)
    }

    fn installed_build(old: &[u8]) -> serde_json::Value {
        data_pack(
            "installed",
            5,
            &["datapack"],
            ("pack-1.zip", "https://cdn.invalid/pack-1.zip", old),
        )
    }

    fn as_version(version: &serde_json::Value) -> carbon_platforms::modrinth::version::Version {
        serde_json::from_value(version.clone()).unwrap()
    }

    /// `pack-1.zip` in World after a Modrinth lookup that knows it as the
    /// build `installed` and answered `newer`, published later, as the
    /// newest build. The stand-in API lists both.
    async fn looked_up_pack(
        app: &App,
        id: InstanceId,
        saves: &Path,
        api: &mut Api,
        newer: serde_json::Value,
    ) -> WorldDatapackRow {
        use crate::managers::metadata::cache::{
            eligibility::WORLD_DATAPACKS, modrinth::test::save_lookup,
        };
        let old = pack_zip("old");
        put_pack(&saves.join("World"), "pack-1.zip", &old);
        let row = scanned_rows(app, id).await.remove(0);
        let installed = installed_build(&old);
        modrinth_serving(api, vec![installed.clone(), newer.clone()]).await;
        save_lookup(
            app,
            id,
            metadata(&row),
            &[WORLD_DATAPACKS],
            as_version(&installed),
            vec![as_version(&newer)],
        )
        .await;
        row
    }

    /// Whether the Worlds list shows the update arrow on World's only pack.
    async fn update_arrow(app: &App, id: InstanceId) -> bool {
        use crate::managers::metadata::cache::CacheEntityId;
        // The list takes the worlds from the instance's addon rows.
        app.meta_cache_manager()
            .override_caching_and_wait(CacheEntityId::Instance(id), false, false)
            .await
            .unwrap();
        let worlds = app.instance_manager().list_worlds(id).await.unwrap();
        let world = worlds
            .iter()
            .find(|world| world.name == "World")
            .expect("World is listed");
        assert_eq!(world.datapacks.len(), 1, "{world:?}");
        assert!(world.datapacks[0].modrinth.is_some(), "{world:?}");
        world.datapacks[0].has_update
    }

    /// The arrow, Update and Update everywhere of a world pack whose newest
    /// build on Modrinth is one the world cannot take: no arrow, Update
    /// answers as for any addon without one, Update everywhere skips the
    /// pack, and the world stays as it was.
    async fn a_build_no_world_takes_is_no_update(name: &str, loaders: &[&str], filename: &str) {
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = fabric_instance(&app, name, &["World"]).await;
        let mut api = api().await;
        let unwanted = pack_zip("not for a world");
        let newer = data_pack(
            "newer",
            10,
            loaders,
            (filename, "https://cdn.invalid/newer", &unwanted),
        );
        let row = looked_up_pack(&app, id, &saves, &mut api, newer).await;
        let before = tree(&saves);

        assert!(!update_arrow(&app, id).await);
        let error = update(&app, &api, id, &row.id).await.unwrap_err();
        assert_eq!(error.downcast_ref(), Some(&WorldRefusal::NoUpdate));
        let instance_updates = InstanceUpdates::new(&app);
        let results = everywhere(
            &app,
            &api,
            id,
            DatapackOrigin::World(row.id.clone()),
            &instance_updates,
        )
        .await;
        assert_eq!(results, vec![]);
        assert_eq!(tree(&saves), before);
    }

    /// A project's author published a resource pack build (tagged
    /// `minecraft` only) after the data pack the world holds.
    #[tokio::test]
    async fn a_newer_resource_pack_build_is_no_update_for_a_world_pack() {
        a_build_no_world_takes_is_no_update("arrow-resource-pack", &["minecraft"], "pack-2.zip")
            .await;
    }

    /// The newer build is tagged as a data pack too, but its file is the
    /// mod jar.
    #[tokio::test]
    async fn a_newer_build_whose_file_is_a_jar_is_no_update_for_a_world_pack() {
        a_build_no_world_takes_is_no_update("arrow-jar", &["datapack", "fabric"], "pack-2.jar")
            .await;
    }

    /// A newer data pack zip shows the arrow, and both Update and Update
    /// everywhere install it.
    #[tokio::test]
    async fn a_newer_data_pack_build_is_an_update_the_actions_install() {
        for everywhere_asked in [false, true] {
            let app = crate::setup_managers_for_test().await;
            without_recycle_bin(&app).await;
            let (id, saves) = fabric_instance(&app, "arrow-data-pack", &["World"]).await;
            let mut api = api().await;
            let new = pack_zip("new");
            let url = api.file("pack-2.zip", &new).await;
            let newer = data_pack("newer", 10, &["datapack"], ("pack-2.zip", &url, &new));
            let row = looked_up_pack(&app, id, &saves, &mut api, newer).await;

            assert!(update_arrow(&app, id).await);
            if everywhere_asked {
                let instance_updates = InstanceUpdates::new(&app);
                let results = everywhere(
                    &app,
                    &api,
                    id,
                    DatapackOrigin::World(row.id.clone()),
                    &instance_updates,
                )
                .await;
                assert_eq!(results, vec![updated(Some("World"))]);
            } else {
                update(&app, &api, id, &row.id).await.unwrap();
            }
            only(&saves.join("World/datapacks"), "pack-2.zip", &new);
        }
    }

    /// On an instance without a mod loader the worlds are updated for what
    /// the instance itself is. The same file as the instance's own data pack
    /// and in a world still gets an answer for each, whichever is saved
    /// first: both show the arrow for a newer data pack build, and neither
    /// for a newer build tagged `minecraft`, which is a resource pack.
    #[tokio::test]
    async fn a_world_copy_and_the_instance_copy_each_get_their_answer_without_a_mod_loader() {
        use crate::managers::metadata::cache::{
            eligibility::WORLD_DATAPACKS, modrinth::test::save_lookup,
        };
        let app = crate::setup_managers_for_test().await;
        let (id, saves) = instance_running(&app, "arrow-no-loader", &[], &["World"]).await;
        let old = pack_zip("old");
        put_pack(&saves.join("World"), "pack-1.zip", &old);
        let (instance_row, instance_metadata, _) =
            instance_pack(&app, id, "pack-1.zip", &old).await;
        let row = scanned_rows(&app, id).await.remove(0);
        assert_eq!(metadata(&row), instance_metadata, "one file, one metadata");
        for (tag, offered) in [("minecraft", false), ("datapack", true)] {
            let newer = data_pack(
                "newer",
                10,
                &[tag],
                ("pack-2.zip", "https://cdn.invalid/pack-2.zip", &old),
            );
            // Each order the two answers can be saved in.
            for kinds in [
                ["datapacks", WORLD_DATAPACKS],
                [WORLD_DATAPACKS, "datapacks"],
            ] {
                save_lookup(
                    &app,
                    id,
                    &instance_metadata,
                    &kinds,
                    as_version(&installed_build(&old)),
                    vec![as_version(&newer)],
                )
                .await;
                assert_eq!(update_arrow(&app, id).await, offered, "{tag} {kinds:?}");
                let listed = app.instance_manager().list_mods(id, None).await.unwrap();
                let own = listed
                    .iter()
                    .find(|addon| addon.id == instance_row)
                    .expect("the instance's copy is listed");
                assert_eq!(own.has_update, offered, "{tag} {kinds:?}");
            }
        }
    }

    /// The pack's bytes were swapped outside the launcher under the same
    /// name: the update is refused and the file kept.
    #[tokio::test]
    async fn a_pack_swapped_on_disk_is_not_updated() {
        let app = crate::setup_managers_for_test().await;
        let (id, world, row, api, _) = recognised_pack(&app, "update-swapped-bytes").await;
        let swapped = pack_zip("the user's own build");
        put_pack(&world, "pack-1.zip", &swapped);
        let root = publication::current_path(&app, id)
            .await
            .unwrap()
            .get_root();
        let before = snapshot(&root, &world);

        let error = update(&app, &api, id, &row.id).await.unwrap_err();

        assert_eq!(error.downcast_ref(), Some(&WorldRefusal::PackChanged));
        assert_eq!(snapshot(&root, &world), before);
        only(&world.join("datapacks"), "pack-1.zip", &swapped);
    }

    /// The world was replaced by another save of the same name holding a
    /// different `pack-1.zip`: refused, and nothing changes.
    #[tokio::test]
    async fn a_world_replaced_by_another_save_is_not_updated() {
        let app = crate::setup_managers_for_test().await;
        let (id, world, row, api, _) = recognised_pack(&app, "update-swapped-world").await;
        let saves = world.parent().unwrap().to_path_buf();
        std::fs::rename(&world, saves.join("World-old")).unwrap();
        std::fs::create_dir_all(world.join("datapacks")).unwrap();
        std::fs::write(world.join("level.dat"), b"another save").unwrap();
        put_pack(&world, "pack-1.zip", &pack_zip("another save's pack"));
        let before = tree(&saves);

        let error = update(&app, &api, id, &row.id).await.unwrap_err();

        assert_eq!(error.downcast_ref(), Some(&WorldRefusal::PackChanged));
        assert_eq!(tree(&saves), before);
    }

    /// A new modification time with the same bytes is the same pack: the
    /// update goes through.
    #[tokio::test]
    async fn a_pack_only_touched_on_disk_is_updated() {
        let app = crate::setup_managers_for_test().await;
        without_recycle_bin(&app).await;
        let (id, world, row, api, new) = recognised_pack(&app, "update-touched").await;
        let pack = world.join("datapacks/pack-1.zip");
        std::fs::File::options()
            .write(true)
            .open(&pack)
            .unwrap()
            .set_modified(
                std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_600_000_000),
            )
            .unwrap();
        assert_ne!(
            current_file_stamp(&pack).await,
            row.file_stamp,
            "the stamp moved on"
        );

        update(&app, &api, id, &row.id).await.unwrap();

        only(&world.join("datapacks"), "pack-2.zip", &new);
    }
}
