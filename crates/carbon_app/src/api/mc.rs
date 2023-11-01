use crate::api::keys::mc::*;
use crate::api::managers::App;
use crate::api::router::router;

use daedalus::{minecraft, modded};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

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

        query GET_NEOFORGE_VERSIONS[app, _args: ()] {
            let res = app.minecraft_manager().get_neoforge_manifest().await?;

            Ok(FEModdedManifest::from(res))
        }

        query GET_FABRIC_VERSIONS[app, _args: ()] {
            let res = app.minecraft_manager().get_fabric_manifest().await?;

            Ok(FEModdedManifest::from(res))
        }

        query GET_QUILT_VERSIONS[app, _args: ()] {
            let res = app.minecraft_manager().get_quilt_manifest().await?;

            Ok(FEModdedManifest::from(res))
        }
    }
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
