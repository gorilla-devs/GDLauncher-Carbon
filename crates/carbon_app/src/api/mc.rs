use std::sync::Arc;

use crate::api::keys::mc::*;
use crate::api::managers::App;
use crate::api::router::router;
use crate::managers::AppInner;
use axum::extract::DefaultBodyLimit;
use daedalus::{minecraft, modded};
use rspc::{RouterBuilderLike, Type};
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

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_MINECRAFT_VERSIONS[app, _args: ()] {
            let res = app.minecraft_manager().get_minecraft_manifest().await?.versions;

            Ok(res.into_iter().map(ManifestVersion::from).collect::<Vec<_>>())
        }

        query GET_FORGE_VERSIONS[app, _args: ()] {
            let res = app.minecraft_manager().get_forge_manifest().await?;

            Ok(FEModdedManifest::from(res))
        }

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

        query GET_INSTANCE_DETAILS[_, _args: String] {
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
        mutation OPEN_INSTANCE_FOLDER_PATH[_, _args: String] { Ok(()) }
        mutation START_INSTANCE[_, _args: String] { Ok(()) }
        mutation STOP_INSTANCE[_, _args: String] { Ok(()) }
        mutation DELETE_INSTANCE[_, _args: String] { Ok(()) }
        // Actions on mods
        mutation ENABLE_MOD[_, _args: String] { Ok(()) }
        mutation DISABLE_MOD[_, _args: String] { Ok(()) }
        mutation REMOVE_MOD[_, _args: String] { Ok(()) }
        mutation REMOVE_MODS[_, _args: Vec<String>] { Ok(()) }
        // Change versions
        mutation SWITCH_MINECRAFT_VERSION[_, _args: String] { Ok(()) }
        mutation SWITCH_MODLOADER[_, _args: String] { Ok(()) }
        mutation SWITCH_MODLOADER_VERSION[_, _args: String] { Ok(()) }
        // Instance settings
        mutation UPDATE_INSTANCE_NAME[_, _args: UpdateInstanceArgs] { Ok(()) }
        query GET_INSTANCE_MEMORY[_, _args: String] { Ok(()) }
        mutation UPDATE_INSTANCE_MEMORY[_, _args: u8] { Ok(()) }
        query GET_INSTANCE_JAVA_ARGS[_, _args: String] { Ok(()) }
        mutation UPDATE_INSTANCE_JAVA_ARGS[_, _args: String] { Ok(()) }
    }
}

pub(super) fn mount_axum_router() -> axum::Router<Arc<AppInner>> {
    axum::Router::new()
        .route(
            "/instanceThumbnail",
            axum::routing::get(|| async {
                // Read params and get the instance id, then return the thumbnail in base64
            }),
        )
        .layer(DefaultBodyLimit::max(4096)) // this is probably enough for a thumbnail
}

#[derive(Type, Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FEModdedManifest {
    pub game_versions: Vec<FEModdedManifestVersion>,
}

impl From<modded::Manifest> for FEModdedManifest {
    fn from(value: modded::Manifest) -> Self {
        FEModdedManifest {
            game_versions: value.game_versions.into_iter().map(|v| v.into()).collect(),
        }
    }
}

#[derive(Type, Serialize, Deserialize, Debug, Clone)]
pub struct FEModdedManifestVersion {
    pub id: String,
    pub stable: bool,
    pub loaders: Vec<FEModdedManifestLoaderVersion>,
}

impl From<modded::Version> for FEModdedManifestVersion {
    fn from(value: modded::Version) -> Self {
        FEModdedManifestVersion {
            id: value.id,
            stable: value.stable,
            loaders: value.loaders.into_iter().map(|v| v.into()).collect(),
        }
    }
}

#[derive(Type, Serialize, Deserialize, Debug, Clone)]
pub struct FEModdedManifestLoaderVersion {
    pub id: String,
}

impl From<modded::LoaderVersion> for FEModdedManifestLoaderVersion {
    fn from(value: modded::LoaderVersion) -> Self {
        FEModdedManifestLoaderVersion { id: value.id }
    }
}

#[derive(Type, Debug, Serialize, Deserialize, Clone)]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: McType,
}

impl From<minecraft::Version> for ManifestVersion {
    fn from(value: minecraft::Version) -> Self {
        ManifestVersion {
            id: value.id,
            type_: value.type_.into(),
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize, Clone)]
pub enum McType {
    #[serde(rename = "old_alpha")]
    OldAlpha,
    #[serde(rename = "old_beta")]
    OldBeta,
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "snapshot")]
    Snapshot,
}

impl From<minecraft::VersionType> for McType {
    fn from(value: minecraft::VersionType) -> Self {
        match value {
            minecraft::VersionType::OldAlpha => Self::OldAlpha,
            minecraft::VersionType::OldBeta => Self::OldBeta,
            minecraft::VersionType::Release => Self::Release,
            minecraft::VersionType::Snapshot => Self::Snapshot,
        }
    }
}

impl From<McType> for String {
    fn from(type_: McType) -> Self {
        match type_ {
            McType::OldAlpha => "old_alpha".to_string(),
            McType::OldBeta => "old_beta".to_string(),
            McType::Release => "release".to_string(),
            McType::Snapshot => "snapshot".to_string(),
        }
    }
}
