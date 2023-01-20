use super::GlobalContext;
use axum::extract::DefaultBodyLimit;
use rspc::{Router, RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

#[derive(Type, Serialize)]
struct Instance {
    id: String,
    name: String,
    mc_version: String,
    modloader: String,
}

struct Mod {
    id: String,
    name: String,
}

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

pub(super) fn mount() -> impl RouterBuilderLike<GlobalContext> {
    Router::<GlobalContext>::new()
        .query("getInstances", |t| {
            t(|_ctx: GlobalContext, _args: ()| async move {
                let mut instances = vec![
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
            t(|_ctx: GlobalContext, args: String| async move {
                let instance = Instance {
                    id: "82h39459345336457".to_string(),
                    name: "All The Mods 6".to_string(),
                };

                Ok(instance)
            })
        })
        .mutation("updateInstanceName", |t| {
            #[derive(Type, Deserialize)]
            struct Args {
                id: String,
                new_name: String,
            }
            t(|_, args: Args| {})
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
