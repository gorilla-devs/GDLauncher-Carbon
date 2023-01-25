use axum::extract::DefaultBodyLimit;
use rspc::{Router, RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use crate::api::app::AppContainer;

#[derive(Type, Serialize)]
struct Instance {
    id: String,
    name: String,
    mc_version: String,
    modloader: String,
}

#[derive(Type, Serialize)]
struct Mod {
    id: String,
    name: String,
}

#[derive(Type, Serialize)]
struct InstanceDetails {
    id: String,
    name: String,
    mc_version: String,
    modloader: String,
    modloader_version: String,
    mods: Vec<Mod>,
    played_time: u32,
    last_played: u32,
    notes: String,
}

#[derive(Type, Serialize)]
struct Instances(Vec<Instance>);

pub(super) fn mount() -> impl RouterBuilderLike<AppContainer> {
    Router::<AppContainer>::new()
        .query("getInstances", |t| {
            t(|_ctx: AppContainer, _args: ()| async move {
                let instances = vec![
                    Instance {
                        id: "88r39459345939453".to_string(),
                        name: "My first instance".to_string(),
                        mc_version: "1.16.5".to_string(),
                        modloader: "Forge".to_string(),
                    },
                    Instance {
                        id: "88r39459345939456".to_string(),
                        name: "My second instance".to_string(),
                        mc_version: "1.16.5".to_string(),
                        modloader: "Fabric".to_string(),
                    },
                    Instance {
                        id: "88r39459345939451".to_string(),
                        name: "Instance with a very long name".to_string(),
                        mc_version: "1.16.5".to_string(),
                        modloader: "Fabric".to_string(),
                    },
                    Instance {
                        id: "88r39459345336457".to_string(),
                        name: "Vanilla Minecraft".to_string(),
                        mc_version: "1.16.5".to_string(),
                        modloader: "Vanilla".to_string(),
                    },
                    Instance {
                        id: "84439459345336457".to_string(),
                        name: "Forge Minecraft".to_string(),
                        mc_version: "1.16.5".to_string(),
                        modloader: "Forge".to_string(),
                    },
                    Instance {
                        id: "82h39459345336457".to_string(),
                        name: "All The Mods 6".to_string(),
                        mc_version: "1.16.5".to_string(),
                        modloader: "Forge".to_string(),
                    },
                ];

                let final_instances = Instances(instances);

                Ok(final_instances)
            })
        })
        .query("getInstanceDetails", |t| {
            t(|_ctx: AppContainer, args: String| async move {
                let instance = InstanceDetails {
                    id: "88r39459345939453".to_string(),
                    name: "My first instance".to_string(),
                    mc_version: "1.16.5".to_string(),
                    modloader: "Forge".to_string(),
                    modloader_version: "1.16.5".to_string(),
                    mods: vec![
                        Mod {
                            id: "88r39459345939453".to_string(),
                            name: "My first instance".to_string(),
                        },
                        Mod {
                            id: "88r39459345939456".to_string(),
                            name: "My second instance".to_string(),
                        },
                        Mod {
                            id: "88r39459345939451".to_string(),
                            name: "Instance with a very long name".to_string(),
                        },
                        Mod {
                            id: "88r39459345336457".to_string(),
                            name: "Vanilla Minecraft".to_string(),
                        },
                        Mod {
                            id: "84439459345336457".to_string(),
                            name: "Forge Minecraft".to_string(),
                        },
                        Mod {
                            id: "82h39459345336457".to_string(),
                            name: "All The Mods 6".to_string(),
                        },
                    ],
                    played_time: 0,
                    last_played: 0,
                    notes: "This is a test instance".to_string(),
                };

                Ok(instance)
            })
        })
        .mutation("openInstanceFolderPath", |t| t(|_, args: String| {}))
        .mutation("startInstance", |t| t(|_, args: String| {}))
        .mutation("stopInstance", |t| t(|_, args: String| {}))
        .mutation("deleteInstance", |t| t(|_, args: String| {}))
        // Actions on mods
        .mutation("enableMod", |t| t(|_, args: String| {}))
        .mutation("disableMod", |t| t(|_, args: String| {}))
        .mutation("removeMod", |t| t(|_, args: String| {}))
        .mutation("removeMods", |t| t(|_, args: Vec<String>| {}))
        // Change versions
        .mutation("switchMinecraftVersion", |t| t(|_, args: String| {}))
        .mutation("switchModloader", |t| t(|_, args: String| {}))
        .mutation("switchModloaderVersion", |t| t(|_, args: String| {}))
        // Instance settings
        .mutation("updateInstanceName", |t| {
            #[derive(Type, Deserialize)]
            struct Args {
                id: String,
                new_name: String,
            }
            t(|_, args: Args| {})
        })
        .query("getInstanceMemory", |t| {
            t(|_ctx: AppContainer, args: String| async move {})
        })
        .mutation("updateInstanceMemory", |t| t(|_, args: u8| {}))
        .query("getInstanceJavaArgs", |t| {
            t(|_ctx: AppContainer, args: String| async move {})
        })
        .mutation("updateInstanceJavaArgs", |t| t(|_, args: String| {}))
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
