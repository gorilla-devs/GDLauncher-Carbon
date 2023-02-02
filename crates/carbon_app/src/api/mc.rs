use crate::api::app::AppContainer;
use crate::app::instance::representation::CreateInstanceDto;
use crate::try_in_router;
use axum::extract::DefaultBodyLimit;
use rspc::{ErrorCode, Router, RouterBuilderLike, Type};
use serde::Deserialize;

pub(super) fn mount() -> impl RouterBuilderLike<AppContainer> {
    Router::<AppContainer>::new()
        .mutation("createInstance", |t| {
            t(|app: AppContainer, dto: CreateInstanceDto| async move {
                let app = app.read().await;
                let instance_manager = try_in_router!(app.get_instance_manager().await)?;
                instance_manager.add_instance(dto).await?
            })
        })
        .query("getInstances", |t| {
            t(|app: AppContainer, _args: ()| async move {
                let app = app.read().await;
                let instance_manager = try_in_router!(app.get_instance_manager().await).await?;
                instance_manager.get_all_instances()
            })
        })
        .query("getInstanceDetails", |t| {
            t(|app: AppContainer, instance_id: String| async move {
                let app = app.read().await;
                let instance_id = try_in_router!(instance_id.parse::<u128>())?;
                let instance_manager = try_in_router!(app.get_instance_manager().await)?;
                let instance =
                    try_in_router!(instance_manager.get_instance_by_id(instance_id).await)?;
                Ok(instance)
            })
        })
        .mutation("openInstanceFolderPath", |t| t(|_, args: String| {}))
        .mutation("startInstance", |t| {
            t(|app: AppContainer, instance_id: String| async move {
                let app = app.read().await;
                let instance_manager = try_in_router!(app.get_instance_manager().await)?;
                let instance =
                    try_in_router!(instance_manager.start_instance_by_id(instance_id).await)?;
                Ok(instance)
            })
        })
        .mutation("stopInstance", |t| {
            struct RemoveInstanceDto {}
            t(|app: AppContainer, instance_id: String| async move {
                let app = app.read().await;
                let instance_manager = try_in_router!(app.get_instance_manager().await)?;
                let instance_id = try_in_router!(instance_id.parse::<u128>())?;
                let instance =
                    try_in_router!(instance_manager.stop_instance_by_id(instance_id).await)?;
                Ok(instance)
            })
        })
        .mutation("deleteInstance", |t| {
            t(|app: AppContainer, instance_id: String| async move {
                let app = app.read().await;
                let instance_manager = try_in_router!(app.get_instance_manager().await)?;
                let instance_id = try_in_router!(instance_id.parse::<u128>())?;
                let instance = try_in_router!(
                    instance_manager
                        .delete_instance_by_id(instance_id, false)
                        .await
                );
                Ok(instance)
            })
        })
        // Actions on mods
        .mutation("enableMod", |t| {
            t(|app: AppContainer, args: String| async move {})
        })
        .mutation("disableMod", |t| {
            t(|app: AppContainer, args: String| async move {})
        })
        .mutation("removeMod", |t| {
            t(|app: AppContainer, args: String| async move {})
        })
        .mutation("removeMods", |t| {
            t(|app: AppContainer, args: Vec<String>| async move {})
        })
        // Change versions
        .mutation("switchMinecraftVersion", |t| {
            t(|app: AppContainer, args: String| async move {})
        })
        .mutation("switchModloader", |t| {
            t(|app: AppContainer, args: String| async move {})
        })
        .mutation("switchModloaderVersion", |t| {
            t(|app: AppContainer, args: String| async move {})
        })
        // Instance settings
        .mutation("updateInstanceName", |t| {
            #[derive(Type, Deserialize)]
            struct Args {
                id: String,
                new_name: String,
            }
            t(|app: AppContainer, args: Args| async move {})
        })
        .query("getInstanceMemory", |t| {
            t(|app: AppContainer, args: String| async move {})
        })
        .mutation("updateInstanceMemory", |t| t(|_, args: u8| {}))
        .query("getInstanceJavaArgs", |t| {
            t(|app: AppContainer, args: String| async move {})
        })
        .mutation("updateInstanceJavaArgs", |t| {
            t(|app: AppContainer, args: String| async move {})
        })
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
