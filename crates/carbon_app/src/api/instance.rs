use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use axum::extract::{Query, State};
use chrono::{DateTime, Utc};
use http::{HeaderMap, HeaderValue, StatusCode};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

use crate::error::{AxumError, FeError};
use crate::managers::instance::InstanceMoveTarget;
use crate::managers::{App, AppInner};

use super::keys::instance::*;
use super::router::router;

use crate::domain::instance as domain;
use crate::managers::instance as manager;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query DEFAULT_GROUP[app, _: ()] {
            Ok(*app.instance_manager()
                .get_default_group()
                .await?)
        }

        query GET_GROUPS[app, _: ()] {
            Ok(app.instance_manager()
                .list_groups()
                .await?
                .into_iter()
                .map(ListGroup::from)
                .collect::<Vec<_>>())
        }

        query GET_INSTANCES_UNGROUPED[app, _: ()] {
            Ok(app.instance_manager()
                .list_groups()
                .await?
                .into_iter()
                .flat_map(|group| {
                    group.instances
                        .into_iter()
                        .map(|instance| UngroupedInstance {
                            favorite: group.name == "localize➽favorite",
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
        }

        mutation CREATE_INSTANCE[app, details: CreateInstance] {
            app.instance_manager()
                .create_instance(
                    details.group.into(),
                    details.name,
                    details.icon.map(PathBuf::from),
                    details.version.into()
                )
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

        mutation UPDATE_INSTANCE[app, details: UpdateInstance] {
            app.instance_manager()
                .update_instance(
                    details.instance.into(),
                    details.name.into(),
                    details.icon.into_option().map(|i| i.map(PathBuf::from))
                )
                .await
        }

        query INSTANCE_DETAILS[app, id: InstanceId] {
            app.instance_manager()
                .instance_details(id.into())
                .await
                .map(InstanceDetails::from)
        }
    }
}

pub(super) fn mount_axum_router() -> axum::Router<Arc<AppInner>> {
    #[derive(Deserialize)]
    struct InstanceIconQuery {
        id: i32,
    }

    axum::Router::new().route(
        "/instanceIcon",
        axum::routing::get(
            |State(app): State<Arc<AppInner>>, Query(query): Query<InstanceIconQuery>| async move {
                let icon = app
                    .instance_manager()
                    .instance_icon(manager::InstanceId(query.id))
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
}
#[derive(Type, Serialize, Deserialize)]
struct GroupId(i32);

#[derive(Type, Serialize, Deserialize)]
struct InstanceId(i32);

impl From<manager::GroupId> for GroupId {
    fn from(value: manager::GroupId) -> Self {
        Self(*value)
    }
}

impl From<manager::InstanceId> for InstanceId {
    fn from(value: manager::InstanceId) -> Self {
        Self(*value)
    }
}

impl From<GroupId> for manager::GroupId {
    fn from(value: GroupId) -> Self {
        Self(value.0)
    }
}

impl From<InstanceId> for manager::InstanceId {
    fn from(value: InstanceId) -> Self {
        Self(value.0)
    }
}

#[derive(Type, Serialize)]
struct ListGroup {
    id: GroupId,
    name: String,
    instances: Vec<ListInstance>,
}

#[derive(Type, Serialize)]
struct ListInstance {
    id: InstanceId,
    name: String,
    status: ListInstanceStatus,
}

#[derive(Type, Serialize)]
struct UngroupedInstance {
    favorite: bool,
    #[serde(flatten)]
    instance: ListInstance,
}

#[derive(Type, Serialize)]
enum ListInstanceStatus {
    Valid(ValidListInstance),
    Invalid(InvalidListInstance),
}

#[derive(Type, Serialize)]
struct ValidListInstance {
    mc_version: String,
    modloader: Option<ModLoaderType>,
    modpack_platform: Option<ModpackPlatform>,
}

#[derive(Type, Serialize)]
enum ModpackPlatform {
    Curseforge,
}

#[derive(Type, Serialize)]
enum InvalidListInstance {
    JsonMissing,
    JsonError(ConfigurationParseError),
    Other(String),
}

#[derive(Type, Serialize)]
struct ConfigurationParseError {
    type_: ConfigurationParseErrorType,
    message: String,
    line: u32,
    config_text: String,
}

#[derive(Type, Serialize)]
enum ConfigurationParseErrorType {
    Syntax,
    Data,
    Eof,
}

#[derive(Type, Deserialize)]
struct CreateInstance {
    group: GroupId,
    name: String,
    icon: Option<String>,
    version: CreateInstanceVersion,
}

#[derive(Type, Deserialize)]
struct UpdateInstance {
    instance: InstanceId,
    name: Update<String>,
    icon: Update<Option<String>>,
    // version
}

#[derive(Type, Deserialize)]
enum Update<T> {
    Changed(T),
    Unchanged,
}

impl<T> Update<T> {
    fn into_option(self) -> Option<T> {
        self.into()
    }
}

impl<T> Into<Option<T>> for Update<T> {
    fn into(self) -> Option<T> {
        match self {
            Self::Unchanged => None,
            Self::Changed(v) => Some(v),
        }
    }
}

#[derive(Type, Deserialize)]
enum CreateInstanceVersion {
    Version(GameVersion),
    // Modpack
}

#[derive(Type, Deserialize)]
enum GameVersion {
    Standard(StandardVersion),
    // Custom(json)
}

#[derive(Type, Deserialize)]
struct StandardVersion {
    release: String,
    modloaders: HashSet<ModLoader>,
}

#[derive(Type, Deserialize)]
struct MoveGroup {
    group: GroupId,
    before: Option<GroupId>,
}

#[derive(Type, Deserialize)]
struct MoveInstance {
    instance: InstanceId,
    target: MoveInstanceTarget,
}

#[derive(Type, Deserialize)]
enum MoveInstanceTarget {
    BeforeInstance(InstanceId),
    BeginningOfGroup(GroupId),
    EndOfGroup(GroupId),
}

#[derive(Type, Serialize)]
pub struct InstanceDetails {
    pub favorite: bool,
    pub name: String,
    pub version: String,
    pub last_played: DateTime<Utc>,
    pub seconds_played: u32,
    pub instance_start_time: Option<DateTime<Utc>>,
    pub modloaders: Vec<ModLoader>,
    pub notes: String,
}

#[derive(Type, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ModLoader {
    pub type_: ModLoaderType,
    pub version: String,
}

#[derive(Type, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModLoaderType {
    Forge,
    Fabric,
}

impl From<domain::InstanceDetails> for InstanceDetails {
    fn from(value: domain::InstanceDetails) -> Self {
        Self {
            favorite: value.favorite,
            name: value.name,
            version: value.version,
            last_played: value.last_played,
            seconds_played: value.seconds_played,
            instance_start_time: value.instance_start_time,
            modloaders: value.modloaders.into_iter().map(Into::into).collect(),
            notes: value.notes,
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

impl From<domain::ModLoader> for ModLoader {
    fn from(value: domain::ModLoader) -> Self {
        Self {
            type_: value.type_.into(),
            version: value.version,
        }
    }
}

impl From<domain::ModLoaderType> for ModLoaderType {
    fn from(value: domain::ModLoaderType) -> Self {
        match value {
            domain::ModLoaderType::Forge => Self::Forge,
            domain::ModLoaderType::Fabirc => Self::Fabric,
        }
    }
}

impl From<CreateInstanceVersion> for manager::InstanceVersionSouce {
    fn from(value: CreateInstanceVersion) -> Self {
        match value {
            CreateInstanceVersion::Version(v) => Self::Version(v.into()),
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

impl From<domain::info::ModLoaderType> for ModLoaderType {
    fn from(value: domain::info::ModLoaderType) -> Self {
        match value {
            domain::info::ModLoaderType::Forge => Self::Forge,
            domain::info::ModLoaderType::Fabric => Self::Fabric,
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
