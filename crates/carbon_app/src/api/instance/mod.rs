use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use axum::body::StreamBody;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use hyper::http::HeaderValue;
use hyper::{HeaderMap, StatusCode};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

use crate::api::modplatforms::RemoteVersion;
use crate::error::{AxumError, FeError};
use crate::managers::instance::log::LogEntrySourceKind;
use crate::managers::instance::InstanceMoveTarget;
use crate::managers::{instance::importer, App, AppInner};

use super::keys::instance::*;
use super::router::router;
use super::translation::Translation;
use super::vtask::FETaskId;
use super::Set;

use crate::domain::instance::{self as domain, InstanceModpackInfo};
use crate::domain::modplatforms as mpdomain;
use crate::managers::instance as manager;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query DEFAULT_GROUP[app, args: ()] {
            Ok(*app.instance_manager()
                .get_default_group()
                .await?)
        }

        query GET_GROUPS[app, args: ()] {
            Ok(app.instance_manager()
                .list_groups()
                .await?
                .into_iter()
                .map(ListGroup::from)
                .collect::<Vec<_>>())
        }

        query GET_ALL_INSTANCES[app, args: ()] {
            Ok(app.instance_manager()
                .list_groups()
                .await?
                .into_iter()
                .flat_map(|group| group.instances.into_iter().map(ListInstance::from))
                .collect::<Vec<_>>())
        }

        mutation CREATE_GROUP[app, name: String] {
            app.instance_manager()
                .create_group(name)
                .await
                .map(FEGroupId::from)
        }

        mutation CREATE_INSTANCE[app, details: CreateInstance] {
            if details.name.is_empty() {
                return Err(anyhow::anyhow!("instance name cannot be empty"));
            }

            app.instance_manager()
                .create_instance(
                    details.group.into(),
                    details.name,
                    details.use_loaded_icon,
                    details.version.try_into()?,
                    details.notes,
                )
                .await
                .map(FEInstanceId::from)
        }

        mutation CHANGE_MODPACK[app, details: ChangeModpack] {
            app.instance_manager()
                .change_modpack(
                    details.instance.into(),
                    details.modpack.into(),
                )
                .await
                .map(FETaskId::from)
        }

        mutation LOAD_ICON_URL[app, url: String] {
            let icon = app.instance_manager()
                .download_icon(url)
                .await?;

            app.instance_manager().set_loaded_icon(icon).await;
            Ok(())
        }

        mutation DELETE_GROUP[app, id: FEGroupId] {
            app.instance_manager()
                .delete_group(id.into())
                .await
        }

        mutation DELETE_INSTANCE[app, id: FEInstanceId] {
            app.instance_manager()
                .delete_instance(id.into())
                .await
        }

        mutation MOVE_GROUP[app, move_data: MoveGroup] {
            app.instance_manager()
                .move_group(
                    move_data.group.into(),
                    move_data.before.map(Into::into)
                )
                .await
        }

        mutation MOVE_INSTANCE[app, move_instance: MoveInstance] {
            app.instance_manager()
                .move_instance(
                    move_instance.instance.into(),
                    match move_instance.target {
                        MoveInstanceTarget::BeforeInstance(instance)
                            => InstanceMoveTarget::Before(instance.into()),
                        MoveInstanceTarget::BeginningOfGroup(group)
                            => InstanceMoveTarget::BeginningOfGroup(group.into()),
                        MoveInstanceTarget::EndOfGroup(group)
                            => InstanceMoveTarget::EndOfGroup(group.into()),
                    }
                )
                .await
        }

        mutation DUPLICATE_INSTANCE[app, details: DuplicateInstance] {
            app.instance_manager()
                .duplicate_instance(
                    details.instance.into(),
                    details.new_name,
                )
                .await
                .map(FEInstanceId::from)
        }

        mutation UPDATE_INSTANCE[app, details: FEUpdateInstance] {
            app.instance_manager()
                .update_instance(details.try_into()?)
                .await
        }

        mutation SET_FAVORITE[app, favorite: SetFavorite] {
            app.instance_manager()
                .set_favorite(
                    favorite.instance.into(),
                    favorite.favorite,
                )
                .await
        }

        query INSTANCE_DETAILS[app, id: Option<FEInstanceId>] {
            let Some(id) = id else {
                return Ok(None);
            };

            let result = app.instance_manager()
                .instance_details(id.into())
                .await
                .map(InstanceDetails::from);

            Ok(Some(result?))
        }

        query GET_MODPACK_INFO[app, id: Option<FEInstanceId>] {
            let Some(id) = id else {
                return Ok(None);
            };

            let result = app.instance_manager()
                .get_modpack_info(id.into())
                .await?
                .map(FEInstanceModpackInfo::from);

            Ok(result)
        }

        query INSTANCE_MODS[app, id: Option<FEInstanceId>] {
            let Some(id) = id else {
                return Ok(None);
            };

            app.meta_cache_manager()
                .watch_and_prioritize(Some(id.into()))
                .await;

            let result = app.instance_manager()
                .list_mods(id.into())
                .await?
                .into_iter()
                .map(Into::into)
                .collect::<Vec<Mod>>();

            Ok(Some(result))
        }

        mutation PREPARE_INSTANCE[app, id: FEInstanceId] {
            let (_, vtask_id) = app.instance_manager()
                .prepare_game(id.into(), None, None, true)
                .await?;

            Ok(FETaskId::from(vtask_id))
        }

        mutation LAUNCH_INSTANCE[app, id: FEInstanceId] {
            let account = app.account_manager()
                .get_active_account()
                .await?;

            let Some(account) = account else {
                return Err(anyhow::anyhow!("attempted to launch instance without an account"));
            };

            app.instance_manager()
                .prepare_game(id.into(), Some(account), None, false)
                .await?;

            Ok(())
        }

        mutation KILL_INSTANCE[app, id: FEInstanceId] {
            app.instance_manager()
                .kill_instance(id.into())
                .await
        }

        query GET_LOGS[app, args: ()] {
            Ok(app.instance_manager()
               .get_logs()
               .await
               .into_iter()
               .map(GameLogEntry::from)
               .collect::<Vec<_>>())
        }

        mutation DELETE_LOG[app, id: GameLogId] {
            app.instance_manager()
                .delete_log(id.into())
                .await
        }

        mutation ENABLE_MOD[app, imod: InstanceMod] {
            app.instance_manager()
                .enable_mod(
                    imod.instance_id.into(),
                    imod.mod_id,
                    true,
                )
                .await
        }

        mutation DISABLE_MOD[app, imod: InstanceMod] {
            app.instance_manager()
                .enable_mod(
                    imod.instance_id.into(),
                    imod.mod_id,
                    false,
                )
                .await
        }

        mutation DELETE_MOD[app, imod: InstanceMod] {
            app.instance_manager()
                .delete_mod(
                    imod.instance_id.into(),
                    imod.mod_id,
                )
                .await
        }

        mutation INSTALL_MOD[app, imod: InstallMod] {
            let task = match imod.mod_source {
                ModSource::Curseforge(cf_mod) => {
                    app.instance_manager()
                        .install_curseforge_mod(
                            imod.instance_id.into(),
                            cf_mod.project_id,
                            cf_mod.file_id,
                            imod.install_deps,
                            imod.replaces_mod,
                        )
                        .await?
                }
                ModSource::Modrinth(mdr_mod) => {
                    app.instance_manager()
                        .install_modrinth_mod(
                            imod.instance_id.into(),
                            mdr_mod.project_id,
                            mdr_mod.version_id,
                            imod.install_deps,
                            imod.replaces_mod,
                        )
                        .await?
                }
            };

            Ok(super::vtask::FETaskId::from(task))
        }

        mutation UPDATE_MOD[app, args: UpdateMod] {
            let task = app.instance_manager().update_mod(
                args.instance_id.into(),
                args.mod_id,
            ).await?;

            Ok(super::vtask::FETaskId::from(task))
        }

        query FIND_MOD_UPDATE[app, args: UpdateMod] {
            app.instance_manager().find_mod_update(
                args.instance_id.into(),
                args.mod_id,
            ).await
            .map(|v| v.map(RemoteVersion::from))
        }

        query GET_MOD_SOURCES[app, instance_id: FEInstanceId] {
            app.instance_manager()
                .get_instance_mod_sources(instance_id.into())
                .await
                .map(super::modplatforms::ModSources::from)
        }

        mutation INSTALL_LATEST_MOD[app, imod: InstallLatestMod] {
            let task = match imod.mod_source {
                LatestModSource::Curseforge(cf_mod) => {
                    app.instance_manager()
                        .install_latest_curseforge_mod(
                            imod.instance_id.into(),
                            cf_mod,
                        )
                        .await?
                }
                LatestModSource::Modrinth(mdr_mod) => {
                    app.instance_manager()
                        .install_latest_modrinth_mod(
                            imod.instance_id.into(),
                            mdr_mod
                        )
                        .await?
                }
            };

            Ok(super::vtask::FETaskId::from(task))
        }

        mutation OPEN_INSTANCE_FOLDER[app, folder: OpenInstanceFolder] {
            app.instance_manager().open_folder(
                folder.instance_id.into(),
                folder.folder.into(),
            )
            .await
        }

        query GET_IMPORTABLE_ENTITIES[_, _args: ()] {
            anyhow::Result::Ok(importer::Entity::list()
                .into_iter()
                .map(|(e, support, selection_type)| ImportEntityStatus {
                    entity: ImportEntity::from(e),
                    supported: support,
                    selection_type: ImportEntitySelectionType::from(selection_type),
                })
                .collect::<Vec<_>>())
        }

        query GET_IMPORT_ENTITY_DEFAULT_PATH[_, entity: ImportEntity] {
            importer::Entity::from(entity)
                .get_default_scan_path().await
        }

        mutation SET_IMPORT_SCAN_TARGET[app, target: (ImportEntity, String)] {
            app.instance_manager()
                .import_manager()
                .set_scan_target(Some((target.0.into(), PathBuf::from(target.1))))
        }

        mutation CANCEL_IMPORT_SCAN[app, args: ()] {
            app.instance_manager()
                .import_manager()
                .set_scan_target(None)
        }

        query GET_IMPORT_SCAN_STATUS[app, args: ()] {
            app.instance_manager()
                .import_manager()
                .scan_status()
                .await
                .map(FullImportScanStatus::from)
        }

        mutation IMPORT_INSTANCE[app, req: ImportRequest] {
            app.instance_manager()
                .import_manager()
                .begin_import(req.index, req.name)
                .await
                .map(FETaskId::from)
        }

        query EXPLORE[app, args: ExploreQuery] {
            app.instance_manager().explore_data(
                args.instance_id.into(),
                args.path,
            ).await
                .map(|entries| entries.into_iter().map(ExploreEntry::from).collect::<Vec<_>>())
        }

        mutation EXPORT[app, args: ExportArgs] {
            let task = app.instance_manager()
                .export_manager()
                .export_instance(
                    args.instance_id.into(),
                    args.target.into(),
                    args.save_path.into(),
                    args.link_mods,
                    args.filter.into(),
                ).await?;

            Ok(FETaskId::from(task))
        }
    }
}

