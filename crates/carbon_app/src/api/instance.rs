use chrono::{DateTime, Utc};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

use crate::managers::instance::InstanceMoveTarget;
use crate::managers::App;

use super::keys::instance::*;
use super::router::router;

use crate::domain::instance as domain;
use crate::managers::instance::{GroupId, InstanceId};

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

        mutation DELETE_GROUP[app, id: i32] {
            app.instance_manager()
                .delete_group(GroupId(id))
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

        query INSTANCE_DETAILS[app, id: i32] {
            app.instance_manager()
                .instance_details(InstanceId(id))
                .await
                .map(InstanceDetails::from)
        }
    }
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

#[derive(Type, Serialize)]
pub struct ModLoader {
    pub type_: ModLoaderType,
    pub version: String,
}

#[derive(Type, Serialize)]
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
