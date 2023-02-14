use super::{AppRef, Managers};
use crate::db::{minecraft_manifest::SetParam, PrismaClient};
use crate::managers::ManagersInner;
use carbon_domain::minecraft::manifest::{ManifestVersion, MinecraftManifest};
use std::sync::{Arc, Weak};
use thiserror::Error;

mod assets;
mod manifest;
mod version;

#[derive(Error, Debug)]
pub enum MinecraftError {
    #[error("Assets error")]
    AssetsError(assets::AssetsError),
}

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

    pub async fn get_minecraft_versions(&self) -> Vec<ManifestVersion> {
        manifest::get_meta(self.app.upgrade().persistence_manager.get_db_client().await)
            .await
            .unwrap()
    }

    pub async fn gather_download_files_list(
        &self,
        mc_version: String,
    ) -> Result<(), MinecraftError> {
        let db_client = self.app.upgrade().persistence_manager.get_db_client().await;

        let manifest = manifest::get_meta(db_client.clone()).await.unwrap();

        let manifest_version = manifest.iter().find(|v| v.id == mc_version).unwrap();

        let version = version::get_meta(db_client.clone(), manifest_version.clone())
            .await
            .unwrap();

        let assets = assets::get_meta(db_client.clone(), version.asset_index.unwrap())
            .await
            .unwrap();

        todo!()
    }
}