pub(super) fn mount_axum_router() -> axum::Router<Arc<AppInner>> {
    #[derive(Deserialize)]
    struct InstanceIconQuery {
        id: i32,
        rev: Option<i32>,
    }

    #[derive(Deserialize)]
    struct ModpackIconQuery {
        instance_id: i32,
    }

    #[derive(Deserialize)]
    struct ModIconQuery {
        instance_id: i32,
        mod_id: String,
        platform: String,
    }

    #[derive(Deserialize)]
    struct IconPathQuery {
        path: String,
    }

    axum::Router::new()
        .route(
            "/instanceIcon",
            axum::routing::get(
                |State(app): State<Arc<AppInner>>, Query(query): Query<InstanceIconQuery>| async move {
                    let icon = app
                        .instance_manager()
                        .instance_icon(domain::InstanceId(query.id))
                        .await
                        .map_err(|e| FeError::from_anyhow(&e).make_axum())?;

                    Ok::<_, AxumError>(match icon {
                        Some((name, icon)) => {
                            let mut headers = HeaderMap::new();
                            headers.insert(
                                "filename",
                                name.parse::<HeaderValue>()
                                    .map_err(|e| FeError::from_anyhow(&anyhow!(e)).make_axum())?,
                            );

                            (StatusCode::OK, headers, icon)
                        }
                        None => (StatusCode::NO_CONTENT, HeaderMap::new(), Vec::new()),
                    })
                },
            ),
        )
        .route(
            "/modIcon",
            axum::routing::get(
                |State(app): State<Arc<AppInner>>, Query(query): Query<ModIconQuery>| async move {
                    let platformid = match &query.platform as &str {
                        "metadata" => 0,
                        "curseforge" => 1,
                        "modrinth" => 2,
                        _ => return Err(FeError::from_anyhow(&anyhow::anyhow!("unsupported platform")).make_axum()),
                    };

                    let icon = app.instance_manager()
                        .get_mod_icon(domain::InstanceId(query.instance_id), query.mod_id, platformid)
                        .await
                        .map_err(|e| FeError::from_anyhow(&e).make_axum())?;

                    Ok::<_, AxumError>(match icon {
                        Some(icon) => {
                            (StatusCode::OK, icon)
                        }
                        None => (StatusCode::NO_CONTENT, Vec::new()),
                    })
                }
            )
        )
        .route(
            "/modpackIcon",
            axum::routing::get(
                |State(app): State<Arc<AppInner>>, Query(query): Query<ModpackIconQuery>| async move {
                    let icon = app.instance_manager()
                        .get_modpack_icon(domain::InstanceId(query.instance_id))
                        .await
                        .map_err(|e| FeError::from_anyhow(&e).make_axum())?;

                        Ok::<_, AxumError>(match icon {
                            Some(icon) => {
                                (StatusCode::OK, icon)
                            }
                            None => (StatusCode::NO_CONTENT, Vec::new()),
                        })
                }
            )
        )
        .route(
            "/loadIcon",
            axum::routing::get(
                |State(app): State<Arc<AppInner>>, Query(query): Query<IconPathQuery>| async move {
                    let icon = app.instance_manager()
                        .load_icon(PathBuf::from(query.path))
                        .await
                        .map_err(|e| FeError::from_anyhow(&e).make_axum())?;

                    app.instance_manager().set_loaded_icon(icon.clone()).await;

                    let icon_bytes = icon.1;
                    Ok::<_, AxumError>(icon_bytes)
                }
            )
        )
        .route("/log", axum::routing::get(log::log_handler))
}
#[derive(Type, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct FEGroupId(i32);

