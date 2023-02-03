use crate::api::app::GlobalContext;
use crate::api::keys::mc::*;
use crate::api::router::router;
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

#[derive(Type, Deserialize)]
struct UpdateInstanceArgs {
    id: String,
    new_name: String,
}

pub(super) fn mount() -> impl RouterBuilderLike<GlobalContext> {
    router! {
        query GET_INSTANCES[_, _args: ()] {
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
        }

        query GET_INSTANCE_DETAILS[_, args: String] {
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
        }
        mutation OPEN_INSTANCE_FOLDER_PATH[_, args: String] {}
        mutation START_INSTANCE[_, args: String] {}
        mutation STOP_INSTANCE[_, args: String] {}
        mutation DELETE_INSTANCE[_, args: String] {}
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
        mutation UPDATE_INSTANCE_NAME[_, args: UpdateInstanceArgs] {}
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
