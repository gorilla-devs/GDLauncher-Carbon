use rspc::{RouterBuilderLike, Type};
use serde::Deserialize;

use crate::managers::App;
use crate::{error::anyhow_into_rspc, managers::instance::InstanceMoveTarget};

use super::keys::instance::*;
use super::router::router;

use crate::managers::instance::{GroupId, InstanceId};

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_GROUPS[app, _: ()] {
            app.instance_manager()
                .list_groups()
                .await
                .map_err(anyhow_into_rspc)
        }

        mutation CREATE_GROUP[app, name: String] {
            app.instance_manager()
                .create_group(name)
                .await
                .map_err(anyhow_into_rspc)
        }

        mutation DELETE_GROUP[app, id: i32] {
            app.instance_manager()
                .delete_group(GroupId(id))
                .await
                .map_err(anyhow_into_rspc)
        }

        mutation MOVE_GROUP[app, move_data: MoveGroup] {
            app.instance_manager()
                .move_group(
                    GroupId(move_data.group),
                    move_data.before.map(GroupId)
                )
                .await
                .map_err(anyhow_into_rspc)
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
                .map_err(anyhow_into_rspc)
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