#[derive(Type, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct FEInstanceId(i32);

impl From<domain::GroupId> for FEGroupId {
    fn from(value: domain::GroupId) -> Self {
        Self(*value)
    }
}

impl From<domain::InstanceId> for FEInstanceId {
    fn from(value: domain::InstanceId) -> Self {
        Self(*value)
    }
}

impl From<FEGroupId> for domain::GroupId {
    fn from(value: FEGroupId) -> Self {
        Self(value.0)
    }
}

impl From<FEInstanceId> for domain::InstanceId {
    fn from(value: FEInstanceId) -> Self {
        Self(value.0)
    }
}

#[derive(Type, Debug, Serialize)]
struct ListGroup {
    id: FEGroupId,
    name: String,
}

#[derive(Type, Debug, Serialize)]
struct ListInstance {
    id: FEInstanceId,
    group_id: FEGroupId,
    name: String,
    favorite: bool,
    status: ListInstanceStatus,
    icon_revision: Option<u32>,
    last_played: Option<DateTime<Utc>>,
    date_created: DateTime<Utc>,
    date_updated: DateTime<Utc>,
    seconds_played: u32,
}

#[derive(Type, Debug, Serialize)]
enum ListInstanceStatus {
    Valid(ValidListInstance),
    Invalid(InvalidListInstance),
}

#[derive(Type, Debug, Serialize)]
struct ValidListInstance {
    mc_version: Option<String>,
    modloader: Option<FEInstanceModloaderType>,
    modpack_platform: Option<ModpackPlatform>,
    state: LaunchState,
}

#[derive(Type, Debug, Serialize)]
enum ModpackPlatform {
    Curseforge,
    Modrinth,
}

#[derive(Type, Debug, Serialize)]
enum InvalidListInstance {
    JsonMissing,
    JsonError(ConfigurationParseError),
    Other(String),
}

#[derive(Type, Debug, Serialize)]
struct ConfigurationParseError {
    type_: ConfigurationParseErrorType,
    message: String,
    line: u32,
    config_text: String,
}

#[derive(Type, Debug, Serialize)]
enum ConfigurationParseErrorType {
    Syntax,
    Data,
    Eof,
    Unknown,
}

#[derive(Type, Debug, Deserialize)]
struct CreateInstance {
    group: FEGroupId,
    name: String,
    use_loaded_icon: bool,
    version: CreateInstanceVersion,
    notes: String,
}

