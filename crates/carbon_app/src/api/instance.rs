use std::collections::HashSet;
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

use crate::error::{AxumError, FeError};
use crate::managers::instance::log::EntryType;
use crate::managers::instance::InstanceMoveTarget;
use crate::managers::{App, AppInner};

use super::keys::instance::*;
use super::router::router;
use super::vtask::TaskId;

use crate::domain::instance as domain;
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

        query GET_INSTANCES_UNGROUPED[app, args: ()] {
            Ok(app.instance_manager()
                .list_groups()
                .await?
                .into_iter()
                .flat_map(|group| {
                    group.instances
                        .into_iter()
                        .map(|instance| UngroupedInstance {
                            favorite: instance.favorite,
                            instance: instance.into(),
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>())
        }

        mutation CREATE_GROUP[app, name: String] {
            app.instance_manager()
                .create_group(name)
                .await
                .map(GroupId::from)
        }

        mutation CREATE_INSTANCE[app, details: CreateInstance] {
            app.instance_manager()
                .create_instance(
                    details.group.into(),
                    details.name,
                    details.use_loaded_icon,
                    details.version.into(),
                    details.notes,
                )
                .await
                .map(InstanceId::from)
        }

        mutation LOAD_ICON_URL[app, url: String] {
            app.instance_manager()
                .download_icon(url)
                .await
        }

        mutation DELETE_GROUP[app, id: GroupId] {
            app.instance_manager()
                .delete_group(id.into())
                .await
        }

        mutation DELETE_INSTANCE[app, id: InstanceId] {
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
                .map(InstanceId::from)
        }

        mutation UPDATE_INSTANCE[app, details: UpdateInstance] {
            app.instance_manager()
                .update_instance(details.into())
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

        query INSTANCE_DETAILS[app, id: InstanceId] {
            app.instance_manager()
                .instance_details(id.into())
                .await
                .map(InstanceDetails::from)
        }

        mutation PREPARE_INSTANCE[app, id: InstanceId] {
            app.instance_manager()
                .prepare_game(id.into(), None)
                .await
        }

        mutation LAUNCH_INSTANCE[app, id: InstanceId] {
            let account = app.account_manager()
                .get_active_account()
                .await?;

            let Some(account) = account else {
                return Err(anyhow::anyhow!("attempted to launch instance without an account"));
            };

            app.instance_manager()
                .prepare_game(id.into(), Some(account))
                .await
        }

        mutation KILL_INSTANCE[app, id: InstanceId] {
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
            let task = app.instance_manager()
                .install_curseforge_mod(
                    imod.instance_id.into(),
                    imod.project_id,
                    imod.file_id,
                )
                .await?;

            Ok(super::vtask::TaskId::from(task))
        }

        mutation OPEN_INSTANCE_FOLDER[app, folder: OpenInstanceFolder] {
            app.instance_manager().open_folder(
                folder.instance_id.into(),
                folder.folder.into(),
            )
            .await
        }
    }
}

pub(super) fn mount_axum_router() -> axum::Router<Arc<AppInner>> {
    #[derive(Deserialize)]
    struct InstanceIconQuery {
        id: i32,
    }

    #[derive(Deserialize)]
    struct IconPathQuery {
        path: String,
    }

    #[derive(Deserialize)]
    struct LogQuery {
        id: i32,
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
            "/loadIcon",
            axum::routing::get(
                |State(app): State<Arc<AppInner>>, Query(query): Query<IconPathQuery>| async move {
                    app.instance_manager()
                        .load_icon(PathBuf::from(query.path))
                        .await
                        .map_err(|e| FeError::from_anyhow(&e).make_axum())
                }
            )
        )
        .route(
            "/log",
            axum::routing::get(
                |State(app): State<Arc<AppInner>>, Query(query): Query<LogQuery>| async move {
                    let log_rx = app.instance_manager()
                        .get_log(domain::GameLogId(query.id))
                        .await;

                    let Ok(mut log_rx) = log_rx else {
                        return IntoResponse::into_response(StatusCode::NOT_FOUND)
                    };

                    #[derive(Serialize)]
                    enum LogEntryType {
                        System,
                        StdOut,
                        StdErr,
                    }

                    #[derive(Serialize)]
                    struct LogEntry<'a> {
                        line: &'a str,
                        type_: LogEntryType,
                    }

                    let s = async_stream::stream! {
                        let mut last_idx = 0;

                        loop {
                            let new_lines = {
                                let log = log_rx.borrow();

                                let new_lines = log.get_region(last_idx..).into_iter().map(|line| {
                                    let entry = LogEntry {
                                        line: line.text,
                                        type_: match line.type_ {
                                            EntryType::System => LogEntryType::System,
                                            EntryType::StdOut => LogEntryType::StdOut,
                                            EntryType::StdErr => LogEntryType::StdErr,
                                        }
                                    };

                                    serde_json::to_vec(&entry)
                                        .expect("serialization of a log entry should be infallible")
                                }).collect::<Vec<_>>();

                                last_idx = log.len();
                                new_lines
                            };

                            for line in new_lines {
                                yield Ok::<_, Infallible>(line)
                            }

                            if let Err(_) = log_rx.changed().await {
                                break
                            }
                        }
                    };

                    IntoResponse::into_response((StatusCode::OK, StreamBody::new(s)))
                }
            )
        )
}
#[derive(Type, Debug, Serialize, Deserialize)]
struct GroupId(i32);

#[derive(Type, Debug, Serialize, Deserialize)]
struct InstanceId(i32);

impl From<domain::GroupId> for GroupId {
    fn from(value: domain::GroupId) -> Self {
        Self(*value)
    }
}

impl From<domain::InstanceId> for InstanceId {
    fn from(value: domain::InstanceId) -> Self {
        Self(*value)
    }
}

impl From<GroupId> for domain::GroupId {
    fn from(value: GroupId) -> Self {
        Self(value.0)
    }
}

impl From<InstanceId> for domain::InstanceId {
    fn from(value: InstanceId) -> Self {
        Self(value.0)
    }
}

#[derive(Type, Debug, Serialize)]
struct ListGroup {
    id: GroupId,
    name: String,
    instances: Vec<ListInstance>,
}

#[derive(Type, Debug, Serialize)]
struct ListInstance {
    id: InstanceId,
    name: String,
    favorite: bool,
    status: ListInstanceStatus,
}

#[derive(Type, Debug, Serialize)]
struct UngroupedInstance {
    favorite: bool,
    #[serde(flatten)]
    instance: ListInstance,
}

#[derive(Type, Debug, Serialize)]
enum ListInstanceStatus {
    Valid(ValidListInstance),
    Invalid(InvalidListInstance),
}

#[derive(Type, Debug, Serialize)]
struct ValidListInstance {
    mc_version: Option<String>,
    modloader: Option<ModLoaderType>,
    modpack_platform: Option<ModpackPlatform>,
    state: LaunchState,
}

#[derive(Type, Debug, Serialize)]
enum ModpackPlatform {
    Curseforge,
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
}

#[derive(Type, Debug, Deserialize)]
struct CreateInstance {
    group: GroupId,
    name: String,
    use_loaded_icon: bool,
    version: CreateInstanceVersion,
    notes: String,
}

#[derive(Type, Debug, Deserialize)]
struct UpdateInstance {
    instance: InstanceId,
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
}

#[derive(Type, Debug, Deserialize)]
struct DuplicateInstance {
    instance: InstanceId,
    new_name: String,
}

#[derive(Type, Debug, Deserialize)]
struct SetFavorite {
    instance: InstanceId,
    favorite: bool,
}

#[derive(Type, Debug, Deserialize)]
enum Set<T> {
    Set(T),
}

impl<T> Set<T> {
    fn inner(self) -> T {
        match self {
            Self::Set(t) => t,
        }
    }
}

#[derive(Type, Debug, Deserialize)]
struct InstanceMod {
    instance_id: InstanceId,
    mod_id: String,
}

#[derive(Type, Debug, Deserialize)]
struct InstallMod {
    instance_id: InstanceId,
    project_id: u32,
    file_id: u32,
}

#[derive(Type, Debug, Serialize, Deserialize)]
struct GameLogId(i32);

#[derive(Type, Debug, Serialize)]
struct GameLogEntry {
    id: GameLogId,
    instance_id: InstanceId,
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
enum Modpack {
    Curseforge(CurseforgeModpack),
}

#[derive(Type, Debug, Serialize, Deserialize)]
struct CurseforgeModpack {
    project_id: u32,
    file_id: u32,
}

#[derive(Type, Debug, Deserialize)]
struct StandardVersion {
    release: String,
    modloaders: HashSet<ModLoader>,
}

#[derive(Type, Debug, Deserialize)]
struct MoveGroup {
    group: GroupId,
    before: Option<GroupId>,
}

#[derive(Type, Debug, Deserialize)]
struct MoveInstance {
    instance: InstanceId,
    target: MoveInstanceTarget,
}

#[derive(Type, Debug, Deserialize)]
enum MoveInstanceTarget {
    BeforeInstance(InstanceId),
    BeginningOfGroup(GroupId),
    EndOfGroup(GroupId),
}

#[derive(Type, Debug, Serialize)]
struct InstanceDetails {
    name: String,
    favorite: bool,
    version: Option<String>,
    modpack: Option<Modpack>,
    global_java_args: bool,
    extra_java_args: Option<String>,
    memory: Option<MemoryRange>,
    last_played: DateTime<Utc>,
    seconds_played: u32,
    modloaders: Vec<ModLoader>,
    notes: String,
    state: LaunchState,
    mods: Vec<Mod>,
}

#[derive(Type, Debug, Serialize, Deserialize)]
pub struct MemoryRange {
    pub min_mb: u16,
    pub max_mb: u16,
}

#[derive(Type, Debug, Deserialize)]
struct OpenInstanceFolder {
    instance_id: InstanceId,
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
    type_: ModLoaderType,
    version: String,
}

#[derive(Type, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
enum ModLoaderType {
    Forge,
    Fabric,
}

#[derive(Type, Debug, Serialize)]
enum LaunchState {
    Inactive {
        failed_task: Option<TaskId>,
    },
    Preparing(TaskId),
    Running {
        start_time: DateTime<Utc>,
        log_id: i32,
    },
}

#[derive(Type, Debug, Serialize)]
struct Mod {
    id: String,
    filename: String,
    enabled: bool,
    modloader: ModLoaderType,
    metadata: ModFileMetadata,
}

#[derive(Type, Debug, Serialize)]
struct ModFileMetadata {
    modid: String,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    authors: Option<String>,
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
            last_played: value.last_played,
            seconds_played: value.seconds_played,
            modloaders: value.modloaders.into_iter().map(Into::into).collect(),
            notes: value.notes,
            state: value.state.into(),
            mods: value.mods.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<domain::info::ModpackPlatform> for ModpackPlatform {
    fn from(value: domain::info::ModpackPlatform) -> Self {
        match value {
            domain::info::ModpackPlatform::Curseforge => Self::Curseforge,
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

impl From<domain::info::ModLoaderType> for ModLoaderType {
    fn from(value: domain::info::ModLoaderType) -> Self {
        use domain::info::ModLoaderType as domain;

        match value {
            domain::Forge => Self::Forge,
            domain::Fabric => Self::Fabric,
        }
    }
}

impl From<CreateInstanceVersion> for manager::InstanceVersionSource {
    fn from(value: CreateInstanceVersion) -> Self {
        match value {
            CreateInstanceVersion::Version(v) => Self::Version(v.into()),
            CreateInstanceVersion::Modpack(m) => Self::Modpack(m.into()),
        }
    }
}

impl From<GameVersion> for domain::info::GameVersion {
    fn from(value: GameVersion) -> Self {
        match value {
            GameVersion::Standard(v) => Self::Standard(v.into()),
        }
    }
}

impl From<Modpack> for domain::info::Modpack {
    fn from(value: Modpack) -> Self {
        match value {
            Modpack::Curseforge(m) => Self::Curseforge(m.into()),
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

impl From<domain::info::Modpack> for Modpack {
    fn from(value: domain::info::Modpack) -> Self {
        match value {
            domain::info::Modpack::Curseforge(m) => Self::Curseforge(m.into()),
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

impl From<StandardVersion> for domain::info::StandardVersion {
    fn from(value: StandardVersion) -> Self {
        Self {
            release: value.release,
            modloaders: value.modloaders.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ModLoader> for domain::info::ModLoader {
    fn from(value: ModLoader) -> Self {
        Self {
            type_: value.type_.into(),
            version: value.version,
        }
    }
}

impl From<ModLoaderType> for domain::info::ModLoaderType {
    fn from(value: ModLoaderType) -> Self {
        match value {
            ModLoaderType::Forge => Self::Forge,
            ModLoaderType::Fabric => Self::Fabric,
        }
    }
}

impl From<manager::ListGroup> for ListGroup {
    fn from(value: manager::ListGroup) -> Self {
        Self {
            id: value.id.into(),
            name: value.name,
            instances: value.instances.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<manager::ListInstance> for ListInstance {
    fn from(value: manager::ListInstance) -> Self {
        Self {
            id: value.id.into(),
            name: value.name,
            favorite: value.favorite,
            status: value.status.into(),
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
        }
    }
}

impl From<domain::Mod> for Mod {
    fn from(value: domain::Mod) -> Self {
        Self {
            id: value.id,
            filename: value.filename,
            enabled: value.enabled,
            modloader: value.modloader.into(),
            metadata: value.metadata.into(),
        }
    }
}

impl From<domain::ModFileMetadata> for ModFileMetadata {
    fn from(value: domain::ModFileMetadata) -> Self {
        Self {
            modid: value.modid,
            name: value.name,
            version: value.version,
            description: value.description,
            authors: value.authors,
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

impl From<UpdateInstance> for domain::InstanceSettingsUpdate {
    fn from(value: UpdateInstance) -> Self {
        Self {
            instance_id: value.instance.into(),
            name: value.name.map(|x| x.inner()),
            use_loaded_icon: value.use_loaded_icon.map(|x| x.inner()),
            notes: value.notes.map(|x| x.inner()),
            version: value.version.map(|x| x.inner()),
            modloader: value.modloader.map(|x| x.inner().map(Into::into)),
            global_java_args: value.global_java_args.map(|x| x.inner()),
            extra_java_args: value.extra_java_args.map(|x| x.inner()),
            memory: value.memory.map(|x| x.inner().map(Into::into)),
        }
    }
}
