use crate::api::keys::mc::*;
use crate::api::managers::Managers;
use crate::api::router::router;
use axum::extract::DefaultBodyLimit;
use rspc::{Router, RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
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
            let instance_uuid = instance_uuid.parse()?;
            let instance = app.instance_manager.get_instance_by_id(instance_uuid).await?;
            Ok(instance)
        }
        mutation UPDATE_INSTANCE[app, args: UpdateInstanceArgs] {
            let instance_uuid = args.id.parse()?;
            app.instance_manager.patch_instance_by_id(instance_uuid, args.new_props).await?;
        }
        mutation DELETE_INSTANCE[app, args: DeleteInstanceArgs] {
            let instance_uuid = args.id.parse()?;
            let instance = app.instance_manager.delete_instance_by_id(instance_uuid,args.move_to_trash_bin).await?;
        }
        mutation OPEN_INSTANCE_FOLDER_PATH[app, instance_uuid: String] {
            let instance_uuid = instance_uuid.parse()?;
            let instance = app.instance_manager.get_instance_by_id(instance_uuid).await?;
        }
        mutation SAVE_NEW_INSTANCE[app, create_instance_dto: CreateInstanceDto] {
            let instance = app.instance_manager.add_instance(create_instance_dto).await?;
        }
        mutation START_INSTANCE[app, args: String] {}
        mutation STOP_INSTANCE[app, args: String] {}
        // Actions on mods
        mutation ENABLE_MOD[_, args: String] {}
        mutation DISABLE_MOD[_, args: String] {}
        mutation REMOVE_MOD[_, args: String] {}
        mutation REMOVE_MODS[_, args: Vec<String>] {}
        // Change versions
        mutation SWITCH_MINECRAFT_VERSION[_, args: String] {}
        mutation SWITCH_MODLOADER[_, args: String] {}
        mutation SWITCH_MODLOADER_VERSION[_, args: String] {}
        // Instance settings
        query GET_INSTANCE_MEMORY[_, args: String] {}
        mutation UPDATE_INSTANCE_MEMORY[_, args: u8] {}
        query GET_INSTANCE_JAVA_ARGS[_, args: String] {}
        mutation UPDATE_INSTANCE_JAVA_ARGS[_, args: String] {}
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
