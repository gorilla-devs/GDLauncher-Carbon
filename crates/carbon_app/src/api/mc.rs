use crate::api::keys::mc::*;
use crate::api::managers::App;
use crate::api::router::router;
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_MINECRAFT_VERSIONS[app, _args: ()] {
            let res = app.minecraft_manager().get_minecraft_versions().await;

            Ok(res.into_iter().map(Into::into).collect::<Vec<ManifestVersion>>())
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize, Clone)]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: Type,
}

impl From<crate::domain::minecraft::manifest::ManifestVersion> for ManifestVersion {
    fn from(value: crate::domain::minecraft::manifest::ManifestVersion) -> Self {
        ManifestVersion {
            id: value.id,
            type_: value.type_.into(),
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize, Clone)]
pub enum Type {
    #[serde(rename = "old_alpha")]
    OldAlpha,
    #[serde(rename = "old_beta")]
    OldBeta,
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "snapshot")]
    Snapshot,
}

impl From<crate::domain::minecraft::manifest::Type> for Type {
    fn from(value: crate::domain::minecraft::manifest::Type) -> Self {
        use crate::domain::minecraft::manifest::Type as domain;

        match value {
            domain::OldAlpha => Self::OldAlpha,
            domain::OldBeta => Self::OldBeta,
            domain::Release => Self::Release,
            domain::Snapshot => Self::Snapshot,
        }
    }
}

impl From<Type> for String {
    fn from(type_: Type) -> Self {
        match type_ {
            Type::OldAlpha => "old_alpha".to_string(),
            Type::OldBeta => "old_beta".to_string(),
            Type::Release => "release".to_string(),
            Type::Snapshot => "snapshot".to_string(),
        }
    }
}