#[derive(Type, Debug, Deserialize)]
struct ChangeModpack {
    instance: FEInstanceId,
    modpack: Modpack,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FEUpdateInstance {
    instance: FEInstanceId,
    #[specta(optional)]
    name: Option<Set<String>>,
    #[specta(optional)]
    use_loaded_icon: Option<Set<bool>>,
    #[specta(optional)]
    notes: Option<Set<String>>,
    #[specta(optional)]
    version: Option<Set<String>>,
    #[specta(optional)]
    modloader: Option<Set<Option<ModLoader>>>,
    #[specta(optional)]
    global_java_args: Option<Set<bool>>,
    #[specta(optional)]
    extra_java_args: Option<Set<Option<String>>>,
    #[specta(optional)]
    memory: Option<Set<Option<MemoryRange>>>,
    #[specta(optional)]
    game_resolution: Option<Set<Option<GameResolution>>>,
    #[specta(optional)]
    mod_sources: Option<Set<Option<super::modplatforms::ModSources>>>,
    #[specta(optional)]
    modpack_locked: Option<Set<Option<bool>>>,
}

#[derive(Type, Debug, Deserialize)]
struct DuplicateInstance {
    instance: FEInstanceId,
    new_name: String,
}

#[derive(Type, Debug, Deserialize)]
struct SetFavorite {
    instance: FEInstanceId,
    favorite: bool,
}

#[derive(Type, Debug, Deserialize)]
struct InstanceMod {
    instance_id: FEInstanceId,
    mod_id: String,
}

#[derive(Type, Debug, Deserialize)]
enum ModSource {
    Curseforge(CurseforgeMod),
    Modrinth(ModrinthMod),
}

#[derive(Type, Debug, Deserialize)]
enum ModSourceType {
    Curseforge,
    Modrinth,
}

#[derive(Type, Debug, Deserialize)]
struct CurseforgeMod {
    project_id: u32,
    file_id: u32,
}

#[derive(Type, Debug, Deserialize)]
struct ModrinthMod {
    project_id: String,
    version_id: String,
}

#[derive(Type, Debug, Deserialize)]
struct InstallMod {
    instance_id: FEInstanceId,
    mod_source: ModSource,
    install_deps: bool,
    replaces_mod: Option<String>,
}

#[derive(Type, Debug, Deserialize)]
struct UpdateMod {
    instance_id: FEInstanceId,
    mod_id: String,
}

#[derive(Type, Debug, Deserialize)]
struct InstallLatestMod {
    instance_id: FEInstanceId,
    mod_source: LatestModSource,
}

#[derive(Type, Debug, Deserialize)]
enum LatestModSource {
    Curseforge(u32),
    Modrinth(String),
}

#[derive(Type, Debug, Serialize, Deserialize)]
struct GameLogId(i32);

#[derive(Type, Debug, Serialize)]
struct GameLogEntry {
    id: GameLogId,
    instance_id: FEInstanceId,
    active: bool,
}

#[derive(Type, Debug, Deserialize)]
enum CreateInstanceVersion {
    Version(GameVersion),
    Modpack(Modpack),
}

#[derive(Type, Debug, Deserialize)]
enum GameVersion {
    Standard(StandardVersion),
    // Custom(json)
}

#[derive(Type, Debug, Serialize, Deserialize)]
struct ModpackInfo {
    modpack: Modpack,
    locked: bool,
}

#[derive(Type, Debug, Serialize, Deserialize)]
enum Modpack {
    Curseforge(CurseforgeModpack),
    Modrinth(ModrinthModpack),
}

#[derive(Type, Debug, Serialize, Deserialize)]
struct CurseforgeModpack {
    project_id: u32,
    file_id: u32,
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct ModrinthModpack {
    pub project_id: String,
    pub version_id: String,
}

#[derive(Type, Debug, Deserialize)]
struct StandardVersion {
    release: String,
    modloaders: HashSet<ModLoader>,
}

#[derive(Type, Debug, Deserialize)]
struct MoveGroup {
    group: FEGroupId,
    before: Option<FEGroupId>,
}

#[derive(Type, Debug, Deserialize)]
struct MoveInstance {
    instance: FEInstanceId,
    target: MoveInstanceTarget,
}

#[derive(Type, Debug, Deserialize)]
enum MoveInstanceTarget {
    BeforeInstance(FEInstanceId),
    BeginningOfGroup(FEGroupId),
    EndOfGroup(FEGroupId),
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum GameResolution {
    Standard(u16, u16),
    Custom(u16, u16),
}

impl From<domain::info::GameResolution> for GameResolution {
    fn from(value: domain::info::GameResolution) -> Self {
        match value {
            domain::info::GameResolution::Standard(w, h) => Self::Standard(w, h),
            domain::info::GameResolution::Custom(w, h) => Self::Custom(w, h),
        }
    }
}

impl From<GameResolution> for domain::info::GameResolution {
    fn from(value: GameResolution) -> Self {
        match value {
            GameResolution::Standard(w, h) => Self::Standard(w, h),
            GameResolution::Custom(w, h) => Self::Custom(w, h),
        }
    }
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstanceDetails {
    name: String,
    favorite: bool,
    version: Option<String>,
    modpack: Option<ModpackInfo>,
    global_java_args: bool,
    extra_java_args: Option<String>,
    memory: Option<MemoryRange>,
    game_resolution: Option<GameResolution>,
    last_played: Option<DateTime<Utc>>,
    seconds_played: u32,
    modloaders: Vec<ModLoader>,
    pre_launch_hook: Option<String>,
    post_exit_hook: Option<String>,
    wrapper_command: Option<String>,
    notes: String,
    state: LaunchState,
    icon_revision: Option<u32>,
    has_pack_update: bool,
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct FEInstanceModpackInfo {
    pub name: String,
    pub version_name: String,
    pub url_slug: String,
    pub has_image: bool,
}

impl From<InstanceModpackInfo> for FEInstanceModpackInfo {
    fn from(value: InstanceModpackInfo) -> Self {
        Self {
            name: value.name,
            version_name: value.version_name,
            url_slug: value.url_slug,
            has_image: value.has_image,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct MemoryRange {
    pub min_mb: u16,
    pub max_mb: u16,
}

#[derive(Type, Debug, Deserialize)]
struct OpenInstanceFolder {
    instance_id: FEInstanceId,
    folder: InstanceFolder,
}

#[derive(Type, Debug, Deserialize)]
enum InstanceFolder {
    Root,
    Data,
    Mods,
    Configs,
    Screenshots,
    Saves,
    Logs,
    CrashReports,
    ResourcePacks,
    TexturePacks,
    ShaderPacks,
}

#[derive(Type, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
struct ModLoader {
    type_: FEInstanceModloaderType,
    version: String,
}

#[derive(Type, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
enum FEInstanceModloaderType {
    Neoforge,
    Forge,
    Fabric,
    Quilt,
}

#[derive(Type, Debug, Serialize)]
enum LaunchState {
    Inactive {
        failed_task: Option<FETaskId>,
    },
    Preparing(FETaskId),
    Running {
        start_time: DateTime<Utc>,
        log_id: i32,
    },
    Deleting,
}

#[derive(Type, Debug, Serialize)]
struct Mod {
    id: String,
    filename: String,
    enabled: bool,
    metadata: Option<ModFileMetadata>,
    curseforge: Option<CurseForgeModMetadata>,
    modrinth: Option<ModrinthModMetadata>,
    has_update: bool,
}

#[derive(Type, Debug, Serialize)]
struct ModFileMetadata {
    id: String,
    modid: Option<String>,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    authors: Option<String>,
    modloaders: Vec<FEInstanceModloaderType>,
    sha_1: String,
    sha_512: String,
    murmur_2: String,
    has_image: bool,
}

#[derive(Type, Serialize, Debug)]
struct CurseForgeModMetadata {
    project_id: u32,
    file_id: u32,
    name: String,
    version: String,
    urlslug: String,
    summary: String,
    authors: String,
    has_image: bool,
}

#[derive(Type, Serialize, Debug)]
struct ModrinthModMetadata {
    project_id: String,
    version_id: String,
    title: String,
    version: String,
    urlslug: String,
    description: String,
    authors: String,
    has_image: bool,
}

#[derive(Type, Deserialize, Debug)]
struct ExploreQuery {
    instance_id: FEInstanceId,
    path: Vec<String>,
}

#[derive(Type, Serialize, Debug)]
struct ExploreEntry {
    name: String,
    #[serde(rename = "type")]
    type_: ExploreEntryType,
}

#[derive(Type, Serialize, Debug)]
enum ExploreEntryType {
    File { size: u32 },
    Directory,
}

#[derive(Type, Deserialize, Debug)]
struct ExportEntry {
    //#[serde(flatten)]
    entries: HashMap<String, Option<ExportEntry>>,
}

#[derive(Type, Deserialize, Debug)]
enum ExportTarget {
    Curseforge,
    Modrinth,
}

#[derive(Type, Deserialize, Debug)]
struct ExportArgs {
    instance_id: FEInstanceId,
    target: ExportTarget,
    save_path: String,
    link_mods: bool,
    filter: ExportEntry,
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub enum ImportEntity {
    LegacyGDLauncher,
    MRPack,
    Modrinth,
    CurseForgeZip,
    CurseForge,
    ATLauncher,
    Technic,
    FTB,
    MultiMC,
    PrismLauncher,
}

#[derive(Type, Debug, Serialize)]
struct ImportableInstance {
    filename: String,
    instance_name: String,
}

#[derive(Type, Debug, Serialize)]
struct InvalidImportEntry {
    name: String,
    reason: Translation,
}

#[derive(Type, Debug, Serialize)]
enum ImportEntry {
    Valid(ImportableInstance),
    Invalid(InvalidImportEntry),
}

#[derive(Type, Debug, Serialize)]
enum ImportScanStatus {
    NoResults,
    SingleResult(ImportEntry),
    MultiResult(Vec<ImportEntry>),
}

#[derive(Type, Debug, Serialize)]
struct FullImportScanStatus {
    scanning: bool,
    status: ImportScanStatus,
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum ImportEntitySelectionType {
    File,
    Directory,
}

impl From<importer::SelectionType> for ImportEntitySelectionType {
    fn from(value: importer::SelectionType) -> Self {
        match value {
            importer::SelectionType::File => Self::File,
            importer::SelectionType::Directory => Self::Directory,
        }
    }
}

#[derive(Type, Debug, Serialize)]
struct ImportEntityStatus {
    entity: ImportEntity,
    supported: bool,
    selection_type: ImportEntitySelectionType,
}

#[derive(Type, Debug, Deserialize)]
struct ImportRequest {
    index: u32,
    name: Option<String>,
}

impl From<domain::InstanceDetails> for InstanceDetails {
    fn from(value: domain::InstanceDetails) -> Self {
        Self {
            favorite: value.favorite,
            name: value.name,
            version: value.version,
            modpack: value.modpack.map(Into::into),
            global_java_args: value.global_java_args,
            extra_java_args: value.extra_java_args,
            memory: value.memory.map(Into::into),
            game_resolution: value.game_resolution.map(Into::into),
            last_played: value.last_played,
            seconds_played: value.seconds_played,
            modloaders: value.modloaders.into_iter().map(Into::into).collect(),
            notes: value.notes,
            state: value.state.into(),
            icon_revision: value.icon_revision,
            has_pack_update: value.has_pack_update,
            pre_launch_hook: value.pre_launch_hook,
            post_exit_hook: value.post_exit_hook,
            wrapper_command: value.wrapper_command,
        }
    }
}

impl From<mpdomain::ModPlatform> for ModpackPlatform {
    fn from(value: mpdomain::ModPlatform) -> Self {
        match value {
            mpdomain::ModPlatform::Curseforge => Self::Curseforge,
            mpdomain::ModPlatform::Modrinth => Self::Modrinth,
        }
    }
}

impl From<domain::info::ModLoader> for ModLoader {
    fn from(value: domain::info::ModLoader) -> Self {
        Self {
            type_: value.type_.into(),
            version: value.version,
        }
    }
}

impl From<domain::info::ModLoaderType> for FEInstanceModloaderType {
    fn from(value: domain::info::ModLoaderType) -> Self {
        use domain::info::ModLoaderType as domain;

        match value {
            domain::Neoforge => Self::Neoforge,
            domain::Forge => Self::Forge,
            domain::Fabric => Self::Fabric,
            domain::Quilt => Self::Quilt,
        }
    }
}

impl TryFrom<CreateInstanceVersion> for manager::InstanceVersionSource {
    type Error = anyhow::Error;

    fn try_from(value: CreateInstanceVersion) -> anyhow::Result<Self> {
        Ok(match value {
            CreateInstanceVersion::Version(v) => Self::Version(v.try_into()?),
            CreateInstanceVersion::Modpack(m) => Self::Modpack(m.into()),
        })
    }
}

impl TryFrom<GameVersion> for domain::info::GameVersion {
    type Error = anyhow::Error;

    fn try_from(value: GameVersion) -> anyhow::Result<Self> {
        match value {
            GameVersion::Standard(v) => Ok(Self::Standard(v.try_into()?)),
        }
    }
}

impl From<ModpackInfo> for domain::info::ModpackInfo {
    fn from(value: ModpackInfo) -> Self {
        Self {
            modpack: value.modpack.into(),
            locked: value.locked,
        }
    }
}

impl From<Modpack> for domain::info::Modpack {
    fn from(value: Modpack) -> Self {
        match value {
            Modpack::Curseforge(m) => Self::Curseforge(m.into()),
            Modpack::Modrinth(m) => Self::Modrinth(m.into()),
        }
    }
}

impl From<CurseforgeModpack> for domain::info::CurseforgeModpack {
    fn from(value: CurseforgeModpack) -> Self {
        Self {
            project_id: value.project_id,
            file_id: value.file_id,
        }
    }
}

impl From<ModrinthModpack> for domain::info::ModrinthModpack {
    fn from(value: ModrinthModpack) -> Self {
        Self {
            project_id: value.project_id,
            version_id: value.version_id,
        }
    }
}

impl From<domain::info::ModpackInfo> for ModpackInfo {
    fn from(value: domain::info::ModpackInfo) -> Self {
        Self {
            modpack: value.modpack.into(),
            locked: value.locked,
        }
    }
}

impl From<domain::info::Modpack> for Modpack {
    fn from(value: domain::info::Modpack) -> Self {
        match value {
            domain::info::Modpack::Curseforge(m) => Self::Curseforge(m.into()),
            domain::info::Modpack::Modrinth(m) => Self::Modrinth(m.into()),
        }
    }
}

impl From<domain::info::CurseforgeModpack> for CurseforgeModpack {
    fn from(value: domain::info::CurseforgeModpack) -> Self {
        Self {
            project_id: value.project_id,
            file_id: value.file_id,
        }
    }
}

impl From<domain::info::ModrinthModpack> for ModrinthModpack {
    fn from(value: domain::info::ModrinthModpack) -> Self {
        Self {
            project_id: value.project_id,
            version_id: value.version_id,
        }
    }
}

impl TryFrom<StandardVersion> for domain::info::StandardVersion {
    type Error = anyhow::Error;

    fn try_from(value: StandardVersion) -> anyhow::Result<Self> {
        let mut modloaders = HashSet::new();

        for modloader in value.modloaders {
            modloaders.insert(modloader.try_into()?);
        }

        Ok(Self {
            release: value.release,
            modloaders,
        })
    }
}

impl TryFrom<ModLoader> for domain::info::ModLoader {
    type Error = anyhow::Error;

    fn try_from(value: ModLoader) -> anyhow::Result<Self> {
        if value.version.is_empty() {
            return Err(anyhow!("modloader version cannot be empty"));
        }

        Ok(Self {
            type_: value.type_.into(),
            version: value.version,
        })
    }
}

impl From<FEInstanceModloaderType> for domain::info::ModLoaderType {
    fn from(value: FEInstanceModloaderType) -> Self {
        match value {
            FEInstanceModloaderType::Neoforge => Self::Neoforge,
            FEInstanceModloaderType::Forge => Self::Forge,
            FEInstanceModloaderType::Fabric => Self::Fabric,
            FEInstanceModloaderType::Quilt => Self::Quilt,
        }
    }
}

impl From<manager::ListGroup> for ListGroup {
    fn from(value: manager::ListGroup) -> Self {
        Self {
            id: value.id.into(),
            name: value.name,
        }
    }
}

impl From<manager::ListInstance> for ListInstance {
    fn from(value: manager::ListInstance) -> Self {
        Self {
            id: value.id.into(),
            group_id: value.group_id.into(),
            name: value.name,
            favorite: value.favorite,
            status: value.status.into(),
            icon_revision: value.icon_revision,
            last_played: value.last_played,
            date_created: value.date_created,
            date_updated: value.date_updated,
            seconds_played: value.seconds_played,
        }
    }
}

impl From<manager::ListInstanceStatus> for ListInstanceStatus {
    fn from(value: manager::ListInstanceStatus) -> Self {
        match value {
            manager::ListInstanceStatus::Valid(status) => Self::Valid(status.into()),
            manager::ListInstanceStatus::Invalid(status) => Self::Invalid(status.into()),
        }
    }
}

impl From<manager::ValidListInstance> for ValidListInstance {
    fn from(value: manager::ValidListInstance) -> Self {
        Self {
            mc_version: value.mc_version,
            modloader: value.modloader.map(Into::into),
            modpack_platform: value.modpack_platform.map(Into::into),
            state: value.state.into(),
        }
    }
}

impl From<manager::InvalidListInstance> for InvalidListInstance {
    fn from(value: manager::InvalidListInstance) -> Self {
        use manager::InvalidListInstance as manager;

        match value {
            manager::JsonMissing => Self::JsonMissing,
            manager::JsonError(e) => Self::JsonError(e.into()),
            manager::Other(e) => Self::Other(e),
        }
    }
}

impl From<manager::ConfigurationParseError> for ConfigurationParseError {
    fn from(value: manager::ConfigurationParseError) -> Self {
        Self {
            type_: value.type_.into(),
            message: value.message,
            line: value.line,
            config_text: value.config_text,
        }
    }
}

impl From<manager::ConfigurationParseErrorType> for ConfigurationParseErrorType {
    fn from(value: manager::ConfigurationParseErrorType) -> Self {
        use manager::ConfigurationParseErrorType as manager;

        match value {
            manager::Syntax => Self::Syntax,
            manager::Data => Self::Data,
            manager::Eof => Self::Eof,
            manager::Unknown => Self::Unknown,
        }
    }
}

impl From<domain::LaunchState> for LaunchState {
    fn from(value: domain::LaunchState) -> Self {
        use domain::LaunchState as domain;

        match value {
            domain::Inactive { failed_task } => Self::Inactive {
                failed_task: failed_task.map(Into::into),
            },
            domain::Preparing(task) => Self::Preparing(task.into()),
            domain::Running { start_time, log_id } => Self::Running {
                start_time,
                log_id: log_id.0,
            },
            domain::Deleting => Self::Deleting,
        }
    }
}

impl From<domain::Mod> for Mod {
    fn from(value: domain::Mod) -> Self {
        Self {
            id: value.id,
            filename: value.filename,
            enabled: value.enabled,
            metadata: value.metadata.map(Into::into),
            curseforge: value.curseforge.map(Into::into),
            modrinth: value.modrinth.map(Into::into),
            has_update: value.has_update,
        }
    }
}

impl From<domain::ModFileMetadata> for ModFileMetadata {
    fn from(value: domain::ModFileMetadata) -> Self {
        Self {
            id: value.id,
            modid: value.modid,
            name: value.name,
            version: value.version,
            description: value.description,
            authors: value.authors,
            modloaders: value.modloaders.into_iter().map(Into::into).collect(),
            sha_1: hex::encode(value.sha_1),
            sha_512: hex::encode(value.sha_512),
            murmur_2: value.murmur_2.to_string(),
            has_image: value.has_image,
        }
    }
}

impl From<domain::CurseForgeModMetadata> for CurseForgeModMetadata {
    fn from(value: domain::CurseForgeModMetadata) -> Self {
        Self {
            project_id: value.project_id,
            file_id: value.file_id,
            name: value.name,
            version: value.version,
            urlslug: value.urlslug,
            summary: value.summary,
            authors: value.authors,
            has_image: value.has_image,
        }
    }
}

impl From<domain::ModrinthModMetadata> for ModrinthModMetadata {
    fn from(value: domain::ModrinthModMetadata) -> Self {
        Self {
            project_id: value.project_id,
            version_id: value.version_id,
            title: value.title,
            version: value.version,
            urlslug: value.urlslug,
            description: value.description,
            authors: value.authors,
            has_image: value.has_image,
        }
    }
}

impl From<domain::GameLogId> for GameLogId {
    fn from(value: domain::GameLogId) -> Self {
        Self(value.0)
    }
}

impl From<GameLogId> for domain::GameLogId {
    fn from(value: GameLogId) -> Self {
        Self(value.0)
    }
}

impl From<domain::GameLogEntry> for GameLogEntry {
    fn from(value: domain::GameLogEntry) -> Self {
        Self {
            id: value.id.into(),
            instance_id: value.instance_id.into(),
            active: value.active,
        }
    }
}

impl From<InstanceFolder> for domain::InstanceFolder {
    fn from(value: InstanceFolder) -> Self {
        match value {
            InstanceFolder::Root => Self::Root,
            InstanceFolder::Data => Self::Data,
            InstanceFolder::Mods => Self::Mods,
            InstanceFolder::Configs => Self::Configs,
            InstanceFolder::Screenshots => Self::Screenshots,
            InstanceFolder::Saves => Self::Saves,
            InstanceFolder::Logs => Self::Logs,
            InstanceFolder::CrashReports => Self::CrashReports,
            InstanceFolder::ResourcePacks => Self::ResourcePacks,
            InstanceFolder::TexturePacks => Self::TexturePacks,
            InstanceFolder::ShaderPacks => Self::ShaderPacks,
        }
    }
}

impl From<(u16, u16)> for MemoryRange {
    fn from(value: (u16, u16)) -> Self {
        Self {
            min_mb: value.0,
            max_mb: value.1,
        }
    }
}

impl From<MemoryRange> for (u16, u16) {
    fn from(value: MemoryRange) -> Self {
        (value.min_mb, value.max_mb)
    }
}

impl TryFrom<FEUpdateInstance> for domain::InstanceSettingsUpdate {
    type Error = anyhow::Error;

    fn try_from(value: FEUpdateInstance) -> anyhow::Result<Self> {
        Ok(Self {
            instance_id: value.instance.into(),
            name: value.name.map(|x| x.inner()),
            use_loaded_icon: value.use_loaded_icon.map(|x| x.inner()),
            notes: value.notes.map(|x| x.inner()),
            version: value.version.map(|x| x.inner()),
            modloader: value
                .modloader
                .map(|x| x.inner().and_then(|v| v.try_into().ok())),
            global_java_args: value.global_java_args.map(|x| x.inner()),
            extra_java_args: value.extra_java_args.map(|x| x.inner()),
            memory: value.memory.map(|x| x.inner().map(Into::into)),
            game_resolution: value.game_resolution.map(|x| x.inner().map(Into::into)),
            mod_sources: value.mod_sources.map(|x| x.inner().map(Into::into)),
            modpack_locked: value.modpack_locked.map(|x| x.inner()),
        })
    }
}

impl From<domain::ExploreEntry> for ExploreEntry {
    fn from(value: domain::ExploreEntry) -> Self {
        Self {
            name: value.name,
            type_: value.type_.into(),
        }
    }
}

impl From<ImportEntity> for importer::Entity {
    fn from(entity: ImportEntity) -> Self {
        match entity {
            ImportEntity::LegacyGDLauncher => Self::LegacyGDLauncher,
            ImportEntity::MRPack => Self::MRPack,
            ImportEntity::Modrinth => Self::Modrinth,
            ImportEntity::CurseForgeZip => Self::CurseForgeZip,
            ImportEntity::CurseForge => Self::CurseForge,
            ImportEntity::ATLauncher => Self::ATLauncher,
            ImportEntity::Technic => Self::Technic,
            ImportEntity::FTB => Self::FTB,
            ImportEntity::MultiMC => Self::MultiMC,
            ImportEntity::PrismLauncher => Self::PrismLauncher,
        }
    }
}

impl From<importer::Entity> for ImportEntity {
    fn from(entity: importer::Entity) -> Self {
        use importer::Entity as backend;

        match entity {
            backend::LegacyGDLauncher => Self::LegacyGDLauncher,
            backend::MRPack => Self::MRPack,
            backend::Modrinth => Self::Modrinth,
            backend::CurseForgeZip => Self::CurseForgeZip,
            backend::CurseForge => Self::CurseForge,
            backend::ATLauncher => Self::ATLauncher,
            backend::Technic => Self::Technic,
            backend::FTB => Self::FTB,
            backend::MultiMC => Self::MultiMC,
            backend::PrismLauncher => Self::PrismLauncher,
        }
    }
}

impl From<importer::ImportableInstance> for ImportableInstance {
    fn from(value: importer::ImportableInstance) -> Self {
        Self {
            filename: value.filename,
            instance_name: value.instance_name,
        }
    }
}

impl From<importer::InvalidImportEntry> for InvalidImportEntry {
    fn from(value: importer::InvalidImportEntry) -> Self {
        Self {
            name: value.name,
            reason: value.reason,
        }
    }
}

impl From<domain::ExploreEntryType> for ExploreEntryType {
    fn from(value: domain::ExploreEntryType) -> Self {
        match value {
            domain::ExploreEntryType::File { size } => Self::File { size },
            domain::ExploreEntryType::Directory => Self::Directory,
        }
    }
}

impl From<importer::ImportEntry> for ImportEntry {
    fn from(value: importer::ImportEntry) -> Self {
        match value {
            importer::ImportEntry::Valid(v) => Self::Valid(v.into()),
            importer::ImportEntry::Invalid(v) => Self::Invalid(v.into()),
        }
    }
}

impl From<ExportTarget> for domain::ExportTarget {
    fn from(value: ExportTarget) -> Self {
        match value {
            ExportTarget::Curseforge => Self::Curseforge,
            ExportTarget::Modrinth => Self::Modrinth,
        }
    }
}

impl From<importer::ImportScanStatus> for ImportScanStatus {
    fn from(value: importer::ImportScanStatus) -> Self {
        use importer::ImportScanStatus as domain;

        match value {
            domain::NoResults => Self::NoResults,
            domain::SingleResult(r) => Self::SingleResult(r.into()),
            domain::MultiResult(r) => Self::MultiResult(r.into_iter().map(Into::into).collect()),
        }
    }
}

impl From<ExportEntry> for domain::ExportEntry {
    fn from(value: ExportEntry) -> Self {
        Self(
            value
                .entries
                .into_iter()
                .map(|(k, v)| (k, v.map(Into::into)))
                .collect(),
        )
    }
}

impl From<importer::FullImportScanStatus> for FullImportScanStatus {
    fn from(value: importer::FullImportScanStatus) -> Self {
        Self {
            scanning: value.scanning,
            status: value.status.into(),
        }
    }
}

mod log {
    use super::*;

    #[derive(Debug, Deserialize)]
    pub struct LogQuery {
        id: i32,
    }

    #[tracing::instrument(skip(app))]
    pub async fn log_handler(
        State(app): State<App>,
        Query(query): Query<LogQuery>,
    ) -> impl IntoResponse {
        tracing::info!("starting log stream");

        let log_rx = app
            .instance_manager()
            .get_log(domain::GameLogId(query.id))
            .await;

        let Ok(mut log_rx) = log_rx else {
            tracing::warn!("log entry not found");

            return StatusCode::NOT_FOUND.into_response();
        };

        let s = async_stream::stream! {
            tracing::trace!("starting log stream");

            let mut last_idx = 0;

            loop {
                tracing::trace!("waiting for log data to come in");

                let new_lines = {
                    let log = log_rx.borrow();

                    let new_lines
                        = log
                            .get_span(last_idx..)
                            .into_iter()
                            .inspect(|entry| tracing::trace!(?entry, "received log entry"))
                            .map(|entry| {
                                serde_json::to_vec(&entry)
                                    .expect(
                                        "serialization of a log entry should be infallible"
                                    )
                            })
                            .collect::<Vec<_>>();

                    last_idx = log.len();

                    new_lines
                };



                for line in new_lines {
                    tracing::trace!("yielding log entry");

                    yield Ok::<_, Infallible>(line)
                }



                if let Err(_) = log_rx.changed().await {
                    tracing::error!("`log_rx` was closed, killing log stream");

                    break
                }
            }
        };

        (StatusCode::OK, StreamBody::new(s)).into_response()
    }
}
