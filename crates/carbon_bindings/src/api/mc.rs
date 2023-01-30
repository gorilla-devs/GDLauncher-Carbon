use std::collections::BTreeMap;
use axum::extract::DefaultBodyLimit;
use rspc::{ErrorCode, Router, RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use carbon_minecraft::instance::Instance;
use crate::api::app::AppContainer;

/*#[derive(Type, Serialize)]
struct Instance {
    id: String,
    name: String,
    mc_version: String,
    modloader: String,
}
*/
#[derive(Type, Serialize)]
struct Mod {
    id: String,
    name: String,
}

#[derive(Type, Serialize)]
struct ModDetails {
    id: String,
    mod_name: String,
}

#[derive(Type, Serialize)]
struct ModLoadersDetails {
    mod_loader_name: String,
    mod_loader_version: String,
}

#[derive(Type, Serialize)]
struct InstanceDetails {
    id: String,
    name: String,
    mc_version: String,
    mod_loaders: Vec<ModLoadersDetails>,
    mods: Vec<ModDetails>,
    played_time: u32,
    last_played: u32,
    notes: String,
}

impl Into<InstanceDetails> for Instance {
    fn into(self) -> InstanceDetails {
        let instance = &self;
        let mod_loaders = instance.minecraft_package.modloaders.iter()
            .map(|mod_loader| ModLoadersDetails {
                mod_loader_name: mod_loader.to_string(),
                mod_loader_version: mod_loader.get_version(),
            })
            .collect();
        let mut mods = instance.minecraft_package.mods.iter()
            .map(|minecraft_mod| ModDetails {
                id: minecraft_mod.id.clone().to_string(),
                mod_name: minecraft_mod.name.clone(),
            })
            .collect();
        InstanceDetails {
            id: instance.id.to_string(),
            name: instance.name.clone(),
            mc_version: instance.minecraft_package.version.clone(),
            mod_loaders,
            mods,
            played_time: instance.played_time.into(),
            last_played: instance.last_played.into(),
            notes: "".to_string(),
        }
    }
}


#[derive(Type, Serialize)]
struct Instances(Vec<Instance>);

pub(super) fn mount() -> impl RouterBuilderLike<AppContainer> {
    Router::<AppContainer>::new()
        .mutation("createInstance", |t| {
            t(|app: AppContainer, args: CreateInstanceDto| async move {
                let app = app.read().await;
                let instance_manager = app.get_instance_manager().await
                    .map_err(|error| rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("{:?}", error),
                    ))?;
            })
        })
        .query("getInstances", |t| {
            t(|app: AppContainer, _args: ()| async move {
                let app = app.read().await;
                let instance_manager = app.get_instance_manager().await
                    .map_err(|error| rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("{:?}", error),
                    ))?;
                instance_manager.get_all_instances()
            })
        })
        .query("getInstanceDetails", |t| {
            t(|app: AppContainer, instance_id: String| async move {
                let app = app.read().await;
                let instance_manager = app.get_instance_manager().await
                    .map_err(|error| rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("{:?}", error),
                    ))?;
                let instance = instance_manager.get_instance_by_id(instance_id).await
                    .ok_or(rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("instance with id {instance_id} not found"),
                    ))?;
                Ok(instance)
            })
        })
        .mutation("openInstanceFolderPath", |t| t(|_, args: String| {}))
        .mutation("startInstance", |t|
            t(|app: AppContainer, instance_id: String| async move {
                let app = app.read().await;
                let instance_manager = app.get_instance_manager().await
                    .map_err(|error| rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("{:?}", error),
                    ))?;
                let instance = instance_manager.run_instance_by_id(instance_id).await
                    .ok_or(rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("instance with id {instance_id} not found"),
                    ))?;
                Ok(instance)
            }))
        .mutation("stopInstance", |t|{
            struct RemoveInstanceDto{

            }
            t(|app: AppContainer, instance_id: String| async move {
                let app = app.read().await;
                let instance_manager = app.get_instance_manager().await
                    .map_err(|error| rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("{:?}", error),
                    ))?;
                let instance = instance_manager.stop_instance_by_id(instance_id).await
                    .ok_or(rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("instance with id {instance_id} not found"),
                    ))?;
                Ok(instance)
            })})
        .mutation("deleteInstance", |t|
            t(|app: AppContainer, instance_id: String| async move {
                let app = app.read().await;
                let instance_manager = app.get_instance_manager().await
                    .map_err(|error| rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("{:?}", error),
                    ))?;
                let instance = instance_manager.delete_instance_by_id(instance_id, false).await
                    .ok_or(rspc::Error::new(
                        ErrorCode::InternalServerError,
                        format!("instance with id {instance_id} not found"),
                    ))?;
                Ok(instance)
            }))
        // Actions on mods
        .mutation("enableMod", |t|
            t(|app: AppContainer, args: String| async move {

            }))
        .mutation("disableMod", |t|
            t(|app: AppContainer, args: String| async move {

        }))
        .mutation("removeMod", |t| t(|app: AppContainer, args: String| async move {

        }))
        .mutation("removeMods", |t| t(|app: AppContainer, args: Vec<String>| async move {

        }))
        // Change versions
        .mutation("switchMinecraftVersion", |t| t(|app: AppContainer, args: String| async move {

        }))
        .mutation("switchModloader", |t| t(|app: AppContainer, args: String| async move {

        }))
        .mutation("switchModloaderVersion", |t| t(|app: AppContainer, args: String| async move {

        }))
        // Instance settings
        .mutation("updateInstanceName", |t| {
            #[derive(Type, Deserialize)]
            struct Args {
                id: String,
                new_name: String,
            }
            t(|app: AppContainer, args: Args| async move {

            })
        })
        .query("getInstanceMemory", |t| {
            t(|app: AppContainer, args: String| async move {

            })
        })
        .mutation("updateInstanceMemory", |t| t(|_, args: u8| {

        }))
        .query("getInstanceJavaArgs", |t| {
            t(|app: AppContainer, args: String| async move {

            })
        })
        .mutation("updateInstanceJavaArgs", |t|
            t(|app: AppContainer, args: String| async move {

        }))
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
