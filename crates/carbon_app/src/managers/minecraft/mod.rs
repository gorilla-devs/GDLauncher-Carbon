use super::{AppRef, Managers};
use crate::db::{minecraft_manifest::SetParam, PrismaClient};
use crate::managers::ManagersInner;
use carbon_domain::minecraft::manifest::MinecraftManifest;
use std::sync::{Arc, Weak};
use thiserror::Error;

mod assets;
mod manifest;
mod version;

// #[derive(Error, Debug)]
// pub enum MinecraftManagerError {
//     #[error("Cannot fetch manifest from HTTP: {0}")]
//     ManifestFetchError(reqwest::Error),
//     #[error("Manifest does not meet expected JSON structure: {0}")]
//     ManifestParseError(reqwest::Error),
// }

pub(crate) struct MinecraftManager {
    app: AppRef,
}

impl MinecraftManager {
    pub fn new() -> Self {
        Self {
            app: AppRef::uninit(),
        }
    }

    pub fn get_appref(&self) -> &AppRef {
        &self.app
    }
}
