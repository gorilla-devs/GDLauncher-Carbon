use crate::api::keys::mc::*;
use crate::api::managers::Managers;
use crate::api::router::{router, try_in_router};
use crate::managers::representation::CreateInstanceDto;
use axum::extract::DefaultBodyLimit;
use carbon_domain::instance::InstanceStatus;
use log::warn;
use rspc::{RouterBuilderLike, Type};
use serde::Deserialize;
use serde_json::Value;

#[derive(Type, Deserialize)]
struct UpdateInstanceArgs {
    id: String,
    new_props: std::collections::BTreeMap<String, Value>,
}

#[derive(Type, Deserialize)]
struct DeleteInstanceArgs {
    id: String,
    move_to_trash_bin: bool,
}

pub(super) fn mount() -> impl RouterBuilderLike<Managers> {
    router! {
        query GET_INSTANCES[app, _: ()] {
            let instances = app.instance_manager.get_all_instances().await;
            Ok(instances)
        }
        query GET_INSTANCE_DETAILS[app, instance_uuid: String] {
            let instance_uuid = try_in_router!(instance_uuid.parse())?;
            let instance = try_in_router!(app.instance_manager.get_instance_by_id(instance_uuid).await)?;
            Ok(instance)
        }
        mutation UPDATE_INSTANCE[app, args: UpdateInstanceArgs] {
            let instance_uuid = try_in_router!(args.id.parse())?;
            try_in_router!(app.instance_manager.patch_instance_by_id(instance_uuid, args.new_props).await)?;
            Ok(())
        }
        mutation DELETE_INSTANCE[app, args: DeleteInstanceArgs] {
            let instance_uuid = try_in_router!(args.id.parse())?;
            try_in_router!(app.instance_manager.delete_instance_by_id(instance_uuid,args.move_to_trash_bin).await)?;
            Ok(())
        }
        mutation OPEN_INSTANCE_FOLDER_PATH[app, instance_uuid: String] {
            let instance_uuid = try_in_router!(instance_uuid.parse())?;
            let instance = try_in_router!(app.instance_manager.get_instance_by_id(instance_uuid).await)?;
            let opened = match instance.persistence_status {
                InstanceStatus::Installing(path) | InstanceStatus::Ready(path) => {
                    try_in_router!(open::that(path))?;
                    true
                }
                InstanceStatus::NotPersisted =>{
                    warn!("cannot open instance with id {instance_uuid} folder in system file manager since is not persisted");
                    false
                }
            };
            app.invalidate(
                OPEN_INSTANCE_FOLDER_PATH,
                Some(opened.into()),
            );
            Ok(())
        }
        mutation SAVE_NEW_INSTANCE[app, create_instance_dto: CreateInstanceDto] {
            try_in_router!(app.instance_manager.add_instance(create_instance_dto).await)?;
            Ok(())
        }
        mutation START_INSTANCE[app, instance_uuid: String] {
            let instance_uuid = try_in_router!(instance_uuid.parse())?;
            try_in_router!(app.instance_manager.start_instance_by_id(instance_uuid).await)?;
            Ok(())
        }
        mutation STOP_INSTANCE[app, instance_uuid: String] {
            let instance_uuid = try_in_router!(instance_uuid.parse())?;
            try_in_router!(app.instance_manager.stop_instance_by_id(instance_uuid).await)?;
            Ok(())
        }
        // Actions on mods
        mutation ENABLE_MOD[_, _args: String] {}
        mutation DISABLE_MOD[_, _args: String] {}
        mutation REMOVE_MOD[_, _args: String] {}
        mutation REMOVE_MODS[_, _args: Vec<String>] {}
        // Change versions
        mutation SWITCH_MINECRAFT_VERSION[_, _args: String] {}
        mutation SWITCH_MODLOADER[_, _args: String] {}
        mutation SWITCH_MODLOADER_VERSION[_, _args: String] {}
        // Instance settings
        query GET_INSTANCE_MEMORY[_, _args: String] {}
        mutation UPDATE_INSTANCE_MEMORY[_, _args: u8] {}
        query GET_INSTANCE_JAVA_ARGS[_, _args: String] {}
        mutation UPDATE_INSTANCE_JAVA_ARGS[_, _args: String] {}
    }
}

pub(super) fn mount_axum_router() -> axum::Router<()> {
    axum::Router::new()
        .route(
            "/instanceThumbnail",
            axum::routing::get(|| async {
                // Read params and get the instance id, then return the thumbnail in base64
            }),
        )
        .layer(DefaultBodyLimit::max(4096)) // this is probably enough for a thumbnail
}
