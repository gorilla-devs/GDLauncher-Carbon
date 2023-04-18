use std::collections::HashSet;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

use crate::managers::instance::InstanceMoveTarget;
use crate::managers::App;

use super::keys::instance::*;
use super::router::router;

use crate::domain::instance as domain;
use crate::managers::instance::{self as manager, GroupId, InstanceId};

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_GROUPS[app, _: ()] {
            app.instance_manager()
                .list_groups()
                .await
        }

        mutation CREATE_GROUP[app, name: String] {
            app.instance_manager()
                .create_group(name)
                .await
        }

        mutation CREATE_INSTANCE[app, details: CreateInstance] {
            app.instance_manager()
                .create_instance(
                    GroupId(details.group),
                    details.name,
                    details.icon.map(PathBuf::from),
                    details.version.into()
                )
                .await
        }

        mutation DELETE_GROUP[app, id: i32] {
            app.instance_manager()
                .delete_group(GroupId(id))
                .await
        }

        mutation DELETE_INSTANCE[app, id: i32] {
            app.instance_manager()
                .delete_instance(InstanceId(id))
                .await
        }

        mutation MOVE_GROUP[app, move_data: MoveGroup] {
            app.instance_manager()
                .move_group(
                    GroupId(move_data.group),
                    move_data.before.map(GroupId)
                )
                .await
        }

        mutation MOVE_INSTANCE[app, move_instance: MoveInstance] {
            app.instance_manager()
                .move_instance(
                    InstanceId(move_instance.instance),
                    match move_instance.target {
                        MoveInstanceTarget::BeforeInstance(instance)
                            => InstanceMoveTarget::Before(InstanceId(instance)),
                        MoveInstanceTarget::EndOfGroup(group)
                            => InstanceMoveTarget::EndOfGroup(GroupId(group)),
                    }
                )
                .await
        }

        mutation UPDATE_INSTANCE[app, details: UpdateInstance] {
            app.instance_manager()
                .update_instance(
                    InstanceId(details.instance),
                    details.name,
                    details.icon.map(|i| i.map(PathBuf::from))
                )
                .await
        }

        query INSTANCE_DETAILS[app, id: i32] {
            app.instance_manager()
                .instance_details(InstanceId(id))
                .await
                .map(InstanceDetails::from)
        }
    }
}

#[derive(Type, Deserialize)]
struct CreateInstance {
    group: i32,
    name: String,
    icon: Option<String>,
    version: CreateInstanceVersion,
}

#[derive(Type, Deserialize)]
struct UpdateInstance {
    instance: i32,
    name: Option<String>,
    icon: Option<Option<String>>,
    // version
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
    group: i32,
    before: Option<i32>,
}

#[derive(Type, Deserialize)]
struct MoveInstance {
    instance: i32,
    target: MoveInstanceTarget,
}

#[derive(Type, Deserialize)]
enum MoveInstanceTarget {
    BeforeInstance(i32),
    EndOfGroup(i32),
}

#[derive(Type, Serialize)]
pub struct InstanceDetails {
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
    Fabirc,
}

impl From<domain::InstanceDetails> for InstanceDetails {
    fn from(value: domain::InstanceDetails) -> Self {
        Self {
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
            domain::ModLoaderType::Fabirc => Self::Fabirc,
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
            ModLoaderType::Fabirc => Self::Fabric,
        }
    }
}
